//! Diesel task models and conversions that map the database schema to domain types.
use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;

use crate::domain::{
    task::{
        NewTask as DomainNewTask, Task as DomainTask, TaskAssignment as DomainTaskAssignment,
        TaskPriority, TaskStatus, UpdateTask as DomainUpdateTask,
    },
    types::{
        ClientId, HubId, TaskDescription, TaskId, TaskTitle, TaskTrack, TypeConstraintError, UserId,
    },
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
    pub client_id: Option<i32>,
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
    pub client_id: Option<i32>,
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
    pub client_id: Option<i32>,
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

impl TryFrom<Task> for DomainTask {
    type Error = TypeConstraintError;

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
            client_id,
        } = value;

        let status = TaskStatus::try_from(status_text.as_str())?;

        let priority = {
            let candidate = TaskPriority::try_from(priority_text.as_str())?;
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
            description: description.map(TaskDescription::new).transpose()?,
            track: track.map(TaskTrack::new).transpose()?,
            priority,
            status,
            due_date,
            assigned_to: assigned_to.map(UserId::new).transpose()?,
            author_id: UserId::new(author_id)?,
            created_at,
            updated_at,
            completed_at,
            client_id: client_id.map(ClientId::new).transpose()?,
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
            client_id: value.client_id.map(|id| id.get()),
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
            client_id: value.client_id.map(|id| id.get()),
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
