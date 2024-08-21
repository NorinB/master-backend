use crate::services::webtransport::context::base::Subject;
use log::info;
use mongodb::Client;
use rxrust::observer::Observer;
use std::collections::HashMap;

use crate::database::collections::board::Board;

pub struct BoardContext {
    pub board_subjects: HashMap<String, BoardSubject>,
}

impl BoardContext {
    pub fn new() -> Self {
        Self {
            board_subjects: HashMap::new(),
        }
    }

    pub fn get_or_create_subject(&mut self, board_id: String) -> &mut BoardSubject {
        self.board_subjects
            .entry(board_id.clone())
            .or_insert_with(|| BoardContext::create_subject(board_id))
    }

    pub fn get_or_create_subject_return_board_id(&mut self, board_id: String) -> String {
        self.board_subjects
            .entry(board_id.clone())
            .or_insert_with(|| BoardContext::create_subject(board_id))
            .board_id
            .clone()
    }

    fn create_subject(board_id: String) -> BoardSubject {
        BoardSubject {
            board_id,
            subject: Subject::default(),
        }
    }

    fn get_subject_for_board_id(&mut self, board_id: String) -> Option<&mut BoardSubject> {
        match self.board_subjects.get_mut(&board_id) {
            Some(subject) => Some(subject),
            None => None,
        }
    }

    pub async fn emit_board_event(
        &mut self,
        database_client: Client,
        board_id: String,
        event: BoardEvent,
    ) {
        if let Ok(board) = Board::get_existing_board(board_id.clone(), &database_client).await {
            if let Some(subject) = self.get_subject_for_board_id(board._id) {
                info!(
                    "Event wird emitted jetzt für Board mit ID {} und event mit message: {}",
                    board_id,
                    event.clone().body
                );
                subject.subject.next(event);
            }
        }
    }
}

pub struct BoardSubject {
    pub board_id: String,
    pub subject: Subject<BoardEvent>,
}

#[derive(Clone)]
pub enum BoardEventType {
    MemberAdded,
    MemberRemoved,
}

impl ToString for BoardEventType {
    fn to_string(&self) -> String {
        match self {
            BoardEventType::MemberAdded => "board_memberadded".to_string(),
            BoardEventType::MemberRemoved => "board_memberremoved".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct BoardEvent {
    pub event_type: BoardEventType,
    pub body: String,
}
