use chrono::Utc;
use core::convert::TryFrom;
use event_sauce::{
    ActionEventBuilder, AggregateCreate, AggregateDelete, AggregateUpdate, DBEvent, EnumEventData,
    Event, EventData,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "event_type", content = "data")]
pub enum UserEventData {
    UserCreated(crate::UserCreated),
    UserUpdated(crate::UserUpdated),
    UserDeleted(crate::UserDeleted),
}

impl EnumEventData for UserEventData {}

// TODO: This should really be added by `#derive(event_sauce_derive::ActionEventData)]` on `UserEventData` enum.
impl EventData for UserEventData {
    type Entity = User;

    type Builder = ActionEventBuilder<Self>;

    fn event_type(&self) -> &'static str {
        match self {
            UserEventData::UserCreated(data) => data.event_type(),
            UserEventData::UserUpdated(data) => data.event_type(),
            UserEventData::UserDeleted(data) => data.event_type(),
        }
    }
}

#[derive(Debug, Clone, event_sauce_derive::Entity)]
#[event_sauce(entity_name = "users")]
pub struct User {
    #[event_sauce(id)]
    pub id: Uuid,

    pub name: String,
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde_derive::Serialize,
    serde_derive::Deserialize,
    event_sauce_derive::CreateEventData,
)]
#[event_sauce(User)]
pub struct UserCreated {
    pub name: String,
}

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

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde_derive::Serialize,
    serde_derive::Deserialize,
    event_sauce_derive::UpdateEventData,
)]
#[event_sauce(User)]
pub struct UserUpdated {
    pub name: String,
}

impl AggregateUpdate<UserUpdated> for User {
    type Error = EventError;

    fn try_aggregate_update(self, event: &Event<UserUpdated>) -> Result<Self, Self::Error> {
        let data = event
            .data
            .as_ref()
            .ok_or(Self::Error::EmptyEventData("User", "UserUpdated"))?;

        Ok(User {
            id: event.entity_id,
            name: data.name.clone(),
        })
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    serde_derive::Serialize,
    serde_derive::Deserialize,
    event_sauce_derive::DeleteEventData,
)]
#[event_sauce(User)]
pub struct UserDeleted;

impl AggregateDelete<UserDeleted> for User {
    type Error = std::convert::Infallible;
}

/// Event creation error.
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    /// The event data payload is empty.
    #[error("Event data must be populated to create {0} from {1} event")]
    EmptyEventData(&'static str, &'static str),
    /// Error converting event data from JSON to object.
    #[error("Conversion error: {0}")]
    ConversionError(#[from] serde_json::Error),
}

#[test]
fn into_event() -> Result<(), EventError> {
    let event_data = UserUpdated {
        name: String::from("Fred"),
    };

    let db_event = DBEvent {
        id: Uuid::new_v4(),
        sequence_number: Some(42),
        event_type: String::from(event_data.event_type()),
        entity_type: String::from("User"),
        entity_id: Uuid::new_v4(),
        session_id: Some(Uuid::new_v4()),
        created_at: Utc::now(),
        purger_id: None,
        purged_at: None,
        data: Some(serde_json::to_value(event_data.clone())?),
    };

    let event = Event::<UserUpdated>::try_from(db_event)?;
    assert!(event.data.is_some());
    assert_eq!(event.data.unwrap(), event_data);

    Ok(())
}

#[test]
fn into_enum_event() -> Result<(), EventError> {
    let event_data = UserUpdated {
        name: String::from("Fred"),
    };

    let db_event = DBEvent {
        id: Uuid::new_v4(),
        sequence_number: Some(42),
        event_type: String::from(event_data.event_type()),
        entity_type: String::from("User"),
        entity_id: Uuid::new_v4(),
        session_id: Some(Uuid::new_v4()),
        created_at: Utc::now(),
        purger_id: None,
        purged_at: None,
        data: Some(serde_json::to_value(event_data.clone())?),
    };

    let enum_event = Event::<UserEventData>::try_enum_event_from_db_event(db_event)?;

    #[allow(clippy::assertions_on_constants)]
    match enum_event
        .data
        .ok_or(EventError::EmptyEventData("enum Event", "DBEvent"))?
    {
        UserEventData::UserCreated(_) => assert!(false),
        // `UserCreated` and `UserUpdated` have identical structure. The only thing distinguishing
        // b/w them is the `DBEvent::event_type` attribute. If the conversion works correctly,
        // it should use that attribute to deserialize the `DBEvent::data` into `UserUpdated`
        // Note that Serde would deserialize into the first value of the enum, i.e. `UserCreated`
        // (which would be a problem here), if it is not explicitly told otherwise by the
        // `#[serde(tag = "event_type", content = "data")]` macro in definition of the
        // `UserEventData` enum.
        UserEventData::UserUpdated(data) => assert_eq!(data, event_data),
        UserEventData::UserDeleted(_) => assert!(false),
    }

    Ok(())
}
