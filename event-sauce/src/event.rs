//! An event

use crate::{db_event::DBEvent, EventData};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::convert::TryFrom;
use uuid::Uuid;

/// Event definition
#[derive(Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
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
    EDENUM: EventData + for<'de> Deserialize<'de>,
{
    /// Convert [`DBEvent`] into a generic `Event<EDENUM>`, where `EDENUM` is an enum of possible [`EventData`].
    pub fn try_enum_event_from_db_event(db_event: DBEvent) -> Result<Event<EDENUM>, serde_json::Error> {
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
}

impl<ED> Event<ED> where ED: EventData {
    /// DOCS
    pub fn from_enum_event<EDENUM: EventData>(enum_event: Event<EDENUM>, event_data: Option<ED>) -> Event<ED> {
        Event {
            id: enum_event.id,
            event_type: enum_event.event_type,
            entity_type: enum_event.entity_type,
            entity_id: enum_event.entity_id,
            session_id: enum_event.session_id,
            purger_id: enum_event.purger_id,
            created_at: enum_event.created_at,
            purged_at: enum_event.purged_at,
            data: event_data,
        }
    }
}

impl<EDENUM> Event<EDENUM> where EDENUM: EventData {
    /// Convert generic `Event<EDENUM>` into concrete `Event<ED>`.
    ///
    /// The `event_data` argument MUST be the same [`EventData`] value as the `self.data` enumeration
    /// value would be after successful `match`.
    ///
    /// Note that the user is required to pass the correct `event_data` as separate argument.
    /// It is needed because the library-function has no way of knowing the `EDENUM` type and so
    /// it can not do the `match` on it (see [`tests/action_builder.rs`] and its implementation of
    /// `AggregateAction::<User, UserEventData>::try_aggregate_action for User` for example of such `match`).
    /// This is exceptionally ugly, because a) the function is being passed the same value twice,
    /// and more importantly b) there are only very limited checks we can perform here. If someone can think
    /// of a solution to this problem, please do change this function.
    ///
    /// TODO: Are there at least any viable checks we can do towards ensuring that `self.data` and `event_data`
    /// are the same?
    pub fn into_event<ED: EventData>(self: Event<EDENUM>, event_data: Option<ED>) -> Event<ED> {
        Event {
            id: self.id,
            event_type: self.event_type,
            entity_type: self.entity_type,
            entity_id: self.entity_id,
            session_id: self.session_id,
            purger_id: self.purger_id,
            created_at: self.created_at,
            purged_at: self.purged_at,
            data: event_data,
        }
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
