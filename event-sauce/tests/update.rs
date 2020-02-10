use event_sauce::{
    prelude::*, Aggregate, AggregateApply, AggregateCreate, Event, OnCreated, OnUpdated, Store,
};

use postgres::NoTls;
use postgres::Transaction;
use r2d2_postgres::PostgresConnectionManager;
use std::error::Error;

#[derive(event_sauce_derive::Entity, Default, Debug)]
// TODO: Remove requirement for this attrib
#[table_name = "models"]
struct Model {
    // TODO: Stop this field being required by derive crate
    id: uuid::Uuid,

    some_field: String,
}

// Noop creation trigger
impl<ED> OnCreated<ED> for Model
where
    ED: EventData,
{
    type E = ();
}

// Noop update trigger
impl<ED> OnUpdated<ED> for Model
where
    ED: EventData,
{
    type E = ();
}

#[derive(event_sauce_derive::CreateEvent, serde_derive::Serialize, serde_derive::Deserialize)]
#[event_store(Model)]
struct CreationEvent {
    some_field: String,
}

#[derive(event_sauce_derive::UpdateEvent, serde_derive::Serialize, serde_derive::Deserialize)]
#[event_store(Model)]
struct UpdateEvent {
    some_field: String,
}

impl Aggregate for Model {
    type Error = postgres::error::Error;

    fn persist(&self, conn: &mut Transaction) -> Result<Self, Self::Error> {
        let row = conn.query_one(
            r#"
            INSERT INTO models
                (id, some_field)
            VALUES
                ($1, $2)
            ON CONFLICT ON CONSTRAINT models_pkey
            DO UPDATE SET
                some_field = excluded.some_field
            RETURNING *"#,
            &[&self.id, &self.some_field],
        )?;

        Ok(Self {
            id: row.get("id"),
            some_field: row.get("some_field"),
        })
    }
}

impl AggregateCreate<CreationEvent> for Model {
    fn new(event: Event<CreationEvent>) -> Result<Self, Box<dyn Error>> {
        let CreationEvent { some_field } = event
            .data
            .ok_or("Cannot create Model from empty CreationEvent event")?;

        Ok(Self {
            id: event.entity_id,
            some_field,
        })
    }
}

impl AggregateApply<UpdateEvent> for Model {
    fn apply(self, event: Event<UpdateEvent>) -> Result<Self, Box<dyn Error>> {
        let UpdateEvent { some_field } = event
            .data
            .ok_or("Cannot update Model from empty UpdateEvent event")?;

        Ok(Self { some_field, ..self })
    }
}

#[test]
fn update() {
    let cm = PostgresConnectionManager::new(
        "postgres://sauce:sauce@localhost/sauce".parse().unwrap(),
        NoTls,
    );
    let pool = r2d2::Pool::new(cm).unwrap();
    let mut client = pool.get().unwrap();

    client
        .batch_execute(
            r#"create table if not exists models (
            id uuid primary key not null,
            some_field varchar(64) not null
        )"#,
        )
        .expect("Could not create models table");

    let mut store = Store::new(pool).expect("Failed to initialise store");

    let model: Model = store
        .create(Event::from_create_payload(
            CreationEvent {
                some_field: "Hello world".to_string(),
            },
            None,
        ))
        .expect("Failed to insert model");

    let evt = Event::from_update_payload(
        &model,
        UpdateEvent {
            some_field: "Hello rust".to_string(),
        },
        None,
    );

    let updated = store.update(model, evt).expect("Failed to update model");

    assert_eq!(updated.some_field, "Hello rust".to_string());
}
