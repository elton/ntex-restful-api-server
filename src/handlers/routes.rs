use ntex::web::{self, Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Response<T> {
    status: String,
    message: String,
    data: Option<T>,
}

/// health check
#[web::get("/health")]
async fn health() -> Result<web::HttpResponse, Error> {
    Ok(web::HttpResponse::Ok().json(&Response::<()> {
        status: "success".to_string(),
        message: "Server is running".to_string(),
        data: None,
    }))
}

/// configure routes
pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/api/v1").service(health));
}
