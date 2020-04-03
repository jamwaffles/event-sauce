use event_sauce::CreateEntity;
use event_sauce::Event;
use event_sauce::EventData;
use event_sauce::Persistable;
use event_sauce::StorageBuilder;
use event_sauce::UpdateEntity;
use event_sauce_storage_sqlx_pg::SqlxPgStore;
use uuid::Uuid;

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
struct User {
    id: Uuid,
    name: String,
    email: String,
}

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
impl Persistable<SqlxPgStore> for User {
    async fn persist(self, store: &SqlxPgStore) -> Result<Self, sqlx::Error> {
        //

        Ok(self)
    }
}

impl CreateEntity<UserCreated> for User {
    fn try_create(
        event: Event<UserCreated>,
    ) -> Result<StorageBuilder<User, UserCreated>, &'static str> {
        let data = event
            .data
            .ok_or("Event data must be populated to create User from UserCreated event")?;

        let entity = User {
            id: event.entity_id,
            name: data.name,
            email: data.email,
        };

        Ok(StorageBuilder::new(entity, event))
    }
}

impl UpdateEntity<UserEmailChanged> for User {
    fn try_update(
        self,
        event: Event<UserEmailChanged>,
    ) -> Result<StorageBuilder<User, UserEmailChanged>, &'static str> {
        let data = event
            .data
            .ok_or("Event data must be populated to update User from UserEmailChanged event")?;

        let entity = User {
            email: data.email,
            ..self
        };

        Ok(StorageBuilder::new(entity, event))
    }
}

#[test]
fn create() {
    let mut postgres = PgPool::new("postgres://sauce:sauce@localhost/sauce")
        .await
        .expect("Error creating postgres pool");

    let store = SqlxPgStore::new(postgres);

    let user = User::try_create(
        UserCreated {
            name: String::new(),
            email: String::new(),
        }
        .into_event(None),
    )
    .unwrap();

    let user = user.persist(&store).await.unwrap();
}
