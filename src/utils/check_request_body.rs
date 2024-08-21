use axum::{
    extract::{rejection::JsonRejection, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub fn check_request_body<T>(payload: Result<Json<T>, JsonRejection>) -> Result<Json<T>, Response> {
    match payload {
        Ok(success_body) => Ok(success_body),
        Err(JsonRejection::JsonDataError(_)) => Err((
            StatusCode::BAD_REQUEST,
            "Request Body has wrong fields".to_string(),
        )
            .into_response()),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "Request Body invalid".to_string(),
        )
            .into_response()),
    }
}
