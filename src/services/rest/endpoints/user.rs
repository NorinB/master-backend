use futures::TryStreamExt;
use std::{collections::HashMap, str::FromStr};
use tracing::info;

use axum::{
    extract::{rejection::JsonRejection, Json, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, Router},
};
use bson::{doc, oid::ObjectId};

use crate::{
    database::{
        collections::{
            client::{Client, CreateClient, DeviceType},
            user::{CreateUser, User},
        },
        document::Document,
    },
    services::{
        rest::payloads::user::{
            CreateUserResponsePayload, LoginUserPayload, LoginUserResponsePayload,
        },
        webtransport::{
            context::client::{ClientEvent, ClientEventType},
            messages::client::ClientCreatedOrUpdatedPayload,
        },
    },
    utils::check_request_body::check_request_body,
    AppState,
};

use super::super::payloads::user::CreateUserPayload;

pub fn get_routes() -> Router<AppState> {
    Router::new()
        .route("/user/:id", get(get_user))
        .route("/register", post(create_user))
        .route("/user", get(get_user_by_email_or_name))
        .route("/login", post(login))
        .route("/logout/:userId", delete(logout))
}

// User services ===============================================

async fn create_user(
    State(AppState {
        database_client, ..
    }): State<AppState>,
    payload: Result<Json<CreateUserPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => return error_response,
    };
    if body.name.is_empty() {
        return (StatusCode::BAD_REQUEST, "Name must be set").into_response();
    }
    if body.email.is_empty() {
        return (StatusCode::BAD_REQUEST, "E-Mail must be set").into_response();
    } else if !body.email.contains('@') {
        return (StatusCode::BAD_REQUEST, "E-Mail is invalid").into_response();
    }
    if body.password.is_empty() {
        return (StatusCode::BAD_REQUEST, "Password must be set").into_response();
    }
    if body.name.contains('@') {
        return (StatusCode::BAD_REQUEST, "Username cannot contain '@'").into_response();
    }
    let query_doc = doc! {
        "email": body.email.clone()
    };
    let existing_user = User::get_document(&database_client, query_doc).await;
    match existing_user {
        Ok(user_option) => {
            if user_option.is_some() {
                return (StatusCode::CONFLICT, "User already exists").into_response();
            }
        }
        Err(error_response) => {
            return error_response;
        }
    }
    let created_user = CreateUser {
        _id: ObjectId::new(),
        name: body.name.to_string(),
        email: body.email.to_string(),
        password: body.password.to_string(),
    };
    let create_user_result = User::create_document(&database_client, created_user.clone()).await;
    match create_user_result {
        Ok(result) => {
            let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
            info!("Created user with ID: {}", inserted_id);
            (
                StatusCode::OK,
                Json(CreateUserResponsePayload {
                    id: created_user._id.to_hex(),
                    email: created_user.email,
                    name: created_user.name,
                }),
            )
                .into_response()
        }
        Err(error_response) => error_response,
    }
}

async fn get_user(
    Path(user_id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "_id": ObjectId::from_str(user_id.clone().as_str()).unwrap()
    };
    let found_user_result = User::get_document(&database_client, query_doc).await;
    match found_user_result {
        Ok(result) => match result {
            Some(user) => {
                info!("User: {}", user._id.clone());
                (StatusCode::OK, Json(user)).into_response()
            }
            None => {
                info!("No User found with ID: {}", user_id.clone());
                (StatusCode::NOT_FOUND, "User not found").into_response()
            }
        },
        Err(error_response) => error_response,
    }
}

async fn get_user_by_email_or_name(
    Query(query_params): Query<HashMap<String, String>>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let mut search_by_name = false;
    if query_params.contains_key("name") {
        search_by_name = true;
    }
    if !search_by_name && query_params.contains_key("email") {
        return (
            StatusCode::BAD_REQUEST,
            "Query param \"email\" needed at least",
        )
            .into_response();
    }
    if search_by_name {
        let query_doc = doc! {
            "name": query_params.get("name").unwrap().clone()
        };
        let found_users_result = User::get_multiple_documents(&database_client, query_doc).await;
        match found_users_result {
            Ok(found_users_cursor) => {
                let all_found_users = found_users_cursor
                    .try_collect()
                    .await
                    .unwrap_or_else(|_| vec![]);
                match all_found_users.len() {
                    0 => (StatusCode::NOT_FOUND, "No user found with that name").into_response(),
                    _ => (StatusCode::OK, Json(all_found_users)).into_response(),
                }
            }
            Err(error_response) => error_response,
        }
    } else {
        let query_doc = doc! {
            "email": query_params.get("email").unwrap().clone()
        };
        let found_user = User::get_document(&database_client, query_doc).await;
        match found_user {
            Ok(user_option) => match user_option {
                Some(existing_user) => (StatusCode::OK, Json(existing_user)).into_response(),
                None => (StatusCode::NOT_FOUND, "No User found with that email").into_response(),
            },
            Err(error_response) => error_response,
        }
    }
}

async fn login(
    State(AppState {
        database_client,
        client_context,
        ..
    }): State<AppState>,
    payload: Result<Json<LoginUserPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => return error_response,
    };
    if body.name.is_none() && body.email.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "Email or Name needs to be provided to login",
        )
            .into_response();
    }
    let device_type = DeviceType::to_enum(body.device_type.clone());
    let query_doc = match body.name.clone() {
        Some(name) => doc! {
            "name": name,
        },
        None => doc! {
            "email": body.email.clone(),
        },
    };
    let existing_user = User::get_document(&database_client, query_doc).await;
    let user = match existing_user {
        Ok(user_option) => match user_option {
            Some(user) => match user.password == body.password {
                false => {
                    return (
                        StatusCode::UNAUTHORIZED,
                        "User password combination does not match",
                    )
                        .into_response()
                }
                true => user,
            },
            None => {
                return (StatusCode::NOT_FOUND, "User not found").into_response();
            }
        },
        Err(error_response) => return error_response,
    };
    let query_doc = doc! {
        "userId": user._id.clone(),
    };
    match Client::delete_document(&database_client, query_doc.clone()).await {
        Ok(_) => match Client::create_document(
            &database_client,
            CreateClient {
                client_id: body.client_id.clone(),
                user_id: user._id.clone(),
                device_type,
            },
        )
        .await
        {
            Ok(_) => {
                info!("Updated Client with User ID: {}", user._id.clone());
                let mut sub_context = client_context.lock().await;
                sub_context
                    .emit_client_event(
                        database_client.clone(),
                        user._id.to_string(),
                        ClientEvent {
                            event_type: ClientEventType::Changed,
                            body: serde_json::to_string(&ClientCreatedOrUpdatedPayload {
                                user_id: user._id.clone(),
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
                    Json(LoginUserResponsePayload {
                        user_id: user._id,
                        name: user.name,
                        email: user.email,
                    }),
                )
                    .into_response()
            }
            Err(error_response) => error_response,
        },
        Err(error_response) => error_response,
    }
}

async fn logout(
    Path(user_id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "userId": user_id.clone(),
    };
    match Client::delete_document(&database_client, query_doc).await {
        Ok(delete_result) => match delete_result.deleted_count {
            0 => (StatusCode::INTERNAL_SERVER_ERROR, "User not logged in").into_response(),
            _ => (StatusCode::OK, Json(user_id.clone())).into_response(),
        },
        Err(error_response) => error_response,
    }
}
