//! Event builder

use crate::event_builder::EventBuilder;
use crate::{Entity, Event, EventData};
use chrono::Utc;
use uuid::Uuid;

/// Event creation builder
///
/// # Examples
///
/// ## Create an event with overridden entity ID
///
/// Event entity IDs are usually generated automatically. This example creates an event with an
/// overridden entity ID using the `.entity_id()` method.
///
/// ```rust
/// use event_sauce::{prelude::*, AggregateCreate, Event, CreateEventBuilder};
/// use uuid::Uuid;
///
/// #[derive(event_sauce_derive::Entity)]
/// #[event_sauce(entity_name = "users")]
/// struct User {
///     #[event_sauce(id)]
///     id: Uuid,
///
///     // ...
/// }
///
/// impl AggregateCreate<UserCreated> for User {
///     type Error = &'static str;
///
///     fn try_aggregate_create(event: &Event<UserCreated>) -> Result<Self, Self::Error> {
///         Ok(User {
///             id: event.entity_id,
///
///             // ...
///         })
///     }
/// }
///
/// #[derive(
///     serde_derive::Serialize,
///     serde_derive::Deserialize,
///     event_sauce_derive::CreateEventData,
///     Clone,
///     Debug,
/// )]
/// #[event_sauce(User)]
/// struct UserCreated {
///     // ...
/// }
///
/// let user_id = Uuid::new_v4();
///
/// let event = CreateEventBuilder::new(UserCreated {}).entity_id(user_id).build();
///
/// assert_eq!(event.entity_id, user_id);
/// ```
pub struct CreateEventBuilder<D: EventData> {
    payload: D,
    session_id: Option<Uuid>,
    entity_id: Uuid,
}

impl<D> CreateEventBuilder<D>
where
    D: EventData,
{
    /// Create a new builder with a given entity ID
    pub fn new_with_entity_id(payload: D, entity_id: Uuid) -> Self {
        Self::new(payload).entity_id(entity_id)
    }

    /// Set a custom entity ID for the event
    pub fn entity_id(mut self, entity_id: Uuid) -> Self {
        self.entity_id = entity_id;

        self
    }

    /// Consume the builder and produce the final event
    pub fn build(self) -> Event<D> {
        Event {
            id: Uuid::new_v4(),
            event_type: D::event_type(&self.payload),
            entity_type: D::Entity::entity_type(),
            entity_id: self.entity_id,
            session_id: self.session_id,
            purger_id: None,
            created_at: Utc::now(),
            purged_at: None,
            data: Some(self.payload),
        }
    }
}

impl<D> EventBuilder<D> for CreateEventBuilder<D>
where
    D: EventData,
{
    /// Create a new event builder with a given event data payload
    fn new(payload: D) -> Self {
        Self {
            payload,
            session_id: None,
            entity_id: Uuid::new_v4(),
        }
    }

    /// Set the session ID field of the event
    fn session_id(mut self, session_id: Uuid) -> Self {
        self.session_id = Some(session_id);

        self
    }
}

impl<D> From<D> for CreateEventBuilder<D>
where
    D: EventData,
{
    fn from(payload: D) -> Self {
        Self::new(payload)
    }
}
