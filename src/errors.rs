use crate::handlers::Response;
use derive_more::Display; // naming it clearly for illustration purposes
use ntex::web::{HttpRequest, HttpResponse, WebResponseError};

#[derive(Debug, Display)]
pub enum AppError {
    #[display(fmt = "Internal Server Error")]
    InternalServerError(String),
    #[display(fmt = "Bad Request: {}", _0)]
    BadRequest(String),
    #[display(fmt = "Unauthorized")]
    Unauthorized,
    #[display(fmt = "Not Found")]
    NotFound,
    #[display(fmt = "Conflict")]
    Conflict,
    #[display(fmt = "Service Unavailable")]
    ServiceUnavailable,
    #[display(fmt = "User Already Exists")]
    UserAlreadyExists(String),
}

// Implement the `std::error::Error` trait for `AppError`
impl std::error::Error for AppError {}

/// Ntex uses `ResponseError` for conversion of errors to a response
impl WebResponseError for AppError {
    fn error_response(&self, _: &HttpRequest) -> HttpResponse {
        match self {
            AppError::InternalServerError(ref message) => {
                HttpResponse::InternalServerError().json(&Response::<()> {
                    status: "failed".to_string(),
                    message: message.clone().to_string(),
                    count: None,
                    data: None,
                })
            }
            AppError::BadRequest(ref message) => HttpResponse::BadRequest().json(&Response::<()> {
                status: "failed".to_string(),
                message: message.clone().to_string(),
                count: None,
                data: None,
            }),
            AppError::Unauthorized => HttpResponse::Unauthorized().json(&Response::<()> {
                status: "failed".to_string(),
                message: "User Unauthorized".to_string(),
                count: None,
                data: None,
            }),
            AppError::NotFound => HttpResponse::NotFound().json(&Response::<()> {
                status: "failed".to_string(),
                message: "User Not Found".to_string(),
                count: None,
                data: None,
            }),
            AppError::Conflict => HttpResponse::Conflict().finish(),
            AppError::ServiceUnavailable => {
                HttpResponse::ServiceUnavailable().json(&Response::<()> {
                    status: "failed".to_string(),
                    message: "Internal Server Error".to_string(),
                    count: None,
                    data: None,
                })
            }

            AppError::UserAlreadyExists(ref message) => {
                HttpResponse::BadRequest().json(&Response::<()> {
                    status: "failed".to_string(),
                    message: message.clone().to_string(),
                    count: None,
                    data: None,
                })
            }
        }
    }
}
