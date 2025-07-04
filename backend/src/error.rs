use actix_web::{HttpResponse, ResponseError, http::StatusCode};
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub message: String,
    #[serde(skip_serializing)]
    pub status: StatusCode,
}

impl ApiError {
    pub fn new<M: Into<String>>(message: M, status: StatusCode) -> Self {
        Self { message: message.into(), status }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ApiError {}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status).json(serde_json::json!({ "error": self.message }))
    }
}
