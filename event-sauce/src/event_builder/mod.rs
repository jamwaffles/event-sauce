mod action_event;
mod create_event;
mod delete_event;
mod purge_event;
mod update_event;

use uuid::Uuid;

pub use action_event::ActionEventBuilder;
pub use create_event::CreateEventBuilder;
pub use delete_event::DeleteEventBuilder;
pub use purge_event::PurgeEventBuilder;
pub use update_event::UpdateEventBuilder;

/// Methods common to all event builders
pub trait EventBuilder<D>: Sized {
    /// Create a new builder with a given payload
    fn new(payload: D) -> Self;

    /// Set the session ID on the event contained within the builder
    fn session_id(self, session_id: Uuid) -> Self;
}
