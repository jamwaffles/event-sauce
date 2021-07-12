//! An event

use crate::{db_event::DBEvent, Entity, EventData};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use uuid::Uuid;

/// Event definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event<D>
where
    D: EventData,
{
    /// Event ID
    pub id: Uuid,

    /// Event type
    ///
    /// The type of this event in PascalCase, like `OrganisationCreated` or `StudyCurated`
    pub event_type: String,

    /// Entity Type
    ///
    /// This field must contain the name of the table the event relates to, like `organisations` or
    /// `model_enquiries`.
    pub entity_type: String,

    /// The ID of the entity (user, organisation, etc) that this event aggregates into
    pub entity_id: Uuid,

    /// The ID of the session which created this event.
    pub session_id: Uuid,

    /// Event data.
    pub data: D,

    /// The time at which this event was created
    pub created_at: DateTime<Utc>,
}

impl<S> TryFrom<DBEvent> for Event<S>
where
    S: EventData + for<'de> Deserialize<'de>,
{
    type Error = serde_json::Error;

    /// Attempt to decode a [`DBEvent`] into an `Event`
    ///
    /// [`DBEvent`]s carry their paylaod as a [`serde_json::Value`]. This method will attempt to
    /// [deserialise that structure into the specialised event payload `S`.
    ///
    /// # Examples
    ///
    /// ## Read a `UserRegistered` event
    ///
    /// ```rust,ignore
    /// use event_sauce::{DBEvent, Event};
    /// use std::convert::TryFrom;
    ///
    ///    # let created_at = chrono::Utc::now();
    ///    # let id = uuid::Uuid::new_v4();
    ///    # let entity_id = uuid::Uuid::new_v4();
    ///
    ///    let db_event = DBEvent {
    ///        data: serde_json::json!({
    ///            "first_name": "Bobby",
    ///            "last_name": "Beans",
    ///            "email": "bobby@bea.ns",
    ///            "password": "HASHEDLOL",
    ///            "company_name": "",
    ///        }),
    ///        event_type: "UserRegistered".to_string(),
    ///        entity_type: "user".to_string(),
    ///        # id,
    ///        # entity_id,
    ///        # session_id: None,
    ///        # created_at,
    ///        # sequence_number: None,
    ///
    ///        // ...
    ///    };
    ///
    ///    let user_created: Event<UserRegistered> = Event::try_from(db_event).unwrap();
    ///
    ///    assert_eq!(user_created, Event {
    ///        data: UserRegistered {
    ///            first_name: "Bobby".to_string(),
    ///            last_name: "Beans".to_string(),
    ///            email: "bobby@bea.ns".to_string(),
    ///            password: "HASHEDLOL".to_string(),
    ///            company_name: "".to_string(),
    ///        },
    ///        event_type: "UserRegistered".to_string(),
    ///        entity_type: "user".to_string(),
    ///        # id,
    ///        # entity_id,
    ///        # session_id: None,
    ///        # created_at,
    ///    });
    /// ```
    ///
    /// [`DBEvent`]: crate::db_event::DBEvent
    fn try_from(other: DBEvent) -> Result<Event<S>, Self::Error> {
        Ok(Event {
            id: other.id,
            event_type: other.event_type,
            entity_type: other.entity_type,
            entity_id: other.entity_id,
            session_id: other.session_id,
            created_at: other.created_at,
            data: serde_json::from_value(other.data)?,
        })
    }
}

/// Blanket impl to convert event data into an event.
impl<D> From<D> for Event<D>
where
    D: EventData,
{
    fn from(data: D) -> Self {
        Self {
            id: Uuid::new_v4(),
            event_type: D::event_type().to_string(),
            entity_type: D::Entity::ENTITY_TYPE.to_string(),
            entity_id: Uuid::new_v4(),
            session_id: Uuid::nil(),
            data,
            created_at: Utc::now(),
        }
    }
}
