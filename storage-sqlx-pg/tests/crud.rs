use event_sauce::{
    AggregateCreate, AggregateUpdate, CreateEntityBuilder, Event, EventData, Persistable,
    UpdateEntityBuilder,
};
// use event_sauce::UpdateEntity;
use event_sauce_storage_sqlx_pg::SqlxPgStore;
use sqlx::{postgres::PgQueryAs, PgPool};
use uuid::Uuid;

const USERS_TABLE: &'static str = "crud_test_users";

#[derive(serde_derive::Serialize, serde_derive::Deserialize, sqlx::FromRow)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

impl CreateEntityBuilder<UserCreated> for User {}
impl UpdateEntityBuilder<UserEmailChanged> for User {}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct UserCreated {
    name: String,
    email: String,
}

impl EventData for UserCreated {
    const EVENT_TYPE: &'static str = "UserCreated";
    const ENTITY_TYPE: &'static str = "users";
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct UserEmailChanged {
    email: String,
}

impl EventData for UserEmailChanged {
    const EVENT_TYPE: &'static str = "UserEmailChanged";
    const ENTITY_TYPE: &'static str = "users";
}

#[async_trait::async_trait]
impl Persistable<SqlxPgStore, User> for User {
    async fn persist(self, store: &SqlxPgStore) -> Result<Self, sqlx::Error> {
        let blah = format!(
            "insert into {} (id, name, email) values ($1, $2, $3) returning *",
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
    fn try_aggregate_create(event: &Event<UserCreated>) -> Result<Self, &'static str> {
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
    fn try_aggregate_update(self, event: &Event<UserEmailChanged>) -> Result<Self, &'static str> {
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

#[async_std::test]
async fn create() -> Result<(), ()> {
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

    let store = SqlxPgStore::new(postgres).await.unwrap();

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
