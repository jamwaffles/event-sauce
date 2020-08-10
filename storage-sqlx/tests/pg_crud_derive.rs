use event_sauce::{
    prelude::*, AggregateCreate, AggregateDelete, AggregateUpdate, Deletable, Event, Persistable,
};
use event_sauce_storage_sqlx::SqlxPgStoreTransaction;
// use event_sauce::UpdateEntity;
use event_sauce_storage_sqlx::SqlxPgStore;
use sqlx::{postgres::PgQueryAs, PgPool};
use uuid::Uuid;

const USERS_TABLE: &str = "crud_test_users";

#[derive(
    serde_derive::Serialize, serde_derive::Deserialize, sqlx::FromRow, event_sauce_derive::Entity,
)]
#[event_sauce(entity_name = "users")]
struct User {
    #[event_sauce(id)]
    id: Uuid,
    name: String,
    email: String,
}

#[derive(
    serde_derive::Serialize, serde_derive::Deserialize, event_sauce_derive::CreateEventData,
)]
#[event_sauce(User)]
struct UserCreated {
    name: String,
    email: String,
}

#[derive(
    serde_derive::Serialize, serde_derive::Deserialize, event_sauce_derive::UpdateEventData,
)]
#[event_sauce(User)]
struct UserEmailChanged {
    email: String,
}

/// Empty create event to test compilation works with unit structs
#[derive(
    serde_derive::Serialize, serde_derive::Deserialize, event_sauce_derive::CreateEventData,
)]
#[event_sauce(User)]
struct TestUnitStructCreate;

/// Empty update event to test compilation works with unit structs
#[derive(
    serde_derive::Serialize, serde_derive::Deserialize, event_sauce_derive::UpdateEventData,
)]
#[event_sauce(User)]
struct TestUnitStructUpdate;

#[derive(
    serde_derive::Serialize, serde_derive::Deserialize, event_sauce_derive::DeleteEventData,
)]
#[event_sauce(User)]
struct UserDeleted;

#[async_trait::async_trait]
impl Persistable<SqlxPgStoreTransaction, User> for User {
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
            USERS_TABLE
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
        sqlx::query(&format!("delete from {} where id = $1", USERS_TABLE))
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
    let postgres = PgPool::new("postgres://sauce:sauce@localhost/sauce")
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
        USERS_TABLE
    ))
    .execute(&postgres)
    .await
    .expect("Failed to creeate test users table");

    let store = SqlxPgStore::new(postgres).await?;

    Ok(store)
}

#[async_std::test]
async fn create() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    let user = User::try_create(UserCreated {
        name: "Bobby Beans".to_string(),
        email: "bobby@bea.ns".to_string(),
    })
    .expect("Failed to create User from UserCreated event")
    .persist(&store)
    .await
    .expect("Failed to persist");

    assert_eq!(user.name, "Bobby Beans".to_string(),);
    assert_eq!(user.email, "bobby@bea.ns".to_string());

    Ok(())
}

#[async_std::test]
async fn create_with_custom_entity_id() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    let entity_id = Uuid::new_v4();

    User::try_create(
        UserCreated {
            name: "Bobby Beans".to_string(),
            email: "bobby@bea.ns".to_string(),
        }
        .into_builder()
        .entity_id(entity_id),
    )
    .expect("Failed to create User from UserCreated event")
    .persist(&store)
    .await
    .expect("Failed to persist");

    let (found,): (i64,) = sqlx::query_as(&format!(
        "select count(*) from {} where id = $1",
        USERS_TABLE
    ))
    .bind(entity_id)
    .fetch_one(&store.pool)
    .await?;

    assert_eq!(found, 1);

    let (found,): (i64,) = sqlx::query_as("select count(*) from events where entity_id = $1")
        .bind(entity_id)
        .fetch_one(&store.pool)
        .await?;

    assert_eq!(found, 1);

    Ok(())
}

#[async_std::test]
async fn update() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    // Create user
    let user = User::try_create(UserCreated {
        name: "Bobby Beans".to_string(),
        email: "bobby@bea.ns".to_string(),
    })
    .expect("Failed to create User from UserCreated event")
    .persist(&store)
    .await
    .expect("Failed to persist");

    // Update user's email address
    let user = user
        .try_update(UserEmailChanged {
            email: "beans@bob.by".to_string(),
        })
        .expect("Failed to update User from UserEmailChanged event")
        .persist(&store)
        .await
        .expect("Failed to persist");

    assert_eq!(user.email, "beans@bob.by".to_string());

    Ok(())
}

#[async_std::test]
async fn delete() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    let user = User::try_create(UserCreated {
        name: "I should be deleted".to_string(),
        email: "bobby@bea.ns".to_string(),
    })
    .expect("Failed to create User from UserCreated event")
    .persist(&store)
    .await
    .expect("Failed to persist");

    let id = user.id;

    let (found,): (i64,) = sqlx::query_as(&format!(
        "select count(*) from {} where id = $1",
        USERS_TABLE
    ))
    .bind(id)
    .fetch_one(&store.pool)
    .await?;

    assert_eq!(found, 1);

    assert_eq!(user.name, "I should be deleted".to_string());
    assert_eq!(user.email, "bobby@bea.ns".to_string());

    user.try_delete(UserDeleted)
        .expect("Failed to create deletion event")
        .delete(&store)
        .await?;

    let (found,): (i64,) = sqlx::query_as(&format!(
        "select count(*) from {} where id = $1",
        USERS_TABLE
    ))
    .bind(id)
    .fetch_one(&store.pool)
    .await?;

    assert_eq!(found, 0);

    // There should be two events with the entity ID of the user
    let (found,): (i64,) = sqlx::query_as("select count(*) from events where entity_id = $1")
        .bind(id)
        .fetch_one(&store.pool)
        .await?;

    assert_eq!(found, 2);

    Ok(())
}
