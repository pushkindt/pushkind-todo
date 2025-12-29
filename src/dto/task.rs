//! DTO structs for task details, events, and modal payloads returned to handlers.
use crate::domain::{task::Task, task_event::TaskEvent, types::TaskTrack, user::User};
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
    /// Ordered list of events associated with the task.
    pub events: Vec<TaskEventWithAuthor>,
}

/// Data needed to render the task modal for editing.
#[derive(Debug, Serialize)]
pub struct TaskModalData {
    /// Task being edited in the modal.
    pub task: Task,
    /// Optional assignee for the task when available in the current hub.
    pub assignee: Option<User>,
    /// Users that can be selected as potential assignees.
    pub users: Vec<User>,
    /// Available task tracks to use for hints
    pub tracks: Vec<TaskTrack>,
}
