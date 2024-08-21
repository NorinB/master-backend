use std::convert::Infallible;

use rxrust::subject::SubjectThreads;

pub enum EventCategory {
    Board,
    Client,
    ActiveMember,
    Element,
}

impl EventCategory {
    pub fn get_category_by_string(category_string: String) -> Result<Self, ()> {
        let category_str = category_string.as_str();
        match category_str {
            "board" => Ok(EventCategory::Board),
            "client" => Ok(EventCategory::Client),
            "active_member" => Ok(EventCategory::ActiveMember),
            "element" => Ok(EventCategory::Element),
            _ => Err(()),
        }
    }
}

pub type Subject<T> = SubjectThreads<T, Infallible>;
