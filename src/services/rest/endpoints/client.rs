use axum::{
    extract::{rejection::JsonRejection, Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Router,
};
use bson::doc;
use tracing::{error, info};

use crate::{
    database::{
        collections::client::{Client, CreateClient, DeviceType, UpdateClient},
        document::Document,
    },
    services::{
        rest::payloads::client::{CreateOrUpdateClientResponsePayload, GetClientReponsePayload},
        webtransport::{
            context::client::{ClientEvent, ClientEventType},
            messages::client::ClientCreatedOrUpdatedPayload,
        },
    },
    utils::check_request_body::check_request_body,
    AppState,
};

use super::super::payloads::client::CreateOrUpdateClientPayload;

pub fn get_routes() -> Router<AppState> {
    Router::new()
        .route("/client", post(create_or_update_client))
        .route("/client/:userId", get(get_client))
        .route("/client/:userId", delete(delete_client))
}

// Client services =================================================

async fn create_or_update_client(
    State(AppState {
        database_client,
        client_context,
        ..
    }): State<AppState>,
    payload: Result<Json<CreateOrUpdateClientPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => {
            return error_response;
        }
    };
    let query_doc = doc! {
        "userId": body.user_id.clone(),
    };
    let existing_client_result = Client::get_document(&database_client, query_doc.clone()).await;
    let existing_client_option = match existing_client_result {
        Ok(current_client_option) => current_client_option,
        Err(error_response) => {
            return error_response;
        }
    };
    match existing_client_option {
        Some(existing_client) => {
            if existing_client.client_id == body.client_id {
                return (
                    StatusCode::NO_CONTENT,
                    Json(CreateOrUpdateClientResponsePayload {
                        client_id: body.client_id.clone(),
                        device_type: body.device_type.clone(),
                        user_id: body.user_id.clone(),
                    }),
                )
                    .into_response();
            }
            let update_result = Client::update_document(
                &database_client,
                query_doc,
                UpdateClient {
                    client_id: Some(body.client_id.clone()),
                    device_type: Some(DeviceType::to_enum(body.device_type.clone())),
                },
            )
            .await;
            match update_result {
                Ok(result) => match result.modified_count {
                    0 => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Client couldn't be updated",
                    )
                        .into_response(),
                    _ => {
                        info!(
                            "Updated Client with User ID: {}",
                            existing_client.user_id.clone()
                        );
                        let mut sub_context = client_context.lock().await;
                        sub_context
                            .emit_client_event(
                                database_client.clone(),
                                existing_client.user_id.to_string(),
                                ClientEvent {
                                    event_type: ClientEventType::Changed,
                                    body: serde_json::to_string(&ClientCreatedOrUpdatedPayload {
                                        user_id: body.user_id.clone(),
                                        device_type: body.device_type.clone(),
                                        client_id: body.client_id.clone(),
                                    })
                                    .unwrap(),
                                },
                            )
                            .await;
                        drop(sub_context);
                        (
                            StatusCode::OK,
                            Json(CreateOrUpdateClientResponsePayload {
                                client_id: body.client_id.clone(),
                                device_type: body.device_type.clone(),
                                user_id: body.user_id.clone(),
                            }),
                        )
                            .into_response()
                    }
                },
                Err(error_response) => error_response,
            }
        }
        None => {
            let create_client_result = Client::create_document(
                &database_client,
                CreateClient {
                    client_id: body.client_id.clone(),
                    user_id: body.user_id.clone(),
                    device_type: DeviceType::to_enum(body.device_type.clone()),
                },
            )
            .await;
            match create_client_result {
                Ok(result) => {
                    let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
                    info!("Created new Client with ID: {}", inserted_id);
                    (
                        StatusCode::OK,
                        Json(CreateOrUpdateClientResponsePayload {
                            client_id: body.client_id.clone(),
                            user_id: body.user_id.clone(),
                            device_type: body.device_type.clone(),
                        }),
                    )
                        .into_response()
                }
                Err(error_response) => error_response,
            }
        }
    }
}

async fn get_client(
    Path(user_id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "userId": user_id.clone(),
    };
    let get_client_result = Client::get_document(&database_client, query_doc).await;
    match get_client_result {
        Ok(client_option) => match client_option {
            Some(client) => (
                StatusCode::OK,
                Json(GetClientReponsePayload {
                    user_id: client.user_id,
                    device_type: client.device_type.to_string(),
                    client_id: client.client_id,
                }),
            )
                .into_response(),
            None => {
                error!("User with ID {} is not logged in", user_id);
                (StatusCode::NOT_FOUND, "User is not logged in currently").into_response()
            }
        },
        Err(_) => {
            error!("Error during fetching of client with User ID {}", user_id);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error during client fetching",
            )
                .into_response()
        }
    }
}

async fn delete_client(
    Path(user_id): Path<String>,
    State(AppState {
        database_client,
        client_context,
        ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "userId": user_id.clone(),
    };
    let delete_client_result = Client::delete_document(&database_client, query_doc).await;
    match delete_client_result {
        Ok(result) => match result.deleted_count {
            0 => (
                StatusCode::NOT_FOUND,
                format!("No Client found with this User ID: {}", user_id,),
            )
                .into_response(),
            _ => {
                info!("Deleted {} Clients", result.deleted_count);
                let mut sub_context = client_context.lock().await;
                sub_context
                    .emit_client_event(
                        database_client.clone(),
                        user_id.to_string(),
                        ClientEvent {
                            event_type: ClientEventType::Deleted,
                            body: user_id.to_string(),
                        },
                    )
                    .await;
                drop(sub_context);
                (StatusCode::OK, Json("Deleted Client".to_string())).into_response()
            }
        },
        Err(error_response) => error_response,
    }
}
