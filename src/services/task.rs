//! Task-focused service operations for loading details and applying updates.
use std::collections::{HashMap, HashSet};

use chrono::Local;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::repository::errors::RepositoryError;
use pushkind_common::routes::ensure_role;
use pushkind_common::zmq::ZmqSenderExt;
use pushkind_emailer::domain::email::{NewEmail, NewEmailRecipient};

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::types::{HubId, TaskId};
use crate::domain::user::NewUser;
use crate::domain::{
    task::{Task, TaskStatus, UpdateTask},
    task_event::{NewTaskEvent, TaskEvent, TaskEventType},
    types::UserId,
    user::User,
};
use crate::forms::task::{
    QuickTaskStatusForm, QuickTaskStatusPayload, TaskCommentForm, TaskCommentPayload,
    UpdateTaskForm, UpdateTaskPayload,
};
use crate::repository::{
    TaskEventReader, TaskEventWriter, TaskReader, TaskWriter, UserListQuery, UserReader, UserWriter,
};
use crate::services::{ServiceError, ServiceResult};
use serde_json::{Value, json};

use super::notifications;
use crate::dto::task::{TaskDetails, TaskEventWithAuthor, TaskModalData};

/// Load a task and its events for the provided user, enriching with user data.
pub fn load_task_details<R>(
    task_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<TaskDetails>
where
    R: TaskReader + TaskEventReader + UserReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;
    let task_id = TaskId::new(task_id)?;

    let task = repo
        .get_task_by_id(task_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let author_id = task.author_id;
    let author = repo.get_user_by_id(author_id, hub_id)?.ok_or_else(|| {
        log::error!(
            "Task {} references missing author {}",
            task.id,
            task.author_id
        );
        ServiceError::Internal
    })?;

    let assignee = match task.assigned_to {
        Some(assignee_id) => match repo.get_user_by_id(assignee_id, hub_id) {
            Ok(user) => user,
            Err(err) => return Err(ServiceError::from(err)),
        },
        None => None,
    };

    let events = repo.list_events_for_task(task.id, hub_id)?;

    let mut author_cache: HashMap<UserId, User> = HashMap::new();
    for event in &events {
        if let Some(author_id) = event.user_id {
            if author_cache.contains_key(&author_id) {
                continue;
            }

            match repo.get_user_by_id(author_id, hub_id) {
                Ok(Some(user)) => {
                    author_cache.insert(author_id, user);
                }
                Ok(None) => {}
                Err(err) => return Err(ServiceError::from(err)),
            }
        }
    }

    let events = events
        .into_iter()
        .map(|event| {
            let author = event.user_id.and_then(|id| author_cache.get(&id).cloned());

            TaskEventWithAuthor { event, author }
        })
        .collect();

    Ok(TaskDetails {
        task,
        author,
        assignee,
        events,
    })
}

/// Load the task along with supporting data required by the modal view.
pub fn load_task_modal<R>(
    task_id: i32,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<TaskModalData>
where
    R: TaskReader + UserReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;
    let task_id = TaskId::new(task_id)?;

    let task = repo
        .get_task_by_id(task_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let assignee = match task.assigned_to {
        Some(assignee_id) => match repo.get_user_by_id(assignee_id, hub_id) {
            Ok(Some(user)) => Some(user),
            Ok(None) => {
                log::warn!(
                    "Task {} references missing assignee {} in hub {}",
                    task.id,
                    assignee_id,
                    user.hub_id
                );
                None
            }
            Err(err) => return Err(ServiceError::from(err)),
        },
        None => None,
    };

    let (_total, users) = repo.list_users(UserListQuery::new(hub_id))?;

    let tracks = repo.list_task_tracks(hub_id)?;

    Ok(TaskModalData {
        task,
        assignee,
        users,
        tracks,
    })
}

/// Update a task with the values submitted from the edit form.
pub fn update_task<R, Z>(
    task_id: i32,
    form: UpdateTaskForm,
    user: &AuthenticatedUser,
    repo: &R,
    zmq_sender: &Z,
) -> ServiceResult<Task>
where
    R: TaskReader
        + TaskWriter
        + TaskEventReader
        + TaskEventWriter
        + UserReader
        + UserWriter
        + ?Sized,
    Z: ZmqSenderExt,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let submission = UpdateTaskPayload::try_from(form)?;

    let hub_id = HubId::new(user.hub_id)?;
    let task_id = TaskId::new(task_id)?;

    let UpdateTaskPayload {
        title,
        description,
        track,
        priority,
        status,
        due_date,
        assignee,
    } = submission;

    let current_task = repo
        .get_task_by_id(task_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let assignee_user = match assignee {
        Some(assignee) => {
            let new_user = assignee.into_domain(hub_id)?;
            Some(repo.create_or_update_user(&new_user)?)
        }
        None => None,
    };

    let mut updates = UpdateTask::from_task(&current_task)
        .title(title)
        .status(status)
        .priority(priority);

    updates = match description {
        Some(body) => updates.description(body),
        None => updates.clear_description(),
    };

    updates = match track {
        Some(body) => updates.track(body),
        None => updates.clear_track(),
    };

    updates = match due_date {
        Some(date) => updates.due_date(date),
        None => updates.clear_due_date(),
    };

    let updates = apply_assignment_updates(
        updates,
        current_task.assigned_to,
        assignee_user.as_ref().map(|user| user.id),
    );

    let updated = repo.update_task(task_id, hub_id, &updates)?;

    let status_event_data = status_event_payload(current_task.status, updated.status);

    let assignment_event_data = if current_task.assigned_to != updated.assigned_to {
        let previous_assignee = match current_task.assigned_to {
            Some(assignee_id) => repo.get_user_by_id(assignee_id, hub_id)?,
            None => None,
        };

        let new_assignee = match updated.assigned_to {
            Some(assignee_id) => repo.get_user_by_id(assignee_id, hub_id)?,
            None => None,
        };

        assignment_event_payload(previous_assignee.as_ref(), new_assignee.as_ref())
    } else {
        None
    };

    let metadata_event_data = metadata_event_payload(&current_task, &updated);

    if status_event_data.is_some()
        || assignment_event_data.is_some()
        || metadata_event_data.is_some()
    {
        let new_user = NewUser::try_from(user)?;
        let actor = repo.create_or_update_user(&new_user)?;

        if let Some(data) = status_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::StatusChanged,
                data,
            );
            repo.record_event(&event)?;
        }

        if let Some(data) = assignment_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::AssignmentChanged,
                data,
            );
            repo.record_event(&event)?;
        }

        if let Some(data) = metadata_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::MetadataUpdated,
                data,
            );
            repo.record_event(&event)?;
        }

        repo.touch_visited_at(actor.id, actor.hub_id)?;
    }

    let author_user = repo.get_user_by_id(updated.author_id, hub_id)?;

    let event_actors = {
        let mut actors = Vec::new();
        let mut seen_actor_ids = HashSet::new();

        let events = repo.list_events_for_task(updated.id, hub_id)?;

        for event in events {
            if let Some(actor_id) = event.user_id {
                if !seen_actor_ids.insert(actor_id) {
                    continue;
                }

                match repo.get_user_by_id(actor_id, hub_id) {
                    Ok(Some(actor)) => actors.push(actor),
                    Ok(None) => {
                        log::warn!(
                            "Task {} event {} references missing actor {}",
                            updated.id,
                            event.id,
                            actor_id
                        );
                    }
                    Err(err) => return Err(ServiceError::from(err)),
                }
            }
        }

        actors
    };

    if let Some(email) = build_task_updated_email(
        &updated,
        author_user.as_ref(),
        assignee_user.as_ref(),
        &event_actors,
        user,
    ) && let Err(err) = notifications::queue_email(zmq_sender, user, email)
    {
        log::error!("Failed to queue task-updated email: {err}");
    }

    Ok(updated)
}

/// Transition the task status with a lightweight quick action from the UI.
pub fn transition_task_status<R, Z>(
    task_id: i32,
    form: QuickTaskStatusForm,
    user: &AuthenticatedUser,
    repo: &R,
    zmq_sender: &Z,
) -> ServiceResult<Task>
where
    R: TaskReader
        + TaskWriter
        + TaskEventReader
        + TaskEventWriter
        + UserReader
        + UserWriter
        + ?Sized,
    Z: ZmqSenderExt,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let payload: QuickTaskStatusPayload = form.try_into()?;

    let hub_id = HubId::new(user.hub_id)?;
    let task_id = TaskId::new(task_id)?;

    let current_task = repo
        .get_task_by_id(task_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    if current_task.status == payload.status {
        return Err(ServiceError::Form("Статус уже установлен.".to_string()));
    }

    let actor_user = NewUser::try_from(user)?;
    let actor = repo.create_or_update_user(&actor_user)?;

    let previous_assignee = match current_task.assigned_to {
        Some(assignee_id) => match repo.get_user_by_id(assignee_id, hub_id) {
            Ok(user) => user,
            Err(err) => return Err(ServiceError::from(err)),
        },
        None => None,
    };

    let mut updates = UpdateTask::from_task(&current_task).status(payload.status);
    if payload.status == TaskStatus::Completed {
        updates = updates.completed_at(Local::now().naive_utc());
    } else {
        updates = updates.clear_completed_at();
    }

    if payload.assign_self && payload.status == TaskStatus::InProgress {
        updates = updates.assign_to(actor.id);
    }

    let updated = repo
        .update_task(task_id, hub_id, &updates)
        .map_err(|err| match err {
            RepositoryError::NotFound => ServiceError::NotFound,
            other => ServiceError::from(other),
        })?;

    let assignee_user = match updated.assigned_to {
        Some(assignee_id) => match repo.get_user_by_id(assignee_id, hub_id) {
            Ok(user) => user,
            Err(err) => return Err(ServiceError::from(err)),
        },
        None => None,
    };

    let status_event_data = status_event_payload(current_task.status, updated.status);
    let assignment_event_data =
        assignment_event_payload(previous_assignee.as_ref(), assignee_user.as_ref());

    if status_event_data.is_some() || assignment_event_data.is_some() {
        let mut recorded = false;

        if let Some(data) = status_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::StatusChanged,
                data,
            );
            repo.record_event(&event)?;
            recorded = true;
        }

        if let Some(data) = assignment_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::AssignmentChanged,
                data,
            );
            repo.record_event(&event)?;
            recorded = true;
        }

        if recorded {
            repo.touch_visited_at(actor.id, actor.hub_id)?;
        }
    }

    let author_user = repo.get_user_by_id(updated.author_id, hub_id)?;

    let assignee_user = match updated.assigned_to {
        Some(assignee_id) => repo.get_user_by_id(assignee_id, hub_id)?,
        None => None,
    };

    let event_actors = {
        let mut actors = Vec::new();
        let mut seen_actor_ids = HashSet::new();

        let events = repo.list_events_for_task(updated.id, hub_id)?;

        for event in events {
            if let Some(actor_id) = event.user_id {
                if !seen_actor_ids.insert(actor_id) {
                    continue;
                }

                match repo.get_user_by_id(actor_id, hub_id) {
                    Ok(Some(actor)) => actors.push(actor),
                    Ok(None) => {
                        log::warn!(
                            "Task {} event {} references missing actor {}",
                            updated.id,
                            event.id,
                            actor_id
                        );
                    }
                    Err(err) => return Err(ServiceError::from(err)),
                }
            }
        }

        actors
    };

    if let Some(email) = build_task_updated_email(
        &updated,
        author_user.as_ref(),
        assignee_user.as_ref(),
        &event_actors,
        user,
    ) && let Err(err) = notifications::queue_email(zmq_sender, user, email)
    {
        log::error!("Failed to queue task-updated email: {err}");
    }

    if let Some(comment_text) = payload.comment {
        let form = TaskCommentForm {
            message: comment_text.to_string(),
        };
        if let Err(err) = add_task_comment(task_id.get(), form, user, repo, zmq_sender) {
            log::error!("Failed to attach quick status comment for task {task_id}: {err}");
        }
    }

    Ok(updated)
}

/// Adjust the update payload to reflect new assignee decisions.
fn apply_assignment_updates(
    updates: UpdateTask,
    current_assigned_to: Option<UserId>,
    new_assignee_id: Option<UserId>,
) -> UpdateTask {
    match new_assignee_id {
        Some(assignee_id) if current_assigned_to != Some(assignee_id) => {
            updates.assign_to(assignee_id)
        }
        Some(_) => updates,
        None if current_assigned_to.is_some() => updates.unassign(),
        None => updates,
    }
}

/// Generate event data when a task status transitions.
fn status_event_payload(current: TaskStatus, updated: TaskStatus) -> Option<Value> {
    if current == updated {
        None
    } else {
        let from_status: &'static str = current.into();
        let to_status: &'static str = updated.into();
        Some(json!({
            "from": from_status,
            "to": to_status,
        }))
    }
}

/// Generate event data describing assignment changes.
fn assignment_event_payload(
    previous_assignee: Option<&User>,
    new_assignee: Option<&User>,
) -> Option<Value> {
    let previous_id = previous_assignee.map(|user| user.id);
    let new_id = new_assignee.map(|user| user.id);

    if previous_id == new_id {
        None
    } else {
        Some(json!({
            "from": previous_assignee.map(assignment_event_user),
            "to": new_assignee.map(assignment_event_user),
        }))
    }
}

/// Compute metadata diffs for task update events.
fn metadata_event_payload(current: &Task, updated: &Task) -> Option<Value> {
    let mut changes = serde_json::Map::new();

    if current.title != updated.title {
        changes.insert(
            "title".to_string(),
            json!({
                "from": current.title.clone(),
                "to": updated.title.clone(),
            }),
        );
    }

    if current.description != updated.description {
        changes.insert(
            "description".to_string(),
            json!({
                "from": current.description.clone(),
                "to": updated.description.clone(),
            }),
        );
    }

    if current.track != updated.track {
        changes.insert(
            "track".to_string(),
            json!({
                "from": current.track.clone(),
                "to": updated.track.clone(),
            }),
        );
    }

    if current.priority != updated.priority {
        let from_priority: &'static str = current.priority.into();
        let to_priority: &'static str = updated.priority.into();
        changes.insert(
            "priority".to_string(),
            json!({
                "from": from_priority,
                "to": to_priority,
            }),
        );
    }

    if current.due_date != updated.due_date {
        changes.insert(
            "due_date".to_string(),
            json!({
                "from": current.due_date.map(|date| date.to_string()),
                "to": updated.due_date.map(|date| date.to_string()),
            }),
        );
    }

    if current.completed_at != updated.completed_at {
        changes.insert(
            "completed_at".to_string(),
            json!({
                "from": current.completed_at,
                "to": updated.completed_at,
            }),
        );
    }

    if changes.is_empty() {
        None
    } else {
        Some(Value::Object(changes))
    }
}

/// Build notification email listing task updates for interested parties.
fn build_task_updated_email(
    task: &Task,
    author: Option<&User>,
    assignee: Option<&User>,
    event_actors: &[User],
    actor: &AuthenticatedUser,
) -> Option<NewEmail> {
    let actor_email = actor.email.trim().to_lowercase();
    let sanitized_title = notifications::sanitize_text(task.title.as_str());
    let sanitized_actor_name = notifications::sanitize_text(&actor.name);

    let mut recipients = Vec::new();
    let mut seen = HashSet::new();

    if let Some(author) = author {
        let email = author.email.as_str().trim().to_lowercase();
        if email != actor_email && seen.insert(email.clone()) {
            match notifications::task_recipient(task, author, "task_updated", "author") {
                Ok(recipient) => recipients.push(recipient),
                Err(err) => log::error!("Failed to build task-updated recipient for author: {err}"),
            }
        }
    }

    if let Some(assignee) = assignee {
        let email = assignee.email.as_str().trim().to_lowercase();
        if email != actor_email && seen.insert(email.clone()) {
            match notifications::task_recipient(task, assignee, "task_updated", "assignee") {
                Ok(recipient) => recipients.push(recipient),
                Err(err) => {
                    log::error!("Failed to build task-updated recipient for assignee: {err}")
                }
            }
        }
    }

    for event_actor in event_actors {
        let email = event_actor.email.as_str().trim().to_lowercase();
        if email != actor_email && seen.insert(email.clone()) {
            match notifications::task_recipient(task, event_actor, "task_updated", "event_actor") {
                Ok(recipient) => recipients.push(recipient),
                Err(err) => {
                    log::error!("Failed to build task-updated recipient for event actor: {err}")
                }
            }
        }
    }

    if recipients.is_empty() {
        return None;
    }

    let mut message = format!(
        "<p>Задача <strong>{}</strong> была обновлена пользователем {} ({}).</p>",
        sanitized_title, sanitized_actor_name, actor.email
    );

    let status: &'static str = task.status.into();
    message.push_str(&format!("<p>Текущий статус: {}.</p>", status));

    if let Some(due_date) = task.due_date {
        message.push_str(&format!("<p>Срок выполнения: {}.</p>", due_date));
    }

    if let Some(assignee) = assignee {
        let sanitized_assignee = notifications::sanitize_text(assignee.name.as_str());
        message.push_str(&format!(
            "<p>Текущий исполнитель: {} ({}).</p>",
            sanitized_assignee,
            assignee.email.as_str()
        ));
    }

    if let Some(description) = &task.description
        && !description.as_str().trim().is_empty()
    {
        message.push_str("<hr>");
        message.push_str(description.as_str());
    }

    NewEmail::try_new(
        actor.hub_id,
        message,
        Some(format!("Обновление задачи: {}", sanitized_title)),
        None,
        None,
        None,
        recipients,
    )
    .ok()
}

/// Record a new comment on the specified task from the current user.
pub fn add_task_comment<R, Z>(
    task_id: i32,
    form: TaskCommentForm,
    user: &AuthenticatedUser,
    repo: &R,
    zmq_sender: &Z,
) -> ServiceResult<TaskEvent>
where
    R: TaskReader + TaskEventReader + TaskEventWriter + UserReader + UserWriter + ?Sized,
    Z: ZmqSenderExt,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;
    let task_id = TaskId::new(task_id)?;

    let payload = TaskCommentPayload::try_from(form)?;
    let task = repo
        .get_task_by_id(task_id, hub_id)?
        .ok_or(ServiceError::NotFound)?;

    let new_user = NewUser::try_from(user)?;
    let comment_author = repo.create_or_update_user(&new_user)?;

    let comment_text = payload.message.to_string();
    let event = NewTaskEvent::new(
        task.id,
        Some(comment_author.id),
        TaskEventType::Comment,
        json!({ "text": comment_text }),
    );

    let recorded = repo.record_event(&event)?;

    repo.touch_visited_at(comment_author.id, comment_author.hub_id)?;

    let task_author = repo.get_user_by_id(task.author_id, hub_id)?;

    let task_assignee = match task.assigned_to {
        Some(assignee_id) => repo.get_user_by_id(assignee_id, hub_id)?,
        None => None,
    };

    let task_events = repo.list_events_for_task(task.id, hub_id)?;

    let actor_email = comment_author.email.as_str().trim().to_lowercase();
    let mut seen = HashSet::new();
    let mut recipients = Vec::new();

    if let Some(author) = task_author {
        let email = author.email.as_str().trim().to_lowercase();
        if email != actor_email && seen.insert(email.clone()) {
            match notifications::task_recipient(&task, &author, "task_commented", "author") {
                Ok(recipient) => recipients.push(recipient),
                Err(err) => {
                    log::error!("Failed to build task-comment recipient for author: {err}")
                }
            }
        }
    }

    if let Some(assignee) = task_assignee {
        let email = assignee.email.as_str().trim().to_lowercase();
        if email != actor_email && seen.insert(email.clone()) {
            match notifications::task_recipient(&task, &assignee, "task_commented", "assignee") {
                Ok(recipient) => recipients.push(recipient),
                Err(err) => {
                    log::error!("Failed to build task-comment recipient for assignee: {err}")
                }
            }
        }
    }

    let mut event_actor_ids = HashSet::new();
    for event in task_events {
        if let Some(user_id) = event.user_id
            && user_id != comment_author.id
        {
            event_actor_ids.insert(user_id);
        }
    }

    for actor_id in event_actor_ids {
        if let Some(actor) = repo.get_user_by_id(actor_id, hub_id)? {
            let email = actor.email.as_str().trim().to_lowercase();
            if email != actor_email && seen.insert(email.clone()) {
                match notifications::task_recipient(&task, &actor, "task_commented", "event_actor")
                {
                    Ok(recipient) => recipients.push(recipient),
                    Err(err) => {
                        log::error!("Failed to build task-comment recipient for event actor: {err}")
                    }
                }
            }
        }
    }

    if let Some(email) = build_task_comment_email(&task, &comment_author, &comment_text, recipients)
        && let Err(err) = notifications::queue_email(zmq_sender, user, email)
    {
        log::error!("Failed to queue task-comment email: {err}");
    }

    Ok(recorded)
}

/// Build notification email for a newly added comment.
fn build_task_comment_email(
    task: &Task,
    comment_author: &User,
    comment_body: &str,
    recipients: Vec<NewEmailRecipient>,
) -> Option<NewEmail> {
    if recipients.is_empty() {
        return None;
    }

    let sanitized_title = notifications::sanitize_text(task.title.as_str());
    let sanitized_author = notifications::sanitize_text(comment_author.name.as_str());
    let sanitized_body = notifications::sanitize_text(comment_body);

    let mut message = format!(
        "<p>Пользователь {} ({}) оставил комментарий к задаче <strong>{}</strong>.</p>",
        sanitized_author,
        comment_author.email.as_str(),
        sanitized_title
    );

    if !sanitized_body.is_empty() {
        message.push_str("<hr>");
        message.push_str(&sanitized_body);
    }

    NewEmail::try_new(
        comment_author.hub_id.get(),
        message,
        Some(format!("Новый комментарий в задаче: {}", sanitized_title)),
        None,
        None,
        None,
        recipients,
    )
    .ok()
}

/// Serialize a user into JSON payload used by assignment events.
fn assignment_event_user(user: &User) -> Value {
    json!({
        "id": user.id,
        "name": user.name,
        "email": user.email,
    })
}

/// Remove the specified task after verifying permissions and existence.
pub fn delete_task<R>(task_id: i32, user: &AuthenticatedUser, repo: &R) -> ServiceResult<()>
where
    R: TaskReader + TaskWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;
    let task_id = TaskId::new(task_id)?;

    if repo.get_task_by_id(task_id, hub_id)?.is_none() {
        return Err(ServiceError::NotFound);
    }

    repo.delete_task(task_id, hub_id)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};
    use pushkind_common::zmq::{SendFuture, ZmqSenderError, ZmqSenderTrait};
    use pushkind_emailer::models::zmq::ZMQSendEmailMessage;
    use serde_json::json;
    use std::cell::RefCell;
    use std::sync::Mutex;

    use crate::domain::types::{TaskDescription, TaskTitle, UserName};
    use crate::domain::{
        task::{
            NewTask as DomainNewTask, TaskAssignment, TaskPriority, TaskStatus,
            UpdateTask as DomainUpdateTask,
        },
        task_event::{NewTaskEvent as DomainNewTaskEvent, TaskEventType},
        types::{HubId, TaskEventId, TaskId, TaskTrack, UserEmail, UserId},
        user::User,
    };
    use crate::forms::task::TaskCommentForm;
    use crate::repository::mock::{
        MockTaskEventReader, MockTaskReader, MockTaskWriter, MockUserReader,
    };
    use crate::repository::{TaskListQuery, UserListQuery};
    use crate::services::mock::MockZmqSender;
    use mockall::Sequence;

    #[derive(Default)]
    struct RecordingZmqSender {
        payloads: Mutex<Vec<Vec<u8>>>,
    }

    impl RecordingZmqSender {
        fn messages(&self) -> Vec<Vec<u8>> {
            self.payloads.lock().unwrap().clone()
        }
    }

    impl ZmqSenderTrait for RecordingZmqSender {
        fn send_bytes<'a>(&'a self, bytes: Vec<u8>) -> SendFuture<'a> {
            {
                let mut payloads = self.payloads.lock().unwrap();
                payloads.push(bytes);
            }
            Box::pin(async { Ok(()) })
        }

        fn try_send_bytes(&self, bytes: Vec<u8>) -> Result<(), ZmqSenderError> {
            self.payloads.lock().unwrap().push(bytes);
            Ok(())
        }

        fn send_multipart<'a>(&'a self, frames: Vec<Vec<u8>>) -> SendFuture<'a> {
            {
                let mut payloads = self.payloads.lock().unwrap();
                payloads.extend(frames);
            }
            Box::pin(async { Ok(()) })
        }
    }

    struct TaskDetailsRepo {
        pub task_reader: MockTaskReader,
        pub event_reader: MockTaskEventReader,
        pub user_reader: MockUserReader,
    }

    impl TaskDetailsRepo {
        fn new() -> Self {
            Self {
                task_reader: MockTaskReader::new(),
                event_reader: MockTaskEventReader::new(),
                user_reader: MockUserReader::new(),
            }
        }
    }

    impl TaskReader for TaskDetailsRepo {
        fn get_task_by_id(&self, id: TaskId, hub_id: HubId) -> RepositoryResult<Option<Task>> {
            self.task_reader.get_task_by_id(id, hub_id)
        }

        fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)> {
            self.task_reader.list_tasks(query)
        }

        fn list_assignments_for_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
        ) -> RepositoryResult<Vec<TaskAssignment>> {
            self.task_reader.list_assignments_for_task(task_id, hub_id)
        }

        fn list_task_tracks(&self, hub_id: HubId) -> RepositoryResult<Vec<TaskTrack>> {
            self.task_reader.list_task_tracks(hub_id)
        }
    }

    impl TaskEventReader for TaskDetailsRepo {
        fn list_events_for_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
        ) -> RepositoryResult<Vec<TaskEvent>> {
            self.event_reader.list_events_for_task(task_id, hub_id)
        }
    }

    impl UserReader for TaskDetailsRepo {
        fn get_user_by_id(&self, id: UserId, hub_id: HubId) -> RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(id, hub_id)
        }

        fn get_user_by_email(
            &self,
            email: &UserEmail,
            hub_id: HubId,
        ) -> RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_email(email, hub_id)
        }

        fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)> {
            self.user_reader.list_users(query)
        }
    }

    struct TaskDeleteRepo {
        pub task_reader: MockTaskReader,
        pub task_writer: MockTaskWriter,
    }

    impl TaskDeleteRepo {
        fn new() -> Self {
            Self {
                task_reader: MockTaskReader::new(),
                task_writer: MockTaskWriter::new(),
            }
        }
    }

    impl TaskReader for TaskDeleteRepo {
        fn get_task_by_id(&self, id: TaskId, hub_id: HubId) -> RepositoryResult<Option<Task>> {
            self.task_reader.get_task_by_id(id, hub_id)
        }

        fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)> {
            self.task_reader.list_tasks(query)
        }

        fn list_assignments_for_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
        ) -> RepositoryResult<Vec<TaskAssignment>> {
            self.task_reader.list_assignments_for_task(task_id, hub_id)
        }

        fn list_task_tracks(&self, hub_id: HubId) -> RepositoryResult<Vec<TaskTrack>> {
            self.task_reader.list_task_tracks(hub_id)
        }
    }

    impl TaskWriter for TaskDeleteRepo {
        fn create_task(&self, new_task: &DomainNewTask) -> RepositoryResult<Task> {
            self.task_writer.create_task(new_task)
        }

        fn update_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
            updates: &DomainUpdateTask,
        ) -> RepositoryResult<Task> {
            self.task_writer.update_task(task_id, hub_id, updates)
        }

        fn delete_task(&self, task_id: TaskId, hub_id: HubId) -> RepositoryResult<()> {
            self.task_writer.delete_task(task_id, hub_id)
        }

        fn record_assignment(&self, assignment: &TaskAssignment) -> RepositoryResult<()> {
            self.task_writer.record_assignment(assignment)
        }

        fn remove_assignment(
            &self,
            task_id: TaskId,
            hub_id: HubId,
            assignee_id: UserId,
        ) -> RepositoryResult<()> {
            self.task_writer
                .remove_assignment(task_id, hub_id, assignee_id)
        }
    }

    fn fixed_datetime() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2024, 1, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .unwrap_or_else(|| {
                NaiveDate::from_ymd_opt(1970, 1, 1)
                    .and_then(|d| d.and_hms_opt(0, 0, 0))
                    .expect("valid fallback date")
            })
    }

    fn sample_task(id: i32, hub_id: i32, assigned_to: Option<i32>, author_id: i32) -> Task {
        Task {
            id: TaskId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            title: TaskTitle::new("Test Task").unwrap(),
            description: Some(TaskDescription::new("Detail").unwrap()),
            track: Some(TaskTrack::new("Default Track").unwrap()),
            priority: TaskPriority::default(),
            status: TaskStatus::Pending,
            due_date: None,
            assigned_to: assigned_to.map(|value| UserId::new(value).unwrap()),
            author_id: UserId::new(author_id).unwrap(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            completed_at: None,
        }
    }

    fn sample_event(id: i32, task_id: TaskId, user_id: Option<UserId>) -> TaskEvent {
        TaskEvent {
            id: TaskEventId::new(id).unwrap(),
            task_id,
            user_id,
            event_type: TaskEventType::Comment,
            event_data: json!({"message": "hi"}),
            created_at: fixed_datetime(),
        }
    }

    fn sample_user(id: i32, hub_id: i32, name: &str, email: &str) -> User {
        User {
            id: UserId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: UserName::new(name).unwrap(),
            email: UserEmail::new(email).unwrap(),
            visited_at: Some(fixed_datetime()),
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 1,
            name: "Test User".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    #[test]
    fn apply_assignment_updates_assigns_and_unassigns() {
        let base_assigned = sample_task(1, 1, Some(1), 2);
        let assigned = apply_assignment_updates(
            UpdateTask::from_task(&base_assigned),
            base_assigned.assigned_to,
            Some(UserId::new(2).unwrap()),
        );
        assert_eq!(assigned.assigned_to, Some(UserId::new(2).unwrap()));

        let base_with_assignee = sample_task(2, 1, Some(1), 2);
        let unassigned = apply_assignment_updates(
            UpdateTask::from_task(&base_with_assignee),
            base_with_assignee.assigned_to,
            None,
        );
        assert!(unassigned.assigned_to.is_none());

        let base_same_assignee = sample_task(3, 1, Some(3), 2);
        let unchanged = apply_assignment_updates(
            UpdateTask::from_task(&base_same_assignee),
            base_same_assignee.assigned_to,
            base_same_assignee.assigned_to,
        );
        assert_eq!(unchanged.assigned_to, base_same_assignee.assigned_to);
    }

    #[test]
    fn status_event_payload_returns_changes() {
        assert!(status_event_payload(TaskStatus::Pending, TaskStatus::Pending).is_none());

        let payload = status_event_payload(TaskStatus::Pending, TaskStatus::Completed)
            .expect("expected payload for status change");
        assert_eq!(payload, json!({"from": "Pending", "to": "Completed"}));
    }

    #[test]
    fn assignment_event_payload_includes_user_data() {
        let previous = sample_user(5, 1, "Prev", "prev@example.com");
        let next = sample_user(6, 1, "Next", "next@example.com");

        let payload = assignment_event_payload(Some(&previous), Some(&next))
            .expect("expected assignment change payload");

        assert_eq!(
            payload,
            json!({
                "from": {
                    "id": previous.id,
                    "name": previous.name,
                    "email": previous.email,
                },
                "to": {
                    "id": next.id,
                    "name": next.name,
                    "email": next.email,
                }
            })
        );

        assert!(assignment_event_payload(Some(&previous), Some(&previous)).is_none());
    }

    #[test]
    fn metadata_event_payload_emits_differences() {
        let current = sample_task(1, 1, None, 2);
        let mut updated = current.clone();
        updated.title = TaskTitle::new("Updated").unwrap();
        updated.description = Some(TaskDescription::new("New").unwrap());
        updated.due_date = Some(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap());
        updated.track = Some(TaskTrack::new("Updated Track").unwrap());
        updated.priority = TaskPriority::High;

        let payload =
            metadata_event_payload(&current, &updated).expect("expected metadata payload");

        let from_priority: &'static str = current.priority.into();
        let to_priority: &'static str = updated.priority.into();

        let expected = json!({
            "title": {"from": current.title.clone(), "to": updated.title.clone()},
            "description": {"from": current.description.clone(), "to": updated.description.clone()},
            "track": {"from": current.track.clone(), "to": updated.track.clone()},
            "priority": {"from": from_priority, "to": to_priority},
            "due_date": {
                "from": current.due_date.map(|date| date.to_string()),
                "to": updated.due_date.map(|date| date.to_string())
            }
        });

        assert_eq!(payload, expected);

        let none_payload = metadata_event_payload(&current, &current);
        assert!(none_payload.is_none());
    }

    #[test]
    fn load_task_details_returns_data() {
        let assignee = sample_user(7, 1, "Assignee", "assignee@example.com");
        let author = sample_user(11, 1, "Author", "author@example.com");

        let task = sample_task(5, 1, Some(assignee.id.get()), author.id.get());
        let event = sample_event(13, task.id, Some(author.id));

        let mut repo = TaskDetailsRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = HubId::new(user.hub_id).unwrap();

        let task_for_return = task.clone();
        repo.task_reader
            .expect_get_task_by_id()
            .return_once(move |id, hub| {
                assert_eq!(id, task_for_return.id);
                assert_eq!(hub, hub_id);
                Ok(Some(task_for_return))
            });

        let event_for_return = event.clone();
        repo.event_reader
            .expect_list_events_for_task()
            .return_once(move |task_id, hub| {
                assert_eq!(task_id, event_for_return.task_id);
                assert_eq!(hub, hub_id);
                Ok(vec![event_for_return])
            });

        let mut sequence = Sequence::new();

        let author_for_author_lookup = author.clone();
        repo.user_reader
            .expect_get_user_by_id()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |id, hub| {
                assert_eq!(id, author_for_author_lookup.id);
                assert_eq!(hub, hub_id);
                Ok(Some(author_for_author_lookup))
            });

        let assignee_for_lookup = assignee.clone();
        repo.user_reader
            .expect_get_user_by_id()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |id, hub| {
                assert_eq!(id, assignee_for_lookup.id);
                assert_eq!(hub, hub_id);
                Ok(Some(assignee_for_lookup))
            });

        let author_for_event_lookup = author.clone();
        repo.user_reader
            .expect_get_user_by_id()
            .times(1)
            .in_sequence(&mut sequence)
            .return_once(move |id, hub| {
                assert_eq!(id, author_for_event_lookup.id);
                assert_eq!(hub, hub_id);
                Ok(Some(author_for_event_lookup))
            });

        let result = load_task_details(task.id.get(), &user, &repo).expect("should load task");

        assert_eq!(result.task.id, task.id);
        assert_eq!(result.author.id, author.id);
        assert_eq!(result.assignee.as_ref().map(|u| u.id), Some(assignee.id));
        assert_eq!(result.events.len(), 1);
        let event_with_author = &result.events[0];
        assert_eq!(event_with_author.event.id, event.id);
        assert_eq!(
            event_with_author.author.as_ref().map(|u| u.id),
            Some(author.id)
        );
    }

    #[test]
    fn load_task_details_requires_role() {
        let repo = TaskDetailsRepo::new();
        let user = user_with_roles(&[]);

        let result = load_task_details(5, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_task_details_returns_not_found_for_missing_task() {
        let mut repo = TaskDetailsRepo::new();
        repo.task_reader
            .expect_get_task_by_id()
            .return_once(|_, _| Ok(None));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = load_task_details(99, &user, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn load_task_details_propagates_repository_error() {
        let mut repo = TaskDetailsRepo::new();
        repo.task_reader
            .expect_get_task_by_id()
            .return_once(|_, _| Err(RepositoryError::Unexpected("boom".to_string())));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = load_task_details(1, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Repository(_))));
    }

    #[test]
    fn delete_task_requires_role() {
        let repo = TaskDeleteRepo::new();
        let user = user_with_roles(&[]);

        let result = delete_task(1, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn delete_task_returns_not_found_when_task_missing() {
        let mut repo = TaskDeleteRepo::new();
        repo.task_reader
            .expect_get_task_by_id()
            .return_once(|_, _| Ok(None));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = delete_task(99, &user, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn delete_task_returns_not_found_when_repository_reports_missing() {
        let task = sample_task(5, 1, None, 3);
        let mut repo = TaskDeleteRepo::new();
        repo.task_reader.expect_get_task_by_id().return_once({
            let task_clone = task.clone();
            move |id, hub| {
                assert_eq!(id, task_clone.id);
                assert_eq!(hub, task_clone.hub_id);
                Ok(Some(task_clone))
            }
        });
        repo.task_writer
            .expect_delete_task()
            .return_once(|_, _| Err(RepositoryError::NotFound));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = delete_task(5, &user, &repo);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn delete_task_returns_unit_on_success() {
        let task = sample_task(7, 1, None, 4);
        let mut repo = TaskDeleteRepo::new();
        repo.task_reader.expect_get_task_by_id().return_once({
            let task_clone = task.clone();
            move |id, hub| {
                assert_eq!(id, task_clone.id);
                assert_eq!(hub, task_clone.hub_id);
                Ok(Some(task_clone))
            }
        });
        repo.task_writer.expect_delete_task().return_once({
            move |id, hub| {
                assert_eq!(id, task.id);
                assert_eq!(hub, task.hub_id);
                Ok(())
            }
        });
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        delete_task(7, &user, &repo).expect("should delete task");
    }

    #[test]
    fn delete_task_propagates_repository_error() {
        let task = sample_task(1, 1, None, 2);
        let mut repo = TaskDeleteRepo::new();
        repo.task_reader.expect_get_task_by_id().return_once({
            let task_clone = task.clone();
            move |id, hub| {
                assert_eq!(id, task_clone.id);
                assert_eq!(hub, task_clone.hub_id);
                Ok(Some(task_clone))
            }
        });
        repo.task_writer
            .expect_delete_task()
            .return_once(|_, _| Err(RepositoryError::Unexpected("boom".to_string())));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = delete_task(1, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Repository(_))));
    }

    struct UpdateRepo {
        task: RefCell<Task>,
        users: RefCell<HashMap<String, User>>,
        events: RefCell<Vec<TaskEvent>>,
        next_user_id: RefCell<i32>,
        next_event_id: RefCell<i32>,
    }

    impl UpdateRepo {
        fn new(task: Task, users: Vec<User>) -> Self {
            let mut map = HashMap::new();
            for user in users {
                map.insert(user.email.as_str().to_lowercase(), user);
            }

            Self {
                task: RefCell::new(task),
                users: RefCell::new(map),
                events: RefCell::new(Vec::new()),
                next_user_id: RefCell::new(10_000),
                next_event_id: RefCell::new(50_000),
            }
        }

        fn user_by_email(&self, email: &str) -> Option<User> {
            self.users
                .borrow()
                .get(&email.trim().to_lowercase())
                .cloned()
        }
    }

    impl TaskReader for UpdateRepo {
        fn get_task_by_id(&self, id: TaskId, hub_id: HubId) -> RepositoryResult<Option<Task>> {
            let task = self.task.borrow();
            if task.id == id && task.hub_id == hub_id {
                Ok(Some(task.clone()))
            } else {
                Ok(None)
            }
        }

        fn list_tasks(&self, _: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)> {
            Ok((1, vec![self.task.borrow().clone()]))
        }

        fn list_assignments_for_task(
            &self,
            _: TaskId,
            _: HubId,
        ) -> RepositoryResult<Vec<TaskAssignment>> {
            Ok(Vec::new())
        }

        fn list_task_tracks(&self, _: HubId) -> RepositoryResult<Vec<TaskTrack>> {
            Ok(Vec::new())
        }
    }

    impl TaskWriter for UpdateRepo {
        fn create_task(&self, _: &DomainNewTask) -> RepositoryResult<Task> {
            Ok(self.task.borrow().clone())
        }

        fn update_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
            updates: &DomainUpdateTask,
        ) -> RepositoryResult<Task> {
            let mut task = self.task.borrow_mut();
            if task.id != task_id || task.hub_id != hub_id {
                return Err(RepositoryError::NotFound);
            }

            task.title = updates.title.clone();
            task.description = updates.description.clone();
            task.track = updates.track.clone();
            task.priority = updates.priority;
            task.status = updates.status;
            task.due_date = updates.due_date;
            task.assigned_to = updates.assigned_to;
            task.completed_at = updates.completed_at;
            task.updated_at = updates.updated_at;

            Ok(task.clone())
        }

        fn delete_task(&self, _: TaskId, _: HubId) -> RepositoryResult<()> {
            Ok(())
        }

        fn record_assignment(&self, _: &TaskAssignment) -> RepositoryResult<()> {
            Ok(())
        }

        fn remove_assignment(&self, _: TaskId, _: HubId, _: UserId) -> RepositoryResult<()> {
            Ok(())
        }
    }

    impl TaskEventReader for UpdateRepo {
        fn list_events_for_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
        ) -> RepositoryResult<Vec<TaskEvent>> {
            let task = self.task.borrow();
            if task.id != task_id || task.hub_id != hub_id {
                return Ok(Vec::new());
            }

            Ok(self.events.borrow().clone())
        }
    }

    impl UserReader for UpdateRepo {
        fn get_user_by_id(&self, id: UserId, hub_id: HubId) -> RepositoryResult<Option<User>> {
            Ok(self
                .users
                .borrow()
                .values()
                .find(|user| user.id == id && user.hub_id == hub_id)
                .cloned())
        }

        fn get_user_by_email(
            &self,
            email: &UserEmail,
            hub_id: HubId,
        ) -> RepositoryResult<Option<User>> {
            Ok(self
                .users
                .borrow()
                .get(&email.as_str().trim().to_lowercase())
                .cloned()
                .filter(|user| user.hub_id == hub_id))
        }

        fn list_users(&self, _: UserListQuery) -> RepositoryResult<(usize, Vec<User>)> {
            let users = self.users.borrow();
            Ok((users.len(), users.values().cloned().collect()))
        }
    }

    impl UserWriter for UpdateRepo {
        fn create_or_update_user(
            &self,
            new_user: &crate::domain::user::NewUser,
        ) -> RepositoryResult<User> {
            if let Some(existing) = self.user_by_email(new_user.email.as_str()) {
                return Ok(existing);
            }

            let id = {
                let mut counter = self.next_user_id.borrow_mut();
                let id = *counter;
                *counter += 1;
                id
            };

            let user = User {
                id: UserId::new(id).unwrap(),
                hub_id: new_user.hub_id,
                name: new_user.name.clone(),
                email: new_user.email.clone(),
                visited_at: Some(fixed_datetime()),
            };

            self.users
                .borrow_mut()
                .insert(user.email.as_str().to_lowercase(), user.clone());

            Ok(user)
        }

        fn update_user(
            &self,
            _: UserId,
            _: HubId,
            _: &crate::domain::user::UpdateUser,
        ) -> RepositoryResult<User> {
            Err(RepositoryError::NotFound)
        }

        fn delete_user(&self, _: UserId, _: HubId) -> RepositoryResult<()> {
            Err(RepositoryError::NotFound)
        }

        fn touch_visited_at(&self, _: UserId, _: HubId) -> RepositoryResult<()> {
            Ok(())
        }
    }

    impl TaskEventWriter for UpdateRepo {
        fn record_event(&self, event: &DomainNewTaskEvent) -> RepositoryResult<TaskEvent> {
            let mut events = self.events.borrow_mut();
            let mut next_id = self.next_event_id.borrow_mut();
            let id = *next_id;
            *next_id += 1;

            let record = TaskEvent {
                id: TaskEventId::new(id).unwrap(),
                task_id: event.task_id,
                user_id: event.user_id,
                event_type: event.event_type,
                event_data: event.event_data.clone(),
                created_at: event.created_at,
            };

            events.push(record.clone());
            Ok(record)
        }

        fn delete_event(&self, _: TaskEventId, _: HubId) -> RepositoryResult<()> {
            Ok(())
        }
    }

    #[test]
    fn update_task_updates_fields_and_assignment() {
        let assignee = sample_user(7, 1, "Executor", "executor@example.com");
        let task = sample_task(42, 1, None, 3);
        let repo = UpdateRepo::new(task, vec![assignee.clone()]);
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let due_date = NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date");
        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated title",
            "message": "Updated description",
            "status": "InProgress",
            "priority": "Middle",
            "due_date": due_date.to_string(),
            "id": assignee.email.as_str(),
            "name": assignee.name.as_str(),
            "email": assignee.email.as_str(),
        }))
        .expect("valid form payload");

        let outcome = update_task(42, form, &user, &repo, &zmq).expect("should update task");

        assert_eq!(outcome.id, TaskId::new(42).unwrap());
        assert_eq!(outcome.title.as_str(), "Updated title");

        let stored = repo.task.borrow().clone();
        assert_eq!(stored.title.as_str(), "Updated title");
        assert_eq!(stored.status, TaskStatus::InProgress);
        assert_eq!(stored.due_date, Some(due_date));
        assert_eq!(
            stored.description.as_ref().map(|value| value.as_str()),
            Some("Updated description")
        );
        assert_eq!(stored.assigned_to, Some(assignee.id));

        let events = repo.events.borrow();
        assert_eq!(events.len(), 3);

        let status_event = &events[0];
        assert_eq!(status_event.event_type, TaskEventType::StatusChanged);
        assert_eq!(
            status_event.event_data,
            json!({ "from": "Pending", "to": "InProgress" })
        );

        let assignment_event = &events[1];
        assert_eq!(
            assignment_event.event_type,
            TaskEventType::AssignmentChanged
        );
        assert_eq!(
            assignment_event.event_data,
            json!({
                "from": serde_json::Value::Null,
                "to": {
                    "id": assignee.id,
                    "name": assignee.name.clone(),
                    "email": assignee.email.clone(),
                }
            })
        );

        let metadata_event = &events[2];
        assert_eq!(metadata_event.event_type, TaskEventType::MetadataUpdated);
        assert_eq!(
            metadata_event.event_data,
            json!({
                "title": {
                    "from": "Test Task",
                    "to": "Updated title",
                },
                "description": {
                    "from": "Detail",
                    "to": "Updated description",
                },
                "due_date": {
                    "from": serde_json::Value::Null,
                    "to": due_date.to_string(),
                },
                "track": {
                    "from": "Default Track",
                    "to": serde_json::Value::Null,
                }
            })
        );
    }

    #[test]
    fn update_task_updates_track_and_priority() {
        let task = sample_task(50, 1, None, 3);
        let repo = UpdateRepo::new(task, Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated title",
            "status": "Pending",
            "track": "New Track",
            "priority": "High",
        }))
        .expect("valid form payload");

        let outcome = update_task(50, form, &user, &repo, &zmq).expect("should update task");

        assert_eq!(outcome.title.as_str(), "Updated title");

        let stored = repo.task.borrow().clone();
        assert_eq!(
            stored.track.as_ref().map(|track| track.as_str()),
            Some("New Track")
        );
        assert_eq!(stored.priority, TaskPriority::High);

        let events = repo.events.borrow();
        let metadata_event = events
            .iter()
            .find(|event| event.event_type == TaskEventType::MetadataUpdated)
            .expect("metadata event should be recorded");

        let from_priority: &'static str = TaskPriority::default().into();
        assert_eq!(
            metadata_event.event_data,
            json!({
                "title": {
                    "from": "Test Task",
                    "to": "Updated title",
                },
                "description": {
                    "from": "Detail",
                    "to": serde_json::Value::Null,
                },
                "track": {
                    "from": "Default Track",
                    "to": "New Track",
                },
                "priority": {
                    "from": from_priority,
                    "to": "High",
                }
            })
        );
    }

    #[test]
    fn update_task_notifies_participants() {
        let author = sample_user(3, 1, "Author", "author@example.com");
        let assignee = sample_user(5, 1, "Executor", "executor@example.com");
        let commenter = sample_user(7, 1, "Commenter", "commenter@example.com");
        let task = sample_task(55, 1, Some(assignee.id.get()), author.id.get());
        let repo = UpdateRepo::new(
            task.clone(),
            vec![author.clone(), assignee.clone(), commenter.clone()],
        );
        repo.events
            .borrow_mut()
            .push(sample_event(77, task.id, Some(commenter.id)));
        let zmq = RecordingZmqSender::default();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated title",
            "message": "Updated description",
            "status": "InProgress",
            "priority": "Middle",
            "due_date": "2024-05-01",
            "name": assignee.name.as_str(),
            "email": assignee.email.as_str(),
        }))
        .expect("valid form payload");

        let outcome =
            update_task(task.id.get(), form, &user, &repo, &zmq).expect("should update task");

        assert_eq!(outcome.title.as_str(), "Updated title");

        let payloads = zmq.messages();
        assert_eq!(payloads.len(), 1);

        let envelope: ZMQSendEmailMessage =
            serde_json::from_slice(&payloads[0]).expect("valid email payload");

        match envelope {
            ZMQSendEmailMessage::NewEmail(message) => {
                let (actor, email) = *message;
                assert_eq!(actor.email, user.email);
                assert_eq!(email.recipients.len(), 3);

                let addresses: std::collections::HashSet<_> = email
                    .recipients
                    .iter()
                    .map(|recipient| recipient.address.as_str())
                    .collect();

                assert!(addresses.contains(author.email.as_str()));
                assert!(addresses.contains(assignee.email.as_str()));
                assert!(addresses.contains(commenter.email.as_str()));
                assert_eq!(
                    email.subject.as_ref().map(|subject| subject.as_str()),
                    Some("Обновление задачи: Updated title"),
                );
                assert!(email.message.as_str().contains("Updated title"));
                assert!(email.message.as_str().contains("Test User"));
            }
            _ => panic!("unexpected email payload variant"),
        }
    }

    #[test]
    fn update_task_creates_user_when_missing() {
        let task = sample_task(7, 1, None, 2);
        let repo = UpdateRepo::new(task, Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "priority": "Middle",
            "id": "auth0|user-1",
            "name": "Fresh User",
            "email": "fresh@example.com",
        }))
        .expect("valid form payload");

        update_task(7, form, &user, &repo, &zmq).expect("should create assignee");

        let stored = repo.task.borrow().clone();
        let created = repo
            .user_by_email("fresh@example.com")
            .expect("user should be created");

        assert_eq!(stored.assigned_to, Some(created.id));

        let events = repo.events.borrow();
        assert_eq!(events.len(), 2);

        let assignment_event = &events[0];
        assert_eq!(
            assignment_event.event_type,
            TaskEventType::AssignmentChanged
        );
        assert_eq!(
            assignment_event.event_data,
            json!({
                "from": serde_json::Value::Null,
                "to": {
                    "id": created.id,
                    "name": created.name.clone(),
                    "email": created.email.clone(),
                }
            })
        );

        let metadata_event = &events[1];
        assert_eq!(metadata_event.event_type, TaskEventType::MetadataUpdated);
        assert_eq!(
            metadata_event.event_data,
            json!({
                "title": {
                    "from": "Test Task",
                    "to": "Updated",
                },
                "description": {
                    "from": "Detail",
                    "to": serde_json::Value::Null,
                },
                "track": {
                    "from": "Default Track",
                    "to": serde_json::Value::Null,
                }
            })
        );
    }

    #[test]
    fn update_task_unassigns_when_selection_missing() {
        let assignee = sample_user(8, 1, "Assigned", "assigned@example.com");
        let task = sample_task(9, 1, Some(assignee.id.get()), 4);
        let repo = UpdateRepo::new(task, vec![assignee.clone()]);
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Keep",
            "status": "Pending",
            "priority": "Middle",
        }))
        .expect("valid form payload");

        update_task(9, form, &user, &repo, &zmq).expect("should unassign");

        let stored = repo.task.borrow().clone();
        assert!(stored.assigned_to.is_none());

        let events = repo.events.borrow();
        assert_eq!(events.len(), 2);

        let assignment_event = &events[0];
        assert_eq!(
            assignment_event.event_type,
            TaskEventType::AssignmentChanged
        );
        assert_eq!(
            assignment_event.event_data,
            json!({
                "from": {
                    "id": assignee.id,
                    "name": assignee.name.clone(),
                    "email": assignee.email.clone(),
                },
                "to": serde_json::Value::Null,
            })
        );

        let metadata_event = &events[1];
        assert_eq!(metadata_event.event_type, TaskEventType::MetadataUpdated);
        assert_eq!(
            metadata_event.event_data,
            json!({
                "title": {
                    "from": "Test Task",
                    "to": "Keep",
                },
                "description": {
                    "from": "Detail",
                    "to": serde_json::Value::Null,
                },
                "track": {
                    "from": "Default Track",
                    "to": serde_json::Value::Null,
                }
            })
        );
    }

    #[test]
    fn update_task_requires_email_for_assignee() {
        let task = sample_task(11, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "priority": "Middle",
            "id": "auth0|no-email",
            "name": "Nameless",
        }))
        .expect("valid form payload");

        let outcome =
            update_task(11, form, &user, &repo, &zmq).expect("expected update to succeed");

        assert_eq!(outcome.id, TaskId::new(11).unwrap());
        assert_eq!(outcome.title.as_str(), "Updated");

        {
            let stored = repo.task.borrow();
            assert_eq!(stored.title.as_str(), "Updated");
            assert!(stored.assigned_to.is_none());
        }

        let events = repo.events.borrow();
        assert_eq!(events.len(), 1);
        let metadata_event = &events[0];
        assert_eq!(metadata_event.event_type, TaskEventType::MetadataUpdated);
    }

    #[test]
    fn update_task_requires_role() {
        let task = sample_task(12, 1, None, 6);
        let repo = UpdateRepo::new(task, Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "priority": "Middle",
        }))
        .expect("valid form payload");

        let result = update_task(12, form, &user, &repo, &zmq);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_task_returns_not_found_for_missing_task() {
        let task = sample_task(13, 2, None, 6);
        let repo = UpdateRepo::new(task, Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "priority": "Middle",
        }))
        .expect("valid form payload");

        let result = update_task(13, form, &user, &repo, &zmq);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn add_task_comment_records_event() {
        let commenter = sample_user(21, 1, "Commenter", "user@example.com");
        let task = sample_task(77, 1, None, commenter.id.get());
        let repo = UpdateRepo::new(task.clone(), vec![commenter.clone()]);
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let zmq = MockZmqSender {};

        let form = TaskCommentForm {
            message: "Новый комментарий".to_string(),
        };

        let recorded =
            add_task_comment(task.id.get(), form, &user, &repo, &zmq).expect("should add comment");
        assert_eq!(recorded.task_id, task.id);
        assert_eq!(recorded.event_type, TaskEventType::Comment);
        assert_eq!(recorded.event_data, json!({"text": "Новый комментарий"}));

        let events = repo.events.borrow();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert_eq!(event.task_id, task.id);
        assert_eq!(event.user_id, Some(commenter.id));
        assert_eq!(event.event_type, TaskEventType::Comment);
        assert_eq!(event.event_data, json!({"text": "Новый комментарий"}));
    }

    #[test]
    fn add_task_comment_creates_user_when_missing() {
        let task = sample_task(81, 1, None, 5);
        let repo = UpdateRepo::new(task.clone(), Vec::new());
        let user = AuthenticatedUser {
            sub: "auth0|user".to_string(),
            email: "fresh@example.com".to_string(),
            hub_id: 1,
            name: "Fresh Author".to_string(),
            roles: vec![SERVICE_ACCESS_ROLE.to_string()],
            exp: 0,
        };
        let zmq = MockZmqSender {};

        let form = TaskCommentForm {
            message: "Комментарий".to_string(),
        };

        add_task_comment(task.id.get(), form, &user, &repo, &zmq).expect("should add comment");

        let events = repo.events.borrow();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        if let Some(user_id) = event.user_id {
            assert!(
                repo.users
                    .borrow()
                    .values()
                    .any(|record| record.id == user_id)
            );
        } else {
            panic!("comment event should include a user id");
        }
    }

    #[test]
    fn add_task_comment_notifies_participants_except_author() {
        let author = sample_user(31, 1, "Author", "author@example.com");
        let assignee = sample_user(32, 1, "Assignee", "assignee@example.com");
        let participant = sample_user(33, 1, "Participant", "participant@example.com");
        let task = sample_task(107, 1, Some(assignee.id.get()), author.id.get());
        let repo = UpdateRepo::new(
            task.clone(),
            vec![author.clone(), assignee.clone(), participant.clone()],
        );
        repo.events
            .borrow_mut()
            .push(sample_event(501, task.id, Some(participant.id)));

        let zmq = RecordingZmqSender::default();
        let user = AuthenticatedUser {
            sub: "auth0|commenter".to_string(),
            email: "commenter@example.com".to_string(),
            hub_id: 1,
            name: "Commenting User".to_string(),
            roles: vec![SERVICE_ACCESS_ROLE.to_string()],
            exp: 0,
        };

        let form = TaskCommentForm {
            message: "Комментарий".to_string(),
        };

        add_task_comment(task.id.get(), form, &user, &repo, &zmq).expect("should add comment");

        let payloads = zmq.messages();
        assert_eq!(payloads.len(), 1);

        let envelope: ZMQSendEmailMessage =
            serde_json::from_slice(&payloads[0]).expect("valid email payload");

        match envelope {
            ZMQSendEmailMessage::NewEmail(message) => {
                let (actor, email) = *message;
                assert_eq!(actor.email, user.email);

                let addresses: HashSet<_> = email
                    .recipients
                    .iter()
                    .map(|recipient| recipient.address.as_str())
                    .collect();

                assert_eq!(addresses.len(), 3);
                assert!(addresses.contains(author.email.as_str()));
                assert!(addresses.contains(assignee.email.as_str()));
                assert!(addresses.contains(participant.email.as_str()));
                assert!(!addresses.contains(user.email.as_str()));

                assert_eq!(
                    email.subject.as_ref().map(|subject| subject.as_str()),
                    Some("Новый комментарий в задаче: Test Task"),
                );
                assert!(email.message.as_str().contains("Комментарий"));
            }
            _ => panic!("unexpected email payload variant"),
        }
    }

    #[test]
    fn add_task_comment_requires_role() {
        let task = sample_task(91, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[]);
        let zmq = MockZmqSender {};

        let form = TaskCommentForm {
            message: "Комментарий".to_string(),
        };

        let result = add_task_comment(91, form, &user, &repo, &zmq);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn add_task_comment_returns_form_error_on_invalid_payload() {
        let task = sample_task(93, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let zmq = MockZmqSender {};

        let form = TaskCommentForm {
            message: String::new(),
        };

        let result = add_task_comment(93, form, &user, &repo, &zmq);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn add_task_comment_returns_not_found_for_missing_task() {
        let task = sample_task(99, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let zmq = MockZmqSender {};

        let form = TaskCommentForm {
            message: "Комментарий".to_string(),
        };

        let result = add_task_comment(123, form, &user, &repo, &zmq);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn transition_task_status_requires_role() {
        let task = sample_task(100, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[]);
        let zmq = MockZmqSender {};

        let form = QuickTaskStatusForm {
            status: TaskStatus::InProgress,
            comment: None,
            assign_self: false,
        };
        let result = transition_task_status(100, form, &user, &repo, &zmq);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn transition_task_status_records_status_event() {
        let task = sample_task(101, 1, None, 5);
        let repo = UpdateRepo::new(task.clone(), Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form = QuickTaskStatusForm {
            status: TaskStatus::InProgress,
            comment: None,
            assign_self: false,
        };
        let updated = transition_task_status(task.id.get(), form, &user, &repo, &zmq)
            .expect("should transition status");

        assert_eq!(updated.status, TaskStatus::InProgress);

        let events = repo.events.borrow();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, TaskEventType::StatusChanged);
    }

    #[test]
    fn transition_task_status_assigns_current_user() {
        let task = sample_task(150, 1, None, 5);
        let repo = UpdateRepo::new(task.clone(), Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form = QuickTaskStatusForm {
            status: TaskStatus::InProgress,
            comment: None,
            assign_self: true,
        };
        transition_task_status(task.id.get(), form, &user, &repo, &zmq)
            .expect("should transition and assign");

        let stored = repo.task.borrow().clone();
        let assigned_user = repo
            .user_by_email(user.email.as_str())
            .expect("actor should be created");

        assert_eq!(stored.assigned_to, Some(assigned_user.id));

        let events = repo.events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, TaskEventType::StatusChanged);
        assert_eq!(events[1].event_type, TaskEventType::AssignmentChanged);
    }

    #[test]
    fn transition_task_status_with_comment_records_comment_event() {
        let task = sample_task(102, 1, None, 5);
        let repo = UpdateRepo::new(task.clone(), Vec::new());
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form = QuickTaskStatusForm {
            status: TaskStatus::Completed,
            comment: Some("Готово".to_string()),
            assign_self: false,
        };
        transition_task_status(task.id.get(), form, &user, &repo, &zmq)
            .expect("should transition status with comment");

        let events = repo.events.borrow();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, TaskEventType::StatusChanged);
        assert_eq!(events[1].event_type, TaskEventType::Comment);
    }
}
