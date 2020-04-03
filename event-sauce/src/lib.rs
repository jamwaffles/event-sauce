//! Event store core crate

#![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]

mod db_event;
mod event;
mod triggers;

pub use crate::{
    db_event::DBEvent,
    event::Event,
    triggers::{OnCreated, OnUpdated},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An event's data payload
pub trait EventData: Serialize + for<'de> Deserialize<'de> {
    /// The type of this event as a `PascalCase` string
    const EVENT_TYPE: &'static str;

    /// The type of this entity as a plural `underscore_case` string
    const ENTITY_TYPE: &'static str;

    /// Get the `EVENT_TYPE` as a `String`
    fn entity_type() -> String {
        Self::ENTITY_TYPE.to_string()
    }

    /// Get the event type/identifier in PascalCase like `UserCreated` or `PasswordChanged`
    fn event_type() -> String {
        Self::EVENT_TYPE.to_string()
    }

    /// Wrap the payload in an [`Event`] with default values for other fields
    fn into_event(self, session_id: Option<Uuid>) -> Event<Self> {
        Event {
            data: Some(self),
            id: Uuid::new_v4(),
            event_type: Self::event_type(),
            entity_type: Self::entity_type(),
            entity_id: Uuid::new_v4(),
            session_id,
            purger_id: None,
            created_at: Utc::now(),
            purged_at: None,
        }
    }
}

/// A trait implemented for any item that can be persisted to a backing store
#[async_trait::async_trait]
pub trait Persistable<Store, Out>: Sized
where
    Store: StorageBackend,
{
    /// Save or update the entity
    ///
    /// This method must be idempotent.
    async fn persist(self, store: &Store) -> Result<Out, Store::Error>;
}

/// Add the ability to create a new entity from a given event
pub trait AggregateCreate<ED>: Sized
where
    ED: EventData,
{
    /// Attempt to create a new entity from an event
    fn try_aggregate_create(event: &Event<ED>) -> Result<Self, &'static str>;
}

/// Add the ability to update an existing entity from a given event
pub trait AggregateUpdate<ED>: Sized
where
    ED: EventData,
{
    /// Attempt to apply the passed event to this entity
    fn try_aggregate_update(self, event: &Event<ED>) -> Result<Self, &'static str>;
}

/// A wrapper trait around [`AggregateCreate`] to handle event-sauce integration boilerplate
pub trait CreateEntityBuilder<ED>: AggregateCreate<ED>
where
    ED: EventData,
{
    /// Create a new entity with an event
    fn try_create(event: Event<ED>) -> Result<StorageBuilder<Self, ED>, &'static str> {
        let entity = Self::try_aggregate_create(&event)?;

        Ok(StorageBuilder::new(entity, event))
    }
}

/// A wrapper trait around [`AggregateUpdate`] to handle event-sauce integration boilerplate
pub trait UpdateEntityBuilder<ED>: AggregateUpdate<ED>
where
    ED: EventData,
{
    /// Update the entity with an event
    fn try_update(self, event: Event<ED>) -> Result<StorageBuilder<Self, ED>, &'static str> {
        let entity = self.try_aggregate_update(&event)?;

        Ok(StorageBuilder::new(entity, event))
    }
}

/// Implemented for all backend storage providers (Postgres, etc)
pub trait StorageBackend {
    /// The type of error returned from the storage backend
    type Error;
}

/// A wrapper around a tuple of event and entity, used to persist them to the database at the same
/// time.
pub struct StorageBuilder<Ent, ED: EventData> {
    /// Event to persist
    pub event: Event<ED>,

    /// Entity to persist
    pub entity: Ent,
}

impl<ED, Ent> StorageBuilder<Ent, ED>
where
    ED: EventData,
{
    /// Create a new entity/event pair
    pub fn new(entity: Ent, event: Event<ED>) -> Self {
        Self { event, entity }
    }
}
