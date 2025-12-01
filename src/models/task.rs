use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use thiserror::Error;

use crate::domain::{
    task::{
        NewTask as DomainNewTask, Task as DomainTask, TaskAssignment as DomainTaskAssignment,
        TaskPriority, TaskStatus, UpdateTask as DomainUpdateTask,
    },
    types::{HubId, TaskDescription, TaskId, TaskTitle, TaskTrack, UserId},
};

use super::user::User;

#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(belongs_to(User, foreign_key = assigned_to))]
pub struct Task {
    pub id: i32,
    pub hub_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub track: Option<String>,
    pub priority: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub assigned_to: Option<i32>,
    pub author_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::tasks)]
pub struct NewTask<'a> {
    pub hub_id: i32,
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub track: Option<&'a str>,
    pub priority: &'a str,
    pub status: &'a str,
    pub due_date: Option<NaiveDate>,
    pub assigned_to: Option<i32>,
    pub author_id: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub completed_at: Option<NaiveDateTime>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = crate::schema::tasks)]
#[diesel(treat_none_as_null = true)]
pub struct UpdateTask<'a> {
    pub title: &'a str,
    pub description: Option<&'a str>,
    pub track: Option<&'a str>,
    pub priority: &'a str,
    pub status: &'a str,
    pub due_date: Option<NaiveDate>,
    pub assigned_to: Option<i32>,
    pub completed_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Identifiable, Queryable, Selectable, Associations)]
#[diesel(table_name = crate::schema::task_assignments)]
#[diesel(belongs_to(Task))]
#[diesel(belongs_to(User, foreign_key = assignee_id))]
pub struct TaskAssignment {
    pub id: i32,
    pub task_id: i32,
    pub hub_id: i32,
    pub assignee_id: i32,
    pub assigned_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::task_assignments)]
pub struct NewTaskAssignment {
    pub task_id: i32,
    pub hub_id: i32,
    pub assignee_id: i32,
    pub assigned_at: NaiveDateTime,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TaskModelError {
    #[error("Unknown task status '{status}'")]
    UnknownStatus { status: String },
    #[error("Unknown task priority '{priority}'")]
    UnknownPriority { priority: String },
    #[error("Invalid type constraint: {0}")]
    TypeConstraint(#[from] crate::domain::types::TypeConstraintError),
}

impl TryFrom<Task> for DomainTask {
    type Error = crate::domain::types::TypeConstraintError;

    fn try_from(value: Task) -> Result<Self, Self::Error> {
        let Task {
            id,
            hub_id,
            title,
            description,
            track,
            priority: priority_text,
            status: status_text,
            due_date,
            assigned_to,
            author_id,
            created_at,
            updated_at,
            completed_at,
        } = value;

        let status = {
            let candidate = TaskStatus::from(status_text.as_str());
            let canonical: &'static str = candidate.into();
            if canonical == status_text.as_str() {
                candidate
            } else {
                log::warn!("Failed to decode task status '{}'", status_text);
                TaskStatus::Pending
            }
        };

        let priority = {
            let candidate = TaskPriority::from(priority_text.as_str());
            let canonical: &'static str = candidate.into();
            if canonical == priority_text.as_str() {
                candidate
            } else {
                log::warn!("Failed to decode task priority '{}'", priority_text);
                TaskPriority::default()
            }
        };

        Ok(Self {
            id: TaskId::new(id)?,
            hub_id: HubId::new(hub_id)?,
            title: TaskTitle::new(title)?,
            description: description.map(TaskDescription::from),
            track: track.map(TaskTrack::new).transpose()?,
            priority,
            status,
            due_date,
            assigned_to: assigned_to.map(UserId::new).transpose()?,
            author_id: UserId::new(author_id)?,
            created_at,
            updated_at,
            completed_at,
        })
    }
}

impl Task {
    pub fn try_into_domain(self) -> Result<DomainTask, TaskModelError> {
        let Self {
            id,
            hub_id,
            title,
            description,
            track,
            status: raw_status,
            priority: raw_priority,
            due_date,
            assigned_to,
            author_id,
            created_at,
            updated_at,
            completed_at,
        } = self;

        let status = TaskStatus::from(raw_status.as_str());
        let canonical: &'static str = status.into();
        if canonical != raw_status.as_str() {
            return Err(TaskModelError::UnknownStatus { status: raw_status });
        }

        let priority = TaskPriority::from(raw_priority.as_str());
        let canonical_priority: &'static str = priority.into();
        if canonical_priority != raw_priority.as_str() {
            return Err(TaskModelError::UnknownPriority {
                priority: raw_priority,
            });
        }

        Ok(DomainTask {
            id: TaskId::new(id)?,
            hub_id: HubId::new(hub_id)?,
            title: TaskTitle::new(title)?,
            description: description.map(TaskDescription::from),
            track: track.map(TaskTrack::new).transpose()?,
            priority,
            status,
            due_date,
            assigned_to: assigned_to.map(UserId::new).transpose()?,
            author_id: UserId::new(author_id)?,
            created_at,
            updated_at,
            completed_at,
        })
    }
}

impl<'a> From<&'a DomainNewTask> for NewTask<'a> {
    fn from(value: &'a DomainNewTask) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            title: value.title.as_str(),
            description: value.description.as_ref().map(|d| d.as_str()),
            track: value.track.as_ref().map(|t| t.as_str()),
            priority: <&'static str>::from(value.priority),
            status: <&'static str>::from(value.status),
            due_date: value.due_date,
            assigned_to: value.assigned_to.map(|id| id.get()),
            author_id: value.author_id.get(),
            created_at: value.created_at,
            updated_at: value.updated_at,
            completed_at: None,
        }
    }
}

impl<'a> From<&'a DomainUpdateTask> for UpdateTask<'a> {
    fn from(value: &'a DomainUpdateTask) -> Self {
        Self {
            title: value.title.as_str(),
            description: value.description.as_ref().map(|d| d.as_str()),
            track: value.track.as_ref().map(|t| t.as_str()),
            priority: <&'static str>::from(value.priority),
            status: <&'static str>::from(value.status),
            due_date: value.due_date,
            assigned_to: value.assigned_to.map(|id| id.get()),
            completed_at: value.completed_at,
            updated_at: value.updated_at,
        }
    }
}

impl TryFrom<TaskAssignment> for DomainTaskAssignment {
    type Error = crate::domain::types::TypeConstraintError;

    fn try_from(value: TaskAssignment) -> Result<Self, Self::Error> {
        Ok(Self {
            task_id: TaskId::new(value.task_id)?,
            hub_id: HubId::new(value.hub_id)?,
            assignee_id: UserId::new(value.assignee_id)?,
            assigned_at: value.assigned_at,
        })
    }
}

impl From<&DomainTaskAssignment> for NewTaskAssignment {
    fn from(value: &DomainTaskAssignment) -> Self {
        Self {
            task_id: value.task_id.get(),
            hub_id: value.hub_id.get(),
            assignee_id: value.assignee_id.get(),
            assigned_at: value.assigned_at,
        }
    }
}
