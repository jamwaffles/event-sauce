#[allow(unused_imports)]
use event_sauce::{
    AggregateCreate, CreateEntityBuilder, CreateEventBuilder, DBEvent, Entity, Event, EventData,
    Persistable, StorageBackend, StorageBackendTransaction, StorageBuilder, StorageBuilderPersist,
};
use futures::executor::block_on;
use std::convert::TryInto;
use uuid::Uuid;


/// Entity: "The Thing".
///
/// `#[derive(event_sauce_derive::Entity)]`
/// ```
/// impl Entity for User {
///     const ENTITY_TYPE: &'static str = "users"; // value from: `#[event_sauce(entity_name = "users")]`
///     fn entity_id(&self) -> Uuid {
///         self.id // field name from: `#[event_sauce(id)]`
///     }
/// }
/// ```
#[derive(Debug, event_sauce_derive::Entity)]
#[event_sauce(entity_name = "users")]
pub struct User {
    /// Entity ID: Unique identifier required by EventSauce.
    #[event_sauce(id)]
    pub id: Uuid,

    /// Name: Some data payload
    pub name: String,
}


/// Make the Entity aggregate-creatable.
/// - Provides `try_aggregate_create` function called by
///   `event_sauce::CreateEntityBuilder<UserCreated>::try_create` function
///   (see above).
impl AggregateCreate<UserCreated> for User {
    type Error = EventError;

    fn try_aggregate_create(event: &Event<UserCreated>) -> Result<Self, Self::Error> {
        let data = event
            .data
            .as_ref()
            .ok_or(Self::Error::EmptyEventData("User", "UserCreated"))?;

        Ok(User {
            id: event.entity_id,
            name: data.name.clone(),
        })
    }
}


/// Make the Entity persistable.
#[async_trait::async_trait]
impl Persistable<Transaction> for User {
    async fn persist(self, txn: &mut Transaction) -> Result<Self, StorageError> {
        txn.persist(format!("{:?}", self)).await?;
        Ok(self)
    }
}


/// Event Data: The data payload of the Event.
///
/// `#[derive(event_sauce_derive::CreateEventData]`:
/// ```
/// // - Provides `try_create(...) -> StorageBuilder` function.
/// // - Requires `AggregateCreate<ED: EventData>` to be implemented
/// //   (`try_create` function calls `AggregateCreate::try_aggregate_create`).
/// impl CreateEntityBuilder<UserCreated> for User {} // struct name from: `#[event_sauce(User)]`
///
/// // - Provides `with_session_id(...) -> Self::Builder` convenience function.
/// // - Provides `into_builder(...) -> Self::Builder` function.
/// impl EventData for UserCreated {
///     type Entity = User; // value from: `#[event_sauce(User)]`
///     type Builder = CreateEventBuilder<UserCreated>;
///     const EVENT_TYPE: &'static str = "User Created";
/// }
/// ```
#[derive(
    Debug, serde_derive::Serialize, serde_derive::Deserialize, event_sauce_derive::CreateEventData,
)]
#[event_sauce(User)]
pub struct UserCreated {
    // Name: Some data payload.
    pub name: String,
}


/// Event creation error.
/// - Used by AggregateCreate trait.
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    /// The event data payload is empty.
    #[error("Event data must be populated to create {0} from {1} event")]
    EmptyEventData(&'static str, &'static str),
}


/// Storage
#[derive(Debug, Clone)]
pub struct Storage;
impl Storage {
    pub async fn transaction(&self) -> Result<Transaction, StorageError> {
        Ok(Transaction::new().await)
    }
}
#[async_trait::async_trait]
impl StorageBackend for Storage {
    type Error = StorageError;
    type Transaction = Transaction;
}


/// Storage Transaction
pub struct Transaction {
    data: Vec<String>,
}
impl Transaction {
    pub async fn new() -> Self {
        Transaction { data: vec![] }
    }
    pub async fn persist(&mut self, s: String) -> Result<(), StorageError> {
        self.data.push(s);
        Ok(())
    }
    pub async fn commit(self) -> Result<(), StorageError> {
        for s in self.data {
            println!("Persisting data: {}", s)
        }
        Ok(())
    }
}
impl StorageBackendTransaction for Transaction {
    type Error = StorageError;
}

/// Tie DBEvent with the dummy Store defined above
#[async_trait::async_trait]
impl Persistable<Transaction, DBEvent> for DBEvent {
    async fn persist(self, txn: &mut Transaction) -> Result<Self, StorageError> {
        txn.persist(format!("{:?}", self)).await?;
        Ok(self)
    }
}

/// Tie StorageBuilder with the dummy Store defined above
#[async_trait::async_trait]
impl<E, ED> StorageBuilderPersist<Storage, E> for StorageBuilder<E, ED>
where
    E: Persistable<Transaction> + Send,
    ED: EventData + Send,
{
    async fn stage_persist(self, tx: &mut Transaction) -> Result<E, StorageError> {
        let db_event: DBEvent = self
            .event
            .try_into()
            .expect("Failed to convert Event into DBEvent");
        db_event.persist(tx).await?;
        self.entity.persist(tx).await
    }
    async fn persist(self, store: &Storage) -> Result<E, StorageError> {
        let mut tx = store.transaction().await?;
        let new = self.stage_persist(&mut tx).await?;
        tx.commit().await?;
        Ok(new)
    }
}


/// Storage Error.
/// - Required by StorageBackendTransaction trait.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Generic error.
    #[error("Storage Error: {0}")]
    Error(&'static str),
}


fn main() {
    // EventData
    let event_data = UserCreated {
        name: String::from("Fred"),
    };
    println!("EventData: {:?}", event_data);

    // EventData::into_builder(...) -> Self::Builder a.k.a CreateEventBuilder<UserCreated>
    let event_builder = event_data.into_builder();

    // CreateEntityBuilder::try_create(...) -> StorageBuilder
    let storage_builder = User::try_create(event_builder).unwrap();

    // StorageBuilder holds both Event and Entity in its public attributes
    println!("Event: {:?}", storage_builder.event);
    println!("Entity: {:?}", storage_builder.entity);

    // Persist the event and the aggreagte-created entity
    let store = Storage;
    let future = storage_builder.persist(&store);
    block_on(future).unwrap();
}


