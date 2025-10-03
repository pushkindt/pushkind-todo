use chrono::NaiveDateTime;
use diesel::prelude::*;
use thiserror::Error;

use crate::domain::task_event::{
    NewTaskEvent as DomainNewTaskEvent, TaskEvent as DomainTaskEvent, TaskEventType,
};

use super::{task::Task, user::User};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(table_name = crate::schema::task_events)]
#[diesel(belongs_to(Task))]
#[diesel(belongs_to(User))]
pub struct TaskEvent {
    pub id: i32,
    pub task_id: i32,
    pub user_id: Option<i32>,
    pub event_type: String,
    pub event_data: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::task_events)]
pub struct NewTaskEvent {
    pub task_id: i32,
    pub user_id: Option<i32>,
    pub event_type: String,
    pub event_data: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Error)]
pub enum TaskEventModelError {
    #[error("Unknown task event type '{event_type}'")]
    UnknownEventType { event_type: String },
    #[error("Failed to parse event payload: {source}")]
    InvalidEventData { source: serde_json::Error },
    #[error("Failed to serialize event payload: {source}")]
    SerializationFailed { source: serde_json::Error },
}

impl TaskEvent {
    pub fn try_into_domain(self) -> Result<DomainTaskEvent, TaskEventModelError> {
        let event_type = event_type_from_db(&self.event_type)?;
        let event_data = serde_json::from_str(&self.event_data)
            .map_err(|source| TaskEventModelError::InvalidEventData { source })?;

        Ok(DomainTaskEvent {
            id: self.id,
            task_id: self.task_id,
            user_id: self.user_id,
            event_type,
            event_data,
            created_at: self.created_at,
        })
    }
}

impl TryFrom<TaskEvent> for DomainTaskEvent {
    type Error = TaskEventModelError;

    fn try_from(value: TaskEvent) -> Result<Self, Self::Error> {
        value.try_into_domain()
    }
}

impl TryFrom<&DomainNewTaskEvent> for NewTaskEvent {
    type Error = TaskEventModelError;

    fn try_from(value: &DomainNewTaskEvent) -> Result<Self, Self::Error> {
        let event_data = serde_json::to_string(&value.event_data)
            .map_err(|source| TaskEventModelError::SerializationFailed { source })?;

        Ok(Self {
            task_id: value.task_id,
            user_id: value.user_id,
            event_type: event_type_to_db(value.event_type).to_string(),
            event_data,
            created_at: value.created_at,
        })
    }
}

fn event_type_from_db(value: &str) -> Result<TaskEventType, TaskEventModelError> {
    match value {
        "Comment" => Ok(TaskEventType::Comment),
        "StatusChanged" => Ok(TaskEventType::StatusChanged),
        "AssignmentChanged" => Ok(TaskEventType::AssignmentChanged),
        "MetadataUpdated" => Ok(TaskEventType::MetadataUpdated),
        other => Err(TaskEventModelError::UnknownEventType {
            event_type: other.to_string(),
        }),
    }
}

fn event_type_to_db(value: TaskEventType) -> &'static str {
    match value {
        TaskEventType::Comment => "Comment",
        TaskEventType::StatusChanged => "StatusChanged",
        TaskEventType::AssignmentChanged => "AssignmentChanged",
        TaskEventType::MetadataUpdated => "MetadataUpdated",
    }
}
