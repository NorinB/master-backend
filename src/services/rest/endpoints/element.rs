use std::str::FromStr;

use axum::{
    extract::{rejection::JsonRejection, Json, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Router,
};
use bson::{doc, oid::ObjectId};
use futures::TryStreamExt;
use mongodb::results::UpdateResult;
use serde::Deserialize;
use tracing::info;

use crate::{
    database::{
        collections::element::{CreateElement, Element, UpdateElement},
        document::Document,
    },
    services::webtransport::{
        context::element::{ElementEvent, ElementEventType},
        messages::element::{
            ElementCreatedEventPayload, ElementLockedEventPayload, ElementMovedEventPayload,
            ElementRemovedEventPayload, ElementUnlockedEventPayload, UpdatedElementEventPayload,
        },
    },
    utils::check_request_body::check_request_body,
    AppState,
};

use super::super::payloads::element::{
    CreateElementPayload, LockElementPayload, LockMultipleElementsPayload,
    MoveMultipleElementsPayload, UnlockElementPayload, UnlockMultipleElementsPayload,
    UpdateElementPayload,
};

pub fn get_routes() -> Router<AppState> {
    Router::new()
        .route("/element/single", post(create_element))
        .route("/element/single/:id", get(get_element))
        .route("/element/single", put(update_element))
        .route(
            "/element/single/:userId/:boardId/:elementId",
            delete(delete_element),
        )
        .route("/element/single/lock", put(lock_element))
        .route("/element/single/unlock", put(unlock_element))
        .route("/element/multiple/unlock-all", put(unlock_all_for_user))
        .route("/element/multiple/move", put(move_multiple_elements))
        .route("/element/multiple/lock", put(lock_multiple_elements))
        .route("/element/multiple/unlock", put(unlock_multiple_elements))
}

// Element services ==============================================

async fn create_element(
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
    payload: Result<Json<CreateElementPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => return error_response,
    };
    let create_element = CreateElement {
        _id: body._id.clone(),
        board_id: body.board_id.clone(),
        selected: body.selected,
        locked_by: body.locked_by.clone(),
        rotation: body.rotation,
        scale_x: body.scale_x,
        scale_y: body.scale_y,
        z_index: body.z_index,
        x: body.x,
        y: body.y,
        element_type: body.element_type.clone(),
        text: body.text.clone(),
        created_at: body.created_at,
        color: body.color.clone(),
    };
    let create_element_result =
        Element::create_document(&database_client, create_element.clone()).await;
    match create_element_result {
        Ok(result) => {
            let inserted_id = result.inserted_id.as_object_id().unwrap().to_hex();
            info!("Created Element with ID: {}", inserted_id);
            let mut sub_context = element_context.lock().await;
            sub_context
                .emit_element_event(
                    body.board_id.clone(),
                    ElementEvent {
                        event_type: ElementEventType::Created,
                        body: serde_json::to_string(&ElementCreatedEventPayload {
                            _id: inserted_id.clone(),
                            user_id: body.user_id.clone(),
                            board_id: create_element.board_id,
                            x: create_element.x,
                            y: create_element.y,
                            text: create_element.text,
                            scale_x: create_element.scale_x,
                            scale_y: create_element.scale_y,
                            z_index: create_element.z_index,
                            selected: create_element.selected,
                            created_at: create_element.created_at,
                            rotation: create_element.rotation,
                            locked_by: create_element.locked_by,
                            element_type: create_element.element_type,
                            color: create_element.color,
                        })
                        .unwrap(),
                    },
                )
                .await;
            drop(sub_context);
            (StatusCode::OK, Json(inserted_id)).into_response()
        }
        Err(error_response) => error_response,
    }
}

async fn get_element(
    Path(id): Path<String>,
    State(AppState {
        database_client, ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "_id": ObjectId::from_str(id.as_str()).unwrap(),
    };
    let get_element_result = Element::get_document(&database_client, query_doc).await;
    match get_element_result {
        Ok(element_option) => match element_option {
            Some(element) => (StatusCode::OK, Json(element)).into_response(),
            None => (StatusCode::NOT_FOUND, "Element not found").into_response(),
        },
        Err(error_response) => error_response,
    }
}

async fn delete_element(
    Path((user_id, board_id, element_id)): Path<(String, String, String)>,
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "_id": ObjectId::from_str(element_id.clone().as_str()).unwrap(),
    };
    let delete_element_result = Element::delete_document(&database_client, query_doc).await;
    match delete_element_result {
        Ok(result) => {
            info!("Deleted {} Elements", result.deleted_count);
            match result.deleted_count {
                0 => (StatusCode::NOT_FOUND, "No Element found to delete").into_response(),
                _ => {
                    let mut sub_context = element_context.lock().await;
                    sub_context
                        .emit_element_event(
                            board_id,
                            ElementEvent {
                                event_type: ElementEventType::Removed,
                                body: serde_json::to_string(&ElementRemovedEventPayload {
                                    _id: element_id.to_string(),
                                    user_id,
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

async fn lock_element(
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
    payload: Result<Json<LockElementPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => {
            return error_response;
        }
    };
    let query_doc = doc! {
        "_id": ObjectId::from_str(body._id.as_str()).unwrap()
    };
    let found_element_result = Element::get_document(&database_client, query_doc.clone()).await;
    match found_element_result {
        Ok(element) => match element {
            Some(element) => {
                if let Some(locked_by) = element.locked_by {
                    if locked_by != body.user_id {
                        return (StatusCode::LOCKED, "Element already locked by someone else")
                            .into_response();
                    } else {
                        return (StatusCode::NO_CONTENT, "Element already locked by yourself")
                            .into_response();
                    }
                }
            }
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    format!("No Element found with ID: {}", body._id),
                )
                    .into_response()
            }
        },
        Err(error_response) => {
            return error_response;
        }
    };
    let update_result = Element::update_document(
        &database_client,
        query_doc,
        UpdateElement {
            selected: None,
            locked_by: Some(Some(body.user_id.clone())),
            x: None,
            y: None,
            rotation: None,
            scale_x: None,
            scale_y: None,
            z_index: None,
            text: None,
            color: None,
        },
    )
    .await;
    match update_result {
        Ok(result) => match result.modified_count {
            0 => (StatusCode::NOT_FOUND, "No Element found to update").into_response(),
            _ => {
                info!("Updated Element with ID: {}", body.user_id.clone());
                let mut sub_context = element_context.lock().await;
                sub_context
                    .emit_element_event(
                        body.board_id.clone(),
                        ElementEvent {
                            event_type: ElementEventType::Locked,
                            body: serde_json::to_string(&ElementLockedEventPayload {
                                _id: body._id.clone(),
                                user_id: body.user_id.clone(),
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

async fn unlock_element(
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
    payload: Result<Json<UnlockElementPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => {
            return error_response;
        }
    };
    let query_doc = doc! {
        "_id": ObjectId::from_str(body._id.as_str()).unwrap()
    };
    let found_element_result = Element::get_document(&database_client, query_doc.clone()).await;
    match found_element_result {
        Ok(element) => match element {
            Some(element) => match element.locked_by {
                Some(locked_by) => {
                    if locked_by != body.user_id {
                        return (
                            StatusCode::LOCKED,
                            "Element currently locked by someone else",
                        )
                            .into_response();
                    }
                }
                None => {
                    return (StatusCode::NO_CONTENT, "Element already unlocked").into_response()
                }
            },
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    format!("No Element found with ID: {}", body._id),
                )
                    .into_response()
            }
        },
        Err(error_response) => {
            return error_response;
        }
    };
    let update_result = Element::update_document(
        &database_client,
        query_doc,
        UpdateElement {
            selected: None,
            locked_by: Some(None),
            x: None,
            y: None,
            rotation: None,
            scale_x: None,
            scale_y: None,
            z_index: None,
            text: None,
            color: None,
        },
    )
    .await;
    match update_result {
        Ok(result) => match result.modified_count {
            0 => (StatusCode::NOT_FOUND, "No Element found to update").into_response(),
            _ => {
                info!("Updated Element with ID: {}", body.user_id.clone(),);
                let mut sub_context = element_context.lock().await;
                sub_context
                    .emit_element_event(
                        body.board_id.clone(),
                        ElementEvent {
                            event_type: ElementEventType::Unlocked,
                            body: serde_json::to_string(&ElementUnlockedEventPayload {
                                _id: body._id.clone(),
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

async fn lock_multiple_elements(
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
    payload: Result<Json<LockMultipleElementsPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => {
            return error_response;
        }
    };
    let query_doc = doc! {
        "_id": doc! { "$in": body.ids.iter().map(|id| ObjectId::from_str(id.as_str()).unwrap()).collect::<Vec<ObjectId>>() }
    };
    let found_element_result =
        Element::get_multiple_documents(&database_client, query_doc.clone()).await;
    let found_elements = match found_element_result {
        Ok(element_cursor) => {
            let retrieved_elements = element_cursor.try_collect::<Vec<Element>>().await;
            match retrieved_elements {
                Ok(retrieved_elements) => match retrieved_elements.len() {
                    0 => return (StatusCode::NOT_FOUND, "No Elements found").into_response(),
                    _ => retrieved_elements,
                },
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Found Elements could not be retrieved",
                    )
                        .into_response();
                }
            }
        }
        Err(error_response) => {
            return error_response;
        }
    };
    if found_elements
        .iter()
        .any(|element| match &element.locked_by {
            Some(locked_by) => *locked_by != body.user_id,
            None => false,
        })
    {
        return (StatusCode::LOCKED, "Some Element is locked by another user").into_response();
    }
    let mut updated_document_results: Vec<UpdateResult> = vec![];
    for element in found_elements.iter() {
        let query_doc = doc! {
            "_id": ObjectId::from_str(element._id.as_str()).unwrap(),
        };
        match Element::update_document(
            &database_client,
            query_doc,
            UpdateElement {
                selected: None,
                locked_by: Some(Some(body.user_id.clone())),
                x: None,
                y: None,
                rotation: None,
                scale_x: None,
                scale_y: None,
                z_index: None,
                text: None,
                color: None,
            },
        )
        .await
        {
            Ok(update_result) => match update_result.modified_count {
                0 => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Lock of Element with ID {} failed", element._id),
                    )
                        .into_response()
                }
                _ => {
                    updated_document_results.push(update_result);
                }
            },
            Err(error_response) => return error_response,
        }
    }
    match updated_document_results.len() {
        0 => (StatusCode::NOT_FOUND, "No Element found to update").into_response(),
        number => {
            info!("Updateded {} Elements", number);
            for element_id in body.ids.iter() {
                let mut sub_context = element_context.lock().await;
                sub_context
                    .emit_element_event(
                        body.board_id.to_string(),
                        ElementEvent {
                            event_type: ElementEventType::Locked,
                            body: serde_json::to_string(&ElementLockedEventPayload {
                                _id: element_id.clone(),
                                user_id: body.user_id.clone(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
            }
            (StatusCode::OK, Json(format!("{}", number))).into_response()
        }
    }
}

async fn unlock_multiple_elements(
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
    payload: Result<Json<UnlockMultipleElementsPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => {
            return error_response;
        }
    };
    let query_doc = doc! {
        "_id": doc! { "$in": body.ids.iter().map(|id| ObjectId::from_str(id.as_str()).unwrap()).collect::<Vec<ObjectId>>() }
    };
    let found_element_result =
        Element::get_multiple_documents(&database_client, query_doc.clone()).await;
    let found_elements = match found_element_result {
        Ok(element_cursor) => {
            let retrieved_elements = element_cursor.try_collect::<Vec<Element>>().await;
            match retrieved_elements {
                Ok(retrieved_elements) => match retrieved_elements.len() {
                    0 => return (StatusCode::NOT_FOUND, "No Elements found").into_response(),
                    _ => retrieved_elements,
                },
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Found Elements could not be retrieved",
                    )
                        .into_response();
                }
            }
        }
        Err(error_response) => {
            return error_response;
        }
    };
    if found_elements
        .iter()
        .any(|element| match &element.locked_by {
            Some(locked_by) => *locked_by != body.user_id,
            None => false,
        })
    {
        return (StatusCode::LOCKED, "Some Element is locked by another user").into_response();
    }
    let mut updated_document_results: Vec<UpdateResult> = vec![];
    for element in found_elements.iter() {
        let query_doc = doc! {
            "_id": ObjectId::from_str(element._id.as_str()).unwrap(),
        };
        match Element::update_document(
            &database_client,
            query_doc,
            UpdateElement {
                selected: None,
                locked_by: Some(None),
                x: None,
                y: None,
                rotation: None,
                scale_x: None,
                scale_y: None,
                z_index: None,
                text: None,
                color: None,
            },
        )
        .await
        {
            Ok(update_result) => match update_result.modified_count {
                0 => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unlock of Element with ID {} failed", element._id),
                    )
                        .into_response()
                }
                _ => {
                    updated_document_results.push(update_result);
                }
            },
            Err(error_response) => return error_response,
        }
    }
    match updated_document_results.len() {
        0 => (StatusCode::NOT_FOUND, "No Element found to update").into_response(),
        number => {
            info!("Updateded {} Elements", number);
            for element_id in body.ids.iter() {
                let mut sub_context = element_context.lock().await;
                sub_context
                    .emit_element_event(
                        body.board_id.to_string(),
                        ElementEvent {
                            event_type: ElementEventType::Unlocked,
                            body: serde_json::to_string(&ElementUnlockedEventPayload {
                                _id: element_id.clone(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
            }
            (StatusCode::OK, Json(format!("{}", number))).into_response()
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnlockAllQueryParams {
    user_id: String,
    board_id: String,
}

async fn unlock_all_for_user(
    query_params: Query<UnlockAllQueryParams>,
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
) -> Response {
    let query_doc = doc! {
        "lockedBy": query_params.user_id.clone()
    };
    let found_elements =
        match Element::get_multiple_documents(&database_client, query_doc.clone()).await {
            Ok(element_cursor) => match element_cursor.try_collect::<Vec<Element>>().await {
                Ok(retrieved_elements) => match retrieved_elements.len() {
                    0 => {
                        return (StatusCode::NO_CONTENT, "No elements are locked by the user")
                            .into_response()
                    }
                    _ => retrieved_elements,
                },
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Found Elements could not be retrieved",
                    )
                        .into_response();
                }
            },
            Err(error_response) => return error_response,
        };
    match Element::update_document(
        &database_client,
        query_doc,
        UpdateElement {
            scale_x: None,
            scale_y: None,
            rotation: None,
            selected: None,
            z_index: None,
            color: None,
            text: None,
            x: None,
            y: None,
            locked_by: Some(None),
        },
    )
    .await
    {
        Ok(_) => {
            let ids = found_elements
                .iter()
                .map(|element| element._id.clone())
                .collect::<Vec<String>>();
            for id in ids.iter() {
                let mut sub_context = element_context.lock().await;
                sub_context
                    .emit_element_event(
                        query_params.board_id.to_string(),
                        ElementEvent {
                            event_type: ElementEventType::Unlocked,
                            body: serde_json::to_string(&ElementUnlockedEventPayload {
                                _id: id.to_string(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
            }
            (StatusCode::OK, Json(ids)).into_response()
        }
        Err(error_response) => error_response,
    }
}

async fn update_element(
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
    payload: Result<Json<UpdateElementPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => {
            return error_response;
        }
    };
    let query_doc = doc! {
        "_id": ObjectId::from_str(body._id.as_str()).unwrap(),
    };
    let found_element_result = Element::get_document(&database_client, query_doc.clone()).await;
    match found_element_result {
        Ok(element) => match element {
            Some(element) => match element.locked_by {
                Some(locked_by) => {
                    if locked_by != body.user_id {
                        return (
                            StatusCode::LOCKED,
                            "Element currently locked by someone else",
                        )
                            .into_response();
                    }
                }
                None => {
                    return (
                        StatusCode::PRECONDITION_REQUIRED,
                        "Element needs to be locked first",
                    )
                        .into_response()
                }
            },
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    format!("No Element found with ID: {}", body._id),
                )
                    .into_response()
            }
        },
        Err(error_response) => {
            return error_response;
        }
    };
    let update_result = Element::update_document(
        &database_client,
        query_doc,
        UpdateElement {
            selected: None,
            locked_by: None,
            x: body.x,
            y: body.y,
            rotation: body.rotation,
            scale_x: body.scale_x,
            scale_y: body.scale_y,
            z_index: body.z_index,
            text: body.text.clone(),
            color: body.color.clone(),
        },
    )
    .await;
    match update_result {
        Ok(result) => match result.modified_count {
            0 => (StatusCode::NOT_FOUND, "No Element found to update").into_response(),
            _ => {
                info!("Updated Element with ID: {}", body._id.clone());
                let mut sub_context = element_context.lock().await;
                sub_context
                    .emit_element_event(
                        body.board_id.clone(),
                        ElementEvent {
                            event_type: ElementEventType::Updated,
                            body: serde_json::to_string(&UpdatedElementEventPayload {
                                _id: body._id.clone(),
                                user_id: body.user_id.clone(),
                                text: body.text.clone(),
                                z_index: body.z_index,
                                scale_x: body.scale_x,
                                scale_y: body.scale_y,
                                rotation: body.rotation,
                                x: body.x,
                                y: body.y,
                                color: body.color.clone(),
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
                (StatusCode::OK, Json(body._id.clone())).into_response()
            }
        },
        Err(error_response) => error_response,
    }
}

async fn move_multiple_elements(
    State(AppState {
        database_client,
        element_context,
        ..
    }): State<AppState>,
    payload: Result<Json<MoveMultipleElementsPayload>, JsonRejection>,
) -> Response {
    let body = match check_request_body(payload) {
        Ok(success_body) => success_body,
        Err(error_response) => {
            return error_response;
        }
    };
    let query_doc = doc! {
        "_id": doc! { "$in": body.ids.iter().map(|id| ObjectId::from_str(id.as_str()).unwrap()).collect::<Vec<ObjectId>>() }
    };
    let found_element_result =
        Element::get_multiple_documents(&database_client, query_doc.clone()).await;
    let found_elements = match found_element_result {
        Ok(element_cursor) => {
            let retrieved_elements = element_cursor.try_collect::<Vec<Element>>().await;
            match retrieved_elements {
                Ok(retrieved_elements) => match retrieved_elements.len() {
                    0 => return (StatusCode::NOT_FOUND, "No Elements found").into_response(),
                    _ => retrieved_elements,
                },
                Err(_) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Found Elements could not be retrieved",
                    )
                        .into_response();
                }
            }
        }
        Err(error_response) => {
            return error_response;
        }
    };
    if found_elements
        .iter()
        .any(|element| match &element.locked_by {
            Some(locked_by) => *locked_by != body.user_id,
            None => false,
        })
    {
        return (StatusCode::LOCKED, "Some Element is locked by another user").into_response();
    }
    let mut updated_document_results: Vec<UpdateResult> = vec![];
    for element in found_elements.iter() {
        let query_doc = doc! {
            "_id": ObjectId::from_str(element._id.as_str()).unwrap(),
        };
        match Element::update_document(
            &database_client,
            query_doc,
            UpdateElement {
                selected: None,
                locked_by: Some(Some(body.user_id.clone())),
                x: Some(element.x + body.x_offset),
                y: Some(element.y + body.y_offset),
                rotation: None,
                scale_x: None,
                scale_y: None,
                z_index: None,
                text: None,
                color: None,
            },
        )
        .await
        {
            Ok(update_result) => match update_result.modified_count {
                0 => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Move of Element with ID {} failed", element._id),
                    )
                        .into_response()
                }
                _ => {
                    updated_document_results.push(update_result);
                }
            },
            Err(error_response) => return error_response,
        }
    }
    match updated_document_results.len() {
        0 => (StatusCode::NOT_FOUND, "No Element found to update").into_response(),
        number => {
            info!("Updateded {} Elements", number);
            for element_id in body.ids.iter() {
                let mut sub_context = element_context.lock().await;
                sub_context
                    .emit_element_event(
                        body.board_id.to_string(),
                        ElementEvent {
                            event_type: ElementEventType::Moved,
                            body: serde_json::to_string(&ElementMovedEventPayload {
                                user_id: body.user_id.clone(),
                                _id: element_id.to_string(),
                                x_offset: body.x_offset,
                                y_offset: body.y_offset,
                            })
                            .unwrap(),
                        },
                    )
                    .await;
                drop(sub_context);
            }
            (StatusCode::OK, Json(format!("{}", number))).into_response()
        }
    }
}
