use std::collections::HashMap;

use log::error;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::domain::emailer::email::{NewEmail, NewEmailRecipient};
use pushkind_common::models::emailer::zmq::ZMQSendEmailMessage;
use pushkind_common::zmq::ZmqSenderExt;

use crate::domain::{task::Task, user::User};
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

pub(super) fn task_recipient(
    task: &Task,
    user: &User,
    notification_kind: &str,
    recipient_role: &str,
) -> NewEmailRecipient {
    let mut fields = HashMap::new();
    fields.insert("task_id".to_string(), task.id.to_string());
    fields.insert("task_title".to_string(), task.title.to_string());
    fields.insert("task_status".to_string(), format!("{:?}", task.status));
    fields.insert(
        "notification_kind".to_string(),
        notification_kind.to_string(),
    );
    fields.insert("recipient_role".to_string(), recipient_role.to_string());

    NewEmailRecipient {
        address: user.email.to_string(),
        name: user.name.to_string(),
        fields,
    }
}

pub(super) fn sanitize_text(value: &str) -> String {
    ammonia::clean(value).trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::sanitize_text;

    #[test]
    fn retains_allowed_markup_in_sanitized_output() {
        let result = sanitize_text("<strong>Hello</strong> world");
        assert_eq!(result, "<strong>Hello</strong> world");
    }

    #[test]
    fn does_not_restore_removed_html_fragments() {
        let result = sanitize_text("<script>alert(1)</script>");
        assert!(result.is_empty());
    }
}
