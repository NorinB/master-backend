use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};

use crate::AppState;

pub fn get_routes() -> Router<AppState> {
    Router::new().route("/ping", get(ping))
}

pub async fn ping() -> Response {
    (StatusCode::OK, Json("Health Check OK")).into_response()
}
