use ntex::http;
use ntex::web::{self, Error};
use serde::Serialize;

pub mod user;
#[derive(Serialize)]
pub struct Response<T> {
    pub status: String,
    pub message: String,
    pub count: Option<i64>,
    pub data: Option<T>,
}

/// health check
async fn health() -> Result<web::HttpResponse, Error> {
    Ok(web::HttpResponse::Ok().json(&Response::<()> {
        status: "success".to_string(),
        message: "Server is running...".to_string(),
        count: None,
        data: None,
    }))
}

// not found handler
async fn not_found_error() -> Result<web::HttpResponse, Error> {
    Ok(web::HttpResponse::NotFound().json(&Response::<()> {
        status: "error".to_string(),
        message: "Not Found".to_string(),
        count: None,
        data: None,
    }))
}
// A guard for checking if a user is authenticated
struct AuthorizationHeader;

impl web::guard::Guard for AuthorizationHeader {
    fn check(&self, req: &http::RequestHead) -> bool {
        req.headers().contains_key(http::header::AUTHORIZATION)
    }
}

/// configure routes
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .guard(web::guard::Header("content-type", "application/json"))
            .service((
                web::resource("/health").route(web::get().to(health)),
                web::resource("/users")
                    .guard(AuthorizationHeader)
                    .route(web::post().to(user::create_user))
                    .route(web::get().to(user::get_user_by_id_or_name::<user::Info>))
                    .route(web::put().to(user::update_user_by_id))
                    .route(web::delete().to(user::delete_user_by_id::<user::UserQuery>)),
                web::resource("/users/search")
                    .guard(AuthorizationHeader)
                    .route(web::post().to(user::search_users)),
                web::resource("/users/login").route(web::post().to(user::user_login)),
                // refresh token should carry a access token even if it's expired
                web::resource("/users/refresh")
                    .guard(AuthorizationHeader)
                    .route(web::post().to(user::refresh_token)),
            ))
            .default_service(web::route().to(not_found_error)),
    );
}
