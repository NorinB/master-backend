use crate::services::webtransport::context::base::Subject;
use log::info;
use rxrust::observer::Observer;
use std::collections::HashMap;

use crate::database::collections::client::Client;

pub struct ClientContext {
    pub client_subjects: HashMap<String, ClientSubject>,
}

impl ClientContext {
    pub fn new() -> Self {
        Self {
            client_subjects: HashMap::new(),
        }
    }

    pub fn get_or_create_subject(&mut self, user_id: String) -> &mut ClientSubject {
        self.client_subjects
            .entry(user_id.clone())
            .or_insert_with(|| ClientContext::create_subject(user_id))
    }

    pub fn get_or_create_subject_return_user_id(&mut self, user_id: String) -> String {
        self.client_subjects
            .entry(user_id.clone())
            .or_insert_with(|| ClientContext::create_subject(user_id))
            .client_id
            .clone()
    }

    fn create_subject(client_id: String) -> ClientSubject {
        ClientSubject {
            client_id,
            subject: Subject::default(),
        }
    }

    fn get_subject_for_user_id(&mut self, client_id: String) -> Option<&mut ClientSubject> {
        match self.client_subjects.get_mut(&client_id) {
            Some(subject) => Some(subject),
            None => None,
        }
    }

    pub async fn emit_client_event(
        &mut self,
        database_client: mongodb::Client,
        user_id: String,
        event: ClientEvent,
    ) {
        if let Ok(client) = Client::get_existing_client(user_id.clone(), &database_client).await {
            if let Some(subject) = self.get_subject_for_user_id(client.user_id) {
                info!(
                    "Event wird emitted jetzt für Client mit ID {} und event mit message: {}",
                    user_id,
                    event.clone().body
                );
                subject.subject.next(event);
            }
        }
    }
}

pub struct ClientSubject {
    pub client_id: String,
    pub subject: Subject<ClientEvent>,
}

#[derive(Clone)]
pub enum ClientEventType {
    Deleted,
    Changed,
}

impl ToString for ClientEventType {
    fn to_string(&self) -> String {
        match self {
            ClientEventType::Deleted => "client_removed".to_string(),
            ClientEventType::Changed => "client_changed".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct ClientEvent {
    pub event_type: ClientEventType,
    pub body: String,
}
