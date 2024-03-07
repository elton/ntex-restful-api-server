use ntex::web::{self, Error};
use serde::Serialize;

pub mod user;
#[derive(Serialize)]
pub struct Response<T> {
    pub status: String,
    pub message: String,
    pub data: Option<T>,
}

/// health check
#[web::get("/health")]
async fn health() -> Result<web::HttpResponse, Error> {
    Ok(web::HttpResponse::Ok().json(&Response::<()> {
        status: "success".to_string(),
        message: "Server is running...".to_string(),
        data: None,
    }))
}

// not found handler
async fn not_found_error() -> Result<web::HttpResponse, Error> {
    Ok(web::HttpResponse::NotFound().json(&Response::<()> {
        status: "error".to_string(),
        message: "Not Found".to_string(),
        data: None,
    }))
}

/// configure routes
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .service(health)
            .service(user::create_user)
            .service(user::get_user_by_id)
            .service(user::get_all_users)
            .service(user::update_user_by_id)
            .service(user::get_users_by_name)
            .service(user::delete_user_by_id)
            .service(user::search_users)
            .default_service(web::route().to(not_found_error)),
    );
}
