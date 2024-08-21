use std::str::FromStr;

use axum::response::Response;
use bson::{oid::ObjectId, serde_helpers::deserialize_hex_string_from_object_id};
use mongodb::{
    bson::doc,
    options::{CreateCollectionOptions, ValidationAction, ValidationLevel},
    results::{DeleteResult, InsertOneResult, UpdateResult},
    Client, Cursor,
};
use serde::{Deserialize, Serialize};

use crate::database::{
    document::{Document, DocumentBase},
    validator::Validator,
};

const USER_COLLECTION_NAME: &str = "user";
const USER_DOCUMENT_NAME: &str = "User";

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(
        deserialize_with = "deserialize_hex_string_from_object_id",
        rename = "_id"
    )]
    pub _id: String,
    pub name: String,
    pub email: String,
    pub password: String,
    pub active_client: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateUser {
    #[serde(rename = "_id")]
    pub _id: ObjectId,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUser {
    pub name: Option<String>,
    pub email: Option<String>,
    pub password: Option<String>,
    pub active_client: Option<String>,
}

impl Document<User, CreateUser, UpdateUser> for User {
    async fn create_collection(client: &Client) -> Result<(), Response> {
        let create_collection_opts = User::get_validation_options().ok();
        DocumentBase::create_collection(
            client,
            "USER_COLLECTION_NAME",
            create_collection_opts,
            USER_DOCUMENT_NAME,
        )
        .await
    }

    async fn create_document(
        client: &Client,
        insert_doc: CreateUser,
    ) -> Result<InsertOneResult, Response> {
        DocumentBase::create_document::<CreateUser>(
            client,
            USER_COLLECTION_NAME,
            insert_doc,
            USER_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<DeleteResult, Response> {
        DocumentBase::delete_document::<User>(
            client,
            USER_COLLECTION_NAME,
            query_doc,
            USER_DOCUMENT_NAME,
        )
        .await
    }

    async fn update_document(
        client: &Client,
        query_doc: bson::Document,
        update_document: UpdateUser,
    ) -> Result<UpdateResult, Response> {
        let mut update_fields = doc! {};
        if let Some(name) = update_document.name {
            update_fields.insert("name", name);
        }
        if let Some(email) = update_document.email {
            update_fields.insert("email", email);
        }
        if let Some(password) = update_document.password {
            update_fields.insert("password", password);
        }
        if let Some(active_client) = update_document.active_client {
            update_fields.insert("activeClient", active_client);
        }
        let update_doc = doc! {
            "$set": update_fields
        };
        DocumentBase::update_document::<User>(
            client,
            USER_COLLECTION_NAME,
            query_doc,
            update_doc,
            USER_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_collection(client: &Client) -> Result<(), Response> {
        DocumentBase::delete_collection::<User>(client, USER_COLLECTION_NAME, USER_DOCUMENT_NAME)
            .await
    }

    async fn get_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Option<User>, Response> {
        DocumentBase::get_document::<User>(
            client,
            USER_COLLECTION_NAME,
            query_doc,
            USER_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_multiple_documents(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Cursor<User>, Response> {
        DocumentBase::get_multiple_documents::<User>(
            client,
            USER_COLLECTION_NAME,
            query_doc,
            USER_DOCUMENT_NAME,
        )
        .await
    }
}

impl User {
    pub async fn get_existing_user(
        user_id: String,
        database_client: &Client,
    ) -> Result<User, String> {
        let query_doc = doc! {
             "_id": ObjectId::from_str(user_id.as_str()).unwrap()
        };
        match User::get_document(database_client, query_doc).await {
            Ok(user_option) => match user_option {
                Some(user) => Ok(user),
                None => Err("No User found with that ID".to_string()),
            },
            Err(_) => Err("Something went wrong when fetching for the user".to_string()),
        }
    }
}

impl Validator for User {
    fn get_validation_options(
    ) -> Result<mongodb::options::CreateCollectionOptions, Box<dyn std::error::Error>> {
        let validator = doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "User Validation",
                "required": vec! ["_id", "name", "email", "password", "active_client"],
                "properties": doc! {
                    "_id": doc! {
                        "bsonType": "string",
                        "description": "ID of the User"
                    },
                    "name": doc! {
                        "bsonType": "string",
                        "description": "Name of the user"
                    },
                    "email": doc! {
                        "bsonType": "string",
                        "description": "Email of the user"
                    },
                    "password": doc! {
                        "bsonType": vec! ["object"],
                        "description": "Member array"
                    },
                    "active_client": doc! {
                        "bsonType": "string",
                        "description": "Current active client device ID"
                    }
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
