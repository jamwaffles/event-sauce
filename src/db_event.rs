//! Database storage for [`Event`]s

use crate::{event::Event, EventData};
use chrono::{DateTime, Utc};
use std::convert::TryFrom;
use uuid::Uuid;

/// Internal event definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct DBEvent {
    /// Event ID
    pub id: Uuid,

    /// Event type
    ///
    /// This field provides information about how this event was originated.
    pub event_type: String,

    /// Entity Type
    ///
    /// This field must contain the name of the table the event relates to
    pub entity_type: String,

    /// The ID of the entity (user, organisation, etc) that this event aggregates into
    pub entity_id: Uuid,

    /// Event data
    ///
    /// This is a generic [`serde_json::Value`] representation of the event payload. It is
    /// deserialised into a more useful form using `Event::try_from()`.
    pub data: serde_json::Value,

    /// The ID of the session which created this event.
    pub session_id: Uuid,

    /// The time at which this event was created
    pub created_at: DateTime<Utc>,
}

impl<S> TryFrom<Event<S>> for DBEvent
where
    S: EventData + serde::Serialize,
{
    type Error = serde_json::Error;

    /// Attempt to convert an [`Event`] into a `DBEvent`
    ///
    /// This serialises the `data` field into a [`serde_json::Value`]. All other fields are left as
    /// is.
    fn try_from(other: Event<S>) -> Result<DBEvent, Self::Error> {
        Ok(DBEvent {
            id: other.id,
            event_type: other.event_type,
            entity_type: other.entity_type,
            entity_id: other.entity_id,
            session_id: other.session_id,
            created_at: other.created_at,
            data: serde_json::to_value(other.data)?,
        })
    }
}
