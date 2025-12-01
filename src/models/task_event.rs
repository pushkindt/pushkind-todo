use chrono::NaiveDateTime;
use diesel::prelude::*;
use thiserror::Error;

use crate::domain::{
    task_event::{NewTaskEvent as DomainNewTaskEvent, TaskEvent as DomainTaskEvent, TaskEventType},
    types::{TaskEventId, TaskId, UserId},
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
    #[error("Invalid type constraint: {0}")]
    TypeConstraint(#[from] crate::domain::types::TypeConstraintError),
}

impl TaskEvent {
    pub fn try_into_domain(self) -> Result<DomainTaskEvent, TaskEventModelError> {
        let Self {
            id,
            task_id,
            user_id,
            event_type: raw_event_type,
            event_data: raw_event_data,
            created_at,
        } = self;

        let event_type = TaskEventType::from(raw_event_type.as_str());
        let canonical: &'static str = event_type.into();
        if canonical != raw_event_type.as_str() {
            return Err(TaskEventModelError::UnknownEventType {
                event_type: raw_event_type,
            });
        }

        let event_data = serde_json::from_str(&raw_event_data)
            .map_err(|source| TaskEventModelError::InvalidEventData { source })?;

        Ok(DomainTaskEvent {
            id: TaskEventId::new(id)?,
            task_id: TaskId::new(task_id)?,
            user_id: user_id.map(UserId::new).transpose()?,
            event_type,
            event_data,
            created_at,
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

        let event_type: &'static str = value.event_type.into();

        Ok(Self {
            task_id: value.task_id.get(),
            user_id: value.user_id.map(|id| id.get()),
            event_type: event_type.to_string(),
            event_data,
            created_at: value.created_at,
        })
    }
}
