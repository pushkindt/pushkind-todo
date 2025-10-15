use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::repository::errors::RepositoryError;
use pushkind_common::routes::check_role;
use serde::Serialize;
use serde_json::{Value, json};
use validator::Validate;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::{
    task::{Task, TaskStatus, UpdateTask},
    task_event::{NewTaskEvent, TaskEvent, TaskEventType},
    user::User,
};
use crate::forms::task::{NewTaskCommentForm, TaskUpdateSubmission, UpdateTaskForm};
use crate::repository::{
    TaskEventReader, TaskEventWriter, TaskReader, TaskWriter, UserListQuery, UserReader, UserWriter,
};
use crate::services::{ServiceError, ServiceResult};

/// Task event accompanied by the optional author information.
#[derive(Debug, Serialize)]
pub struct TaskEventWithAuthor {
    /// Persisted event data.
    pub event: TaskEvent,
    /// Author of the event, if present and accessible within the hub.
    pub author: Option<User>,
}

/// Aggregated task information with the related event history.
#[derive(Debug, Serialize)]
pub struct TaskDetails {
    /// Task metadata shown on the details page.
    pub task: Task,
    /// Author of the task.
    pub author: User,
    /// Task assignee when available in the current hub.
    pub assignee: Option<User>,
    /// Ordered list of events associated with the task.
    pub events: Vec<TaskEventWithAuthor>,
}

/// Load a task and its events for the provided user, enriching with user data.
pub fn load_task_details<R>(
    repo: &R,
    user: &AuthenticatedUser,
    task_id: i32,
) -> ServiceResult<TaskDetails>
where
    R: TaskReader + TaskEventReader + UserReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let task = repo
        .get_task_by_id(task_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or(ServiceError::NotFound)?;

    let author = repo
        .get_user_by_id(task.author_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or_else(|| {
            log::error!(
                "Task {} references missing author {}",
                task.id,
                task.author_id
            );
            ServiceError::Internal
        })?;

    let assignee = match task.assigned_to {
        Some(assignee_id) => match repo.get_user_by_id(assignee_id, user.hub_id) {
            Ok(user) => user,
            Err(err) => return Err(ServiceError::from(err)),
        },
        None => None,
    };

    let events = repo
        .list_events_for_task(task.id, user.hub_id)
        .map_err(ServiceError::from)?;

    let mut author_cache: HashMap<i32, User> = HashMap::new();
    for event in &events {
        if let Some(author_id) = event.user_id {
            if author_cache.contains_key(&author_id) {
                continue;
            }

            match repo.get_user_by_id(author_id, user.hub_id) {
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

/// Data needed to render the task modal for editing.
#[derive(Debug, Serialize)]
pub struct TaskModalData {
    /// Task being edited in the modal.
    pub task: Task,
    /// Optional assignee for the task when available in the current hub.
    pub assignee: Option<User>,
    /// Users that can be selected as potential assignees.
    pub users: Vec<User>,
}

/// Load the task along with supporting data required by the modal view.
pub fn load_task_modal<R>(
    repo: &R,
    user: &AuthenticatedUser,
    task_id: i32,
) -> ServiceResult<TaskModalData>
where
    R: TaskReader + UserReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let task = repo
        .get_task_by_id(task_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or(ServiceError::NotFound)?;

    let assignee = match task.assigned_to {
        Some(assignee_id) => match repo.get_user_by_id(assignee_id, user.hub_id) {
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

    let (_total, users) = repo
        .list_users(UserListQuery::new(user.hub_id))
        .map_err(ServiceError::from)?;

    Ok(TaskModalData {
        task,
        assignee,
        users,
    })
}

/// Update a task with the values submitted from the edit form.
pub fn update_task<R>(
    repo: &R,
    user: &AuthenticatedUser,
    task_id: i32,
    form: UpdateTaskForm,
) -> ServiceResult<Task>
where
    R: TaskReader + TaskWriter + TaskEventWriter + UserReader + UserWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if let Err(err) = form.validate() {
        log::error!("Failed to validate form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let submission = match form.into_submission(task_id) {
        Ok(submission) => submission,
        Err(err) => {
            log::error!("Failed to validate form: {err}");
            return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
        }
    };

    let TaskUpdateSubmission {
        task_id,
        updates,
        assignee,
    } = submission;

    let current_task = repo
        .get_task_by_id(task_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or(ServiceError::NotFound)?;

    let assignee_user = match assignee {
        Some(assignee) => {
            let new_user = assignee.into_new_user(user.hub_id);
            Some(repo.create_or_update_user(&new_user)?)
        }
        None => None,
    };

    let updates = apply_assignment_updates(
        updates,
        current_task.assigned_to,
        assignee_user.as_ref().map(|user| user.id),
    );

    let updated = repo
        .update_task(task_id, user.hub_id, &updates)
        .map_err(|err| match err {
            RepositoryError::NotFound => ServiceError::NotFound,
            other => ServiceError::from(other),
        })?;

    let status_event_data = status_event_payload(current_task.status, updated.status);

    let assignment_event_data = if current_task.assigned_to != updated.assigned_to {
        let previous_assignee = match current_task.assigned_to {
            Some(assignee_id) => repo
                .get_user_by_id(assignee_id, user.hub_id)
                .map_err(ServiceError::from)?,
            None => None,
        };

        let new_assignee = match updated.assigned_to {
            Some(assignee_id) => repo
                .get_user_by_id(assignee_id, user.hub_id)
                .map_err(ServiceError::from)?,
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
        let new_user = user.into();
        let actor = repo.create_or_update_user(&new_user)?;

        if let Some(data) = status_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::StatusChanged,
                data,
            );
            repo.record_event(&event).map_err(ServiceError::from)?;
        }

        if let Some(data) = assignment_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::AssignmentChanged,
                data,
            );
            repo.record_event(&event).map_err(ServiceError::from)?;
        }

        if let Some(data) = metadata_event_data {
            let event = NewTaskEvent::new(
                updated.id,
                Some(actor.id),
                TaskEventType::MetadataUpdated,
                data,
            );
            repo.record_event(&event).map_err(ServiceError::from)?;
        }

        repo.touch_visited_at(actor.id, actor.hub_id)?;
    }

    Ok(updated)
}

fn apply_assignment_updates(
    updates: UpdateTask,
    current_assigned_to: Option<i32>,
    new_assignee_id: Option<i32>,
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

/// Record a new comment on the specified task from the current user.
pub fn add_task_comment<R>(
    repo: &R,
    user: &AuthenticatedUser,
    task_id: i32,
    form: NewTaskCommentForm,
) -> ServiceResult<TaskEvent>
where
    R: TaskReader + TaskEventWriter + UserReader + UserWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if let Err(err) = form.validate() {
        log::error!("Failed to validate comment form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    repo.get_task_by_id(task_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or(ServiceError::NotFound)?;

    let new_user = user.into();
    let author = repo.create_or_update_user(&new_user)?;

    let submission = form.into_submission();
    let event = NewTaskEvent::new(
        task_id,
        Some(author.id),
        TaskEventType::Comment,
        json!({ "text": submission.text }),
    );

    let recorded = repo.record_event(&event).map_err(ServiceError::from)?;

    repo.touch_visited_at(author.id, author.hub_id)?;

    Ok(recorded)
}

fn assignment_event_user(user: &User) -> Value {
    json!({
        "id": user.id,
        "name": user.name,
        "email": user.email,
    })
}

/// Remove the specified task after verifying permissions and existence.
pub fn delete_task<R>(repo: &R, user: &AuthenticatedUser, task_id: i32) -> ServiceResult<()>
where
    R: TaskReader + TaskWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if repo
        .get_task_by_id(task_id, user.hub_id)
        .map_err(ServiceError::from)?
        .is_none()
    {
        return Err(ServiceError::NotFound);
    }

    repo.delete_task(task_id, user.hub_id)
        .map_err(|err| match err {
            RepositoryError::NotFound => ServiceError::NotFound,
            other => ServiceError::from(other),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveDateTime};
    use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};
    use serde_json::json;
    use std::cell::RefCell;

    use crate::domain::{
        task::{
            NewTask as DomainNewTask, TaskAssignment, TaskStatus, UpdateTask as DomainUpdateTask,
        },
        task_event::{NewTaskEvent as DomainNewTaskEvent, TaskEventType},
        user::User,
    };
    use crate::forms::task::NewTaskCommentForm;
    use crate::repository::mock::{
        MockTaskEventReader, MockTaskReader, MockTaskWriter, MockUserReader,
    };
    use crate::repository::{TaskListQuery, UserListQuery};
    use mockall::Sequence;

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
        fn get_task_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Task>> {
            self.task_reader.get_task_by_id(id, hub_id)
        }

        fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)> {
            self.task_reader.list_tasks(query)
        }

        fn list_assignments_for_task(
            &self,
            task_id: i32,
            hub_id: i32,
        ) -> RepositoryResult<Vec<TaskAssignment>> {
            self.task_reader.list_assignments_for_task(task_id, hub_id)
        }
    }

    impl TaskEventReader for TaskDetailsRepo {
        fn list_events_for_task(
            &self,
            task_id: i32,
            hub_id: i32,
        ) -> RepositoryResult<Vec<TaskEvent>> {
            self.event_reader.list_events_for_task(task_id, hub_id)
        }

        fn get_event_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<TaskEvent>> {
            self.event_reader.get_event_by_id(id, hub_id)
        }
    }

    impl UserReader for TaskDetailsRepo {
        fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(id, hub_id)
        }

        fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<User>> {
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
        fn get_task_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Task>> {
            self.task_reader.get_task_by_id(id, hub_id)
        }

        fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)> {
            self.task_reader.list_tasks(query)
        }

        fn list_assignments_for_task(
            &self,
            task_id: i32,
            hub_id: i32,
        ) -> RepositoryResult<Vec<TaskAssignment>> {
            self.task_reader.list_assignments_for_task(task_id, hub_id)
        }
    }

    impl TaskWriter for TaskDeleteRepo {
        fn create_task(&self, new_task: &DomainNewTask) -> RepositoryResult<Task> {
            self.task_writer.create_task(new_task)
        }

        fn update_task(
            &self,
            task_id: i32,
            hub_id: i32,
            updates: &DomainUpdateTask,
        ) -> RepositoryResult<Task> {
            self.task_writer.update_task(task_id, hub_id, updates)
        }

        fn delete_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<()> {
            self.task_writer.delete_task(task_id, hub_id)
        }

        fn record_assignment(&self, assignment: &TaskAssignment) -> RepositoryResult<()> {
            self.task_writer.record_assignment(assignment)
        }

        fn remove_assignment(
            &self,
            task_id: i32,
            hub_id: i32,
            assignee_id: i32,
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
            id,
            hub_id,
            title: "Test Task".to_string(),
            description: Some("Detail".to_string()),
            status: TaskStatus::Pending,
            due_date: None,
            assigned_to,
            author_id,
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            completed_at: None,
        }
    }

    fn sample_event(id: i32, task_id: i32, user_id: Option<i32>) -> TaskEvent {
        TaskEvent {
            id,
            task_id,
            user_id,
            event_type: TaskEventType::Comment,
            event_data: json!({"message": "hi"}),
            created_at: fixed_datetime(),
        }
    }

    fn sample_user(id: i32, hub_id: i32, name: &str, email: &str) -> User {
        User {
            id,
            hub_id,
            name: name.to_string(),
            email: email.to_string(),
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
        let base = UpdateTask::new();
        let assigned = apply_assignment_updates(base, Some(1), Some(2));
        assert_eq!(assigned.assigned_to, Some(Some(2)));

        let base = UpdateTask::new();
        let unassigned = apply_assignment_updates(base, Some(1), None);
        assert_eq!(unassigned.assigned_to, Some(None));

        let unchanged = apply_assignment_updates(UpdateTask::new(), Some(3), Some(3));
        assert_eq!(unchanged.assigned_to, None);
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
        updated.title = "Updated".to_string();
        updated.description = Some("New".to_string());
        updated.due_date = Some(NaiveDate::from_ymd_opt(2024, 5, 1).unwrap());

        let payload =
            metadata_event_payload(&current, &updated).expect("expected metadata payload");

        let expected = json!({
            "title": {"from": current.title.clone(), "to": updated.title.clone()},
            "description": {"from": current.description.clone(), "to": updated.description.clone()},
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

        let task = sample_task(5, 1, Some(assignee.id), author.id);
        let event = sample_event(13, task.id, Some(author.id));

        let mut repo = TaskDetailsRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let hub_id = user.hub_id;

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

        let result = load_task_details(&repo, &user, task.id).expect("should load task");

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

        let result = load_task_details(&repo, &user, 5);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_task_details_returns_not_found_for_missing_task() {
        let mut repo = TaskDetailsRepo::new();
        repo.task_reader
            .expect_get_task_by_id()
            .return_once(|_, _| Ok(None));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = load_task_details(&repo, &user, 99);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn load_task_details_propagates_repository_error() {
        let mut repo = TaskDetailsRepo::new();
        repo.task_reader
            .expect_get_task_by_id()
            .return_once(|_, _| Err(RepositoryError::Unexpected("boom".to_string())));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = load_task_details(&repo, &user, 1);

        assert!(matches!(result, Err(ServiceError::Repository(_))));
    }

    #[test]
    fn delete_task_requires_role() {
        let repo = TaskDeleteRepo::new();
        let user = user_with_roles(&[]);

        let result = delete_task(&repo, &user, 1);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn delete_task_returns_not_found_when_task_missing() {
        let mut repo = TaskDeleteRepo::new();
        repo.task_reader
            .expect_get_task_by_id()
            .return_once(|_, _| Ok(None));
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = delete_task(&repo, &user, 99);

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

        let result = delete_task(&repo, &user, 5);

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

        delete_task(&repo, &user, 7).expect("should delete task");
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

        let result = delete_task(&repo, &user, 1);

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
                map.insert(user.email.to_lowercase(), user);
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
        fn get_task_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Task>> {
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
            _: i32,
            _: i32,
        ) -> RepositoryResult<Vec<TaskAssignment>> {
            Ok(Vec::new())
        }
    }

    impl TaskWriter for UpdateRepo {
        fn create_task(&self, _: &DomainNewTask) -> RepositoryResult<Task> {
            Ok(self.task.borrow().clone())
        }

        fn update_task(
            &self,
            task_id: i32,
            hub_id: i32,
            updates: &DomainUpdateTask,
        ) -> RepositoryResult<Task> {
            let mut task = self.task.borrow_mut();
            if task.id != task_id || task.hub_id != hub_id {
                return Err(RepositoryError::NotFound);
            }

            if let Some(ref title) = updates.title {
                task.title = title.clone();
            }

            if let Some(ref description) = updates.description {
                task.description = description.clone();
            }

            if let Some(status) = updates.status {
                task.status = status;
            }

            if let Some(due_date) = updates.due_date {
                task.due_date = due_date;
            }

            if let Some(assigned_to) = updates.assigned_to {
                task.assigned_to = assigned_to;
            }

            if let Some(completed_at) = updates.completed_at {
                task.completed_at = completed_at;
            }

            task.updated_at = updates.updated_at;

            Ok(task.clone())
        }

        fn delete_task(&self, _: i32, _: i32) -> RepositoryResult<()> {
            Ok(())
        }

        fn record_assignment(&self, _: &TaskAssignment) -> RepositoryResult<()> {
            Ok(())
        }

        fn remove_assignment(&self, _: i32, _: i32, _: i32) -> RepositoryResult<()> {
            Ok(())
        }
    }

    impl UserReader for UpdateRepo {
        fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>> {
            Ok(self
                .users
                .borrow()
                .values()
                .find(|user| user.id == id && user.hub_id == hub_id)
                .cloned())
        }

        fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<User>> {
            Ok(self
                .users
                .borrow()
                .get(&email.trim().to_lowercase())
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
            if let Some(existing) = self.user_by_email(&new_user.email) {
                return Ok(existing);
            }

            let id = {
                let mut counter = self.next_user_id.borrow_mut();
                let id = *counter;
                *counter += 1;
                id
            };

            let user = User {
                id,
                hub_id: new_user.hub_id,
                name: new_user.name.clone(),
                email: new_user.email.clone(),
                visited_at: Some(fixed_datetime()),
            };

            self.users
                .borrow_mut()
                .insert(user.email.to_lowercase(), user.clone());

            Ok(user)
        }

        fn update_user(
            &self,
            _: i32,
            _: i32,
            _: &crate::domain::user::UpdateUser,
        ) -> RepositoryResult<User> {
            Err(RepositoryError::NotFound)
        }

        fn delete_user(&self, _: i32, _: i32) -> RepositoryResult<()> {
            Err(RepositoryError::NotFound)
        }

        fn touch_visited_at(&self, _: i32, _: i32) -> RepositoryResult<()> {
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
                id,
                task_id: event.task_id,
                user_id: event.user_id,
                event_type: event.event_type,
                event_data: event.event_data.clone(),
                created_at: event.created_at,
            };

            events.push(record.clone());
            Ok(record)
        }

        fn delete_event(&self, _: i32, _: i32) -> RepositoryResult<()> {
            Ok(())
        }
    }

    #[test]
    fn update_task_updates_fields_and_assignment() {
        let assignee = sample_user(7, 1, "Executor", "executor@example.com");
        let task = sample_task(42, 1, None, 3);
        let repo = UpdateRepo::new(task, vec![assignee.clone()]);
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let due_date = NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid date");
        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated title",
            "message": "Updated description",
            "status": "InProgress",
            "due_date": due_date.to_string(),
            "id": assignee.email,
            "name": assignee.name,
            "email": assignee.email,
        }))
        .expect("valid form payload");

        let outcome = update_task(&repo, &user, 42, form).expect("should update task");

        assert_eq!(outcome.id, 42);
        assert_eq!(outcome.title, "Updated title");

        let stored = repo.task.borrow().clone();
        assert_eq!(stored.title, "Updated title");
        assert_eq!(stored.status, TaskStatus::InProgress);
        assert_eq!(stored.due_date, Some(due_date));
        assert_eq!(stored.description.as_deref(), Some("Updated description"));
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
                }
            })
        );
    }

    #[test]
    fn update_task_creates_user_when_missing() {
        let task = sample_task(7, 1, None, 2);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "id": "auth0|user-1",
            "name": "Fresh User",
            "email": "fresh@example.com",
        }))
        .expect("valid form payload");

        update_task(&repo, &user, 7, form).expect("should create assignee");

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
                }
            })
        );
    }

    #[test]
    fn update_task_unassigns_when_selection_missing() {
        let assignee = sample_user(8, 1, "Assigned", "assigned@example.com");
        let task = sample_task(9, 1, Some(assignee.id), 4);
        let repo = UpdateRepo::new(task, vec![assignee.clone()]);
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Keep",
            "status": "Pending",
        }))
        .expect("valid form payload");

        update_task(&repo, &user, 9, form).expect("should unassign");

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
                }
            })
        );
    }

    #[test]
    fn update_task_requires_email_for_assignee() {
        let task = sample_task(11, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
            "id": "auth0|no-email",
            "name": "Nameless",
        }))
        .expect("valid form payload");

        let outcome = update_task(&repo, &user, 11, form).expect("expected update to succeed");

        assert_eq!(outcome.id, 11);
        assert_eq!(outcome.title, "Updated");

        {
            let stored = repo.task.borrow();
            assert_eq!(stored.title, "Updated");
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
        let user = user_with_roles(&[]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
        }))
        .expect("valid form payload");

        let result = update_task(&repo, &user, 12, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn update_task_returns_not_found_for_missing_task() {
        let task = sample_task(13, 2, None, 6);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form: UpdateTaskForm = serde_json::from_value(json!({
            "title": "Updated",
            "status": "Pending",
        }))
        .expect("valid form payload");

        let result = update_task(&repo, &user, 13, form);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn add_task_comment_records_event() {
        let commenter = sample_user(21, 1, "Commenter", "user@example.com");
        let task = sample_task(77, 1, None, commenter.id);
        let repo = UpdateRepo::new(task.clone(), vec![commenter.clone()]);
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form = NewTaskCommentForm {
            message: "Новый комментарий".to_string(),
        };

        let recorded = add_task_comment(&repo, &user, task.id, form).expect("should add comment");
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

        let form = NewTaskCommentForm {
            message: "Комментарий".to_string(),
        };

        add_task_comment(&repo, &user, task.id, form).expect("should add comment");

        let events = repo.events.borrow();
        assert_eq!(events.len(), 1);
        let event = &events[0];
        assert!(
            repo.users
                .borrow()
                .values()
                .any(|record| record.id == event.user_id.unwrap_or_default())
        );
    }

    #[test]
    fn add_task_comment_requires_role() {
        let task = sample_task(91, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[]);

        let form = NewTaskCommentForm {
            message: "Комментарий".to_string(),
        };

        let result = add_task_comment(&repo, &user, 91, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn add_task_comment_returns_form_error_on_invalid_payload() {
        let task = sample_task(93, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form = NewTaskCommentForm {
            message: String::new(),
        };

        let result = add_task_comment(&repo, &user, 93, form);

        assert!(matches!(result, Err(ServiceError::Form(_))));
    }

    #[test]
    fn add_task_comment_returns_not_found_for_missing_task() {
        let task = sample_task(99, 1, None, 5);
        let repo = UpdateRepo::new(task, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let form = NewTaskCommentForm {
            message: "Комментарий".to_string(),
        };

        let result = add_task_comment(&repo, &user, 123, form);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }
}
