use chrono::NaiveDate;
use pushkind_common::routes::empty_string_as_none;
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use validator::Validate;

use crate::domain::task::{TaskStatus, UpdateTask};

/// Form payload submitted from the task edit modal.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTaskForm {
    /// Updated task title provided by the user.
    #[validate(length(min = 1))]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub title: Option<String>,
    /// Updated task description in HTML rendered from Markdown.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
    /// Optional task due date in `YYYY-MM-DD` format.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub due_date: Option<String>,
    /// Updated status selected in the form.
    pub status: TaskStatus,
    /// Assignee data captured by the modal.
    #[serde(flatten, default)]
    pub assignee: AssigneeSelectionForm,
}

/// Form payload submitted when leaving a new comment on a task.
#[derive(Debug, Deserialize, Validate)]
pub struct NewTaskCommentForm {
    /// Free-form comment body written by the user.
    #[validate(length(min = 1))]
    #[serde(deserialize_with = "deserialize_trimmed_string")]
    pub text: String,
}

impl NewTaskCommentForm {
    /// Normalize the submitted comment text by trimming surrounding whitespace.
    pub fn into_submission(self) -> TaskCommentSubmission {
        TaskCommentSubmission { text: self.text }
    }
}

/// Normalized data constructed from the new task comment form submission.
#[derive(Debug)]
pub struct TaskCommentSubmission {
    /// Trimmed comment text provided by the user.
    pub text: String,
}

impl UpdateTaskForm {
    /// Convert the submitted form into a normalized update payload.
    pub fn into_submission(
        self,
        task_id: i32,
    ) -> Result<TaskUpdateSubmission, UpdateTaskFormError> {
        let UpdateTaskForm {
            title,
            message,
            due_date,
            status,
            assignee,
        } = self;

        let title = title.ok_or(UpdateTaskFormError::MissingTitle)?;

        let due_date = match due_date {
            Some(value) => Some(parse_due_date(&value)?),
            None => None,
        };

        let mut updates = UpdateTask::new().title(title).status(status);

        updates = match message {
            Some(body) => {
                let sanitized = ammonia::clean(&body);
                if sanitized.trim().is_empty() {
                    updates.clear_description()
                } else {
                    updates.description(sanitized)
                }
            }
            None => updates.clear_description(),
        };

        updates = match due_date {
            Some(date) => updates.due_date(date),
            None => updates.clear_due_date(),
        };

        Ok(TaskUpdateSubmission {
            task_id,
            updates,
            assignee: assignee.into_selection(),
        })
    }
}

/// Normalized data constructed from the update task form submission.
#[derive(Debug)]
pub struct TaskUpdateSubmission {
    /// Identifier of the task being updated.
    pub task_id: i32,
    /// Domain update payload built from the form fields.
    pub updates: UpdateTask,
    /// Optional assignee selected in the form.
    pub assignee: Option<AssigneeSelection>,
}

/// User data captured by the modal when assigning a task.
#[derive(Debug, Clone)]
pub struct AssigneeSelection {
    /// External identifier returned by the identity provider (when available).
    pub id: Option<String>,
    /// Display name for the selected user.
    pub name: Option<String>,
    /// Email address for the selected user.
    pub email: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct AssigneeSelectionForm {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub id: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,
}

impl AssigneeSelectionForm {
    fn into_selection(self) -> Option<AssigneeSelection> {
        let selection = AssigneeSelection {
            id: self.id,
            name: self.name,
            email: self.email.map(|value| value.to_lowercase()),
        };

        if selection.id.is_none() && selection.name.is_none() && selection.email.is_none() {
            None
        } else {
            Some(selection)
        }
    }
}

/// Errors that can occur while converting the task update form.
#[derive(Debug, Error)]
pub enum UpdateTaskFormError {
    #[error("Title is required")]
    MissingTitle,
    #[error("Invalid due date value '{value}'. Expected format YYYY-MM-DD.")]
    InvalidDueDate { value: String },
}

fn parse_due_date(value: &str) -> Result<NaiveDate, UpdateTaskFormError> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| UpdateTaskFormError::InvalidDueDate {
        value: value.to_string(),
    })
}

fn deserialize_trimmed_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    Ok(value.trim().to_string())
}
