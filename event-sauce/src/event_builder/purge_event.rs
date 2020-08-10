//! Event builder

use crate::{Entity, Event, EventBuilder, EventData};
use chrono::Utc;
use uuid::Uuid;

/// Delete event builder
///
/// Build an [`Event`] from a session id, used to purge entities. Purging deletes the entities.
/// It keeps the event history but sets the `data` property of all events related to the entity to None.
///
/// # Examples
///
/// ## Build a purge event with session ID
///
/// ```rust
/// # fn main() -> Result<(), ()> {
/// use event_sauce::{prelude::*, Event, PurgeEventBuilder, PurgeEntityBuilder, AggregatePurge};
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
///     event_sauce_derive::PurgeEventData,
///     Clone,
///     Debug,
/// )]
/// #[event_sauce(User)]
/// struct UserPurged;
///
///
/// impl AggregatePurge<UserPurged> for User {
///     type Error = ();
/// }
/// let session_id = Uuid::new_v4();
///
/// let user = User { id: Uuid::new_v4() };
///
/// user.try_purge(UserPurged {}.with_session_id(session_id))?;
/// # Ok(()) }
/// ```
pub struct PurgeEventBuilder<D: EventData> {
    session_id: Option<Uuid>,
    payload: D,
}

impl<D: EventData> PurgeEventBuilder<D> {
    /// Consume the builder and produce the final event
    pub fn build(self, entity: &D::Entity) -> Event<D> {
        self.build_with_entity_id(entity.entity_id())
    }

    /// Workaround method to get an entity ID out of entities when implementing
    /// `DeleteEntityBuilder`
    pub(crate) fn build_with_entity_id(self, entity_id: Uuid) -> Event<D> {
        Event {
            data: None,
            id: Uuid::new_v4(),
            event_type: D::event_type(),
            entity_type: D::Entity::entity_type(),
            entity_id,
            session_id: self.session_id,
            purger_id: self.session_id,
            created_at: Utc::now(),
            purged_at: Some(Utc::now()),
        }
    }

    fn new(payload: D) -> Self {
        Self {
            session_id: None,
            payload,
        }
    }
}

impl<D> EventBuilder<D> for PurgeEventBuilder<D>
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

impl<D> From<D> for PurgeEventBuilder<D>
where
    D: EventData,
{
    fn from(payload: D) -> Self {
        Self::new(payload)
    }
}
