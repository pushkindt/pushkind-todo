//! Forms and normalization helpers used when editing tasks or recording comments.
use chrono::NaiveDate;
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::domain::{
    task::{TaskPriority, TaskStatus},
    types::{
        HubId, TaskComment, TaskDescription, TaskTitle, TaskTrack, TypeConstraintError, UserEmail,
        UserName,
    },
    user::NewUser,
};
use crate::forms::FormError;

/// Form payload submitted from the task edit modal.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTaskForm {
    /// Updated task title provided by the user.
    #[validate(length(min = 1))]
    pub title: String,
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
    pub priority: String,
    /// Assignee data captured by the modal.
    #[serde(flatten, default)]
    pub assignee: AssigneeSelectionForm,
}

#[derive(Debug, Default, Deserialize, Validate)]
pub struct AssigneeSelectionForm {
    #[validate(length(min = 1))]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub name: Option<String>,
    #[validate(email)]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,
}

/// Normalized data constructed from the update task form submission.
#[derive(Debug)]
pub struct UpdateTaskPayload {
    /// Updated title provided in the form.
    pub title: TaskTitle,
    /// Sanitized HTML description or `None` to clear it.
    pub description: Option<TaskDescription>,
    /// Desired track update action.
    pub track: Option<TaskTrack>,
    /// Desired priority update, if supplied.
    pub priority: TaskPriority,
    /// Desired status for the task after the update.
    pub status: TaskStatus,
    /// Parsed due date value, if provided.
    pub due_date: Option<NaiveDate>,
    /// Optional assignee selected in the form.
    pub assignee: Option<AssigneeSelectionPayload>,
}

/// User data captured by the modal when assigning a task.
#[derive(Debug, Clone)]
pub struct AssigneeSelectionPayload {
    /// Display name for the selected user.
    pub name: UserName,
    /// Email address for the selected user.
    pub email: UserEmail,
}

/// Form payload submitted when leaving a new comment on a task.
#[derive(Debug, Deserialize, Validate)]
pub struct TaskCommentForm {
    /// Free-form comment body written by the user.
    #[validate(length(min = 1))]
    pub message: String,
}

pub struct TaskCommentPayload {
    pub message: TaskComment,
}

/// Form payload used for quick status actions on the task page.
#[derive(Debug, Deserialize)]
pub struct QuickTaskStatusForm {
    /// Desired status to set on the task.
    pub status: TaskStatus,
    /// Optional comment to record alongside the status change.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub comment: Option<String>,
    /// Whether the current user should become the assignee.
    #[serde(default)]
    pub assign_self: bool,
}

pub struct QuickTaskStatusPayload {
    /// Desired status to set on the task.
    pub status: TaskStatus,
    /// Optional comment to record alongside the status change.
    pub comment: Option<TaskComment>,
    /// Whether the current user should become the assignee.
    pub assign_self: bool,
}

impl TryFrom<QuickTaskStatusForm> for QuickTaskStatusPayload {
    type Error = FormError;

    fn try_from(form: QuickTaskStatusForm) -> Result<Self, Self::Error> {
        let QuickTaskStatusForm {
            status,
            comment,
            assign_self,
        } = form;

        Ok(Self {
            status,
            comment: comment
                .map(TaskComment::new)
                .transpose()
                .map_err(|_| FormError::InvalidComment)?,
            assign_self,
        })
    }
}

impl TryFrom<TaskCommentForm> for TaskCommentPayload {
    type Error = FormError;

    fn try_from(form: TaskCommentForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        Ok(Self {
            message: TaskComment::new(form.message).map_err(|_| FormError::InvalidComment)?,
        })
    }
}

impl TryFrom<UpdateTaskForm> for UpdateTaskPayload {
    type Error = FormError;

    fn try_from(form: UpdateTaskForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;
        let UpdateTaskForm {
            title,
            message,
            due_date,
            status,
            track,
            priority,
            assignee,
        } = form;

        Ok(Self {
            title: TaskTitle::new(title).map_err(|_| FormError::InvalidTitle)?,
            description: message
                .map(TaskDescription::new)
                .transpose()
                .map_err(|_| FormError::InvalidDescription)?,
            track: track
                .map(TaskTrack::new)
                .transpose()
                .map_err(|_| FormError::InvalidTrack)?,
            priority: TaskPriority::try_from(priority.as_str())
                .map_err(|_| FormError::InvalidPriority)?,
            status,
            due_date: due_date
                .map(|value| NaiveDate::parse_from_str(value.as_str(), "%Y-%m-%d"))
                .transpose()
                .map_err(|_| FormError::InvalidDueDate)?,
            assignee: assignee.try_into()?,
        })
    }
}

impl AssigneeSelectionPayload {
    /// Convert the selection into a new user payload.
    pub fn into_domain(self, hub_id: HubId) -> Result<NewUser, TypeConstraintError> {
        let name = UserName::new(self.name)?;
        let email = UserEmail::new(self.email)?;

        Ok(NewUser::new(hub_id, name, email))
    }
}

impl TryFrom<AssigneeSelectionForm> for Option<AssigneeSelectionPayload> {
    type Error = FormError;
    fn try_from(form: AssigneeSelectionForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;
        match (form.name, form.email) {
            (Some(name), Some(email)) => Ok(Some(AssigneeSelectionPayload {
                name: UserName::new(name).map_err(|_| FormError::InvalidAssigneeName)?,
                email: UserEmail::new(email).map_err(|_| FormError::InvalidAssigneeEmail)?,
            })),
            _ => Ok(None),
        }
    }
}
