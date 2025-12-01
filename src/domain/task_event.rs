//! Domain task event types capturing the recorded changes and activity on tasks.
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::types::{TaskEventId, TaskId, UserId};

/// Type of event that can occur on a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskEventType {
    /// A user left a new comment on the task.
    Comment,
    /// The task status transitioned to a different state.
    StatusChanged,
    /// The task was assigned to or unassigned from a user.
    AssignmentChanged,
    /// Miscellaneous metadata on the task was updated.
    MetadataUpdated,
}

/// Domain representation of recorded task activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    /// Unique identifier of the task event.
    pub id: TaskEventId,
    /// Identifier of the task the event belongs to.
    pub task_id: TaskId,
    /// Identifier of the user who triggered the event, if available.
    pub user_id: Option<UserId>,
    /// Specific type of event.
    pub event_type: TaskEventType,
    /// Additional data describing the event.
    pub event_data: Value,
    /// Timestamp when the event was created.
    pub created_at: NaiveDateTime,
}

impl TaskEvent {
    /// Create a new event with the provided payload and current timestamp.
    pub fn new(
        id: TaskEventId,
        task_id: TaskId,
        user_id: Option<UserId>,
        event_type: TaskEventType,
        event_data: Value,
    ) -> Self {
        Self {
            id,
            task_id,
            user_id,
            event_type,
            event_data,
            created_at: chrono::Local::now().naive_utc(),
        }
    }
}

/// Parameters required to record a new task event.
#[derive(Debug, Clone)]
pub struct NewTaskEvent {
    /// Identifier of the task the event belongs to.
    pub task_id: TaskId,
    /// Identifier of the user who triggered the event, if available.
    pub user_id: Option<UserId>,
    /// Specific type of event.
    pub event_type: TaskEventType,
    /// Additional data describing the event.
    pub event_data: Value,
    /// Timestamp when the event was created.
    pub created_at: NaiveDateTime,
}

impl NewTaskEvent {
    /// Create a new event payload using the current timestamp.
    pub fn new(
        task_id: TaskId,
        user_id: Option<UserId>,
        event_type: TaskEventType,
        event_data: Value,
    ) -> Self {
        Self {
            task_id,
            user_id,
            event_type,
            event_data,
            created_at: chrono::Local::now().naive_utc(),
        }
    }
}

impl From<TaskEventType> for &'static str {
    fn from(value: TaskEventType) -> Self {
        match value {
            TaskEventType::Comment => "Comment",
            TaskEventType::StatusChanged => "StatusChanged",
            TaskEventType::AssignmentChanged => "AssignmentChanged",
            TaskEventType::MetadataUpdated => "MetadataUpdated",
        }
    }
}

impl From<&str> for TaskEventType {
    fn from(value: &str) -> Self {
        match value {
            "Comment" => TaskEventType::Comment,
            "StatusChanged" => TaskEventType::StatusChanged,
            "AssignmentChanged" => TaskEventType::AssignmentChanged,
            "MetadataUpdated" => TaskEventType::MetadataUpdated,
            _ => TaskEventType::MetadataUpdated,
        }
    }
}
