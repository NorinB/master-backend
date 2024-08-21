use std::str::FromStr;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bson::{oid::ObjectId, serde_helpers::deserialize_hex_string_from_object_id};
use mongodb::{
    bson::doc,
    options::{CreateCollectionOptions, ValidationAction, ValidationLevel},
    results::{DeleteResult, InsertOneResult, UpdateResult},
    Client,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

use crate::database::{
    config::DATABASE_NAME,
    document::{Document, DocumentBase},
    validator::Validator,
};

use super::user::User;

const BOARD_COLLECTION_NAME: &str = "board";
const BOARD_DOCUMENT_NAME: &str = "Board";

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    #[serde(
        deserialize_with = "deserialize_hex_string_from_object_id",
        rename = "_id"
    )]
    pub _id: String,
    pub name: String,
    pub host: String,
    pub allowed_members: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateBoard {
    pub name: String,
    pub host: String,
    pub allowed_members: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBoard {
    pub name: Option<String>,
    pub host: Option<String>,
    pub allowed_members: Option<Vec<String>>,
}

impl Document<Board, CreateBoard, UpdateBoard> for Board {
    async fn create_collection(client: &Client) -> Result<(), Response> {
        let create_collection_opts = Board::get_validation_options().ok();
        DocumentBase::create_collection(
            client,
            BOARD_COLLECTION_NAME,
            create_collection_opts,
            BOARD_DOCUMENT_NAME,
        )
        .await
    }

    async fn create_document(
        client: &Client,
        insert_doc: CreateBoard,
    ) -> Result<InsertOneResult, Response> {
        DocumentBase::create_document::<CreateBoard>(
            client,
            BOARD_COLLECTION_NAME,
            insert_doc,
            BOARD_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<DeleteResult, Response> {
        DocumentBase::delete_document::<Board>(
            client,
            BOARD_COLLECTION_NAME,
            query_doc,
            BOARD_DOCUMENT_NAME,
        )
        .await
    }

    async fn update_document(
        client: &Client,
        query_doc: bson::Document,
        update_document: UpdateBoard,
    ) -> Result<UpdateResult, Response> {
        let mut update_fields = doc! {};
        if let Some(name) = update_document.name {
            update_fields.insert("name", name);
        }
        if let Some(host) = update_document.host {
            update_fields.insert("host", host);
        }
        if let Some(allowed_members) = update_document.allowed_members {
            update_fields.insert("allowedMembers", allowed_members);
        }
        let update_doc = doc! {
            "$set": update_fields,
        };
        DocumentBase::update_document::<Board>(
            client,
            BOARD_COLLECTION_NAME,
            query_doc,
            update_doc,
            BOARD_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_collection(client: &Client) -> Result<(), Response> {
        DocumentBase::delete_collection::<Board>(client, BOARD_COLLECTION_NAME, BOARD_DOCUMENT_NAME)
            .await
    }

    async fn get_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Option<Board>, Response> {
        DocumentBase::get_document::<Board>(
            client,
            BOARD_COLLECTION_NAME,
            query_doc,
            BOARD_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_multiple_documents(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<mongodb::Cursor<Board>, Response> {
        DocumentBase::get_multiple_documents::<Board>(
            client,
            BOARD_COLLECTION_NAME,
            query_doc,
            BOARD_DOCUMENT_NAME,
        )
        .await
    }
}

impl Board {
    pub async fn get_existing_board(
        board_id: String,
        database_client: &Client,
    ) -> Result<Board, Response> {
        let query_doc = doc! {
            "_id": ObjectId::from_str(board_id.clone().as_str()).unwrap(),
        };
        info!("{}", board_id);
        let board_result = Board::get_document(database_client, query_doc).await;
        match board_result {
            Ok(board_option) => match board_option {
                Some(board) => Ok(board),
                None => {
                    error!("Board with ID {} does not exist", board_id);
                    Err((StatusCode::NOT_FOUND, "Board does not exist").into_response())
                }
            },
            Err(error_response) => {
                error!("Board with ID {} could not be fetched", board_id);
                Err(error_response)
            }
        }
    }

    pub async fn add_member(
        board_id: String,
        member_id: String,
        database_client: &Client,
    ) -> Result<String, String> {
        let _ = match User::get_existing_user(member_id.clone(), database_client).await {
            Ok(user) => user._id,
            Err(message) => return Err(message),
        };
        let mut current_board_members =
            match Board::get_existing_board(board_id.clone(), database_client).await {
                Ok(board) => board.allowed_members,
                Err(_) => return Err("Board does not exist".to_string()),
            };
        if current_board_members.contains(&member_id) {
            return Err("Member already part of this board".to_string());
        }
        current_board_members.push(member_id.clone());
        let query_doc = doc! {
            "_id": ObjectId::from_str(board_id.as_str()).unwrap(),
        };
        let update_doc = doc! {
            "$set": doc! {
              "allowedMembers": current_board_members,
            }
        };
        let result = database_client
            .database(DATABASE_NAME())
            .collection::<Board>(BOARD_COLLECTION_NAME)
            .update_one(query_doc, update_doc, None)
            .await;
        match result {
            Ok(result) => match result.modified_count {
                0 => Err("Member was not added".to_string()),
                _ => Ok(member_id),
            },
            Err(_) => Err("Error during add member update".to_string()),
        }
    }

    pub async fn remove_member(
        board_id: String,
        member_id: String,
        database_client: &Client,
    ) -> Result<String, String> {
        let mut current_board_members =
            match Board::get_existing_board(board_id.clone(), database_client).await {
                Ok(board) => board.allowed_members,
                Err(_) => return Err("Board does not exist".to_string()),
            };
        if let Some(position) = current_board_members
            .iter()
            .position(|member| *member == member_id)
        {
            current_board_members.remove(position);
        } else {
            return Err("Member not part of this board".to_string());
        }
        let query_doc = doc! {
            "_id": ObjectId::from_str(board_id.as_str()).unwrap(),
        };
        let update_doc = doc! {
            "$set": doc! {
              "allowedMembers": current_board_members,
            }
        };
        let result = database_client
            .database(DATABASE_NAME())
            .collection::<Board>(BOARD_COLLECTION_NAME)
            .update_one(query_doc, update_doc, None)
            .await;
        match result {
            Ok(result) => match result.modified_count {
                0 => Err("Member was not removed".to_string()),
                _ => Ok(member_id),
            },
            Err(_) => Err("Error during remove member update".to_string()),
        }
    }
}

impl Validator for Board {
    fn get_validation_options() -> Result<CreateCollectionOptions, Box<dyn std::error::Error>> {
        let validator = doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Board Validation",
                "required": vec! ["_id", "name", "host", "active_members"],
                "properties": doc! {
                    "_id": doc! {
                        "bsonType": "int",
                        "description": "ID of the Board"
                    },
                    "name": doc! {
                        "bsonType": "string",
                        "description": "Name of the Board given by the user"
                    },
                    "host": doc! {
                        "bsonType": "int",
                        "description": "ID of the host member"
                    },
                    "allowed_members": doc! {
                        "bsonType": vec! ["object"],
                        "description": "Member array"
                    },
                }
            }
        };

        let validation_opts = CreateCollectionOptions::builder()
            .validator(validator)
            .validation_action(Some(ValidationAction::Error))
            .validation_level(Some(ValidationLevel::Moderate))
            .build();

        Ok(validation_opts)
    }
}
