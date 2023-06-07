use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub struct HTTPError {
    detail: String,
    status_code: StatusCode,
}

impl HTTPError {
    pub fn new(detail: &str) -> Self {
        Self {
            detail: detail.to_string(),
            status_code: StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    pub const fn with_status(mut self, status_code: StatusCode) -> Self {
        self.status_code = status_code;
        self
    }
}

impl IntoResponse for HTTPError {
    fn into_response(self) -> Response {
        (self.status_code, Json(json!({ "detail": self.detail }))).into_response()
    }
}
