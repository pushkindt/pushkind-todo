use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use thiserror::Error;

use crate::domain::task::{
    NewTask as DomainNewTask, Task as DomainTask, TaskAssignment as DomainTaskAssignment,
    TaskStatus, UpdateTask as DomainUpdateTask,
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
pub struct UpdateTask<'a> {
    pub title: Option<&'a str>,
    pub description: Option<Option<&'a str>>,
    pub status: Option<&'a str>,
    pub due_date: Option<Option<NaiveDate>>,
    pub assigned_to: Option<Option<i32>>,
    pub completed_at: Option<Option<NaiveDateTime>>,
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
}

impl From<Task> for DomainTask {
    fn from(value: Task) -> Self {
        let Task {
            id,
            hub_id,
            title,
            description,
            status: status_text,
            due_date,
            assigned_to,
            author_id,
            created_at,
            updated_at,
            completed_at,
        } = value;

        match status_from_db(&status_text) {
            Ok(status) => Self {
                id,
                hub_id,
                title,
                description,
                status,
                due_date,
                assigned_to,
                author_id,
                created_at,
                updated_at,
                completed_at,
            },
            Err(err) => {
                log::warn!("Failed to decode task status: {err}");
                Self {
                    id,
                    hub_id,
                    title,
                    description,
                    status: TaskStatus::Pending,
                    due_date,
                    assigned_to,
                    author_id,
                    created_at,
                    updated_at,
                    completed_at,
                }
            }
        }
    }
}

impl Task {
    pub fn try_into_domain(self) -> Result<DomainTask, TaskModelError> {
        let status = status_from_db(&self.status)?;
        Ok(DomainTask {
            id: self.id,
            hub_id: self.hub_id,
            title: self.title,
            description: self.description,
            status,
            due_date: self.due_date,
            assigned_to: self.assigned_to,
            author_id: self.author_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            completed_at: self.completed_at,
        })
    }
}

impl<'a> From<&'a DomainNewTask> for NewTask<'a> {
    fn from(value: &'a DomainNewTask) -> Self {
        Self {
            hub_id: value.hub_id,
            title: value.title.as_str(),
            description: value.description.as_deref(),
            status: status_to_db(value.status),
            due_date: value.due_date,
            assigned_to: value.assigned_to,
            author_id: value.author_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            completed_at: None,
        }
    }
}

impl<'a> From<&'a DomainUpdateTask> for UpdateTask<'a> {
    fn from(value: &'a DomainUpdateTask) -> Self {
        Self {
            title: value.title.as_deref(),
            description: value
                .description
                .as_ref()
                .map(|opt| opt.as_ref().map(|text| text.as_str())),
            status: value.status.map(status_to_db),
            due_date: value.due_date,
            assigned_to: value.assigned_to,
            completed_at: value.completed_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<TaskAssignment> for DomainTaskAssignment {
    fn from(value: TaskAssignment) -> Self {
        Self {
            task_id: value.task_id,
            hub_id: value.hub_id,
            assignee_id: value.assignee_id,
            assigned_at: value.assigned_at,
        }
    }
}

impl From<&DomainTaskAssignment> for NewTaskAssignment {
    fn from(value: &DomainTaskAssignment) -> Self {
        Self {
            task_id: value.task_id,
            hub_id: value.hub_id,
            assignee_id: value.assignee_id,
            assigned_at: value.assigned_at,
        }
    }
}

pub(crate) fn status_to_db(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "Pending",
        TaskStatus::InProgress => "InProgress",
        TaskStatus::Blocked => "Blocked",
        TaskStatus::Completed => "Completed",
        TaskStatus::Archived => "Archived",
    }
}

fn status_from_db(status: &str) -> Result<TaskStatus, TaskModelError> {
    match status {
        "Pending" => Ok(TaskStatus::Pending),
        "InProgress" => Ok(TaskStatus::InProgress),
        "Blocked" => Ok(TaskStatus::Blocked),
        "Completed" => Ok(TaskStatus::Completed),
        "Archived" => Ok(TaskStatus::Archived),
        other => Err(TaskModelError::UnknownStatus {
            status: other.to_string(),
        }),
    }
}
