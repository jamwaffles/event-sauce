//! Event builder

use crate::event_builder::EventBuilder;
use crate::{Entity, Event, EventData};
use chrono::Utc;
use uuid::Uuid;

/// Delete event builder
///
/// Build an [`Event`] from a given payload used to delete entities. The deletion behaviour is
/// defined by the entity's implementation of the [`crate::AggregateDelete`] trait.
///
/// # Examples
///
/// ## Build a deletion even with session ID
///
/// ```rust
/// # fn main() -> Result<(), &'static str> {
/// use event_sauce::{prelude::*, AggregateDelete, Event, DeleteEventBuilder};
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
/// #[derive(
///     serde_derive::Serialize,
///     serde_derive::Deserialize,
///     event_sauce_derive::DeleteEventData,
///     Clone,
///     Debug,
/// )]
/// #[event_sauce(User)]
/// struct UserDeleted;
///
/// impl AggregateDelete<UserDeleted> for User {
///     type Error = &'static str;
///
///     fn try_aggregate_delete(self, event: &Event<UserDeleted>) -> Result<Self, Self::Error> {
///         Ok(self)
///     }
/// }
///
/// let user = User { id: Uuid::new_v4() };
///
/// let updated = user.try_delete(UserDeleted)?;
/// # Ok(()) }
/// ```
pub struct DeleteEventBuilder<D: EventData> {
    payload: D,
    session_id: Option<Uuid>,
}

impl<D> DeleteEventBuilder<D>
where
    D: EventData,
{
    /// Consume the builder and produce the final event
    pub fn build(self, entity: &D::Entity) -> Event<D> {
        Event {
            id: Uuid::new_v4(),
            event_type: D::event_type(&self.payload),
            entity_type: D::Entity::entity_type(),
            entity_id: entity.entity_id(),
            session_id: self.session_id,
            purger_id: None,
            created_at: Utc::now(),
            purged_at: None,
            data: Some(self.payload),
        }
    }

    /// Workaround method to get an entity ID out of entities when implementing
    /// `DeleteEntityBuilder`
    pub(crate) fn build_with_entity_id(self, entity_id: Uuid) -> Event<D> {
        Event {
            id: Uuid::new_v4(),
            event_type: D::event_type(&self.payload),
            entity_type: D::Entity::entity_type(),
            entity_id,
            session_id: self.session_id,
            purger_id: None,
            created_at: Utc::now(),
            purged_at: None,
            data: Some(self.payload),
        }
    }
}

impl<D> EventBuilder<D> for DeleteEventBuilder<D>
where
    D: EventData,
{
    /// Create a new event builder with a given event data payload
    fn new(payload: D) -> Self {
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

impl<D> From<D> for DeleteEventBuilder<D>
where
    D: EventData,
{
    fn from(payload: D) -> Self {
        Self::new(payload)
    }
}
