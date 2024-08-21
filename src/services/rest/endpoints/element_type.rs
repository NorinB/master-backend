use std::str::FromStr;

use axum::{
    extract::{rejection::JsonRejection, Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use tracing::info;

use crate::{
    database::{
        collections::element_type::{CreateElementType, ElementType},
        document::Document,
    },
    utils::check_request_body::check_request_body,
    AppState,
};

use super::super::payloads::element_type::CreateElementTypePayload;

pub fn get_routes() -> Router<AppState> {
    Router::new()
        .route("/element-type", post(create_element_type))
        .route("/element-type/:id", get(get_element_type))
        .route("/element-types", get(get_all_element_types))
}

// Element type services ========================================

async fn create_element_type(
    State(AppState {
        database_client, ..
    }): State<AppState>,
    payload: Result<Json<CreateElementTypePayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => return error_response,
    };
    let create_element_type_result = ElementType::create_document(
        &database_client,
        CreateElementType {
            name: body.name.clone(),
            path: body.path.clone(),
        },
    )
    .await;
    match create_element_type_result {
        Ok(result) => {
            let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
            info!("Created Element Type with ID: {}", inserted_id);
            (StatusCode::OK, Json(inserted_id)).into_response()
        }
        Err(error_response) => error_response,
    }
}

async fn get_element_type(
    Path(id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "_id": ObjectId::from_str(id.as_str()).unwrap(),
    };
    let get_element_type_result = ElementType::get_document(&database_client, query_doc).await;
    match get_element_type_result {
        Ok(get_element_type_option) => match get_element_type_option {
            Some(element_type) => (StatusCode::OK, Json(element_type)).into_response(),
            None => (StatusCode::NOT_FOUND, "Element Type not found").into_response(),
        },
        Err(error_response) => error_response,
    }
}

async fn get_all_element_types(
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {};
    let element_types = match ElementType::get_multiple_documents(&database_client, query_doc).await
    {
        Ok(element_type_cursor) => {
            let retrieved_element_types =
                element_type_cursor.try_collect::<Vec<ElementType>>().await;
            match retrieved_element_types {
                Ok(retrieved_element_types) => retrieved_element_types,
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Found Element Types could not be retrieved",
                    )
                        .into_response()
                }
            }
        }
        Err(error_response) => return error_response,
    };
    match element_types.len() {
        0 => (StatusCode::NOT_FOUND, "No Element Types found").into_response(),
        _ => (StatusCode::OK, Json(element_types)).into_response(),
    }
}
