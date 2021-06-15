mod db_event;
mod event;
mod event_builder;

pub use db_event::DBEvent;
pub use event::Event;
use event_builder::EventBuilder;
use serde::Serialize;
use uuid::Uuid;

/// An entity to apply events to.
pub trait Entity {
    /// The type of this entity as a plural `underscore_case` string.
    const ENTITY_TYPE: &'static str;

    /// Get the ID of this entity.
    fn entity_id(&self) -> Uuid;
}

pub trait EventData: Serialize + Sized {
    /// The entity to bind this event to
    type Entity: Entity;

    // /// The type of builder this event can be used with
    // type Builder: EventBuilder<Self>;

    /// Get the event type/identifier in PascalCase like `UserCreated` or `PasswordChanged`.
    fn event_type() -> &'static str;

    fn into_builder(self) -> EventBuilder<Self> {
        EventBuilder::from(self)
    }

    // /// Convert the event into a builder with a given session ID
    // ///
    // /// This is a convenience method to shorten `Event {}.into_builder().session_id(id)` to
    // /// `Event {}.with_session_id(id)`.
    // fn with_session_id(self, session_id: Uuid) -> Self::Builder {
    //     Self::Builder::new(self).session_id(session_id)
    // }

    // /// Convert the event into a builder
    // fn into_builder(self) -> Self::Builder {
    //     Self::Builder::new(self)
    // }

    // /// Wrap this event data in an [`Event`] and assign the given entity ID.
    // fn with_entity_id(self, entity_id: Uuid) -> Event<Self> {
    //     Event {
    //         entity_id,
    //         ..Event::from(self)
    //     }
    // }

    //   /// Wrap this event data in an [`Event`] and assign the given entity ID.
    // fn with_session_id(self, entity_id: Uuid) -> Event<Self> {
    //     Event {
    //         entity_id,
    //         ..Event::from(self)
    //     }
    // }
}

pub trait Create<D>: Entity + Sized
where
    D: EventData,
{
    /// Create an instance of this entity from the given event.
    fn create_from(event: &Event<D>) -> Self;
}

/// Create a [`Persister`] from an event.
///
/// This trait may not be implemented by consuming code. See the [`Create`] trait instead.
pub trait CreatePersister<D>: Create<D>
where
    D: EventData,
{
    fn create(event: impl Into<Event<D>>) -> Persister<D, Self> {
        let event = event.into();

        let entity = Self::create_from(&event);

        Persister { entity, event }
    }
}

/// Blanket impl
impl<D, C> CreatePersister<D> for C
where
    C: Create<D>,
    D: EventData,
{
}

#[non_exhaustive]
pub struct Persister<D, E>
where
    D: EventData,
    E: Entity,
{
    pub event: Event<D>,
    pub entity: E,
}

pub trait Persistable<S, Out = Self>
where
    S: Storage,
{
    fn persist(self, storage: &mut S) -> Result<Out, S::Error>;
}

pub trait Storage {
    type Error;
}
