use std::str::FromStr;

use axum::{
    extract::{rejection::JsonRejection, Json, Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use tracing::info;

use crate::{
    database::{
        collections::{
            active_member::{ActiveMember, CreateActiveMember, UpdateActiveMember},
            board::Board,
            element::{Element, UpdateElement},
        },
        document::Document,
    },
    services::webtransport::{
        context::active_member::{ActiveMemberEvent, ActiveMemberEventType},
        messages::active_member::{
            CreatedActiveMemberEventPayload, RemovedActiveMemberEventPayload,
            UpdatedPositionEventPayload,
        },
    },
    utils::check_request_body::check_request_body,
    AppState,
};

use super::super::payloads::active_member::{
    ChangeActiveBoardPayload, CreateActiveMemberPayload, UpdatePostionPayload,
};

pub fn get_routes() -> Router<AppState> {
    Router::new()
        .route("/active-member", post(create_active_member))
        .route("/active-member/:id", get(get_active_member))
        .route(
            "/active-member/board/:boardId",
            get(get_active_members_for_board),
        )
        .route(
            "/active-member/:id/board/:boardId",
            delete(delete_active_member),
        )
        .route("/active-member/board", put(change_active_board))
        .route("/active-member/position", put(update_position))
}

// Active Member services ======================================

async fn create_active_member(
    State(AppState {
        database_client,
        active_member_context,
        ..
    }): State<AppState>,
    payload: Result<Json<CreateActiveMemberPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => return error_response,
    };
    let is_part_of_board =
        match Board::get_existing_board(body.board_id.clone(), &database_client).await {
            Ok(board) => board.allowed_members.contains(&body.user_id),
            Err(error_response) => return error_response,
        };
    if !is_part_of_board {
        return (StatusCode::FORBIDDEN, "User is not part of this board").into_response();
    }
    let query_doc = doc! {
        "userId": body.user_id.clone(),
    };
    match ActiveMember::get_document(&database_client, query_doc).await {
        Ok(active_member_option) => {
            if active_member_option.is_some() {
                return (StatusCode::CONFLICT, "Active member already exists").into_response();
            }
        }
        Err(error_response) => return error_response,
    };
    let create_active_member_result = ActiveMember::create_document(
        &database_client,
        CreateActiveMember {
            user_id: body.user_id.clone(),
            board_id: body.board_id.clone(),
            x: 0.0,
            y: 0.0,
        },
    )
    .await;
    match create_active_member_result {
        Ok(result) => {
            let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
            info!("Created Active Member with ID: {}", inserted_id);
            let mut sub_context = active_member_context.lock().await;
            sub_context
                .emit_active_member_event(
                    body.board_id.clone(),
                    ActiveMemberEvent {
                        event_type: ActiveMemberEventType::Created,
                        body: serde_json::to_string(&CreatedActiveMemberEventPayload {
                            _id: inserted_id.clone(),
                            board_id: body.board_id.clone(),
                            user_id: body.user_id.clone(),
                        })
                        .unwrap(),
                    },
                )
                .await;
            drop(sub_context);
            (
                StatusCode::OK,
                Json(ActiveMember {
                    _id: inserted_id,
                    user_id: body.user_id.clone(),
                    board_id: body.board_id.clone(),
                    x: 0.0,
                    y: 0.0,
                }),
            )
                .into_response()
        }
        Err(error_response) => error_response,
    }
}

async fn get_active_member(
    Path(id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "_id": ObjectId::from_str(id.as_str()).unwrap(),
    };
    let get_active_member_result = ActiveMember::get_document(&database_client, query_doc).await;
    match get_active_member_result {
        Ok(active_member_option) => match active_member_option {
            Some(found_active_member) => {
                (StatusCode::OK, Json(found_active_member)).into_response()
            }
            None => (StatusCode::NOT_FOUND, "Active Member not found").into_response(),
        },
        Err(error_response) => error_response,
    }
}

async fn get_active_members_for_board(
    Path(board_id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "boardId": board_id.clone()
    };
    let active_member_result =
        ActiveMember::get_multiple_documents(&database_client, query_doc).await;
    match active_member_result {
        Ok(active_member_cursor) => {
            let retrieved_active_members = active_member_cursor
                .try_collect::<Vec<ActiveMember>>()
                .await;
            match retrieved_active_members {
                Ok(retrieved_active_members) => match retrieved_active_members.len() {
                    0 => (
                        StatusCode::NOT_FOUND,
                        "No Active Members are currently working on that board",
                    )
                        .into_response(),
                    _ => (StatusCode::OK, Json(retrieved_active_members)).into_response(),
                },
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Active Members could not be retrieved",
                )
                    .into_response(),
            }
        }
        Err(error_response) => error_response,
    }
}

async fn delete_active_member(
    Path((user_id, board_id)): Path<(String, String)>,
    State(AppState {
        database_client,
        active_member_context,
        ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
       "userId": user_id.clone(),
    };
    let delete_active_member_result =
        ActiveMember::delete_document(&database_client, query_doc).await;
    match delete_active_member_result {
        Ok(result) => {
            info!("Deleted {} Active Members", result.deleted_count);
            match result.deleted_count {
                0 => (StatusCode::NOT_FOUND, "No Active Member found to delete").into_response(),
                _ => {
                    let query_doc = doc! {
                        "lockedBy": user_id.clone(),
                    };
                    match Element::update_document(
                        &database_client,
                        query_doc,
                        UpdateElement {
                            x: None,
                            y: None,
                            scale_y: None,
                            scale_x: None,
                            text: None,
                            color: None,
                            z_index: None,
                            selected: None,
                            rotation: None,
                            locked_by: Some(None),
                        },
                    )
                    .await
                    {
                        Ok(_) => {}
                        Err(error_response) => return error_response,
                    };
                    let mut sub_context = active_member_context.lock().await;
                    sub_context
                        .emit_active_member_event(
                            board_id.clone(),
                            ActiveMemberEvent {
                                event_type: ActiveMemberEventType::Removed,
                                body: serde_json::to_string(&RemovedActiveMemberEventPayload {
                                    user_id: user_id.clone(),
                                })
                                .unwrap(),
                            },
                        )
                        .await;
                    drop(sub_context);
                    (StatusCode::OK, Json(format!("{}", result.deleted_count))).into_response()
                }
            }
        }
        Err(error_response) => error_response,
    }
}

async fn change_active_board(
    State(AppState {
        database_client,
        active_member_context,
        ..
    }): State<AppState>,
    payload: Result<Json<ChangeActiveBoardPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => return error_response,
    };
    let is_part_of_board =
        match Board::get_existing_board(body.new_board_id.clone(), &database_client).await {
            Ok(board) => board.allowed_members.contains(&body.user_id),
            Err(error_response) => return error_response,
        };
    if !is_part_of_board {
        return (StatusCode::FORBIDDEN, "User is not part of the new board").into_response();
    }
    let query_doc = doc! {
       "userId": body.user_id.clone(),
    };
    let mut current_active_member = match ActiveMember::get_existing_active_member_by_user_id(
        body.user_id.clone(),
        &database_client,
    )
    .await
    {
        Ok(active_member) => active_member,
        Err(error_response) => return error_response,
    };
    if current_active_member.board_id == body.new_board_id {
        return (StatusCode::OK, Json(current_active_member)).into_response();
    }
    let old_board_id = current_active_member.board_id.clone();
    let update_result = ActiveMember::update_document(
        &database_client,
        query_doc,
        UpdateActiveMember {
            board_id: Some(body.new_board_id.clone()),
            x: Some(0.0),
            y: Some(0.0),
        },
    )
    .await;
    match update_result {
        Ok(result) => match result.modified_count {
            0 => (StatusCode::NOT_FOUND, "No active member found to update").into_response(),
            _ => {
                info!(
                    "Updated Active Member with User ID: {}",
                    body.user_id.clone(),
                );
                let mut sub_context = active_member_context.lock().await;
                sub_context
                    .emit_active_member_event(
                        old_board_id,
                        ActiveMemberEvent {
                            event_type: ActiveMemberEventType::Removed,
                            body: serde_json::to_string(&RemovedActiveMemberEventPayload {
                                user_id: body.user_id.clone(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                sub_context
                    .emit_active_member_event(
                        body.new_board_id.clone(),
                        ActiveMemberEvent {
                            event_type: ActiveMemberEventType::Created,
                            body: serde_json::to_string(&CreatedActiveMemberEventPayload {
                                _id: current_active_member._id.clone(),
                                user_id: body.user_id.clone(),
                                board_id: body.new_board_id.clone(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
                current_active_member
                    .board_id
                    .clone_from(&body.new_board_id);
                (StatusCode::OK, Json(current_active_member)).into_response()
            }
        },
        Err(error_response) => error_response,
    }
}

async fn update_position(
    State(AppState {
        database_client,
        active_member_context,
        ..
    }): State<AppState>,
    payload: Result<Json<UpdatePostionPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => return error_response,
    };
    let query_doc = doc! {
        "userId": body.user_id.clone(),
    };
    let update_result = ActiveMember::update_document(
        &database_client,
        query_doc,
        UpdateActiveMember {
            x: Some(body.x),
            y: Some(body.y),
            board_id: None,
        },
    )
    .await;
    match update_result {
        Ok(result) => match result.modified_count {
            0 => (StatusCode::NOT_FOUND, "No active member found to update").into_response(),
            _ => {
                info!(
                    "Updated Active Member with User ID: {}",
                    body.user_id.clone(),
                );
                let mut sub_context = active_member_context.lock().await;
                sub_context
                    .emit_active_member_event(
                        body.board_id.clone(),
                        ActiveMemberEvent {
                            event_type: ActiveMemberEventType::PositionUpdated,
                            body: serde_json::to_string(&UpdatedPositionEventPayload {
                                user_id: body.user_id.clone(),
                                x: body.x,
                                y: body.y,
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
                (StatusCode::OK, Json(body.user_id.clone())).into_response()
            }
        },
        Err(error_response) => error_response,
    }
}
