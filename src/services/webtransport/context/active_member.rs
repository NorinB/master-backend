use crate::services::webtransport::context::base::Subject;
use log::info;
use rxrust::observer::Observer;
use std::collections::HashMap;

pub struct ActiveMemberContext {
    pub board_active_member_subjects: HashMap<String, ActiveMemberSubject>,
}

impl ActiveMemberContext {
    pub fn new() -> Self {
        Self {
            board_active_member_subjects: HashMap::new(),
        }
    }

    pub fn get_or_create_subject(&mut self, board_id: String) -> &mut ActiveMemberSubject {
        self.board_active_member_subjects
            .entry(board_id.clone())
            .or_insert_with(|| ActiveMemberContext::create_subject(board_id))
    }

    pub fn get_or_create_subject_return_board_id(&mut self, board_id: String) -> String {
        self.board_active_member_subjects
            .entry(board_id.clone())
            .or_insert_with(|| ActiveMemberContext::create_subject(board_id))
            .board_id
            .clone()
    }

    fn create_subject(board_id: String) -> ActiveMemberSubject {
        ActiveMemberSubject {
            board_id,
            subject: Subject::default(),
        }
    }

    fn get_subject_for_board_id(&mut self, board_id: String) -> Option<&mut ActiveMemberSubject> {
        match self.board_active_member_subjects.get_mut(&board_id) {
            Some(subject) => Some(subject),
            None => None,
        }
    }

    pub async fn emit_active_member_event(&mut self, board_id: String, event: ActiveMemberEvent) {
        if let Some(subject) = self.get_subject_for_board_id(board_id.clone()) {
            info!(
                "Event wird emitted jetzt Board ID {} und event mit message: {}",
                board_id,
                event.clone().body
            );
            subject.subject.next(event);
        }
    }
}

pub struct ActiveMemberSubject {
    pub board_id: String,
    pub subject: Subject<ActiveMemberEvent>,
}

#[derive(Clone)]
pub enum ActiveMemberEventType {
    Created,
    Removed,
    PositionUpdated,
}

impl ToString for ActiveMemberEventType {
    fn to_string(&self) -> String {
        match self {
            ActiveMemberEventType::Created => "activemember_created".to_string(),
            ActiveMemberEventType::Removed => "activemember_removed".to_string(),
            ActiveMemberEventType::PositionUpdated => "activemember_positionupdated".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct ActiveMemberEvent {
    pub event_type: ActiveMemberEventType,
    pub body: String,
}
