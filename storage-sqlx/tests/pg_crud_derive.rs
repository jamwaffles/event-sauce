use event_sauce::{
    AggregateCreate, AggregateUpdate, CreateEntityBuilder, Event, EventData, Persistable,
    UpdateEntityBuilder,
};
// use event_sauce::UpdateEntity;
use event_sauce_storage_sqlx::SqlxPgStore;
use sqlx::{postgres::PgQueryAs, PgPool};
use uuid::Uuid;

const USERS_TABLE: &'static str = "crud_test_users";

#[derive(
    serde_derive::Serialize, serde_derive::Deserialize, sqlx::FromRow, event_sauce_derive::Entity,
)]
#[event_sauce(entity_name = "users")]
struct User {
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

#[async_trait::async_trait]
impl Persistable<SqlxPgStore, User> for User {
    async fn persist(self, store: &SqlxPgStore) -> Result<Self, sqlx::Error> {
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
            .fetch_one(&store.pool)
            .await?;

        Ok(new)
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

    let user = User::try_create(
        UserCreated {
            name: "Bobby Beans".to_string(),
            email: "bobby@bea.ns".to_string(),
        }
        .into_event(None),
    )
    .expect("Failed to create User from UserCreated event")
    .persist(&store)
    .await
    .expect("Failed to persist");

    assert_eq!(user.name, "Bobby Beans".to_string(),);
    assert_eq!(user.email, "bobby@bea.ns".to_string());

    Ok(())
}

#[async_std::test]
async fn update() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    // Create user
    let user = User::try_create(
        UserCreated {
            name: "Bobby Beans".to_string(),
            email: "bobby@bea.ns".to_string(),
        }
        .into_event(None),
    )
    .expect("Failed to create User from UserCreated event")
    .persist(&store)
    .await
    .expect("Failed to persist");

    // Update user's email address
    let user = user
        .try_update(
            UserEmailChanged {
                email: "beans@bob.by".to_string(),
            }
            .into_event(None),
        )
        .expect("Failed to update User from UserEmailChanged event")
        .persist(&store)
        .await
        .expect("Failed to persist");

    assert_eq!(user.email, "beans@bob.by".to_string());

    Ok(())
}
