//! # Event sauce SQLX storage backend
//!
//! [![Build Status](https://circleci.com/gh/jamwaffles/event-sauce/tree/master.svg?style=shield)](https://circleci.com/gh/jamwaffles/event-sauce/tree/master)
//! [![Docs.rs](https://docs.rs/event-sauce-storage-sqlx/badge.svg)](https://docs.rs/event-sauce-storage-sqlx)
//!
//! [sqlx](https://crates.io/crates/sqlx) storage adapter for event-sauce-storage-sqlx.
//!
//! ## Features
//!
//! - `with-postgres` (enabled by default) - Enable support for Postgres databases by exposing the `SqlxPgStore` storage adapter.

#![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]

use event_sauce::{
    DBEvent, Deletable, DeleteBuilder, EventData, Persistable, StorageBackend, StorageBuilder,
};
use sqlx::{postgres::PgQueryAs, PgPool};
use std::convert::TryInto;

/// [sqlx](https://docs.rs/sqlx)-based Postgres backing store
pub struct SqlxPgStore {
    /// sqlx [`PgPool`](sqlx::PgPool) to commnicate with the database
    pub pool: PgPool,
}

impl SqlxPgStore {
    /// Create a new backing store instance with a given [`PgPool`](sqlx::PgPool)
    pub async fn new(pool: PgPool) -> Result<Self, sqlx::Error> {
        Self::create_events_table(&pool).await?;

        Ok(Self { pool })
    }

    async fn create_events_table(pool: &PgPool) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        sqlx::query(r#"create extension if not exists "uuid-ossp";"#)
            .execute(&mut tx)
            .await?;

        sqlx::query(r#"
            create table if not exists events(
                id uuid primary key,
                sequence_number serial,
                event_type varchar(64) not null,
                entity_type varchar(64) not null,
                entity_id uuid not null,
                -- This field is null if the event is purged, in such case purged_at and purger_id should be populated.
                data jsonb,
                session_id uuid null,
                created_at timestamp with time zone not null,
                purger_id uuid null,
                purged_at timestamp with time zone null
            );
        "#).execute(&mut tx).await?;

        tx.commit().await?;

        Ok(())
    }
}

impl StorageBackend for SqlxPgStore {
    type Error = sqlx::Error;
}

#[async_trait::async_trait]
impl Persistable<SqlxPgStore, DBEvent> for DBEvent {
    async fn persist(self, store: &SqlxPgStore) -> Result<Self, sqlx::Error> {
        let saved: Self = sqlx::query_as(
            r#"insert into events (
                id,
                event_type,
                entity_type,
                entity_id,
                data,
                session_id,
                created_at
            ) values (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                $7
            )
            on conflict (id)
            do update set
            data = excluded.data
            returning *"#,
        )
        .bind(self.id)
        .bind(self.event_type)
        .bind(self.entity_type)
        .bind(self.entity_id)
        .bind(self.data)
        .bind(self.session_id)
        .bind(self.created_at)
        .fetch_one(&store.pool)
        .await?;

        log::trace!("Persisted event {}: {:?}", saved.id, saved);

        Ok(saved)
    }
}

#[async_trait::async_trait]
impl<Ent, ED> Persistable<SqlxPgStore, Ent> for StorageBuilder<Ent, ED>
where
    ED: EventData + std::marker::Send,
    Ent: Persistable<SqlxPgStore, Ent> + std::marker::Send,
{
    async fn persist(self, store: &SqlxPgStore) -> Result<Ent, sqlx::Error> {
        // TODO: Enum error type to handle this unwrap
        let db_event: DBEvent = self
            .event
            .try_into()
            .expect("Failed to convert Event into DBEvent");

        db_event.persist(&store).await?;

        self.entity.persist(&store).await
    }
}

#[async_trait::async_trait]
impl<Ent, ED> Deletable<SqlxPgStore> for DeleteBuilder<Ent, ED>
where
    ED: EventData + std::marker::Send,
    Ent: Deletable<SqlxPgStore> + std::marker::Send,
{
    async fn delete(self, store: &SqlxPgStore) -> Result<(), sqlx::Error> {
        // TODO: Enum error type to handle this unwrap
        let db_event: DBEvent = self
            .event
            .try_into()
            .expect("Failed to convert Event into DBEvent");

        db_event.persist(&store).await?;

        println!("Event persisted");

        self.entity.delete(&store).await
    }
}
