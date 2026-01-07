use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use serde_json::json;

#[derive(Serialize)]
pub struct ErrorResponse {
    pub status: bool,
    pub error: String,
}
pub fn res_success<T: Serialize>(data: T) -> Response {
    let body = json!({
        "status": true,
        "data": data
    });
    (StatusCode::OK, Json(body)).into_response()
}

pub fn res_error<E: std::error::Error>(err: E) -> Response {
    let body = ErrorResponse {
        status: false,
        error: err.to_string(),
    };
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}

pub fn res_error_msg<T: Serialize>(err: T) -> Response {
    let body = json!({
        "status": false,
        "error": err
    });
    (StatusCode::UNPROCESSABLE_ENTITY, Json(body)).into_response()
}