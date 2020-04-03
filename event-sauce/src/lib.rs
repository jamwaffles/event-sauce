//! Event store module

// #![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]

mod db_event;
mod event;
mod triggers;

pub use crate::db_event::DBEvent;
pub use crate::event::Event;
pub use crate::triggers::{OnCreated, OnUpdated};
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use uuid::Uuid;

pub trait EventData: Serialize + for<'de> Deserialize<'de> {
    /// The type of this event as a `PascalCase` string
    const EVENT_TYPE: &'static str;

    /// The type of this entity as a plural `underscore_case` string
    const ENTITY_TYPE: &'static str;

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

pub trait Entity {}

/// TODO: Docs
#[async_trait::async_trait]
pub trait Persistable<Store, Out>: Sized
where
    Store: StorageBackend,
{
    /// TODO: Docs
    async fn persist(self, store: &Store) -> Result<Out, Store::Error>;
}

// TODO: Better error type here
#[derive(Debug)]
pub struct CreateEntityError;
impl Error for CreateEntityError {}
impl fmt::Display for CreateEntityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(TODO debug {:?})", self)
    }
}

// TODO: Better error type here
#[derive(Debug)]
pub struct UpdateEntityError;
impl Error for UpdateEntityError {}
impl fmt::Display for UpdateEntityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(TODO debug {:?})", self)
    }
}

pub trait CreateEntity<ED>: Sized
where
    ED: EventData,
{
    fn try_create(event: Event<ED>) -> Result<StorageBuilder<Self, ED>, &'static str>;
}

pub trait UpdateEntity<ED>: Sized
where
    ED: EventData,
{
    fn try_update(self, event: Event<ED>) -> Result<StorageBuilder<Self, ED>, &'static str>;
}

pub trait StorageBackend {
    type Error;
}

pub struct StorageBuilder<Ent, ED: EventData> {
    pub event: Event<ED>,
    pub entity: Ent,
}

impl<ED, Ent> StorageBuilder<Ent, ED>
where
    ED: EventData,
{
    pub fn new(entity: Ent, event: Event<ED>) -> Self {
        Self { event, entity }
    }
}
