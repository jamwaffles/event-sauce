//! Event store module

#![deny(intra_doc_link_resolution_failure)]

mod db_event;
mod event;
pub mod prelude;
mod triggers;

use crate::db_event::DBEvent;
pub use crate::triggers::{OnCreated, OnUpdated};
use log::error;
use postgres::types::ToSql;
use postgres::NoTls;
use postgres::Transaction;
use r2d2_postgres::PostgresConnectionManager;
use serde::{de::Deserialize, Serialize};
use std::fmt;
use std::{convert::TryInto, error::Error};
use uuid::Uuid;

pub use crate::event::Event;

#[derive(Debug)]
struct PlaceholderError;

impl fmt::Display for PlaceholderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TODO: Better error type")
    }
}

impl std::error::Error for PlaceholderError {
    // TODO
}

fn create_table(
    pool: &mut r2d2::Pool<PostgresConnectionManager<NoTls>>,
) -> Result<(), Box<dyn Error>> {
    pool.get()?.batch_execute(r#"
        create extension if not exists "uuid-ossp";

        create table if not exists events(
            id uuid primary key default uuid_generate_v4(),
            sequence_number serial,
            event_type varchar(64) not null,
            entity_type varchar(64) not null,
            entity_id uuid not null,
            data jsonb, -- This field is null if the event is purged, in such case purged_at and purger_id won't be null either.
            session_id uuid null,
            created_at timestamp with time zone not null,
            purger_id uuid null,
            purged_at timestamp with time zone null
        );
    "#)?;

    Ok(())
}

/// Trait implemented for all event payloads
///
/// **Note:** This is implemented automatically by the `EventData` derive. This should not be
/// implemented by hand.
pub trait EventData: Serialize + for<'de> Deserialize<'de> {
    /// Get the lowercase, plural entity type for this event like `users` or `password_resets`
    fn entity_type() -> String;

    /// Get the event type/identifier in PascalCase like `UserCreated` or `PasswordChanged`
    fn event_type() -> String;
}

/// Trait implemented for all entities
///
/// **Note:** This is implemented automatically by the `EventData` derive. This should not be
/// implemented by hand.
pub trait EntityId {
    /// Retrieve this entity's unique ID
    fn entity_id(&self) -> Uuid;

    /// Get the entity's type as a lowercase, plural string like `users` or `password_resets`
    fn entity_type() -> String;
}

/// Allow an entity creation event to be created from a given payload
pub trait FromCreatePayload<ED>
where
    ED: EventData,
{
    /// Create an entity creation [`Event`] from a given event payload and optional session ID
    fn from_create_payload(data: ED, session_id: Option<Uuid>) -> Event<ED> {
        Event {
            data: Some(data),
            session_id,
            ..Event::default()
        }
    }
}

/// Allow an entity update event to be created from a given payload
pub trait FromUpdatePayload<ED>
where
    ED: EventData,
{
    /// The target entity
    type Entity: EntityId;

    /// Create an update [`Event`] from a given event payload, base entity and optional session ID
    ///
    /// This event will be applied on top of the entity by the [`Store`].
    fn from_update_payload(entity: &Self::Entity, data: ED, session_id: Option<Uuid>) -> Event<ED> {
        Event {
            data: Some(data),
            entity_id: entity.entity_id(),
            session_id,
            ..Event::default()
        }
    }
}

/// Allow an entity delete event to be created from a given payload
///
/// The payload for these events will most often be empty. A deletion event can be created from the
/// [`Event`] struct fields. Other data can be added to the payload if required.
pub trait FromDeletePayload<ED>
where
    ED: EventData,
{
    /// The target entity
    type Entity: EntityId;

    /// Create a deletion [`Event`] from a given event payload and optional session ID
    ///
    /// This event will mark an entity as deleted. How this is applied is dependent on the entity's
    /// [`AggregateDelete`] implementation.
    fn from_delete_payload(entity: &Self::Entity, data: ED, session_id: Option<Uuid>) -> Event<ED> {
        Event {
            data: Some(data),
            entity_id: entity.entity_id(),
            session_id,
            ..Event::default()
        }
    }
}

/// Defines how an entity should be created from a given event
///
/// This trait can be implemented multiple times for multiple creation methods
pub trait AggregateCreate<ED>: Sized
where
    ED: EventData,
{
    /// Create a new entity from an event
    fn new(event: Event<ED>) -> Result<Self, Box<dyn Error>>;
}

/// Defines how an entity should be updated from a given event
pub trait AggregateApply<ED>: Sized
where
    ED: EventData,
{
    /// Apply an update event onto the entity, consuming the entity and returning a new instance
    fn apply(self, event: Event<ED>) -> Result<Self, Box<dyn Error>>;
}

/// Insert or update an entity in the chosen backing store
pub trait Aggregate: Sized {
    type Error;

    /// Insert or update the current entity
    fn persist(&self, conn: &mut Transaction) -> Result<Self, Self::Error>;
}

/// Delete an entity from the backing store
pub trait AggregateDelete: Sized {
    type Error;

    /// Remove the aggregated entity from its table
    ///
    /// This could be implemented as a deletion from the table, or the addition of a "deleted at"
    /// timestamp in the appropriate column.
    fn delete(self, conn: &mut Transaction) -> Result<(), Self::Error>;
}

/// Event store
///
/// Handles creation, updating and persistence of events in the backing store
///
/// # Examples
///
/// ## Derive events and entity for a `User`
///
/// Note that Diesel's `table_name` attribute is required on entity structs. No other database
/// backends are supported at this time.
///
/// ```rust,ignore
/// use uuid::Uuid;
///
/// /// Simple user entity
/// #[derive(event_store_derive::Entity)]
/// #[table_name = "users"]
/// struct User {
///     id: Uuid,
///     name: String,
///     email: String,
///     password: String,
/// }
///
/// /// A user creation event
/// #[derive(
///     serde_derive::Deserialize,
///     serde_derive::Serialize,
///     event_store_derive::CreateEvent,
/// )]
/// #[event_store(User)]
/// pub struct UserCreated {
///     pub user_id: Uuid,
///     pub name: String,
///     pub email: String,
///     pub password: String,
/// }
///
/// #[derive(
///     serde_derive::Deserialize,
///     serde_derive::Serialize,
///     event_store_derive::UpdateEvent,
/// )]
/// #[event_store(User)]
/// pub struct UserEmailChanged {
///     pub email: String
/// }
///
/// let session_id = Uuid::new_v4();
///
/// // Create the user
/// let user = User::from_create_payload(UserCreated {
///     user_id: Uuid::nil(),
///     name: "Bobby Beans".to_string(),
///     email: "bobby@bea.ns".to_string(),
///     password: "Haha this is supposed to be hashed".to_string(),
/// }, Some(session_id));
///
/// // Update the user's email address
/// let user = User::from_update_payload(UserEmailChanged {
///     email: "beans@bob.by".to_string(),
/// }, user, Some(session_id));
///
/// // Check that user was updated
/// assert_eq!(
///     user,
///     User {
///         user_id: Uuid::nil(),
///         name: "Bobby Beans".to_string(),
///         email: "beans@bob.by".to_string(),
///         password: "Haha this is supposed to be hashed".to_string(),
///     }
/// )
/// ```
#[derive(Clone)]
pub struct Store {
    /// Postgres database connection pool
    pool: r2d2::Pool<PostgresConnectionManager<NoTls>>,
}

impl Store {
    /// Create a new event store instance
    pub fn new(
        mut pool: r2d2::Pool<PostgresConnectionManager<NoTls>>,
    ) -> Result<Self, Box<dyn Error>> {
        create_table(&mut pool)?;

        Ok(Store { pool })
    }

    /// Create a new entity `E` given an event with payload `ED`
    pub fn create<ED, E>(&mut self, event: Event<ED>) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        E: Aggregate + AggregateCreate<ED> + OnCreated<ED> + Default,
    {
        self.create_raw(&event.try_into()?)
    }

    fn insert_event(txn: &mut Transaction, db_event: &DBEvent) -> Result<DBEvent, Box<dyn Error>> {
        let inserted = txn
            .query_one(
                r#"INSERT INTO events (
                id,
                event_type,
                entity_type,
                entity_id,
                data,
                session_id,
                created_at,
                purger_id,
                purged_at
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9
            ) RETURNING *
            "#,
                &[
                    &db_event.id as &(dyn ToSql + Sync),
                    &db_event.event_type,
                    &db_event.entity_type,
                    &db_event.entity_id,
                    &db_event.data,
                    &db_event.session_id,
                    &db_event.created_at,
                    &db_event.purger_id,
                    &db_event.purged_at,
                ],
            )
            .map_err(Box::new)?
            .try_into()?;

        Ok(inserted)
    }

    /// Create a new entity from a raw [`DBEvent`]
    ///
    /// The [`Store::create`] method should be preferred. This method is used to ingest legacy
    /// events during a migration. The [`DBEvent`] is inserted into the event log verbatim without
    /// any payload shape checks.
    pub fn create_raw<ED, E>(&mut self, db_event: &DBEvent) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        E: Aggregate + AggregateCreate<ED> + OnCreated<ED> + Default,
    {
        let mut conn = self.pool.get()?;
        let mut transaction = conn.transaction()?;

        // Save event into events table
        let db_event = Self::insert_event(&mut transaction, db_event)?;

        let DBEvent {
            id: event_id,
            event_type,
            ..
        } = db_event.clone();

        // Create a new entity using this event
        let state = E::new(db_event.try_into()?)?;

        // Save the entity into its data store
        let created_entity = state
            .persist(&mut transaction)
            .map_err(|_| Box::new(PlaceholderError))?;

        transaction.commit()?;

        // Trigger side effect. Log and swallow error on failure.
        match created_entity.on_created() {
            Ok(_) => (),
            Err(e) => error!(
                "Failed to trigger creation side effect for event {} (ID {}): {:?}",
                event_type, event_id, e
            ),
        };

        Ok(created_entity)
    }

    /// Apply an event onto a given entity
    pub fn update<ED, E>(&mut self, state: E, event: Event<ED>) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        E: Aggregate + AggregateApply<ED> + OnUpdated<ED>,
    {
        self.update_raw(state, &event.try_into()?)
    }

    /// Apply a raw [`DBEvent`] event onto a given entity
    ///
    /// The [`Store::update`] method should be preferred. This method is used to ingest legacy
    /// events during a migration. The [`DBEvent`] is inserted into the event log verbatim without
    /// any payload shape checks.
    pub fn update_raw<ED, E>(&mut self, entity: E, db_event: &DBEvent) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        E: Aggregate + AggregateApply<ED> + OnUpdated<ED>,
    {
        let mut conn = self.pool.get()?;
        let mut transaction = conn.transaction()?;

        // Save event into events table
        let db_event = Self::insert_event(&mut transaction, db_event)?;

        let DBEvent {
            id: event_id,
            event_type,
            ..
        } = db_event.clone();

        // Update entity in memory
        let entity: E = entity.apply(db_event.try_into()?)?;

        // Save the updated entity into its data store
        let created_entity = entity
            .persist(&mut transaction)
            .map_err(|_| Box::new(PlaceholderError))?;

        transaction.commit()?;

        // Trigger side effect. Log and swallow error on failure.
        match created_entity.on_updated() {
            Ok(_) => (),
            Err(e) => error!(
                "Failed to trigger update side effect for event {} (ID {}): {:?}",
                event_type, event_id, e
            ),
        };

        Ok(created_entity)
    }

    /// Delete an entity using a given event
    ///
    /// As mentioned in [`FromDeletePayload`], how this is applied is dependent on the entity's
    /// [`AggregateDelete`] implementation. It could remove the record from the database, or add a
    /// "deleted at" timestamp to an appropriate column.
    pub fn delete<ED, E>(&mut self, state: E, event: Event<ED>) -> Result<(), Box<dyn Error>>
    where
        ED: EventData,
        E: AggregateDelete,
    {
        self.delete_raw::<ED, E>(state, &event.try_into()?)
    }

    /// Delete an entity using a [`DBEvent`]
    ///
    /// The [`Store::delete`] method should be preferred. This method is used to ingest legacy
    /// events during a migration. The [`DBEvent`] is inserted into the event log verbatim without
    /// any payload shape checks.
    pub fn delete_raw<ED, E>(&mut self, state: E, db_event: &DBEvent) -> Result<(), Box<dyn Error>>
    where
        ED: EventData,
        E: AggregateDelete,
    {
        let mut conn = self.pool.get()?;
        let mut transaction = conn.transaction()?;

        // Save event into events table
        Self::insert_event(&mut transaction, db_event)?;

        state
            .delete(&mut transaction)
            .map_err(|_| Box::new(PlaceholderError))?;

        transaction.commit()?;

        Ok(())
    }
}
