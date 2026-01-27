use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::domain::{
    client::Client,
    task::{Task, TaskPriority, TaskStatus},
    user::User,
};

#[derive(Serialize, Deserialize)]
pub struct ZmqTaskAuthor {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct ZmqTaskAssignee {
    pub name: String,
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct ZmqTaskClient {
    pub name: String,
    pub public_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct ZmqTask {
    pub public_id: String,
    pub hub_id: i32,
    pub title: String,
    pub priority: TaskPriority,
    pub status: TaskStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub due_date: Option<NaiveDate>,
    pub completed_at: Option<NaiveDateTime>,
    pub author: ZmqTaskAuthor,
    pub client: Option<ZmqTaskClient>,
    pub assignee: Option<ZmqTaskAssignee>,
    pub description: Option<String>,
    pub track: Option<String>,
}

/// Errors produced when assembling a ZMQ task payload from domain entities.
#[derive(Debug, Error)]
pub enum ZmqTaskBuildError {
    /// Task lacks a public identifier required for ZMQ snapshots.
    #[error("task public_id is missing")]
    MissingPublicId,
}

impl From<&User> for ZmqTaskAuthor {
    fn from(value: &User) -> Self {
        Self {
            name: value.name.to_string(),
            email: value.email.to_string(),
        }
    }
}

impl From<&User> for ZmqTaskAssignee {
    fn from(value: &User) -> Self {
        Self {
            name: value.name.to_string(),
            email: value.email.to_string(),
        }
    }
}

impl From<&Client> for ZmqTaskClient {
    fn from(value: &Client) -> Self {
        Self {
            name: value.name.to_string(),
            public_id: value.public_id.to_string(),
        }
    }
}

impl TryFrom<(&Task, &User, Option<&User>, Option<&Client>)> for ZmqTask {
    type Error = ZmqTaskBuildError;

    fn try_from(
        value: (&Task, &User, Option<&User>, Option<&Client>),
    ) -> Result<Self, Self::Error> {
        let (task, author, assignee, client) = value;
        let public_id = task
            .public_id
            .as_ref()
            .map(ToString::to_string)
            .ok_or(ZmqTaskBuildError::MissingPublicId)?;

        Ok(Self {
            public_id,
            hub_id: task.hub_id.get(),
            title: task.title.to_string(),
            priority: task.priority,
            status: task.status,
            created_at: task.created_at,
            updated_at: task.updated_at,
            due_date: task.due_date,
            completed_at: task.completed_at,
            author: author.into(),
            client: client.map(Into::into),
            assignee: assignee.map(Into::into),
            description: task.description.as_ref().map(ToString::to_string),
            track: task.track.as_ref().map(ToString::to_string),
        })
    }
}
