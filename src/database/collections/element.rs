use axum::response::Response;
use bson::{
    doc,
    serde_helpers::{
        deserialize_bson_datetime_from_rfc3339_string, deserialize_hex_string_from_object_id,
        serialize_bson_datetime_as_rfc3339_string, serialize_hex_string_as_object_id,
    },
    DateTime,
};
use mongodb::{
    options::{CreateCollectionOptions, ValidationAction, ValidationLevel},
    results::{DeleteResult, InsertOneResult, UpdateResult},
    Client, Cursor,
};
use serde::{Deserialize, Serialize};

use crate::database::{
    document::{Document, DocumentBase},
    validator::Validator,
};

const ELEMENT_COLLECTION_NAME: &str = "element";
const ELEMENT_DOCUMENT_NAME: &str = "Element";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Element {
    #[serde(
        deserialize_with = "deserialize_hex_string_from_object_id",
        rename = "_id"
    )]
    pub _id: String,
    pub selected: bool,
    pub locked_by: Option<String>,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub z_index: i32,
    #[serde(deserialize_with = "deserialize_bson_datetime_from_rfc3339_string")]
    pub created_at: DateTime,
    pub text: String,
    pub element_type: String,
    pub board_id: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateElement {
    #[serde(
        serialize_with = "serialize_hex_string_as_object_id",
        deserialize_with = "deserialize_hex_string_from_object_id",
        rename = "_id"
    )]
    pub _id: String,
    pub selected: bool,
    pub locked_by: Option<String>,
    pub x: f32,
    pub y: f32,
    pub rotation: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub z_index: i32,
    #[serde(serialize_with = "serialize_bson_datetime_as_rfc3339_string")]
    pub created_at: DateTime,
    pub text: String,
    pub element_type: String,
    pub board_id: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateElement {
    pub selected: Option<bool>,
    pub locked_by: Option<Option<String>>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub rotation: Option<f32>,
    pub scale_x: Option<f32>,
    pub scale_y: Option<f32>,
    pub z_index: Option<i32>,
    pub text: Option<String>,
    pub color: Option<String>,
}

impl Document<Element, CreateElement, UpdateElement> for Element {
    async fn create_collection(client: &Client) -> Result<(), Response> {
        let create_collection_opts = Element::get_validation_options().ok();
        DocumentBase::create_collection(
            client,
            ELEMENT_COLLECTION_NAME,
            create_collection_opts,
            ELEMENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn create_document(
        client: &Client,
        insert_doc: CreateElement,
    ) -> Result<InsertOneResult, Response> {
        DocumentBase::create_document::<CreateElement>(
            client,
            ELEMENT_COLLECTION_NAME,
            insert_doc,
            ELEMENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<DeleteResult, Response> {
        DocumentBase::delete_document::<Element>(
            client,
            ELEMENT_COLLECTION_NAME,
            query_doc,
            ELEMENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn update_document(
        client: &Client,
        query_doc: bson::Document,
        update_document: UpdateElement,
    ) -> Result<UpdateResult, Response> {
        let mut update_fields = doc! {};
        if let Some(x) = update_document.x {
            update_fields.insert("x", x);
        };
        if let Some(y) = update_document.y {
            update_fields.insert("y", y);
        };
        if let Some(selected) = update_document.selected {
            update_fields.insert("selected", selected);
        };
        if let Some(locked_by) = update_document.locked_by {
            update_fields.insert("lockedBy", locked_by);
        };
        if let Some(rotation) = update_document.rotation {
            update_fields.insert("rotation", rotation);
        };
        if let Some(scale_x) = update_document.scale_x {
            update_fields.insert("scaleX", scale_x);
        };
        if let Some(scale_y) = update_document.scale_y {
            update_fields.insert("scaleY", scale_y);
        };
        if let Some(z_index) = update_document.z_index {
            update_fields.insert("zIndex", z_index);
        };
        if let Some(text) = update_document.text {
            update_fields.insert("text", text);
        };
        if let Some(color) = update_document.color {
            update_fields.insert("color", color);
        };
        let update_doc = doc! {
            "$set": update_fields
        };
        DocumentBase::update_document::<Element>(
            client,
            ELEMENT_COLLECTION_NAME,
            query_doc,
            update_doc,
            ELEMENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn delete_collection(client: &Client) -> Result<(), Response> {
        DocumentBase::delete_collection::<Element>(
            client,
            ELEMENT_COLLECTION_NAME,
            ELEMENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_document(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Option<Element>, Response> {
        DocumentBase::get_document::<Element>(
            client,
            ELEMENT_COLLECTION_NAME,
            query_doc,
            ELEMENT_DOCUMENT_NAME,
        )
        .await
    }

    async fn get_multiple_documents(
        client: &Client,
        query_doc: bson::Document,
    ) -> Result<Cursor<Element>, Response> {
        DocumentBase::get_multiple_documents::<Element>(
            client,
            ELEMENT_COLLECTION_NAME,
            query_doc,
            ELEMENT_DOCUMENT_NAME,
        )
        .await
    }
}

impl Validator for Element {
    fn get_validation_options() -> Result<CreateCollectionOptions, Box<dyn std::error::Error>> {
        let validator = doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Element Validation",
                "required": vec!["_id", "selected", "x_position", "y_position", "rotation", "scale", "z_index", "created_at", "text", "element_type", "board_id", "color"],
                "properties": doc! {
                    "_id": doc! {
                        "bsonType": "string",
                        "description": "ID fo the element",
                    },
                    "selected": doc! {
                        "bsonType": "bool",
                        "description": "Whether the element is selected",
                    },
                    "lockedBy": doc! {
                        "bsonType": "string",
                        "description": "The User ID of the user currently locking the elemnent"
                    },
                    "x": doc! {
                        "bsonType": "double",
                        "description": "The x coordinate of the element"
                    },
                    "y": doc! {
                        "bsonType": "double",
                        "description": "The y coordinate of the element"
                    },
                    "rotation": doc! {
                        "bsonType": "double",
                        "description": "The rotation of the element in degrees"
                    },
                    "scaleX": doc! {
                        "bsonType": "double",
                        "description": "The scale of the element in x direction"
                    },
                    "scaleY": doc! {
                        "bsonType": "double",
                        "description": "The scale of the element in y direction"
                    },
                    "zIndex": doc! {
                        "bsonType": "int",
                        "description": "The z-Index of the element"
                    },
                    "createdAt": doc! {
                        "bsonType": "string",
                        "description": "The timestamp of the creation of the element"
                    },
                    "text": doc! {
                        "bsonType": "string",
                        "description": "The text inside the element"
                    },
                    "elementType": doc! {
                        "bsonType": "string",
                        "description": "The type of the element"
                    },
                    "boardId": doc! {
                        "bsonType": "string",
                        "description": "The ID of the board, this element is contained in"
                    },
                    "color": doc! {
                        "bsonType": "string",
                        "description": "The fill color of the element"
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
