use event_sauce::{prelude::*, Aggregate, AggregateCreate, Event, OnCreated, Store};

use postgres::Transaction;
use postgres::{Client, NoTls};
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

#[derive(event_sauce_derive::CreateEvent, serde_derive::Serialize, serde_derive::Deserialize)]
#[event_store(Model)]
struct CreationEvent {
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

#[test]
fn create() {
    let mut client = Client::connect("postgres://sauce:sauce@localhost/sauce", NoTls)
        .expect("Failed to connect to test DB");

    client
        .batch_execute(
            r#"create table if not exists models (
            id uuid primary key not null,
            some_field varchar(64) not null
        )"#,
        )
        .expect("Could not create models table");

    let mut store = Store::new(client).expect("Failed to initialise store");

    let model: Model = store
        .create(Event::from_create_payload(
            CreationEvent {
                some_field: "Hello world".to_string(),
            },
            None,
        ))
        .expect("Failed to insert model");

    println!("{:#?}", model);
}
