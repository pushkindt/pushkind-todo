//! DTO structs for task details and related events returned to handlers.
use crate::domain::{client::Client, task::Task, task_event::TaskEvent, user::User};
use serde::Serialize;

/// Task event accompanied by the optional author information.
#[derive(Debug, Serialize)]
pub struct TaskEventWithAuthor {
    /// Persisted event data.
    pub event: TaskEvent,
    /// Author of the event, if present and accessible within the hub.
    pub author: Option<User>,
}

/// Aggregated task information with the related event history.
#[derive(Debug, Serialize)]
pub struct TaskDetails {
    /// Task metadata shown on the details page.
    pub task: Task,
    /// Author of the task.
    pub author: User,
    /// Task assignee when available in the current hub.
    pub assignee: Option<User>,
    /// Task client when available in the current hub.
    pub client: Option<Client>,
    /// Ordered list of events associated with the task.
    pub events: Vec<TaskEventWithAuthor>,
}
