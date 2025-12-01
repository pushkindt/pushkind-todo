use chrono::NaiveDate;
use pushkind_common::routes::empty_string_as_none;
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use validator::Validate;

use crate::domain::{
    task::{TaskPriority, TaskStatus},
    types::{TaskDescription, TaskTitle, TaskTrack},
    user::NewUser,
};

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
    /// Updated track value submitted in the form.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub track: Option<String>,
    /// Updated priority level submitted in the form.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub priority: Option<String>,
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
    pub message: String,
}

impl NewTaskCommentForm {
    /// Normalize the submitted comment text by trimming surrounding whitespace.
    pub fn into_submission(self) -> TaskCommentSubmission {
        TaskCommentSubmission {
            text: ammonia::clean(&self.message),
        }
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
            track,
            priority,
            assignee,
        } = self;

        let title = title
            .ok_or(UpdateTaskFormError::MissingTitle)
            .and_then(|t| TaskTitle::new(t).map_err(|_| UpdateTaskFormError::MissingTitle))?;

        let due_date = match due_date {
            Some(value) => Some(parse_due_date(&value)?),
            None => None,
        };

        let priority = match priority {
            Some(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    let parsed = TaskPriority::from(trimmed);
                    let canonical: &str = <&str>::from(parsed);

                    if canonical == trimmed {
                        Some(parsed)
                    } else {
                        return Err(UpdateTaskFormError::InvalidPriority { value });
                    }
                }
            }
            None => None,
        };

        Ok(TaskUpdateSubmission {
            task_id,
            title,
            description: match message {
                Some(body) => {
                    let sanitized = ammonia::clean(&body);
                    if sanitized.trim().is_empty() {
                        None
                    } else {
                        Some(TaskDescription::from(sanitized))
                    }
                }
                None => None,
            },
            track: track.and_then(|t| TaskTrack::new(t.trim()).ok()),
            priority,
            status,
            due_date,
            assignee: assignee.into_selection(),
        })
    }
}

/// Normalized data constructed from the update task form submission.
#[derive(Debug)]
pub struct TaskUpdateSubmission {
    /// Identifier of the task being updated.
    pub task_id: i32,
    /// Updated title provided in the form.
    pub title: TaskTitle,
    /// Sanitized HTML description or `None` to clear it.
    pub description: Option<TaskDescription>,
    /// Desired track update action.
    pub track: Option<TaskTrack>,
    /// Desired priority update, if supplied.
    pub priority: Option<TaskPriority>,
    /// Desired status for the task after the update.
    pub status: TaskStatus,
    /// Parsed due date value, if provided.
    pub due_date: Option<NaiveDate>,
    /// Optional assignee selected in the form.
    pub assignee: Option<AssigneeSelection>,
}

/// User data captured by the modal when assigning a task.
#[derive(Debug, Clone)]
pub struct AssigneeSelection {
    /// Display name for the selected user.
    pub name: String,
    /// Email address for the selected user.
    pub email: String,
}

impl AssigneeSelection {
    /// Convert the selection into a new user payload.
    pub fn into_new_user(
        self,
        hub_id: i32,
    ) -> Result<NewUser, crate::domain::types::TypeConstraintError> {
        use crate::domain::types::{HubId, UserEmail, UserName};

        let hub_id = HubId::new(hub_id)?;
        let name = UserName::new(self.name)?;
        let email = UserEmail::new(self.email)?;

        Ok(NewUser::new(hub_id, name, email))
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct AssigneeSelectionForm {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,
}

impl AssigneeSelectionForm {
    pub(crate) fn into_selection(self) -> Option<AssigneeSelection> {
        match self.email {
            Some(email) => Some(AssigneeSelection {
                name: self.name.unwrap_or_default(),
                email,
            }),
            None => None,
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
    #[error("Invalid priority value '{value}'.")]
    InvalidPriority { value: String },
    #[error("Invalid track value")]
    InvalidTrack,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn into_submission_sanitizes_track_and_priority() {
        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "track": "Alpha",
            "priority": "High",
        }))
        .expect("valid form payload");

        let submission = form
            .into_submission(7)
            .expect("expected successful conversion");

        assert_eq!(submission.track.as_ref().map(|t| t.as_str()), Some("Alpha"));
        assert_eq!(submission.title.as_str(), "Updated");
        assert_eq!(submission.priority, Some(TaskPriority::High));
    }

    #[test]
    fn into_submission_clears_track_when_empty() {
        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "track": "",
        }))
        .expect("valid form payload");

        let submission = form
            .into_submission(3)
            .expect("expected successful conversion");

        assert!(submission.track.is_none());
        assert!(submission.priority.is_none());
    }

    #[test]
    fn into_submission_rejects_invalid_priority() {
        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "priority": "Invalid",
        }))
        .expect("valid form payload");

        let err = form
            .into_submission(5)
            .expect_err("expected invalid priority");

        match err {
            UpdateTaskFormError::InvalidPriority { value } => assert_eq!(value, "Invalid"),
            other => panic!("expected invalid priority error, got {other:?}"),
        }
    }
}
