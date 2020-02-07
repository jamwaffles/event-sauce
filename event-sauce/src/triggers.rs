//! Traits attached to events to allow side effects when creating/updating entities

use crate::EventData;
use std::fmt::Debug;

/// Perform actions on an entity when it is created
///
/// This trait can be implemented to allow side effects like emitting of events or calling HTTP APIs
/// to be performed when an entity is created.
pub trait OnCreated<ED>
where
    ED: EventData,
{
    /// The error type to return if the trigger failed
    type E: Debug;

    /// On create trigger
    ///
    /// Defaults to a noop
    fn on_created(&self) -> Result<(), Self::E> {
        Ok(())
    }
}

/// Perform actions on an entity when it is updated
///
/// This trait can be implemented to allow side effects like emitting of events or calling HTTP APIs
/// to be performed when an entity is updated.
pub trait OnUpdated<ED>
where
    ED: EventData,
{
    /// The error type to return if the trigger failed
    type E: Debug;

    /// On update trigger
    ///
    /// Defaults to a noop
    fn on_updated(&self) -> Result<(), Self::E> {
        Ok(())
    }
}
