use event_sauce::{
    ActionEntityBuilder, AggregateAction, AggregateConflict, AggregateCreate, AggregateDelete,
    AggregateUpdate, ConflictCheck, ConflictData, ConflictEntityBuilder, Event, EventData,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

/// User
#[derive(Debug, Clone, PartialEq, event_sauce_derive::Entity)]
#[event_sauce(entity_name = "users")]
pub struct User {
    #[event_sauce(id)]
    pub id: Uuid,
    pub name: String,
    pub conflicted: bool,
}

/// EnumEventData: Enumeration of EventData structures for all possible Event types
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, event_sauce_derive::EnumEventData)]
#[serde(tag = "event_type", content = "data")]
#[event_sauce(User)]
pub enum UserEventData {
    UserCreated(crate::UserCreated),
    UserUpdated(crate::UserUpdated),
    UserDeleted(crate::UserDeleted),
}

impl ConflictEntityBuilder<UserEventData, UserEventData> for User {}

/// Make events on User conflict-checkable.
impl ConflictCheck<UserEventData> for UserEventData {
    fn check_conflict(
        self,
        applied_event: &Event<UserEventData>,
    ) -> Result<Self, ConflictData<UserEventData, Self>> {
        match &self {
            // create event never conflicts (this is only theoretical branch: it is not possible to get two create events for the same User)
            UserEventData::UserCreated(_) => Ok(self),
            UserEventData::UserUpdated(_) => Err(ConflictData {
                applied_event: applied_event.clone(),
                conflicting_event_data: self,
            }),
            UserEventData::UserDeleted(_) => Err(ConflictData {
                applied_event: applied_event.clone(),
                conflicting_event_data: self,
            }),
        }
    }
}

/// Make the entity Aggregate-Actionable
///
/// Match the type of the event and invoke the corresponding aggregation action
impl AggregateAction<UserEventData> for User {
    type Error = EventError;

    fn try_aggregate_action(
        entity: Option<Self>,
        event: &Event<UserEventData>,
    ) -> Result<Self, Self::Error> {
        if let Some(ref data) = event.data {
            match data {
                UserEventData::UserCreated(_) => {
                    let create_event =
                        event
                            .clone()
                            .try_into_variant::<UserCreated>()
                            .map_err(|_| {
                                EventError::ConversionError(
                                    "Event<UserEventData>",
                                    "Event<UserCreated>",
                                )
                            })?;

                    Self::try_aggregate_create(&create_event)
                }
                UserEventData::UserUpdated(_) => {
                    let update_event =
                        event
                            .clone()
                            .try_into_variant::<UserUpdated>()
                            .map_err(|_| {
                                EventError::ConversionError(
                                    "Event<UserEventData>",
                                    "Event<UserUpdated>",
                                )
                            })?;

                    entity
                        .ok_or(EventError::MissingEntity("User", "UserUpdated"))?
                        .try_aggregate_update(&update_event)
                }
                UserEventData::UserDeleted(_) => {
                    let delete_event =
                        event
                            .clone()
                            .try_into_variant::<UserDeleted>()
                            .map_err(|_| {
                                EventError::ConversionError(
                                    "Event<UserEventData>",
                                    "Event<UserDeleted>",
                                )
                            })?;

                    entity
                        .ok_or(EventError::MissingEntity("User", "UserDeleted"))?
                        .try_aggregate_delete(&delete_event)
                        .map_err(EventError::Infallible)
                }
            }
        } else if let Some(entity) = entity {
            // If payload is empty, this event is a noop
            Ok(entity)
        } else {
            Err(EventError::MissingEntity("User", "N/A"))
        }
    }
}

/// UserCreated Event payload
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    event_sauce_derive::CreateEventData,
)]
#[event_sauce(User)]
pub struct UserCreated {
    pub name: String,
}

/// Make User aggregate-creatable
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
            conflicted: false,
        })
    }
}

/// UserUpdated Event payload
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    event_sauce_derive::UpdateEventData,
)]
#[event_sauce(User)]
pub struct UserUpdated {
    pub name: String,
}

/// Make User aggregate-updatable
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
            conflicted: false,
        })
    }
}

/// UserDeleted Event payload
#[derive(
    Debug,
    Clone,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
    event_sauce_derive::DeleteEventData,
)]
#[event_sauce(User)]
pub struct UserDeleted;

/// Make User event-deletable
impl AggregateDelete<UserDeleted> for User {
    type Error = std::convert::Infallible;
}

/// Make User event-deletable
impl<EDA, EDC> AggregateConflict<EDA, EDC> for User
where
    EDA: EventData,
    EDC: EventData,
{
    type Error = std::convert::Infallible;

    fn try_aggregate_conflict(
        self,
        _: &Event<ConflictData<EDA, EDC>>,
    ) -> Result<Self, Self::Error> {
        Ok(User {
            conflicted: true,
            ..self
        })
    }
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
    /// Conversion error.
    #[error("Can not convert {0} into {1}")]
    ConversionError(&'static str, &'static str),
    /// An error that shall never occur :crossed_fingers:
    #[error("Fehler fehler fehler fehler!")]
    Infallible(#[from] std::convert::Infallible),
}

#[test]
fn conflicting_updates() -> Result<(), EventError> {
    // the original entity as known by all parties involved
    let entity = User {
        id: Uuid::new_v4(),
        name: String::from("Henry Jekyll"),
        conflicted: false,
    };

    // the entity gets updated locally
    let user_name_updated_applied = String::from("Edward Hyed");
    let event_data_applied = UserEventData::UserUpdated(UserUpdated {
        name: user_name_updated_applied.clone(),
    });
    let event_builder_applied = event_data_applied.into_builder();
    let action_builder_applied = User::try_action(event_builder_applied, Some(entity.clone()))?;

    // a conflicting update is sent from a remote party
    let user_name_updated_conflicting = String::from("Danvers Carew");
    let event_data_conflicting = UserEventData::UserUpdated(UserUpdated {
        name: user_name_updated_conflicting.clone(),
    });

    // check for conflicts and applly the event
    match event_data_conflicting
        .clone()
        .check_conflict(&action_builder_applied.event)
    {
        Ok(event_data) => {
            let event_builder_nonconflicted = event_data.clone().into_builder();
            let action_builder_nonconflicted =
                User::try_action(event_builder_nonconflicted, Some(entity.clone()))?;

            // ActionBuilder {
            //     event: Event {
            //         id: d757860a-22fe-4e32-9a29-a725b4fd14e4,
            //         event_type: "UserUpdated",
            //         entity_type: "users",
            //         entity_id: 10969215-ca5e-49d4-95e9-6201ef6e6872,
            //         session_id: None,
            //         purger_id: None,
            //         data: Some(UserUpdated(UserUpdated {
            //             name: "Danvers Carew"
            //         })),
            //         created_at: 2021-01-11T23:44:43.430395806Z,
            //         purged_at: None
            //     },
            //     entity: User {
            //         id: 10969215-ca5e-49d4-95e9-6201ef6e6872,
            //         name: "Danvers Carew",
            //         conflicted: false
            //     }
            // }
            assert_eq!(
                action_builder_nonconflicted.event.event_type,
                String::from("UserUpdated")
            );
            assert_eq!(
                action_builder_nonconflicted.event.entity_type,
                String::from("users")
            );
            assert!(action_builder_nonconflicted.event.data.is_some());
            assert_eq!(action_builder_nonconflicted.event.data.unwrap(), event_data);
            assert_eq!(action_builder_nonconflicted.entity.id, entity.id);
            assert_eq!(
                action_builder_nonconflicted.entity.name,
                user_name_updated_conflicting
            );
            assert_eq!(action_builder_nonconflicted.entity.conflicted, false);

            // The above code in this match-branch is here only for documentation purposes,
            // to show what it would look like, would there be no conflict. This match-branch
            // shall not be reached, because the `event_data` conflicts with the already
            // applied event.
            unreachable!("The call to `conflict_ckeck` should return conflict");
        }
        Err(conflict_data) => {
            let event_builder_conflicted = conflict_data.into_builder();
            let conflict_builder_conflicted = action_builder_applied
                .entity
                .try_flag_conflict(event_builder_conflicted)?;

            // StorageBuilder {
            //     event: Event {
            //         id: cde6ce75-49c5-4924-be59-cc617dfd0084,
            //         event_type: "ConflictData",
            //         entity_type: "users",
            //         entity_id: 1258c19b-0412-4c79-a99f-a3cd16c530dd,
            //         session_id: None,
            //         purger_id: None,
            //         data: Some(ConflictData {
            //             applied_event: Event {
            //                 id: 42494424-10a8-401d-8d04-3937b01882d0,
            //                 event_type: "UserUpdated",
            //                 entity_type: "users",
            //                 entity_id: 1258c19b-0412-4c79-a99f-a3cd16c530dd,
            //                 session_id: None,
            //                 purger_id: None,
            //                 data: Some(UserUpdated(UserUpdated {
            //                     name: "Edward Hyed"
            //                 })),
            //                 created_at: 2021-01-11T23:57:54.594690142Z,
            //                 purged_at: None
            //             },
            //             conflicting_event_data: UserUpdated(UserUpdated {
            //                 name: "Danvers Carew"
            //             })
            //         }),
            //         created_at: 2021-01-11T23:57:54.594705058Z,
            //         purged_at: None
            //     },
            //     entity: User {
            //       id: 1258c19b-0412-4c79-a99f-a3cd16c530dd,
            //       name: "Edward Hyed",
            //       conflicted: true
            //     }
            // }
            assert_eq!(conflict_builder_conflicted.event.event_type, "ConflictData");
            assert_eq!(conflict_builder_conflicted.event.entity_type, "users");
            assert_eq!(conflict_builder_conflicted.event.entity_id, entity.id);
            assert!(conflict_builder_conflicted.event.data.is_some());
            assert_eq!(
                conflict_builder_conflicted
                    .event
                    .data
                    .clone()
                    .unwrap()
                    .applied_event,
                action_builder_applied.event
            );
            assert_eq!(
                conflict_builder_conflicted
                    .event
                    .data
                    .unwrap()
                    .conflicting_event_data,
                event_data_conflicting
            );
            assert_eq!(conflict_builder_conflicted.entity.id, entity.id);
            assert_eq!(
                conflict_builder_conflicted.entity.name,
                user_name_updated_applied
            );
            assert_eq!(conflict_builder_conflicted.entity.conflicted, true);
        }
    };

    Ok(())
}
