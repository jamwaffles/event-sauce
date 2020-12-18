use crate::event_builder::EventBuilder;
use crate::{Entity, Event, EventData};
use chrono::Utc;
use uuid::Uuid;

/// Generic event builder for an action specified by its EventData
pub struct ActionEventBuilder<EDENUM>
where
    EDENUM: EventData
{
    payload: EDENUM,
    session_id: Option<Uuid>,
}

impl<EDENUM> ActionEventBuilder<EDENUM>
where
    EDENUM: EventData,
{
    /// DOCS
    pub fn build<E: Entity>(self, entity: &Option<E>) -> Event<EDENUM> {
        Event {
            id: Uuid::new_v4(),
            event_type: String::from(self.payload.event_type()),
            entity_type: E::entity_type(),
            entity_id: entity.as_ref().map_or(Uuid::new_v4(), |e| e.entity_id()),
            session_id: self.session_id,
            purger_id: None,
            created_at: Utc::now(),
            purged_at: None,
            data: Some(self.payload),
        }
    }
}

impl<EDENUM> EventBuilder<EDENUM> for ActionEventBuilder<EDENUM>
where
    EDENUM: EventData,
{
    /// Create a new event builder with a given event data payload
    fn new(payload: EDENUM) -> Self {
        Self {
            payload,
            session_id: None,
        }
    }

    /// Set the session ID field of the event
    fn session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);

        self
    }
}

impl<EDENUM> From<EDENUM> for ActionEventBuilder<EDENUM>
where
    EDENUM: EventData,
{
    fn from(payload: EDENUM) -> Self {
        Self::new(payload)
    }
}
