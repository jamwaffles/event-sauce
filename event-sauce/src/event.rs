//! An event

use crate::{db_event::DBEvent, EnumEventData, EventData};
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

    /// The ID of the creator of this event
    pub session_id: Option<Uuid>,

    /// Purger subject ID
    ///
    /// Will be `None` if event is not purged
    pub purger_id: Option<Uuid>,

    /// Event data
    ///
    /// If the event has been purged, this will be `None` for security/compliance reasons - the data
    /// must be deleted from both the event log and the aggregate tables. Check the `purged_at` or
    /// `purger_id` fields to check the purge status.
    pub data: Option<D>,

    /// The time at which this event was created
    pub created_at: DateTime<Utc>,

    /// The time at which this event was purged, if any
    pub purged_at: Option<DateTime<Utc>>,
}

impl<EDENUM> Event<EDENUM>
where
    EDENUM: EnumEventData + for<'de> Deserialize<'de>,
{
    /// Attempt to convert a general [`DBEvent`] to an event with an enum variant as its data
    /// payload.
    pub fn try_from_db_event(db_event: DBEvent) -> Result<Event<EDENUM>, serde_json::Error> {
        let intermediate =
            serde_json::json!({ "data": db_event.data, "event_type": db_event.event_type });
        let enum_data: EDENUM = serde_json::from_value(intermediate)?;

        Ok(Event {
            id: db_event.id,
            event_type: db_event.event_type,
            entity_type: db_event.entity_type,
            entity_id: db_event.entity_id,
            session_id: db_event.session_id,
            purger_id: db_event.purger_id,
            created_at: db_event.created_at,
            purged_at: db_event.purged_at,
            data: Some(enum_data),
        })
    }

    /// Attempt to convert an event with enum payload into a concrete variant.
    pub fn try_into_variant<ED>(self) -> Result<Event<ED>, ED::Error>
    where
        ED: EventData + TryFrom<EDENUM>,
    {
        Ok(Event {
            id: self.id,
            event_type: self.event_type,
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            session_id: self.session_id,
            purger_id: self.purger_id,
            created_at: self.created_at,
            purged_at: self.purged_at,
            data: self.data.map(ED::try_from).transpose()?,
        })
    }
}

impl<S: EventData + for<'de> Deserialize<'de>> TryFrom<DBEvent> for Event<S> {
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
    ///        data: Some(serde_json::json!({
    ///            "first_name": "Bobby",
    ///            "last_name": "Beans",
    ///            "email": "bobby@bea.ns",
    ///            "password": "HASHEDLOL",
    ///            "company_name": "",
    ///        })),
    ///        event_type: "UserRegistered".to_string(),
    ///        entity_type: "user".to_string(),
    ///        # id,
    ///        # entity_id,
    ///        # session_id: None,
    ///        # purger_id: None,
    ///        # created_at,
    ///        # purged_at: None,
    ///        # sequence_number: None,
    ///
    ///        // ...
    ///    };
    ///
    ///    let user_created: Event<UserRegistered> = Event::try_from(db_event).unwrap();
    ///
    ///    assert_eq!(user_created, Event {
    ///        data: Some(UserRegistered {
    ///            first_name: "Bobby".to_string(),
    ///            last_name: "Beans".to_string(),
    ///            email: "bobby@bea.ns".to_string(),
    ///            password: "HASHEDLOL".to_string(),
    ///            company_name: "".to_string(),
    ///        }),
    ///        event_type: "UserRegistered".to_string(),
    ///        entity_type: "user".to_string(),
    ///        # id,
    ///        # entity_id,
    ///        # session_id: None,
    ///        # purger_id: None,
    ///        # created_at,
    ///        # purged_at: None,
    ///    });
    /// ```
    ///
    /// [`DBEvent`]: crate::db_event::DBEvent
    fn try_from(other: DBEvent) -> Result<Event<S>, Self::Error> {
        let data: Option<S> = if let Some(d) = other.data {
            serde_json::from_value(d)?
        } else {
            None
        };

        Ok(Event {
            id: other.id,
            event_type: other.event_type,
            entity_type: other.entity_type,
            entity_id: other.entity_id,
            session_id: other.session_id,
            purger_id: other.purger_id,
            created_at: other.created_at,
            purged_at: other.purged_at,
            data,
        })
    }
}
