//! Notification helpers that build and enqueue emails via ZeroMQ.
use std::collections::BTreeMap;

use log::error;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::zmq::ZmqSenderExt;
use pushkind_emailer::domain::email::{NewEmail, NewEmailRecipient};
use pushkind_emailer::domain::types::{RecipientEmail, RecipientName};
use pushkind_emailer::models::zmq::ZMQSendEmailMessage;

use crate::domain::{task::Task, user::User};
use crate::dto::zmq::ZmqTask;
use crate::services::{ServiceError, ServiceResult};

/// Serialize the email payload and enqueue it for delivery via ZeroMQ.
pub(super) fn queue_email<Z>(
    zmq_sender: &Z,
    actor: &AuthenticatedUser,
    email: NewEmail,
) -> ServiceResult<()>
where
    Z: ZmqSenderExt + ?Sized,
{
    if email.recipients.is_empty() {
        return Ok(());
    }

    let message = ZMQSendEmailMessage::NewEmail(Box::new((actor.clone(), email)));

    let payload = serde_json::to_vec(&message).map_err(|err| {
        error!("Failed to serialize email payload: {err}");
        ServiceError::Internal
    })?;

    pushkind_common::zmq::ZmqSenderTrait::try_send_bytes(zmq_sender, payload)
        .map_err(ServiceError::from)
}

/// Serialize the task snapshot and enqueue it for delivery via ZeroMQ.
pub(super) fn queue_task_snapshot<Z>(zmq_sender: &Z, task: ZmqTask) -> ServiceResult<()>
where
    Z: ZmqSenderExt + ?Sized,
{
    let payload = serde_json::to_vec(&task).map_err(|err| {
        error!("Failed to serialize task snapshot payload: {err}");
        ServiceError::Internal
    })?;

    pushkind_common::zmq::ZmqSenderTrait::try_send_bytes(zmq_sender, payload)
        .map_err(ServiceError::from)
}

/// Build an email recipient entry for task-related notifications.
pub(super) fn task_recipient(
    task: &Task,
    user: &User,
    notification_kind: &str,
    recipient_role: &str,
) -> ServiceResult<NewEmailRecipient> {
    let mut fields = BTreeMap::new();
    fields.insert("task_id".to_string(), task.id.to_string());
    fields.insert("task_title".to_string(), task.title.to_string());
    fields.insert("task_status".to_string(), format!("{:?}", task.status));
    fields.insert(
        "notification_kind".to_string(),
        notification_kind.to_string(),
    );
    fields.insert("recipient_role".to_string(), recipient_role.to_string());

    Ok(NewEmailRecipient {
        address: RecipientEmail::new(user.email.as_str())?,
        name: RecipientName::new(user.name.as_str())?,
        fields,
    })
}

/// Clean and normalize text before embedding in outgoing notifications.
pub(super) fn sanitize_text(value: &str) -> String {
    ammonia::clean(value).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::sanitize_text;

    #[test]
    /// Confirms sanitizer keeps allowed HTML tags intact.
    fn retains_allowed_markup_in_sanitized_output() {
        let result = sanitize_text("<strong>Hello</strong> world");
        assert_eq!(result, "<strong>Hello</strong> world");
    }

    #[test]
    /// Ensures sanitizer omits disallowed tags entirely.
    fn does_not_restore_removed_html_fragments() {
        let result = sanitize_text("<script>alert(1)</script>");
        assert!(result.is_empty());
    }
}
