//! Forms and normalization helpers used when editing tasks or recording comments.
use chrono::NaiveDate;
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::domain::{
    client::NewClient,
    task::{TaskPriority, TaskStatus},
    types::{
        ClientName, ClientPublicId, HubId, TaskComment, TaskDescription, TaskTitle, TaskTrack,
        UserEmail, UserName,
    },
    user::NewUser,
};
use crate::forms::FormError;

/// Form payload submitted from the task edit modal.
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateTaskForm {
    /// Updated task title provided by the user.
    #[serde(default)]
    #[validate(length(min = 1, message = "Укажите название задачи."))]
    pub title: String,
    /// Updated task description in HTML rendered from Markdown.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
    /// Optional task due date in `YYYY-MM-DD` format.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub due_date: Option<String>,
    /// Updated status selected in the form.
    #[serde(default)]
    #[validate(length(min = 1, message = "Выберите статус задачи."))]
    pub status: String,
    /// Updated track value submitted in the form.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub track: Option<String>,
    /// Updated priority level submitted in the form.
    #[serde(default)]
    #[validate(length(min = 1, message = "Выберите приоритет задачи."))]
    pub priority: String,
    /// Assignee data captured by the modal.
    #[serde(flatten, default)]
    pub assignee: AssigneeSelectionForm,
    /// Client data captured by the modal.
    #[serde(flatten, default)]
    pub client: ClientSelectionForm,
}

#[derive(Debug, Default, Deserialize, Validate)]
pub struct AssigneeSelectionForm {
    #[validate(length(min = 1, message = "Укажите имя исполнителя."))]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub name: Option<String>,
    #[validate(email(message = "Укажите корректный email исполнителя."))]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub email: Option<String>,
}

#[derive(Debug, Default, Deserialize, Validate)]
pub struct ClientSelectionForm {
    #[validate(length(min = 1, message = "Укажите клиента."))]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub client_name: Option<String>,
    #[validate(length(min = 1, message = "Укажите корректный идентификатор клиента."))]
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub client_public_id: Option<String>,
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
    /// Optional client selected in the form.
    pub client: Option<ClientSelectionPayload>,
}

/// User data captured by the modal when assigning a task.
#[derive(Debug, Clone)]
pub struct AssigneeSelectionPayload {
    /// Display name for the selected user.
    pub name: UserName,
    /// Email address for the selected user.
    pub email: UserEmail,
}

/// Client data captured by the modal when assigning a task.
#[derive(Debug, Clone)]
pub struct ClientSelectionPayload {
    /// Display name for the selected client.
    pub name: ClientName,
    /// Public identifier for the selected client.
    pub public_id: ClientPublicId,
}

/// Form payload submitted when leaving a new comment on a task.
#[derive(Debug, Deserialize, Validate)]
pub struct TaskCommentForm {
    /// Free-form comment body written by the user.
    #[serde(default)]
    #[validate(length(min = 1, message = "Введите комментарий."))]
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct TaskCommentPayload {
    pub message: TaskComment,
}

/// Form payload used for quick status actions on the task page.
#[derive(Debug, Deserialize, Validate)]
pub struct QuickTaskStatusForm {
    /// Desired status to set on the task.
    #[serde(default)]
    #[validate(length(min = 1, message = "Выберите статус задачи."))]
    pub status: String,
    /// Optional comment to record alongside the status change.
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub comment: Option<String>,
    /// Whether the current user should become the assignee.
    #[serde(default)]
    pub assign_self: bool,
}

#[derive(Debug, Clone)]
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
        form.validate().map_err(FormError::Validation)?;

        let QuickTaskStatusForm {
            status,
            comment,
            assign_self,
        } = form;

        Ok(Self {
            status: TaskStatus::try_from(status.as_str()).map_err(|_| FormError::InvalidStatus)?,
            comment: comment
                .map(TaskComment::new)
                .transpose()
                .map_err(|_| FormError::InvalidQuickComment)?,
            assign_self,
        })
    }
}

impl TryFrom<TaskCommentForm> for TaskCommentPayload {
    type Error = FormError;

    fn try_from(form: TaskCommentForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        Ok(Self {
            message: TaskComment::new(form.message)
                .map_err(|_| FormError::InvalidCommentMessage)?,
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
            client,
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
            status: TaskStatus::try_from(status.as_str()).map_err(|_| FormError::InvalidStatus)?,
            due_date: due_date
                .map(|value| NaiveDate::parse_from_str(value.as_str(), "%Y-%m-%d"))
                .transpose()
                .map_err(|_| FormError::InvalidDueDate)?,
            assignee: assignee.try_into()?,
            client: client.try_into()?,
        })
    }
}

impl AssigneeSelectionPayload {
    /// Convert the selection into a new user payload.
    pub fn into_domain(self, hub_id: HubId) -> NewUser {
        NewUser::new(hub_id, self.name, self.email)
    }
}

impl TryFrom<AssigneeSelectionForm> for Option<AssigneeSelectionPayload> {
    type Error = FormError;

    fn try_from(form: AssigneeSelectionForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        match (form.name, form.email) {
            (None, None) => Ok(None),
            (Some(name), Some(email)) => Ok(Some(AssigneeSelectionPayload {
                name: UserName::new(name).map_err(|_| FormError::InvalidAssigneeName)?,
                email: UserEmail::new(email).map_err(|_| FormError::InvalidAssigneeEmail)?,
            })),
            (Some(_), None) => Err(FormError::InvalidAssigneeEmail),
            (None, Some(_)) => Err(FormError::InvalidAssigneeName),
        }
    }
}

impl ClientSelectionPayload {
    /// Convert the selection into a new client payload.
    pub fn into_domain(self, hub_id: HubId) -> NewClient {
        NewClient::new(hub_id, self.name, self.public_id)
    }
}

impl TryFrom<ClientSelectionForm> for Option<ClientSelectionPayload> {
    type Error = FormError;

    fn try_from(form: ClientSelectionForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        match (form.client_name, form.client_public_id) {
            (None, None) => Ok(None),
            (Some(name), Some(public_id)) => Ok(Some(ClientSelectionPayload {
                name: ClientName::new(name).map_err(|_| FormError::InvalidClientName)?,
                public_id: ClientPublicId::new(public_id)
                    .map_err(|_| FormError::InvalidClientPublicId)?,
            })),
            (Some(_), None) => Err(FormError::InvalidClientPublicId),
            (None, Some(_)) => Err(FormError::InvalidClientName),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_task_payload_reports_invalid_status_with_field_mapping() {
        let error = UpdateTaskPayload::try_from(UpdateTaskForm {
            title: "Task".to_string(),
            message: None,
            due_date: None,
            status: "invalid".to_string(),
            track: None,
            priority: "middle".to_string(),
            assignee: AssigneeSelectionForm::default(),
            client: ClientSelectionForm::default(),
        })
        .expect_err("status should be invalid");

        assert_eq!(error.to_string(), "Выберите статус задачи.");
        assert_eq!(error.field_errors()[0].field, "status");
    }

    #[test]
    fn assignee_payload_requires_both_name_and_email() {
        let error = Option::<AssigneeSelectionPayload>::try_from(AssigneeSelectionForm {
            name: Some("Исполнитель".to_string()),
            email: None,
        })
        .expect_err("email should be required when name is provided");

        assert_eq!(error.to_string(), "Укажите корректный email исполнителя.");
        assert_eq!(error.field_errors()[0].field, "email");
    }

    #[test]
    fn task_comment_payload_uses_localized_comment_error() {
        let error = TaskCommentPayload::try_from(TaskCommentForm {
            message: "   ".to_string(),
        })
        .expect_err("blank comment should be rejected");

        assert_eq!(error.to_string(), "Введите комментарий.");
        assert_eq!(error.field_errors()[0].field, "message");
    }
}
