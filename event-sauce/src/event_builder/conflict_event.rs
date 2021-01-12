//! Event builder

use crate::event_builder::EventBuilder;
use crate::{ConflictData, Entity, Event, EventData};
use chrono::Utc;
use uuid::Uuid;

/// Event conflict builder
pub struct ConflictEventBuilder<EDA: EventData, EDC: EventData> {
    payload: ConflictData<EDA, EDC>,
    session_id: Option<Uuid>,
}

impl<EDA, EDC> ConflictEventBuilder<EDA, EDC>
where
    EDA: EventData,
    EDC: EventData,
{
    /// Consume the builder and produce the final event
    pub fn build(self, entity: &EDA::Entity) -> Event<ConflictData<EDA, EDC>> {
        Event {
            id: Uuid::new_v4(),
            event_type: String::from(self.payload.event_type()),
            entity_type: EDA::Entity::entity_type(),
            entity_id: entity.entity_id(),
            session_id: self.session_id,
            purger_id: None,
            created_at: Utc::now(),
            purged_at: None,
            data: Some(self.payload),
        }
    }

    /// Workaround method to get an entity ID out of entities when implementing
    /// `ConflictEntityBuilder`
    pub(crate) fn build_with_entity_id(self) -> Event<ConflictData<EDA, EDC>> {
        Event {
            id: Uuid::new_v4(),
            event_type: String::from(self.payload.event_type()),
            entity_type: EDA::Entity::entity_type(),
            entity_id: self.payload.applied_event.entity_id,
            session_id: self.session_id,
            purger_id: None,
            created_at: Utc::now(),
            purged_at: None,
            data: Some(self.payload),
        }
    }
}

impl<EDA, EDC> EventBuilder<ConflictData<EDA, EDC>> for ConflictEventBuilder<EDA, EDC>
where
    EDA: EventData,
    EDC: EventData,
{
    /// Create a new event builder with a given event data payload
    fn new(payload: ConflictData<EDA, EDC>) -> Self {
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

impl<EDA, EDC> From<ConflictData<EDA, EDC>> for ConflictEventBuilder<EDA, EDC>
where
    EDA: EventData,
    EDC: EventData,
{
    fn from(payload: ConflictData<EDA, EDC>) -> Self {
        Self::new(payload)
    }
}
