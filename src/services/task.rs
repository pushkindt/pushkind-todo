use std::collections::HashMap;

use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::repository::errors::RepositoryError;
use pushkind_common::routes::check_role;
use serde::Serialize;
use serde_json::{Value, json};
use validator::Validate;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::{
    task::Task,
    task_event::{NewTaskEvent, TaskEvent, TaskEventType},
    user::User,
};
use crate::forms::task::{NewTaskCommentForm, TaskUpdateSubmission, UpdateTaskForm};
use crate::models::task::status_to_db;
use crate::repository::{
    TaskEventReader, TaskEventWriter, TaskReader, TaskWriter, UserListQuery, UserReader, UserWriter,
};
use crate::services::{RedirectSuccess, ServiceError, ServiceResult};

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
) -> ServiceResult<RedirectSuccess>
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
        mut updates,
        assignee,
    } = submission;

    let current_task = repo
        .get_task_by_id(task_id, user.hub_id)
        .map_err(ServiceError::from)?
        .ok_or(ServiceError::NotFound)?;

    match assignee {
        Some(assignee) => {
            let new_user = assignee.into_new_user(user.hub_id);
            let assignee = repo.create_or_update_user(&new_user)?;
            if current_task.assigned_to != Some(assignee.id) {
                updates = updates.assign_to(assignee.id);
            }
        }
        None => updates = updates.unassign(),
    }

    let updated = repo
        .update_task(task_id, user.hub_id, &updates)
        .map_err(|err| match err {
            RepositoryError::NotFound => ServiceError::NotFound,
            other => ServiceError::from(other),
        })?;

    let status_event_data = (current_task.status != updated.status).then(|| {
        json!({
            "from": status_to_db(current_task.status),
            "to": status_to_db(updated.status),
        })
    });

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

        Some(json!({
            "from": previous_assignee
                .as_ref()
                .map(assignment_event_user),
            "to": new_assignee.as_ref().map(assignment_event_user),
        }))
    } else {
        None
    };

    let metadata_event_data = {
        let mut changes = serde_json::Map::new();

        if current_task.title != updated.title {
            changes.insert(
                "title".to_string(),
                json!({
                    "from": current_task.title.clone(),
                    "to": updated.title.clone(),
                }),
            );
        }

        if current_task.description != updated.description {
            changes.insert(
                "description".to_string(),
                json!({
                    "from": current_task.description.clone(),
                    "to": updated.description.clone(),
                }),
            );
        }

        if current_task.due_date != updated.due_date {
            changes.insert(
                "due_date".to_string(),
                json!({
                    "from": current_task.due_date.map(|date| date.to_string()),
                    "to": updated.due_date.map(|date| date.to_string()),
                }),
            );
        }

        if current_task.completed_at != updated.completed_at {
            changes.insert(
                "completed_at".to_string(),
                json!({
                    "from": current_task.completed_at,
                    "to": updated.completed_at,
                }),
            );
        }

        if changes.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(changes))
        }
    };

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
    }

    Ok(RedirectSuccess {
        message: "Задача обновлена.".to_string(),
        redirect_to: format!("/task/{}", updated.id),
    })
}

/// Record a new comment on the specified task from the current user.
pub fn add_task_comment<R>(
    repo: &R,
    user: &AuthenticatedUser,
    task_id: i32,
    form: NewTaskCommentForm,
) -> ServiceResult<RedirectSuccess>
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

    let submission = form.into_submission();
    let new_user = user.into();
    let author = repo.create_or_update_user(&new_user)?;

    let event = NewTaskEvent::new(
        task_id,
        Some(author.id),
        TaskEventType::Comment,
        json!({ "text": submission.text }),
    );

    repo.record_event(&event).map_err(ServiceError::from)?;

    Ok(RedirectSuccess {
        message: "Комментарий добавлен.".to_string(),
        redirect_to: format!("/task/{}", task_id),
    })
}

fn assignment_event_user(user: &User) -> Value {
    json!({
        "id": user.id,
        "name": user.name,
        "email": user.email,
    })
}

/// Remove the specified task after verifying permissions and existence.
pub fn delete_task<R>(
    repo: &R,
    user: &AuthenticatedUser,
    task_id: i32,
) -> ServiceResult<RedirectSuccess>
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

    Ok(RedirectSuccess {
        message: "Задача удалена.".to_string(),
        redirect_to: "/".to_string(),
    })
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
    use crate::repository::{TaskListQuery, UserListQuery};

    #[derive(Default)]
    struct StubRepo {
        task: Option<Task>,
        events: Vec<TaskEvent>,
        hub_id: i32,
        fail_with_error: bool,
        delete_returns_not_found: bool,
        users: HashMap<i32, User>,
    }

    impl StubRepo {
        fn with_data(task: Task, events: Vec<TaskEvent>, hub_id: i32, users: Vec<User>) -> Self {
            let users = users
                .into_iter()
                .map(|user| (user.id, user))
                .collect::<HashMap<_, _>>();

            Self {
                task: Some(task),
                events,
                hub_id,
                fail_with_error: false,
                delete_returns_not_found: false,
                users,
            }
        }

        fn with_error() -> Self {
            Self {
                task: None,
                events: Vec::new(),
                hub_id: 1,
                fail_with_error: true,
                delete_returns_not_found: false,
                users: HashMap::new(),
            }
        }

        fn repo_error<T>(&self) -> RepositoryResult<T> {
            Err(RepositoryError::Unexpected("boom".to_string()))
        }
    }

    impl TaskReader for StubRepo {
        fn get_task_by_id(&self, _: i32, hub_id: i32) -> RepositoryResult<Option<Task>> {
            if self.fail_with_error {
                return self.repo_error();
            }

            if hub_id != self.hub_id {
                return Ok(None);
            }

            Ok(self.task.clone())
        }

        fn list_tasks(&self, _: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok((0, Vec::new()))
        }

        fn list_assignments_for_task(
            &self,
            _: i32,
            _: i32,
        ) -> RepositoryResult<Vec<TaskAssignment>> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok(Vec::new())
        }
    }

    impl TaskEventReader for StubRepo {
        fn list_events_for_task(&self, _: i32, hub_id: i32) -> RepositoryResult<Vec<TaskEvent>> {
            if self.fail_with_error {
                return self.repo_error();
            }

            if hub_id != self.hub_id {
                return Ok(Vec::new());
            }

            Ok(self.events.clone())
        }

        fn get_event_by_id(&self, _: i32, _: i32) -> RepositoryResult<Option<TaskEvent>> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok(None)
        }
    }

    impl UserReader for StubRepo {
        fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok(self
                .users
                .get(&id)
                .cloned()
                .filter(|user| user.hub_id == hub_id))
        }

        fn get_user_by_email(&self, _: &str, _: i32) -> RepositoryResult<Option<User>> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok(None)
        }

        fn list_users(&self, _: UserListQuery) -> RepositoryResult<(usize, Vec<User>)> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok((self.users.len(), self.users.values().cloned().collect()))
        }
    }

    impl TaskWriter for StubRepo {
        fn create_task(&self, _: &DomainNewTask) -> RepositoryResult<Task> {
            self.repo_error()
        }

        fn update_task(&self, _: i32, _: i32, _: &DomainUpdateTask) -> RepositoryResult<Task> {
            self.repo_error()
        }

        fn delete_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<()> {
            if self.fail_with_error {
                return self.repo_error();
            }

            if self.delete_returns_not_found {
                return Err(RepositoryError::NotFound);
            }

            match self.task {
                Some(ref task) if task.id == task_id && task.hub_id == hub_id => Ok(()),
                _ => Err(RepositoryError::NotFound),
            }
        }

        fn record_assignment(&self, _: &TaskAssignment) -> RepositoryResult<()> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok(())
        }

        fn remove_assignment(&self, _: i32, _: i32, _: i32) -> RepositoryResult<()> {
            if self.fail_with_error {
                return self.repo_error();
            }

            Ok(())
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
    fn load_task_details_returns_data() {
        let assignee = sample_user(7, 1, "Assignee", "assignee@example.com");
        let author = sample_user(11, 1, "Author", "author@example.com");

        let task = sample_task(5, 1, Some(assignee.id), author.id);
        let event = sample_event(13, task.id, Some(author.id));

        let repo = StubRepo::with_data(
            task.clone(),
            vec![event.clone()],
            1,
            vec![assignee.clone(), author.clone()],
        );
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

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
        let repo = StubRepo::default();
        let user = user_with_roles(&[]);

        let result = load_task_details(&repo, &user, 5);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_task_details_returns_not_found_for_missing_task() {
        let repo = StubRepo {
            task: None,
            events: Vec::new(),
            hub_id: 1,
            fail_with_error: false,
            delete_returns_not_found: false,
            users: HashMap::new(),
        };
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = load_task_details(&repo, &user, 99);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn load_task_details_propagates_repository_error() {
        let repo = StubRepo::with_error();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = load_task_details(&repo, &user, 1);

        assert!(matches!(result, Err(ServiceError::Repository(_))));
    }

    #[test]
    fn delete_task_requires_role() {
        let repo = StubRepo::with_data(sample_task(1, 1, None, 2), Vec::new(), 1, Vec::new());
        let user = user_with_roles(&[]);

        let result = delete_task(&repo, &user, 1);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn delete_task_returns_not_found_when_task_missing() {
        let repo = StubRepo::default();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = delete_task(&repo, &user, 99);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn delete_task_returns_not_found_when_repository_reports_missing() {
        let repo = StubRepo {
            task: Some(sample_task(5, 1, None, 3)),
            events: Vec::new(),
            hub_id: 1,
            fail_with_error: false,
            delete_returns_not_found: true,
            users: HashMap::new(),
        };
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let result = delete_task(&repo, &user, 5);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }

    #[test]
    fn delete_task_returns_redirect_on_success() {
        let repo = StubRepo::with_data(sample_task(7, 1, None, 4), Vec::new(), 1, Vec::new());
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);

        let outcome = delete_task(&repo, &user, 7).expect("should delete task");

        assert_eq!(outcome.message, "Задача удалена.");
        assert_eq!(outcome.redirect_to, "/");
    }

    #[test]
    fn delete_task_propagates_repository_error() {
        let repo = StubRepo::with_error();
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

        assert_eq!(outcome.message, "Задача обновлена.");
        assert_eq!(outcome.redirect_to, "/task/42");

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

        assert_eq!(outcome.message, "Задача обновлена.");
        assert_eq!(outcome.redirect_to, "/task/11");

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
            text: "Новый комментарий".to_string(),
        };

        let outcome = add_task_comment(&repo, &user, task.id, form).expect("should add comment");

        assert_eq!(outcome.message, "Комментарий добавлен.");
        assert_eq!(outcome.redirect_to, format!("/task/{}", task.id));

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
            text: "Комментарий".to_string(),
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
            text: "Комментарий".to_string(),
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
            text: String::new(),
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
            text: "Комментарий".to_string(),
        };

        let result = add_task_comment(&repo, &user, 123, form);

        assert!(matches!(result, Err(ServiceError::NotFound)));
    }
}
