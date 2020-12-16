//! Event builder

use crate::event_builder::EventBuilder;
use crate::{Entity, Event, EventData};
use chrono::Utc;
use uuid::Uuid;

/// Event update builder
///
/// # Examples
///
/// ## Update an event with a set session ID
///
/// Events have no session ID by default. If possible, one should always be set using the
/// [`UpdateEventBuilder::session_id`] method.
///
/// ```rust
/// # fn main() -> Result<(), &'static str> {
/// use event_sauce::{prelude::*, AggregateUpdate, Event, UpdateEventBuilder};
/// use uuid::Uuid;
///
/// #[derive(event_sauce_derive::Entity)]
/// #[event_sauce(entity_name = "users")]
/// struct User {
///     #[event_sauce(id)]
///     id: Uuid,
///
///     login_count: u32,
///
///     // ...
/// }
///
/// #[derive(
///     serde_derive::Serialize,
///     serde_derive::Deserialize,
///     event_sauce_derive::UpdateEventData,
///     Clone,
///     Debug,
/// )]
/// #[event_sauce(User)]
/// struct UserLoggedIn {
///     // ...
/// }
///
/// impl AggregateUpdate<UserLoggedIn> for User {
///     type Error = &'static str;
///
///     fn try_aggregate_update(self, event: &Event<UserLoggedIn>) -> Result<Self, Self::Error> {
///         Ok(User {
///             login_count: self.login_count + 1,
///             ..self
///         })
///     }
/// }
///
/// let session_id = Uuid::new_v4();
///
/// let user_id = Uuid::new_v4();
///
/// let user = User { id: user_id, login_count: 0 };
///
/// let updated = user.try_update(UserLoggedIn {}.with_session_id(session_id))?;
///
/// assert_eq!(updated.entity.login_count, 1);
/// assert_eq!(updated.event.session_id, Some(session_id));
/// assert_eq!(updated.event.entity_id, user_id);
/// # Ok(()) }
/// ```
pub struct UpdateEventBuilder<D: EventData> {
    payload: D,
    session_id: Option<Uuid>,
}

impl<D> UpdateEventBuilder<D>
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
    /// `UpdateEntityBuilder`
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

impl<D> EventBuilder<D> for UpdateEventBuilder<D>
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

impl<D> From<D> for UpdateEventBuilder<D>
where
    D: EventData,
{
    fn from(payload: D) -> Self {
        Self::new(payload)
    }
}
