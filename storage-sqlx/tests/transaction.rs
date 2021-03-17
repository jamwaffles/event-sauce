use event_sauce::{
    prelude::*, AggregateCreate, AggregateDelete, AggregateUpdate, Deletable, Event, Persistable,
};
use event_sauce_storage_sqlx::SqlxPgStoreTransaction;
// use event_sauce::UpdateEntity;
use event_sauce_storage_sqlx::SqlxPgStore;
use sqlx::{postgres::PgQueryAs, PgPool};
use uuid::Uuid;

#[derive(
    serde::Serialize,
    serde::Deserialize,
    sqlx::FromRow,
    event_sauce_derive::Entity,
    PartialEq,
    Debug,
)]
#[event_sauce(entity_name = "crud_test_users_txn")]
struct User {
    #[event_sauce(id)]
    id: Uuid,
    name: String,
    email: String,
}

#[derive(serde::Serialize, serde::Deserialize, event_sauce_derive::CreateEventData)]
#[event_sauce(User)]
struct UserCreated {
    name: String,
    email: String,
}

#[derive(serde::Serialize, serde::Deserialize, event_sauce_derive::UpdateEventData)]
#[event_sauce(User)]
struct UserEmailChanged {
    email: String,
}

/// Empty create event to test compilation works with unit structs
#[derive(serde::Serialize, serde::Deserialize, event_sauce_derive::CreateEventData)]
#[event_sauce(User)]
struct TestUnitStructCreate;

/// Empty update event to test compilation works with unit structs
#[derive(serde::Serialize, serde::Deserialize, event_sauce_derive::UpdateEventData)]
#[event_sauce(User)]
struct TestUnitStructUpdate;

#[derive(serde::Serialize, serde::Deserialize, event_sauce_derive::DeleteEventData)]
#[event_sauce(User)]
struct UserDeleted;

#[async_trait::async_trait]
impl Persistable<SqlxPgStoreTransaction> for User {
    async fn persist(self, tx: &mut SqlxPgStoreTransaction) -> Result<Self, sqlx::Error> {
        let blah = format!(
            "insert into {}
                    (id, name, email)
                values
                    ($1, $2, $3)
                on conflict (id)
                do update set
                name = excluded.name,
                email = excluded.email
            returning *",
            User::entity_type()
        );

        let new = sqlx::query_as(&blah)
            .bind(self.id)
            .bind(self.name)
            .bind(self.email)
            .fetch_one(tx.get())
            .await?;

        Ok(new)
    }
}

#[async_trait::async_trait]
impl Deletable<SqlxPgStoreTransaction> for User {
    async fn delete(self, tx: &mut SqlxPgStoreTransaction) -> Result<(), sqlx::Error> {
        sqlx::query(&format!(
            "delete from {} where id = $1",
            User::entity_type()
        ))
        .bind(self.id)
        .execute(tx.get())
        .await?;

        Ok(())
    }
}

impl AggregateCreate<UserCreated> for User {
    type Error = &'static str;

    fn try_aggregate_create(event: &Event<UserCreated>) -> Result<Self, Self::Error> {
        let data = event
            .data
            .as_ref()
            .ok_or("Event data must be populated to create User from UserCreated event")?;

        Ok(User {
            id: event.entity_id,
            name: data.name.clone(),
            email: data.email.clone(),
        })
    }
}

impl AggregateUpdate<UserEmailChanged> for User {
    type Error = &'static str;

    fn try_aggregate_update(self, event: &Event<UserEmailChanged>) -> Result<Self, Self::Error> {
        let data = event
            .data
            .as_ref()
            .ok_or("Event data must be populated to update User from UserEmailChanged event")?;

        let entity = User {
            email: data.email.clone(),
            ..self
        };

        Ok(entity)
    }
}

impl AggregateCreate<TestUnitStructCreate> for User {
    type Error = &'static str;

    fn try_aggregate_create(event: &Event<TestUnitStructCreate>) -> Result<Self, Self::Error> {
        Ok(User {
            id: event.entity_id,
            name: String::new(),
            email: String::new(),
        })
    }
}

impl AggregateUpdate<TestUnitStructUpdate> for User {
    type Error = &'static str;

    fn try_aggregate_update(
        self,
        _event: &Event<TestUnitStructUpdate>,
    ) -> Result<Self, Self::Error> {
        Ok(self)
    }
}

impl AggregateDelete<UserDeleted> for User {
    type Error = &'static str;

    fn try_aggregate_delete(self, _event: &Event<UserDeleted>) -> Result<Self, Self::Error> {
        // No changes are made to the object as the `delete` impl completely removes it. If the
        // delete behaviour was e.g. to add a "deleted_at" date flag, the code here should update
        // the entity.
        Ok(self)
    }
}

async fn connect() -> Result<SqlxPgStore, sqlx::Error> {
    let postgres = PgPool::connect("postgres://sauce:sauce@localhost:5433/sauce")
        .await
        .expect("Error creating postgres pool");

    sqlx::query(&format!(
        r#"
            create table if not exists {} (
                id uuid primary key,
                name varchar not null,
                email varchar not null
            );
        "#,
        User::entity_type()
    ))
    .execute(&postgres)
    .await
    .expect("Failed to creeate transactions table");

    let store = SqlxPgStore::new(postgres).await?;

    Ok(store)
}

#[async_std::test]
async fn create() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    let mut tx = store.transaction().await?;

    let user = User::try_create(UserCreated {
        name: "I should be deleted".to_string(),
        email: "bobby@bea.ns".to_string(),
    })
    .expect("Failed to create User from UserCreated event")
    .stage_persist(&mut tx)
    .await
    .expect("Failed to persist");

    let id = user.id;

    let user: Option<User> = sqlx::query_as(&format!(
        "select * from {} where id = $1",
        User::entity_type()
    ))
    .bind(id)
    .fetch_optional(&store.pool)
    .await?;

    // User should not be present in the DB yet as the transaction has not been committed
    assert_eq!(user, None);

    tx.commit().await?;

    let users: Vec<User> = sqlx::query_as(&format!(
        "select * from {} where id = $1",
        User::entity_type()
    ))
    .bind(id)
    .fetch_all(&store.pool)
    .await?;

    // User should exist now as transaction was committed
    assert_eq!(
        users,
        vec![User {
            id,
            name: "I should be deleted".to_string(),
            email: "bobby@bea.ns".to_string(),
        }]
    );

    Ok(())
}

#[async_std::test]
async fn create_with_store() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    let user = User::try_create(UserCreated {
        name: "Created with store".to_string(),
        email: "bobby@bea.ns".to_string(),
    })
    .expect("Failed to create User from UserCreated event")
    .persist(&store)
    .await
    .expect("Failed to persist");

    let id = user.id;

    assert_eq!(
        user,
        User {
            id,
            name: "Created with store".to_string(),
            email: "bobby@bea.ns".to_string(),
        }
    );

    Ok(())
}
