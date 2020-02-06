//! Event store module

#[macro_use]
extern crate diesel;

mod db_event;
mod event;
mod schema;
mod triggers;

use crate::{
    db_event::DBEvent,
    event::Event,
    schema::events,
    triggers::{OnCreated, OnUpdated},
};
use diesel::{
    pg::PgConnection,
    prelude::*,
    query_builder::{AsChangeset, InsertStatement},
    query_dsl::methods::LoadQuery,
    r2d2::{ConnectionManager, Pool},
};
use log::error;
use serde::{de::Deserialize, Serialize};
use std::{convert::TryInto, error::Error};
use uuid::Uuid;

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
    /// This event will be applied on top of the entity by the [`EventStore`].
    fn from_update_payload(data: ED, entity: &Self::Entity, session_id: Option<Uuid>) -> Event<ED> {
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
    fn from_delete_payload(data: ED, entity: &Self::Entity, session_id: Option<Uuid>) -> Event<ED> {
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
pub trait Aggregate: Sized + AsChangeset {
    /// Insert or update the current entity
    fn persist(&self, conn: &PgConnection) -> Result<Self, diesel::result::Error>;
}

/// Delete an entity from the backing store
pub trait AggregateDelete: Sized + AsChangeset {
    /// Remove the aggregated entity from its table
    ///
    /// This could be implemented as a deletion from the table, or the addition of a "deleted at"
    /// timestamp in the appropriate column.
    fn delete(self, conn: &PgConnection) -> Result<(), diesel::result::Error>;
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
pub struct EventStore {
    /// Postgres database connection
    connection: Pool<ConnectionManager<PgConnection>>,
}

impl EventStore {
    /// Create a new event store instance
    pub fn new(connection: Pool<ConnectionManager<PgConnection>>) -> EventStore {
        EventStore { connection }
    }

    /// Create a new entity `E` given an event with payload `ED`
    pub fn create<ED, E, S>(&self, event: Event<ED>) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        S: Table,
        E: Aggregate + AggregateCreate<ED> + Insertable<S> + OnCreated<ED>,
        InsertStatement<S, E::Values>: LoadQuery<PgConnection, E>,
    {
        self.create_raw(&event.try_into()?)
    }

    /// Create a new entity from a raw [`DBEvent`]
    ///
    /// The [`EventStore::create`] method should be preferred. This method is used to ingest legacy
    /// events during a migration. The [`DBEvent`] is inserted into the event log verbatim without
    /// any payload shape checks.
    pub fn create_raw<ED, E, S>(&self, db_event: &DBEvent) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        S: Table,
        E: Aggregate + AggregateCreate<ED> + Insertable<S> + OnCreated<ED>,
        InsertStatement<S, E::Values>: LoadQuery<PgConnection, E>,
    {
        let conn = self.connection.get()?;
        let created_entity = conn.transaction::<E, Box<dyn Error>, _>(|| {
            let db_event = diesel::insert_into(events::table)
                .values(db_event)
                .on_conflict(events::dsl::id)
                .do_update()
                .set(db_event)
                .get_result::<DBEvent>(&conn)?;

            let state = E::new(db_event.try_into()?)?;

            let result = state.persist(&conn)?;

            Ok(result)
        })?;

        // Trigger side effect. Log and swallow error on failure.
        match created_entity.on_created() {
            Ok(_) => (),
            Err(e) => error!(
                "Failed to trigger creation side effect for event {} (ID {}): {:?}",
                db_event.event_type, db_event.id, e
            ),
        };

        Ok(created_entity)
    }

    /// Apply an event onto a given entity
    pub fn update<ED, E>(&self, state: E, event: Event<ED>) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        E: Aggregate + AggregateApply<ED> + OnUpdated<ED>,
    {
        self.update_raw(state, &event.try_into()?)
    }

    /// Apply a raw [`DBEvent`] event onto a given entity
    ///
    /// The [`EventStore::update`] method should be preferred. This method is used to ingest legacy
    /// events during a migration. The [`DBEvent`] is inserted into the event log verbatim without
    /// any payload shape checks.
    pub fn update_raw<ED, E>(&self, state: E, db_event: &DBEvent) -> Result<E, Box<dyn Error>>
    where
        ED: EventData,
        E: Aggregate + AggregateApply<ED> + OnUpdated<ED>,
    {
        let conn = self.connection.get()?;

        let updated_entity = conn.transaction::<E, Box<dyn Error>, _>(|| {
            let db_event = diesel::insert_into(events::table)
                .values(db_event)
                .on_conflict(events::dsl::id)
                .do_update()
                .set(db_event)
                .get_result::<DBEvent>(&conn)?;

            let state: E = state.apply(db_event.try_into()?)?;

            let result = state.persist(&conn)?;

            Ok(result)
        })?;

        // Trigger side effect. Log and swallow error on failure.
        match updated_entity.on_updated() {
            Ok(_) => (),
            Err(e) => error!(
                "Failed to trigger update side effect for event {} (ID {}): {:?}",
                db_event.event_type, db_event.id, e
            ),
        };

        Ok(updated_entity)
    }

    /// Delete an entity using a given event
    ///
    /// As mentioned in [`FromDeletePayload`], how this is applied is dependent on the entity's
    /// [`AggregateDelete`] implementation. It could remove the record from the database, or add a
    /// "deleted at" timestamp to an appropriate column.
    pub fn delete<ED, E>(&self, state: E, event: Event<ED>) -> Result<(), Box<dyn Error>>
    where
        ED: EventData,
        E: AggregateDelete,
    {
        self.delete_raw::<ED, E>(state, &event.try_into()?)
    }

    /// Delete an entity using a [`DBEvent`]
    ///
    /// The [`EventStore::delete`] method should be preferred. This method is used to ingest legacy
    /// events during a migration. The [`DBEvent`] is inserted into the event log verbatim without
    /// any payload shape checks.
    pub fn delete_raw<ED, E>(&self, state: E, db_event: &DBEvent) -> Result<(), Box<dyn Error>>
    where
        ED: EventData,
        E: AggregateDelete,
    {
        let conn = self.connection.get()?;

        conn.transaction::<(), Box<dyn Error>, _>(|| {
            let _db_event = diesel::insert_into(events::table)
                .values(db_event)
                .on_conflict(events::dsl::id)
                .do_update()
                .set(db_event)
                .get_result::<DBEvent>(&conn)?;

            state.delete(&conn)?;

            Ok(())
        })?;

        Ok(())
    }
}
