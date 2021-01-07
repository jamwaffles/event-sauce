use event_sauce::{
    ActionEntityBuilder, ActionEventBuilder, AggregateAction, AggregateCreate, AggregateDelete,
    AggregateUpdate, EnumEventData, Event, EventData,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "event_type", content = "data")]
pub enum UserEventData {
    UserCreated(crate::UserCreated),
    UserUpdated(crate::UserUpdated),
    UserDeleted(crate::UserDeleted),
}

// TODO: Move into a custom derive for idk, EnumEventData or something
impl TryFrom<UserEventData> for UserCreated {
    type Error = ();

    fn try_from(value: UserEventData) -> Result<Self, Self::Error> {
        match value {
            UserEventData::UserCreated(e) => Ok(e),
            _ => Err(()),
        }
    }
}

// TODO: Move into a custom derive for idk, EnumEventData or something
impl TryFrom<UserEventData> for UserUpdated {
    type Error = ();

    fn try_from(value: UserEventData) -> Result<Self, Self::Error> {
        match value {
            UserEventData::UserUpdated(e) => Ok(e),
            _ => Err(()),
        }
    }
}

// TODO: Move into a custom derive for idk, EnumEventData or something
impl TryFrom<UserEventData> for UserDeleted {
    type Error = ();

    fn try_from(value: UserEventData) -> Result<Self, Self::Error> {
        match value {
            UserEventData::UserDeleted(e) => Ok(e),
            _ => Err(()),
        }
    }
}

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

// TODO: Derive for EnumEventData
impl EnumEventData for UserEventData {}

// TODO: This should really be added by `#derive(event_sauce_derive::ActionEventData)]` on `UserEventData` enum.
impl ActionEntityBuilder<UserEventData> for User {}

impl AggregateAction<UserEventData> for User {
    type Error = EventError;

    fn try_aggregate_action(
        entity: Option<Self>,
        event: &Event<UserEventData>,
    ) -> Result<Self, Self::Error> {
        if let Some(ref data) = event.data {
            match data {
                UserEventData::UserCreated(_) => {
                    // let create_event = event.clone().into_event::<UserCreated>(Some(data.clone()));
                    let create_event = event
                        .clone()
                        .try_into_variant::<UserCreated>()
                        // TODO: Better error variant
                        .map_err(|_e| EventError::Infallible())?;

                    Self::try_aggregate_create(&create_event)
                }
                UserEventData::UserUpdated(_) => {
                    let update_event = event
                        .clone()
                        .try_into_variant::<UserUpdated>()
                        // TODO: Better error variant
                        .map_err(|_e| EventError::Infallible())?;

                    entity
                        .ok_or(EventError::MissingEntity("User", "UserUpdated"))?
                        .try_aggregate_update(&update_event)
                }
                UserEventData::UserDeleted(_) => {
                    let delete_event = event
                        .clone()
                        .try_into_variant::<UserDeleted>()
                        // TODO: Better error variant
                        .map_err(|_e| EventError::Infallible())?;

                    entity
                        .ok_or(EventError::MissingEntity("User", "UserDeleted"))?
                        .try_aggregate_delete(&delete_event)
                        .map_err(|_| EventError::Infallible())
                }
            }
        } else if let Some(entity) = entity {
            // If payload is empty, this event is a noop
            Ok(entity)
        } else {
            Err(EventError::MissingEntity("User", ""))
        }
    }
}

#[derive(Debug, Clone, PartialEq, event_sauce_derive::Entity)]
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
    /// The event data payload is empty.
    #[error("Entity {0} is required for action {1}")]
    MissingEntity(&'static str, &'static str),
    /// An error that shall never occur :crossed_fingers:
    #[error("Fehler fehler fehler fehler!")]
    Infallible(),
}

#[test]
fn apply_enum_event_create() -> Result<(), EventError> {
    let user_name = String::from("Henry Jekyll");
    let event_data = UserEventData::UserCreated(UserCreated {
        name: user_name.clone(),
    });

    let event_builder = event_data.clone().into_builder();

    // ActionBuilder {
    //     event: Event {
    //         id: 622f6431-5e84-43cf-b882-01cd8dcd788f,
    //         event_type: "UserCreated",
    //         entity_type: "users",
    //         entity_id: e7e16954-a85c-4b06-9edc-d21dcccdd0d1,
    //         session_id: None,
    //         purger_id: None,
    //         data: Some(UserCreated(UserCreated { name: "Henry Jekyll" })),
    //         created_at: 2021-01-05T17:20:29.051231753Z,
    //         purged_at: None
    //     },
    //     entity: User {
    //         id: e7e16954-a85c-4b06-9edc-d21dcccdd0d1,
    //         name: "Henry Jekyll"
    //     }
    // }
    let action_builder = User::try_action(event_builder, None)?;
    assert_eq!(action_builder.event.event_type, String::from("UserCreated"));
    assert_eq!(action_builder.event.entity_type, String::from("users"));
    assert!(action_builder.event.data.is_some());
    assert_eq!(action_builder.event.data.unwrap(), event_data);
    assert_eq!(action_builder.entity.name, user_name);

    Ok(())
}

#[test]
fn apply_enum_event_update() -> Result<(), EventError> {
    let entity = User {
        id: Uuid::new_v4(),
        name: String::from("Henry Jekyll"),
    };
    let user_name_updated = String::from("Edward Hyde");
    let event_data = UserEventData::UserUpdated(UserUpdated {
        name: user_name_updated.clone(),
    });

    let event_builder = event_data.clone().into_builder();

    // ActionBuilder {
    //     event: Event {
    //         id: ac6407e5-364b-4957-9807-32df47166f67,
    //         event_type: "UserUpdated",
    //         entity_type: "users",
    //         entity_id: a65db487-1ec6-4a7b-840a-129ce1a1c6b6,
    //         session_id: None,
    //         purger_id: None,
    //         data: Some(UserUpdated(UserUpdated { name: "Edward Hyde" })),
    //         created_at: 2021-01-05T17:29:28.631149788Z,
    //         purged_at: None
    //    },
    //    entity: User {
    //        id: a65db487-1ec6-4a7b-840a-129ce1a1c6b6,
    //        name: "Edward Hyde"
    //    }
    // }
    let action_builder = User::try_action(event_builder, Some(entity.clone()))?;
    assert_eq!(action_builder.event.event_type, String::from("UserUpdated"));
    assert_eq!(action_builder.event.entity_type, String::from("users"));
    assert!(action_builder.event.data.is_some());
    assert_eq!(action_builder.event.data.unwrap(), event_data);
    assert_eq!(action_builder.entity.id, entity.id);
    assert_eq!(action_builder.entity.name, user_name_updated);

    Ok(())
}

#[test]
fn apply_enum_event_delete() -> Result<(), EventError> {
    let entity = User {
        id: Uuid::new_v4(),
        name: String::from("Henry Jekyll"),
    };
    let event_data = UserEventData::UserDeleted(UserDeleted);

    let event_builder = event_data.clone().into_builder();

    // ActionBuilder {
    //     event: Event {
    //         id: fe7ec29a-7c00-4681-b85f-48b8f24e3804,
    //         event_type: "UserDeleted",
    //         entity_type: "users",
    //         entity_id: a1d84fc8-a3b0-4e09-b82b-2656ecbacd50,
    //         session_id: None,
    //         purger_id: None,
    //         data: Some(UserDeleted(UserDeleted)),
    //         created_at: 2021-01-05T17:32:05.344503386Z,
    //         purged_at: None
    //     },
    //     entity: User {
    //         id: a1d84fc8-a3b0-4e09-b82b-2656ecbacd50,
    //         name: "Henry Jekyll"
    //     }
    // }
    let action_builder = User::try_action(event_builder, Some(entity.clone()))?;
    assert_eq!(action_builder.event.event_type, String::from("UserDeleted"));
    assert_eq!(action_builder.event.entity_type, String::from("users"));
    assert!(action_builder.event.data.is_some());
    assert_eq!(action_builder.event.data.unwrap(), event_data);
    assert_eq!(action_builder.entity, entity);

    Ok(())
}
