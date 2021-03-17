use event_sauce::{
    prelude::*, AggregateCreate, AggregateUpdate, CreateEventBuilder, Deletable, Entity, Event,
    EventData, Persistable, UpdateEventBuilder,
};
use event_sauce_storage_sqlx::SqlxPgStoreTransaction;
// use event_sauce::UpdateEntity;
use event_sauce_storage_sqlx::SqlxPgStore;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(serde::Serialize, serde::Deserialize, sqlx::FromRow)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

impl Entity for User {
    const ENTITY_TYPE: &'static str = "crud_test_users";

    fn entity_id(&self) -> Uuid {
        self.id
    }
}

impl CreateEntityBuilder<UserCreated> for User {}
impl UpdateEntityBuilder<UserEmailChanged> for User {}

#[derive(serde::Serialize, serde::Deserialize)]
struct UserCreated {
    name: String,
    email: String,
}

impl EventData for UserCreated {
    type Entity = User;
    type Builder = CreateEventBuilder<Self>;

    fn event_type(&self) -> &'static str {
        "UserCreated"
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct UserEmailChanged {
    email: String,
}

impl EventData for UserEmailChanged {
    type Entity = User;
    type Builder = UpdateEventBuilder<Self>;

    fn event_type(&self) -> &'static str {
        "UserEmailChanged"
    }
}

#[async_trait::async_trait]
impl<'c> Persistable<SqlxPgStoreTransaction, User> for User {
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
impl<'c> Deletable<SqlxPgStoreTransaction> for User {
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
    .expect("Failed to create test users table");

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
