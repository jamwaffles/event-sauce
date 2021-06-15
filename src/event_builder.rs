use crate::{Entity, Event, EventData};
use chrono::Utc;
use uuid::Uuid;

pub struct EventBuilder<D>
where
    D: EventData,
{
    event: Event<D>,
}

impl<D> EventBuilder<D>
where
    D: EventData,
{
    /// Set the entity ID for this event.
    pub fn entity_id(self, entity_id: Uuid) -> Self {
        Self {
            event: Event {
                entity_id,
                ..self.event
            },
        }
    }

    /// Consume the builder and produce an event.
    pub fn build(self) -> Event<D> {
        Event::from(self.event)
    }
}

/// Blanket impl to convert event data into an event.
impl<D> From<D> for EventBuilder<D>
where
    D: EventData,
{
    fn from(data: D) -> Self {
        Self {
            event: Event {
                id: Uuid::new_v4(),
                event_type: D::event_type().to_string(),
                entity_type: D::Entity::ENTITY_TYPE.to_string(),
                entity_id: Uuid::new_v4(),
                session_id: Uuid::nil(),
                data,
                created_at: Utc::now(),
            },
        }
    }
}

/// Convenience impl to convert a builder into an event.
impl<D> From<EventBuilder<D>> for Event<D>
where
    D: EventData,
{
    fn from(builder: EventBuilder<D>) -> Self {
        builder.event
    }
}
