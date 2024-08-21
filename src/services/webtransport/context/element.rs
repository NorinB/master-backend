use crate::services::webtransport::context::base::Subject;
use log::info;
use rxrust::observer::Observer;
use std::collections::HashMap;

pub struct ElementContext {
    pub board_element_subjects: HashMap<String, ElementSubject>,
}

impl ElementContext {
    pub fn new() -> Self {
        Self {
            board_element_subjects: HashMap::new(),
        }
    }

    pub fn get_or_create_subject(&mut self, board_id: String) -> &mut ElementSubject {
        self.board_element_subjects
            .entry(board_id.clone())
            .or_insert_with(|| ElementContext::create_subject(board_id))
    }

    pub fn get_or_create_subject_return_board_id(&mut self, board_id: String) -> String {
        self.board_element_subjects
            .entry(board_id.clone())
            .or_insert_with(|| ElementContext::create_subject(board_id))
            .board_id
            .clone()
    }

    fn create_subject(board_id: String) -> ElementSubject {
        ElementSubject {
            board_id,
            subject: Subject::default(),
        }
    }

    fn get_subject_for_board_id(&mut self, board_id: String) -> Option<&mut ElementSubject> {
        match self.board_element_subjects.get_mut(&board_id) {
            Some(subject) => Some(subject),
            None => None,
        }
    }

    pub async fn emit_element_event(&mut self, board_id: String, event: ElementEvent) {
        if let Some(subject) = self.get_subject_for_board_id(board_id.clone()) {
            info!(
                "Event wird emitted jetzt für Element mit ID {} und event mit message: {}",
                board_id,
                event.clone().body
            );
            subject.subject.next(event);
        }
    }
}

pub struct ElementSubject {
    pub board_id: String,
    pub subject: Subject<ElementEvent>,
}

#[derive(Clone)]
pub enum ElementEventType {
    Created,
    Removed,
    Moved,
    Locked,
    Unlocked,
    Updated,
}

impl ToString for ElementEventType {
    fn to_string(&self) -> String {
        match self {
            ElementEventType::Created => "element_created".to_string(),
            ElementEventType::Removed => "element_removed".to_string(),
            ElementEventType::Moved => "element_moved".to_string(),
            ElementEventType::Locked => "element_locked".to_string(),
            ElementEventType::Unlocked => "element_unlocked".to_string(),
            ElementEventType::Updated => "element_updated".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct ElementEvent {
    pub event_type: ElementEventType,
    pub body: String,
}
