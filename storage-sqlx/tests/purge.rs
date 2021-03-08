use event_sauce::{prelude::*, AggregateCreate, DBEvent, Event, Persistable};
use event_sauce_storage_sqlx::{SqlxPgStore, SqlxPgStoreTransaction};
// use event_sauce::UpdateEntity;
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
#[event_sauce(entity_name = "crud_test_users_purge")]
struct User {
    #[event_sauce(id)]
    id: Uuid,
    name: String,
    email: String,
}

/// The event used to create users in this test suite
#[derive(serde::Serialize, serde::Deserialize, event_sauce_derive::CreateEventData)]
#[event_sauce(User)]
struct UserCreated {
    name: String,
    email: String,
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

/// Event used to purge users in this test suite.
#[derive(serde::Serialize, serde::Deserialize, event_sauce_derive::PurgeEventData)]
#[event_sauce(User)]
struct UserPurged;

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
        User::entity_type()
    ))
    .execute(&postgres)
    .await
    .expect("Failed to creeate purge table");

    let store = SqlxPgStore::new(postgres).await?;

    Ok(store)
}

#[async_std::test]
async fn purge() -> Result<(), sqlx::Error> {
    let store = connect().await?;

    let mut tx = store.transaction().await?;

    // Create user
    let user = User::try_create(UserCreated {
        name: "Bobby Beans".to_string(),
        email: "bobby@bea.ns".to_string(),
    })
    .expect("Failed to create User from UserCreated event")
    .stage_persist(&mut tx)
    .await
    .expect("Failed to persist");

    let user_id = user.id;

    let res: (i64,) = sqlx::query_as("select count(*) from events where entity_id = $1")
        .bind(user.id)
        .fetch_one(tx.get())
        .await?;

    // Created event is on the database
    assert_eq!(res.0, 1);

    let res: (i64,) = sqlx::query_as(&format!(
        "select count(*) from {} where id = $1",
        User::entity_type()
    ))
    .bind(user.id)
    .fetch_one(tx.get())
    .await?;

    // The entity exists in the database
    assert_eq!(res.0, 1);

    user.try_purge(UserPurged {})
        .stage_purge(&mut tx)
        .await
        .expect("Failed to run purge");

    let res: Vec<DBEvent> =
        sqlx::query_as("select * from events where entity_id = $1 order by created_at asc")
            .bind(user_id)
            .fetch_all(tx.get())
            .await?;

    // Both the create and purge events exist in the database
    assert_eq!(res.len(), 2);

    let data = res[0].data.clone();

    // There is no data for the created event
    assert_eq!(data, None);

    let res: (i64,) = sqlx::query_as(&format!(
        "select count(*) from {} where id = $1",
        User::entity_type()
    ))
    .bind(user_id)
    .fetch_one(tx.get())
    .await?;

    // The entity does not exist in the database
    assert_eq!(res.0, 0);

    Ok(())
}
