use std::{fs::File, io::Read};

use bson::doc;
use mongodb::Client;
use serde::Deserialize;
use tracing::warn;

use crate::database::{
    collections::element_type::{CreateElementType, ElementType, UpdateElementType},
    document::Document,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementDefintion {
    name: String,
    path: String,
}

pub async fn generate_elements(database_client: &Client) -> Result<(), String> {
    let mut file =
        File::open("assets/elements.json").expect("JSON containing Element Types not found");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Could not read elements JSON file");
    let elements = serde_json::from_str::<Vec<ElementDefintion>>(contents.as_str())
        .expect("Element JSON is not valid");
    for element in elements.iter() {
        let query_doc = doc! {
            "name": element.name.clone()
        };
        match ElementType::get_document(database_client, query_doc.clone()).await {
            Ok(element_type_option) => match element_type_option {
                Some(_) => {
                    match ElementType::update_document(
                        database_client,
                        query_doc,
                        UpdateElementType {
                            name: None,
                            path: Some(element.path.clone()),
                        },
                    )
                    .await
                    {
                        Ok(result) => match result.modified_count {
                            0 => {
                                warn!("Didn't update element with name: {}", element.name);
                                continue;
                            }
                            _ => continue,
                        },
                        Err(_) => {
                            return Err(format!(
                                "Couldn't update Element Type with name: {}",
                                element.name
                            ))
                        }
                    }
                }
                None => match ElementType::create_document(
                    database_client,
                    CreateElementType {
                        name: element.name.clone(),
                        path: element.path.clone(),
                    },
                )
                .await
                {
                    Ok(_) => continue,
                    Err(_) => {
                        return Err(format!(
                            "Couldn't create Element Type with name: {}",
                            element.name
                        ))
                    }
                },
            },
            Err(_) => {
                return Err(format!(
                    "Couldn't fetch current element type with name: {}",
                    element.name
                ))
            }
        };
    }
    Ok(())
}
