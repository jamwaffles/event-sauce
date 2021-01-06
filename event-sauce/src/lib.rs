//! # `event-sauce`
//!
//! [![Build Status](https://circleci.com/gh/jamwaffles/event-sauce/tree/master.svg?style=shield)](https://circleci.com/gh/jamwaffles/event-sauce/tree/master)
//! [![Crates.io](https://img.shields.io/crates/v/event-sauce.svg)](https://crates.io/crates/event-sauce)
//! [![Docs.rs](https://docs.rs/event-sauce/badge.svg)](https://docs.rs/event-sauce)
//!
//! Core crate following the event sourcing paradigm.

#![deny(missing_docs)]
#![deny(broken_intra_doc_links)]

mod db_event;
mod event;
mod event_builder;
pub mod prelude;
mod triggers;

pub use crate::{
    db_event::DBEvent,
    event::Event,
    event_builder::{
        CreateEventBuilder, DeleteEventBuilder, EventBuilder, PurgeEventBuilder, UpdateEventBuilder,
    },
    triggers::{OnCreated, OnUpdated},
};
use serde::Serialize;
use uuid::Uuid;

/// An entity to apply events to
pub trait Entity {
    /// The type of this entity as a plural `underscore_case` string
    const ENTITY_TYPE: &'static str;

    /// Get the `EVENT_TYPE` as a `String`
    fn entity_type() -> String {
        Self::ENTITY_TYPE.to_string()
    }

    /// Get the ID of this entity
    fn entity_id(&self) -> Uuid;
}

/// An event's data payload
pub trait EventData: Serialize + Sized {
    /// The entity to bind this event to
    type Entity: Entity;

    /// The type of builder this event can be used with
    type Builder: EventBuilder<Self>;

    /// Get the event type/identifier in PascalCase like `UserCreated` or `PasswordChanged`
    fn event_type(&self) -> &'static str;

    /// Convert the event into a builder with a given session ID
    ///
    /// This is a convenience method to shorten `Event {}.into_builder().session_id(id)` to
    /// `Event {}.with_session_id(id)`.
    fn with_session_id(self, session_id: Uuid) -> Self::Builder {
        Self::Builder::new(self).session_id(session_id)
    }

    /// Convert the event into a builder
    fn into_builder(self) -> Self::Builder {
        Self::Builder::new(self)
    }
}

/// A trait implemented for any item that can be persisted to a backing store
#[async_trait::async_trait]
pub trait Persistable<Txn, Out = Self>: Sized
where
    Txn: StorageBackendTransaction,
{
    /// Save or update the entity
    ///
    /// This method must be idempotent.
    async fn persist(self, store: &mut Txn) -> Result<Out, Txn::Error>;
}

/// Implemented for all entities that can be removed or otherwise marked as deleted in the database
#[async_trait::async_trait]
pub trait Deletable<Txn>
where
    Txn: StorageBackendTransaction,
{
    /// Delete an entity
    ///
    /// Implementations of this method may either remove the entity from the database entirely, set
    /// a `deleted_at` column to the current time, or something else.
    /// Event data for the entity must always be retained. To fully delete the entity and any event
    /// data associated with it (to comply with the GDPR for example), see the [`PurgeEventBuilder`] and [`PurgeBuilderExecute`] traits.
    async fn delete(self, store: &mut Txn) -> Result<(), Txn::Error>;
}

/// Add the ability to create a new entity from a given event
pub trait AggregateCreate<ED>: Sized
where
    ED: EventData,
{
    /// The error type to return when the entity could not be created
    type Error;

    /// Attempt to create a new entity from an event
    fn try_aggregate_create(event: &Event<ED>) -> Result<Self, Self::Error>;
}

/// Add the ability to update an existing entity from a given event
pub trait AggregateUpdate<ED>: Sized
where
    ED: EventData,
{
    /// The error type to return when the entity could not be updated
    type Error;

    /// Attempt to apply the passed event to this entity
    fn try_aggregate_update(self, event: &Event<ED>) -> Result<Self, Self::Error>;
}

/// Add the ability to delete an entity
pub trait AggregateDelete<ED>: Sized
where
    ED: EventData,
{
    /// The error type to return when the entity could not be updated
    type Error;

    /// Attempt to apply the passed event to this entity
    ///
    /// The default implementation of this method is a noop and returns `Ok(self)`.
    ///
    /// If the entity's implementation of [`Deletable`] removes it from the database entirely, the
    /// implementation of this method should not update `self` and should instead simply return
    /// `Ok(self)` as any updates will not be applied, and will be lost on deletion.
    ///
    /// If the entity's [`Deletable`] implementation sets a deleted flag or does not otherwise
    /// delete the entire row, use this method to update the entity.
    fn try_aggregate_delete(self, _event: &Event<ED>) -> Result<Self, Self::Error> {
        Ok(self)
    }
}

/// A wrapper trait around [`AggregateCreate`] to handle event-sauce integration boilerplate
pub trait CreateEntityBuilder<ED>: AggregateCreate<ED>
where
    ED: EventData,
{
    /// Create a new entity with an event
    fn try_create<B>(builder: B) -> Result<StorageBuilder<Self, ED>, Self::Error>
    where
        B: Into<CreateEventBuilder<ED>>,
    {
        let event = builder.into().build();

        let entity = Self::try_aggregate_create(&event)?;

        Ok(StorageBuilder::new(entity, event))
    }
}

/// A wrapper trait around [`AggregateUpdate`] to handle event-sauce integration boilerplate
pub trait UpdateEntityBuilder<ED>: AggregateUpdate<ED> + Entity
where
    ED: EventData,
{
    /// Update the entity with an event
    fn try_update<B>(self, builder: B) -> Result<StorageBuilder<Self, ED>, Self::Error>
    where
        B: Into<UpdateEventBuilder<ED>>,
    {
        let event = builder.into().build_with_entity_id(self.entity_id());

        let entity = self.try_aggregate_update(&event)?;

        Ok(StorageBuilder::new(entity, event))
    }
}

/// A wrapper trait around [`AggregateDelete`] to handle event-sauce integration boilerplate
pub trait DeleteEntityBuilder<ED>: AggregateDelete<ED> + Entity
where
    ED: EventData,
{
    /// Mark the entity for deletion
    fn try_delete<B>(self, builder: B) -> Result<DeleteBuilder<Self, ED>, Self::Error>
    where
        B: Into<DeleteEventBuilder<ED>>,
    {
        let event = builder.into().build_with_entity_id(self.entity_id());

        let entity = self.try_aggregate_delete(&event)?;

        Ok(DeleteBuilder::new(entity, event))
    }
}

/// Trait to provide a PurgeBuilder to any Entity
pub trait PurgeEntityBuilder<ED>: Entity + Sized
where
    ED: EventData,
{
    /// Creates a PurgeBuilder
    fn try_purge<B>(self, builder: B) -> PurgeBuilder<Self, ED>
    where
        B: Into<PurgeEventBuilder<ED>>,
    {
        let event = builder.into().build_with_entity_id(self.entity_id());

        PurgeBuilder::new(self, event)
    }
}

/// Implemented for all backend storage providers (Postgres, etc)
#[async_trait::async_trait]
pub trait StorageBackend {
    /// The type of error returned from the storage backend
    type Error;

    /// Transaction type
    type Transaction: StorageBackendTransaction;

    // /// DOCS
    // async fn transaction(&self) -> Result<Self::Transaction, Self::Error>;
}

/// Storage backend transaction
#[async_trait::async_trait]
pub trait StorageBackendTransaction {
    /// DOCS
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

/// A wrapper around a tuple of event and entity, used to delete an entity in the database
pub struct DeleteBuilder<Ent, ED: EventData> {
    /// Deletion event to persist
    pub event: Event<ED>,

    /// Entity to delete
    pub entity: Ent,
}

impl<ED, Ent> DeleteBuilder<Ent, ED>
where
    ED: EventData,
{
    /// Create a new entity/event pair
    pub fn new(entity: Ent, event: Event<ED>) -> Self {
        Self { event, entity }
    }
}

/// DOCS
#[async_trait::async_trait]
pub trait StorageBuilderPersist<S, E>
where
    S: StorageBackend,
    E: Persistable<S::Transaction, E>,
{
    /// Stage a deletion in a given transaction
    async fn stage_persist(self, tx: &mut S::Transaction) -> Result<E, S::Error>;

    /// Delete immediately
    async fn persist(self, store: &S) -> Result<E, S::Error>;
}

/// DOCS
#[async_trait::async_trait]
pub trait DeleteBuilderPersist<S>
where
    S: StorageBackend,
{
    /// Stage a deletion in a given transaction
    async fn stage_delete(self, tx: &mut S::Transaction) -> Result<(), S::Error>;

    /// Delete immediately
    async fn delete(self, store: &S) -> Result<(), S::Error>;
}

/// A wrapper around a tuple of event and entity, used to purge an entity in the database
pub struct PurgeBuilder<Ent: Entity, ED: EventData> {
    /// Purge event to persist
    pub event: Event<ED>,
    /// The entity to purge
    pub entity: Ent,
}

impl<ED, Ent> PurgeBuilder<Ent, ED>
where
    ED: EventData,
    Ent: Entity,
{
    /// Create a new entity/event pair
    pub fn new(entity: Ent, event: Event<ED>) -> Self {
        Self { event, entity }
    }
}

/// Helper trait to purge entities
///
/// This must be implemented for [`PurgeBuilder`] by backend storages to allow entity purge.
///
/// The implementation must:
///
/// - remove all eventdata for all events with the entity_id provided, but preserve all other event data.
/// - remove the entity aggregation
/// - insert the purge event in the store. The event data of the purge event must be preserved.
#[async_trait::async_trait]
pub trait PurgeBuilderExecute<S>
where
    S: StorageBackend,
{
    /// Stage the entity purge in a given transaction
    async fn stage_purge(self, tx: &mut S::Transaction) -> Result<(), S::Error>;

    /// Purge an entity
    async fn purge(self, store: &S) -> Result<(), S::Error>;
}
