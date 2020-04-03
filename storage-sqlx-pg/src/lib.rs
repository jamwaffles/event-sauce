// #![deny(missing_docs)]
#![deny(intra_doc_link_resolution_failure)]

use event_sauce::DBEvent;
use event_sauce::Event;
use event_sauce::EventData;
use event_sauce::Persistable;
use event_sauce::StorageBackend;
use event_sauce::StorageBuilder;
use sqlx::PgPool;
use std::convert::TryInto;
use std::future::Future;

pub struct SqlxPgStore {
    pool: PgPool,
}

impl SqlxPgStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl StorageBackend for SqlxPgStore {
    type Error = sqlx::Error;
}

#[async_trait::async_trait]
impl Persistable<SqlxPgStore, DBEvent> for DBEvent {
    async fn persist(self, store: &SqlxPgStore) -> Result<Self, sqlx::Error> {
        sqlx::query("insert into events () values ()")
            .execute(&store.pool)
            .await?;

        Ok(self)
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
        let db_event: DBEvent = self.event.try_into().unwrap();

        db_event.persist(&store).await?;

        let ent = self.entity.persist(&store).await?;

        Ok(ent)
    }
}
