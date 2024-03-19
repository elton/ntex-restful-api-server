use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web::{Error, ErrorRenderer, WebRequest, WebResponse};
use ntex::{http, web};

use crate::errors::AppError;
use crate::repository;
use crate::utils::jwt;

pub struct Auth;

impl<S> Middleware<S> for Auth {
    type Service = AuthMiddleware<S>;

    fn create(&self, service: S) -> Self::Service {
        AuthMiddleware { service }
    }
}

pub struct AuthMiddleware<S> {
    // This is special: We need this to avoid lifetime issues.
    service: S,
}

impl<S, Err> Service<WebRequest<Err>> for AuthMiddleware<S>
where
    S: Service<WebRequest<Err>, Response = WebResponse, Error = Error> + 'static,
    Err: ErrorRenderer + 'static,
{
    type Response = WebResponse;
    type Error = Error;

    ntex::forward_poll_ready!(service);
    ntex::forward_poll_shutdown!(service);

    async fn call(
        &self,
        req: WebRequest<Err>,
        ctx: ServiceCtx<'_, Self>,
    ) -> Result<Self::Response, Self::Error> {
        log::info!("path: {}", req.path());
        // Get the token from the request headers
        if let Some(token) = req.headers().get(http::header::AUTHORIZATION) {
            let token = token.to_str().unwrap().replace("Bearer ", "");
            log::info!("token: {:?}", token);

            // Verify the token
            // Get a connection to the Redis server
            let mut conn = repository::redis::new()
                .ok()
                .unwrap()
                .clone()
                .get_multiplexed_async_connection()
                .await
                .ok()
                .unwrap();

            // Get the user_id from the token
            if let Some(user_id) =
                jwt::get_user_id_from_redis(&mut conn, jwt::TokenType::AccessToken, &token)
                    .await
                    .map_err(|e| {
                        log::error!("Error getting user_id from redis: {}", e);
                        AppError::InternalServerError(e.to_string())
                    })
                    .ok()
            {
                log::info!("user_id: {:?}", user_id);
                //if get user_id, Call the next service in the chain
                let res = ctx.call(&self.service, req).await?;
                Ok(res)
            } else {
                log::info!("Invalid token");
                // If no token is found, redirect to the login page
                if req.path() == "/api/v1/users/login" {
                    ctx.call(&self.service, req).await
                } else {
                    Ok(req.into_response(redirect_to_login()))
                }
            }
        // If no token is found, redirect to the login page
        } else {
            log::error!("No token found");
            // If no token is found, redirect to the login page
            if req.path() == "/api/v1/users/login" {
                ctx.call(&self.service, req).await
            } else {
                Ok(req.into_response(redirect_to_login()))
            }
        }
    }
}

fn redirect_to_login() -> web::HttpResponse {
    web::HttpResponse::Found()
        .header(
            http::header::LOCATION,
            "http://localhost:4321/api/v1/users/login",
        )
        .finish()
}
