use ntex::http::Method;
use ntex::service::{Middleware, Service, ServiceCtx};
use ntex::web::{Error, ErrorRenderer, WebRequest, WebResponse};
use ntex::{http, web};

use crate::errors::AppError;
use crate::handlers::Response;
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
        match req.head().method {
            // 1. Check the preflight request first, and set the CORS header correctly.
            // Note: preflight request is a request with the OPTIONS method
            Method::OPTIONS => {
                let res = ctx.call(&self.service, req).await?;
                return Ok(add_cors_header(res, "*"));
            }
            _ => {
                // skip the auth check, if the request is for the refresh_token endpoint
                if req.path() == "/api/v1/auth/refresh_token" {
                    log::info!("enter refresh_token endpoint");
                    let res = ctx.call(&self.service, req).await?;
                    return Ok(add_cors_header(res, "*"));
                }
                // 2. After the preflight request, we can get the AUTHORIZATION header from the standard request.
                if let Some(token) = req.headers().get(http::header::AUTHORIZATION) {
                    let token = token.to_str().unwrap().replace("Bearer ", "");
                    log::info!("token: {:?}", token);

                    // 3. Verify the token by checking the Redis server.
                    // Get a connection to the Redis server
                    let mut conn = repository::redis::new()
                        .ok()
                        .unwrap()
                        .clone()
                        .get_multiplexed_async_connection()
                        .await
                        .ok()
                        .unwrap();

                    // 4. Call the next service in the chain if the token exists in the Redis server and can be **decoded** to the user ID correctly.
                    if jwt::get_user_id_from_redis(&mut conn, jwt::TokenType::AccessToken, &token)
                        .await
                        .map_err(|e| {
                            log::error!("Error getting user_id from redis: {}", e);
                            AppError::Unauthorized
                        })
                        .ok()
                        .is_some()
                    {
                        //if get user_id, Call the next service in the chain
                        let res = ctx.call(&self.service, req).await?;
                        Ok(add_cors_header(res, "*"))
                    } else {
                        log::error!("Invalid token");
                        match req.path() {
                            // logout and refresh token should carry a access token even if it's expired
                            "/api/v1/auth/login"
                            | "/api/v1/auth/logout"
                            | "/api/v1/auth/refresh_token" => {
                                let res = ctx.call(&self.service, req).await?;
                                Ok(add_cors_header(res, "*"))
                            }
                            _ => {
                                let res = req.into_response(
                                    web::HttpResponse::Unauthorized().json(&Response::<()> {
                                        status: "fail".to_string(),
                                        message: "Invalid token".to_string(),
                                        count: None,
                                        data: None,
                                    }),
                                );
                                Ok(add_cors_header(res, "*"))
                            }
                        }
                    }
                // If no token is found, redirect to the login page
                } else {
                    log::error!("No token found");
                    // If no token is found, redirect to the login page
                    match req.path() {
                        "/api/v1/auth/login" | "/api/v1/auth/register" => {
                            let res = ctx.call(&self.service, req).await?;
                            Ok(add_cors_header(res, "*"))
                        }
                        _ => {
                            let res = req.into_response(web::HttpResponse::Unauthorized().json(
                                &Response::<()> {
                                    status: "fail".to_string(),
                                    message: "No token found".to_string(),
                                    count: None,
                                    data: None,
                                },
                            ));
                            Ok(add_cors_header(res, "*"))
                        }
                    }
                }
            }
        }
    }
}

// add access_control_allow_origin header
fn add_cors_header(mut res: WebResponse, origin: &'static str) -> WebResponse {
    res.headers_mut().insert(
        http::header::ACCESS_CONTROL_ALLOW_ORIGIN,
        http::header::HeaderValue::from_static(origin),
    );
    res
}
