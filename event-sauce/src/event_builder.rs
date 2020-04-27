//! Event builder

use crate::{Event, EventData};
use uuid::Uuid;

/// Event builder
///
/// # Examples
///
/// ## Create an event with overridden entity ID
///
/// Event entity IDs are usually generated inside calls to [`EventData::into_event`]
/// (crate::EventData::into_event). This example creates an event with an entity ID that is created
/// outside the event instead.
///
/// ```rust
/// use event_sauce::{AggregateCreate, Event, EventBuilder};
/// use uuid::Uuid;
///
/// #[derive(event_sauce_derive::Entity)]
/// #[event_sauce(entity_name = "users")]
/// struct User {
///     // ...
/// }
///
/// impl AggregateCreate<UserCreated> for User {
///     type Error = &'static str;
///
///     fn try_aggregate_create(_event: &Event<UserCreated>) -> Result<Self, Self::Error> {
///         Ok(User {
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
/// let event = EventBuilder::new(UserCreated {}).entity_id(user_id).build();
///
/// assert_eq!(event.entity_id, user_id);
/// ```
pub struct EventBuilder<D: EventData> {
    event: Event<D>,
}

impl<D> EventBuilder<D>
where
    D: EventData,
{
    /// Create a new event builder with a given event data payload
    pub fn new(payload: D) -> Self {
        Self {
            event: payload.into_event(None),
        }
    }

    /// Set the session ID field of the event
    pub fn session_id(mut self, session_id: Uuid) -> Self {
        self.event.session_id = Some(session_id);

        self
    }

    /// Set a custom entity ID for the event
    pub fn entity_id(mut self, entity_id: Uuid) -> Self {
        self.event.entity_id = entity_id;

        self
    }

    /// Consume the builder and produce the final event
    pub fn build(self) -> Event<D> {
        self.event
    }
}
