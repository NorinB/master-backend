use std::str::FromStr;

use axum::{
    extract::{rejection::JsonRejection, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use tracing::{error, info};

use crate::{
    database::{
        collections::{
            board::{Board, CreateBoard, UpdateBoard},
            element::Element,
        },
        document::Document,
    },
    services::webtransport::{
        context::board::{BoardEvent, BoardEventType},
        messages::board::{MemberAddedEventPayload, MemberRemovedEventPayload},
    },
    utils::check_request_body::check_request_body,
    AppState,
};

use super::super::payloads::board::CreateBoardRequestPayload;

pub fn get_routes() -> Router<AppState> {
    Router::new()
        .route("/board/:id", get(get_board))
        .route("/board/:id/elements", get(get_all_elements_of_board))
        .route("/board", post(create_board))
        .route("/board/:boardId/allowed-member/:userId", put(add_member))
        .route(
            "/board/:boardId/allowed-member/:userId",
            delete(remove_member),
        )
        .route("/boards/:userId", get(get_all_boards_with_user))
}

// Board services ============================================

async fn create_board(
    State(AppState {
        database_client, ..
    }): State<AppState>,
    payload: Result<Json<CreateBoardRequestPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(err_response) => return err_response,
    };
    let create_board_result = Board::create_document(
        &database_client,
        CreateBoard {
            name: body.name.to_string(),
            host: body.host.to_string(),
            allowed_members: vec![body.host.to_string()],
        },
    )
    .await;
    match create_board_result {
        Ok(result) => {
            let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
            info!("Created Board with ID: {}", inserted_id);
            (StatusCode::OK, Json(inserted_id)).into_response()
        }
        Err(error_response) => error_response,
    }
}

async fn get_board(
    Path(board_id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "_id": ObjectId::from_str(board_id.clone().as_str()).unwrap()
    };
    let found_board_result = Board::get_document(&database_client, query_doc).await;
    match found_board_result {
        Ok(result) => match result {
            Some(board) => {
                info!("Board with ID {} fetched", board._id.clone());
                (StatusCode::OK, Json(board)).into_response()
            }
            None => {
                error!("No Board found with ID: {}", board_id.clone());
                (StatusCode::NOT_FOUND, "Board not found").into_response()
            }
        },
        Err(error_response) => error_response,
    }
}

async fn add_member(
    Path((board_id, user_id)): Path<(String, String)>,
    State(AppState {
        database_client,
        board_context,
        ..
    }): State<AppState>,
) -> Response {
    let board = match Board::get_existing_board(board_id.clone(), &database_client).await {
        Ok(board) => board,
        Err(error_response) => {
            return error_response;
        }
    };
    match board.allowed_members.contains(&user_id) {
        true => {
            return (StatusCode::CONFLICT, "Member already part of this board").into_response();
        }
        false => {}
    }
    let mut current_allowed_members = board.allowed_members;
    current_allowed_members.push(user_id.clone());
    let query_doc = doc! {
        "_id": ObjectId::from_str(board_id.as_str()).unwrap(),
    };
    let result = Board::update_document(
        &database_client,
        query_doc,
        UpdateBoard {
            name: None,
            host: None,
            allowed_members: Some(current_allowed_members),
        },
    )
    .await;
    match result {
        Ok(result) => match result.modified_count {
            0 => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Allowed Member has not been added",
            )
                .into_response(),
            _ => {
                let mut sub_context = board_context.lock().await;
                sub_context
                    .emit_board_event(
                        database_client.clone(),
                        board._id,
                        BoardEvent {
                            event_type: BoardEventType::MemberAdded,
                            body: serde_json::to_string(&MemberAddedEventPayload {
                                user_id: user_id.to_string(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
                (StatusCode::OK, Json(user_id.clone())).into_response()
            }
        },
        Err(error_response) => error_response,
    }
}

async fn remove_member(
    Path((board_id, user_id)): Path<(String, String)>,
    State(AppState {
        database_client,
        board_context,
        ..
    }): State<AppState>,
) -> Response {
    let board = match Board::get_existing_board(board_id.clone(), &database_client).await {
        Ok(board) => board,
        Err(error_response) => {
            return error_response;
        }
    };
    match board.allowed_members.contains(&user_id) {
        false => {
            return (StatusCode::CONFLICT, "Member not part of this board").into_response();
        }
        true => {}
    };
    let mut current_allowed_members = board.allowed_members;
    let member_position = current_allowed_members
        .iter()
        .position(|member_id| *member_id == user_id.clone())
        .unwrap();
    current_allowed_members.remove(member_position);
    let update_board = UpdateBoard {
        name: None,
        host: None,
        allowed_members: Some(current_allowed_members),
    };
    let query_doc = doc! {
        "_id": ObjectId::from_str(board_id.as_str()).unwrap(),
    };
    let update_result = Board::update_document(&database_client, query_doc, update_board).await;
    match update_result {
        Ok(result) => match result.modified_count {
            0 => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Allowed Member has not been updated",
            )
                .into_response(),
            _ => {
                let mut sub_context = board_context.lock().await;
                sub_context
                    .emit_board_event(
                        database_client.clone(),
                        board._id,
                        BoardEvent {
                            event_type: BoardEventType::MemberRemoved,
                            body: serde_json::to_string(&MemberRemovedEventPayload {
                                user_id: user_id.to_string(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
                (StatusCode::OK, Json(user_id.clone())).into_response()
            }
        },
        Err(error_response) => error_response,
    }
}

async fn get_all_boards_with_user(
    Path(user_id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "allowedMembers": doc!{ "$in": vec![user_id] }
    };
    let get_boards_result = Board::get_multiple_documents(&database_client, query_doc).await;
    match get_boards_result {
        Ok(board_cursor) => {
            let all_boards = board_cursor.try_collect().await.unwrap_or_else(|_| vec![]);
            match all_boards.len() {
                0 => (StatusCode::NOT_FOUND, "User is not part of any board").into_response(),
                _ => (StatusCode::OK, Json(all_boards)).into_response(),
            }
        }
        Err(error_response) => error_response,
    }
}

async fn get_all_elements_of_board(
    Path(board_id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "boardId": board_id.clone()
    };
    let get_elements_result = Element::get_multiple_documents(&database_client, query_doc).await;
    match get_elements_result {
        Ok(element_cursor) => {
            let retrieved_elements = element_cursor.try_collect::<Vec<Element>>().await;
            match retrieved_elements {
                Ok(retrieved_elements) => match retrieved_elements.len() {
                    0 => (StatusCode::NOT_FOUND, "Board has no Elements currently").into_response(),
                    _ => (StatusCode::OK, Json(retrieved_elements)).into_response(),
                },
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Found Elements could not be retrieved",
                )
                    .into_response(),
            }
        }
        Err(error_response) => error_response,
    }
}
