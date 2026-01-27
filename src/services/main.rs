//! Core service operations powering the index page, user tracking, and pagination logic.
use chrono::{NaiveDate, NaiveDateTime};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::ensure_role;
use pushkind_common::zmq::ZmqSenderExt;
use pushkind_emailer::domain::email::NewEmail;
use std::collections::HashMap;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::task::{Task, TaskPriority, TaskStatus};
use crate::domain::types::{ClientId, HubId, TaskTrack, UserId};
use crate::domain::user::{NewUser, User};
use crate::dto::zmq::ZmqTask;
use crate::forms::main::{AddTaskForm, AddTaskPayload, UploadTasksForm};
use crate::repository::{
    ClientReader, TaskListQuery, TaskReader, TaskWriter, UserListQuery, UserReader, UserWriter,
};
use crate::services::{ServiceError, ServiceResult};

use super::notifications;
use crate::dto::main::{IndexPageData, IndexPageFilters, IndexQuery, IndexTask};

/// Loads the tasks list for the main index page.
pub fn load_index_page<R>(
    query: IndexQuery,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<IndexPageData>
where
    R: TaskReader + UserReader + UserWriter + ClientReader + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let IndexQuery {
        search,
        page,
        status,
        track,
        assignee,
        client,
        priority,
        updated_after,
        updated_before,
    } = query;

    let page = page.unwrap_or(1);

    let hub_id = HubId::new(user.hub_id)?;

    let mut list_query = TaskListQuery::new(hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);
    list_query.filters_mut().hide_terminal_statuses = true;

    let mut status_filter = None;
    if let Some(status_value) = status
        .as_deref()
        .and_then(|value| TaskStatus::try_from(value).ok())
    {
        list_query.filters_mut().status = Some(status_value);
        list_query.filters_mut().hide_terminal_statuses = false;
        status_filter = Some(status_value);
    }

    let mut track_filter = None;
    if let Some(track_value) = track
        .as_deref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        && let Ok(track) = TaskTrack::new(track_value)
    {
        list_query.filters_mut().track = Some(track.clone());
        track_filter = Some(track);
    }

    let mut priority_filter = None;
    if let Some(priority_value) = priority
        .as_deref()
        .and_then(|value| TaskPriority::try_from(value).ok())
    {
        list_query.filters_mut().priority = Some(priority_value);
        priority_filter = Some(priority_value);
    }

    let mut assignee_filter = None;
    if let Some(assignee_id) = assignee
        && let Ok(user_id) = UserId::new(assignee_id)
    {
        list_query.filters_mut().assignee_id = Some(user_id);
        assignee_filter = Some(user_id);
    }

    let mut client_filter = None;
    if let Some(client_id) = client
        && let Ok(client_id) = ClientId::new(client_id)
    {
        list_query.filters_mut().client_id = Some(client_id);
        client_filter = Some(client_id);
    }

    let mut updated_after_filter = None;
    if let Some(updated_after_value) = updated_after.as_deref().and_then(parse_date_filter)
        && let Some(timestamp) = start_of_day(updated_after_value)
    {
        list_query.filters_mut().updated_after = Some(timestamp);
        updated_after_filter = Some(updated_after_value);
    }

    let mut updated_before_filter = None;
    if let Some(updated_before_value) = updated_before.as_deref().and_then(parse_date_filter)
        && let Some(timestamp) = end_of_day(updated_before_value)
    {
        list_query.filters_mut().updated_before = Some(timestamp);
        updated_before_filter = Some(updated_before_value);
    }

    if let Some(value) = search.as_ref().map(|s| s.trim())
        && !value.is_empty()
    {
        list_query.filters_mut().search = Some(value.to_string());
    }

    let new_user: NewUser = user.try_into()?;
    let user = repo.create_or_update_user(&new_user)?;
    let visited_at = user.visited_at;

    repo.touch_visited_at(user.id, user.hub_id)?;

    let (total, tasks) = repo.list_tasks(list_query)?;
    let (_, users) = repo.list_users(UserListQuery::new(user.hub_id))?;

    let users_by_id = users
        .iter()
        .cloned()
        .map(|user| (user.id, user))
        .collect::<HashMap<_, _>>();

    let recently_updated_task_ids = visited_at
        .map(|visited| {
            tasks
                .iter()
                .filter(|task| task.updated_at > visited)
                .map(|task| task.id)
                .collect()
        })
        .unwrap_or_default();

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let task_entries = tasks
        .into_iter()
        .map(|task| IndexTask {
            assignee: task
                .assigned_to
                .and_then(|assignee_id| users_by_id.get(&assignee_id).cloned()),
            task,
        })
        .collect::<Vec<_>>();
    let tasks = Paginated::new(task_entries, page, total_pages);

    let filters = IndexPageFilters {
        search,
        status: status_filter,
        track: track_filter,
        assignee: assignee_filter,
        priority: priority_filter,
        updated_after: updated_after_filter,
        updated_before: updated_before_filter,
        client: client_filter,
    };

    let tracks = repo.list_task_tracks(user.hub_id)?;

    let clients = repo.list_clients(user.hub_id)?;

    Ok(IndexPageData {
        tasks,
        filters,
        users,
        recently_updated_task_ids,
        tracks,
        clients,
    })
}

/// Parse a `YYYY-MM-DD` date filter into a NaiveDate.
fn parse_date_filter(input: &str) -> Option<NaiveDate> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok()
}

/// Return the start-of-day timestamp for the given date.
fn start_of_day(date: NaiveDate) -> Option<NaiveDateTime> {
    date.and_hms_opt(0, 0, 0)
}

/// Return the end-of-day timestamp for the given date.
fn end_of_day(date: NaiveDate) -> Option<NaiveDateTime> {
    date.and_hms_micro_opt(23, 59, 59, 999_999)
        .or_else(|| date.and_hms_opt(23, 59, 59))
}

/// Validates the add-task form and persists a new task record.
pub fn add_task<R, ZE, ZT>(
    form: AddTaskForm,
    user: &AuthenticatedUser,
    repo: &R,
    zmq_email_sender: &ZE,
    zmq_task_sender: &ZT,
) -> ServiceResult<Task>
where
    R: TaskWriter + UserReader + UserWriter + ?Sized,
    ZE: ZmqSenderExt,
    ZT: ZmqSenderExt,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let hub_id = HubId::new(user.hub_id)?;
    let new_user: NewUser = user.try_into()?;
    let author = repo.create_or_update_user(&new_user)?;

    let payload = AddTaskPayload::try_from(form)?;
    let assignee_selection = payload.assignee.clone();

    let mut new_task = payload.into_domain(author.id, hub_id)?;

    let assignee_user = match assignee_selection {
        Some(selection) => {
            let new_user = selection.into_domain(hub_id)?;
            Some(repo.create_or_update_user(&new_user)?)
        }
        None => None,
    };

    if let Some(assignee) = assignee_user.as_ref() {
        new_task = new_task.assign_to(assignee.id);
    }

    let created = repo.create_task(&new_task)?;

    if let Some(assignee) = assignee_user.as_ref() {
        match build_task_created_email(&created, &author, assignee, user) {
            Ok(Some(email)) => {
                if let Err(err) = notifications::queue_email(zmq_email_sender, user, email) {
                    log::error!("Failed to queue task-created email: {err}");
                }
            }
            Ok(None) => {}
            Err(err) => {
                log::error!("Failed to build task-created email: {err}");
            }
        }
    }

    match ZmqTask::try_from((&created, &author, assignee_user.as_ref(), None)) {
        Ok(snapshot) => {
            if let Err(err) = notifications::queue_task_snapshot(zmq_task_sender, snapshot) {
                log::error!("Failed to queue task snapshot: {err}");
            }
        }
        Err(err) => {
            log::warn!("Skipping task snapshot: {err}");
        }
    }

    repo.touch_visited_at(author.id, author.hub_id)?;

    Ok(created)
}

/// Parses the uploaded CSV file and creates task records in bulk.
pub fn upload_tasks<R>(
    form: UploadTasksForm,
    user: &AuthenticatedUser,
    repo: &R,
) -> ServiceResult<usize>
where
    R: TaskWriter + UserReader + UserWriter + ?Sized,
{
    ensure_role(user, SERVICE_ACCESS_ROLE)?;

    let mut form = form;

    let hub_id = HubId::new(user.hub_id)?;
    let new_user: NewUser = user.try_into()?;
    let author = repo.create_or_update_user(&new_user)?;

    let new_tasks = form.parse(author.id, hub_id).map_err(|err| {
        log::error!("Failed to parse tasks: {err}");
        ServiceError::Form("Ошибка при парсинге задач".to_string())
    })?;

    let created_count = new_tasks.len();

    for new_task in new_tasks {
        repo.create_task(&new_task)?;
    }

    repo.touch_visited_at(author.id, author.hub_id)?;

    Ok(created_count)
}

/// Build a notification email informing the assignee of a new task.
fn build_task_created_email(
    task: &Task,
    author: &User,
    assignee: &User,
    actor: &AuthenticatedUser,
) -> ServiceResult<Option<NewEmail>> {
    let actor_email = actor.email.trim().to_lowercase();
    let assignee_email = assignee.email.as_str().trim().to_lowercase();

    if actor_email == assignee_email {
        return Ok(None);
    }

    let sanitized_title = notifications::sanitize_text(task.title.as_str());
    let sanitized_author_name = notifications::sanitize_text(author.name.as_str());

    let mut message = format!(
        "<p>Вам назначена новая задача <strong>{}</strong> от {} ({}).</p>",
        sanitized_title,
        sanitized_author_name,
        author.email.as_str()
    );

    if let Some(description) = &task.description
        && !description.as_str().trim().is_empty()
    {
        message.push_str("<hr>");
        message.push_str(description.as_str());
    }

    let recipient = notifications::task_recipient(task, assignee, "task_created", "assignee")?;

    let email = NewEmail::try_new(
        actor.hub_id,
        message,
        Some(format!("Вам назначена задача: {}", sanitized_title)),
        None,
        None,
        None,
        vec![recipient],
    )
    .map_err(|err| {
        log::error!("Failed to build task-created email payload: {err}");
        ServiceError::Internal
    })?;

    Ok(Some(email))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_multipart::form::tempfile::TempFile;
    use chrono::{Duration, NaiveDate, NaiveDateTime};
    use mockall::Sequence;
    use pushkind_common::repository::errors::RepositoryError;
    use pushkind_common::zmq::{SendFuture, ZmqSenderError, ZmqSenderTrait};
    use pushkind_emailer::models::zmq::ZMQSendEmailMessage;
    use serde_json::Value;
    use tempfile::NamedTempFile;

    use crate::SERVICE_ACCESS_ROLE;
    use crate::domain::client;
    use crate::domain::task::{
        NewTask as DomainNewTask, Task, TaskAssignment as DomainTaskAssignment, TaskPriority,
        TaskStatus, UpdateTask as DomainUpdateTask,
    };
    use crate::domain::types::{
        HubId, TaskDescription, TaskId, TaskTitle, TaskTrack, UserEmail, UserId, UserName,
    };
    use crate::domain::user::{UpdateUser, User};
    use crate::forms::task::AssigneeSelectionForm;
    use crate::repository::mock::{
        MockClientReader, MockTaskReader, MockTaskWriter, MockUserReader, MockUserWriter,
    };
    use crate::repository::{ClientReader, TaskWriter, UserListQuery, UserReader, UserWriter};
    use crate::services::mock::MockZmqSender;

    use std::io::Write;
    use std::sync::Mutex;

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_task(id: i32, hub_id: i32, title: &str) -> Task {
        Task {
            id: TaskId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            title: TaskTitle::new(title).unwrap(),
            description: None,
            track: None,
            priority: TaskPriority::Middle,
            status: TaskStatus::Pending,
            due_date: None,
            assigned_to: None,
            author_id: UserId::new(1).unwrap(),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            completed_at: None,
            client_id: None,
            public_id: None,
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 99,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    fn sample_user_record(id: i32, hub_id: i32, email: &str, name: &str) -> User {
        User {
            id: UserId::new(id).unwrap(),
            hub_id: HubId::new(hub_id).unwrap(),
            name: UserName::new(name).unwrap(),
            email: UserEmail::new(email).unwrap(),
            visited_at: Some(fixed_datetime()),
        }
    }

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

    struct TaskReaderUserRepo {
        pub task_reader: MockTaskReader,
        pub client_reader: MockClientReader,
        pub user_reader: MockUserReader,
        pub user_writer: MockUserWriter,
    }

    impl TaskReaderUserRepo {
        fn new() -> Self {
            Self {
                task_reader: MockTaskReader::new(),
                client_reader: MockClientReader::new(),
                user_reader: MockUserReader::new(),
                user_writer: MockUserWriter::new(),
            }
        }
    }

    impl TaskReader for TaskReaderUserRepo {
        fn get_task_by_id(
            &self,
            id: TaskId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<Task>> {
            self.task_reader.get_task_by_id(id, hub_id)
        }

        fn list_tasks(
            &self,
            query: TaskListQuery,
        ) -> pushkind_common::repository::errors::RepositoryResult<(usize, Vec<Task>)> {
            self.task_reader.list_tasks(query)
        }

        fn list_assignments_for_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Vec<DomainTaskAssignment>>
        {
            self.task_reader.list_assignments_for_task(task_id, hub_id)
        }

        fn list_task_tracks(
            &self,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Vec<TaskTrack>> {
            self.task_reader.list_task_tracks(hub_id)
        }
    }

    impl UserReader for TaskReaderUserRepo {
        fn get_user_by_id(
            &self,
            id: UserId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(id, hub_id)
        }

        fn get_user_by_email(
            &self,
            email: &UserEmail,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_email(email, hub_id)
        }

        fn list_users(
            &self,
            query: UserListQuery,
        ) -> pushkind_common::repository::errors::RepositoryResult<(usize, Vec<User>)> {
            self.user_reader.list_users(query)
        }
    }

    impl ClientReader for TaskReaderUserRepo {
        fn get_client_by_id(
            &self,
            id: ClientId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<client::Client>> {
            self.client_reader.get_client_by_id(id, hub_id)
        }

        fn list_clients(
            &self,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Vec<client::Client>> {
            self.client_reader.list_clients(hub_id)
        }
    }

    impl UserWriter for TaskReaderUserRepo {
        fn create_or_update_user(
            &self,
            new_user: &NewUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.create_or_update_user(new_user)
        }

        fn update_user(
            &self,
            user_id: UserId,
            hub_id: HubId,
            updates: &UpdateUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.update_user(user_id, hub_id, updates)
        }

        fn delete_user(
            &self,
            user_id: UserId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.user_writer.delete_user(user_id, hub_id)
        }

        fn touch_visited_at(
            &self,
            user_id: UserId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.user_writer.touch_visited_at(user_id, hub_id)
        }
    }

    struct TaskWriterUserRepo {
        pub task_writer: MockTaskWriter,
        pub user_reader: MockUserReader,
        pub user_writer: MockUserWriter,
    }

    impl TaskWriterUserRepo {
        fn new() -> Self {
            Self {
                task_writer: MockTaskWriter::new(),
                user_reader: MockUserReader::new(),
                user_writer: MockUserWriter::new(),
            }
        }
    }

    impl TaskWriter for TaskWriterUserRepo {
        fn create_task(
            &self,
            new_task: &DomainNewTask,
        ) -> pushkind_common::repository::errors::RepositoryResult<Task> {
            self.task_writer.create_task(new_task)
        }

        fn update_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
            updates: &DomainUpdateTask,
        ) -> pushkind_common::repository::errors::RepositoryResult<Task> {
            self.task_writer.update_task(task_id, hub_id, updates)
        }

        fn delete_task(
            &self,
            task_id: TaskId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.task_writer.delete_task(task_id, hub_id)
        }

        fn record_assignment(
            &self,
            assignment: &DomainTaskAssignment,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.task_writer.record_assignment(assignment)
        }

        fn remove_assignment(
            &self,
            task_id: TaskId,
            hub_id: HubId,
            assignee_id: UserId,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.task_writer
                .remove_assignment(task_id, hub_id, assignee_id)
        }
    }

    impl UserReader for TaskWriterUserRepo {
        fn get_user_by_id(
            &self,
            id: UserId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(id, hub_id)
        }

        fn get_user_by_email(
            &self,
            email: &UserEmail,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_email(email, hub_id)
        }

        fn list_users(
            &self,
            query: UserListQuery,
        ) -> pushkind_common::repository::errors::RepositoryResult<(usize, Vec<User>)> {
            self.user_reader.list_users(query)
        }
    }

    impl UserWriter for TaskWriterUserRepo {
        fn create_or_update_user(
            &self,
            new_user: &NewUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.create_or_update_user(new_user)
        }

        fn update_user(
            &self,
            user_id: UserId,
            hub_id: HubId,
            updates: &UpdateUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.update_user(user_id, hub_id, updates)
        }

        fn delete_user(
            &self,
            user_id: UserId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.user_writer.delete_user(user_id, hub_id)
        }

        fn touch_visited_at(
            &self,
            user_id: UserId,
            hub_id: HubId,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.user_writer.touch_visited_at(user_id, hub_id)
        }
    }

    fn assignee_selection_form_none() -> AssigneeSelectionForm {
        AssigneeSelectionForm {
            email: None,
            name: None,
        }
    }

    fn upload_form(contents: &str) -> UploadTasksForm {
        let mut file = match NamedTempFile::new() {
            Ok(file) => file,
            Err(err) => panic!("failed to create temp file: {err}"),
        };

        if let Err(err) = file.write_all(contents.as_bytes()) {
            panic!("failed to write csv contents: {err}");
        }

        UploadTasksForm {
            csv: TempFile {
                file,
                content_type: None,
                file_name: Some("upload.csv".to_string()),
                size: contents.len(),
            },
        }
    }

    #[test]
    fn load_index_page_returns_unauthorized_when_role_missing() {
        let repo = TaskReaderUserRepo::new();
        let user = user_with_roles(&[]);

        let result = load_index_page(IndexQuery::default(), &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_index_page_returns_paginated_data() {
        let mut repo = TaskReaderUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = IndexQuery {
            search: Some("alp".to_string()),
            page: Some(2),
            ..Default::default()
        };

        let expected_hub = user.hub_id;
        let hub_for_assert = expected_hub;
        let hub_for_return = expected_hub;
        let expected_email = user.email.clone();
        let expected_name = user.name.clone();
        let expected_user = sample_user_record(5, expected_hub, &expected_email, &expected_name);
        let expected_tracks = vec![
            TaskTrack::new("Activation".to_string()).unwrap(),
            TaskTrack::new("Retention".to_string()).unwrap(),
        ];

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning({
                let expected_hub_id = expected_hub;
                let expected_email = expected_email.clone();
                let expected_name = expected_name.clone();
                let expected_user = expected_user.clone();
                move |new_user| {
                    assert_eq!(new_user.hub_id, HubId::new(expected_hub_id).unwrap());
                    assert_eq!(new_user.email, UserEmail::new(&expected_email).unwrap());
                    assert_eq!(new_user.name, UserName::new(&expected_name).unwrap());
                    Ok(expected_user.clone())
                }
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning({
                let expected_user = expected_user.clone();
                move |user_id, hub_id| {
                    assert_eq!(user_id, expected_user.id);
                    assert_eq!(hub_id, expected_user.hub_id);
                    Ok(())
                }
            });

        repo.user_reader
            .expect_list_users()
            .times(1)
            .withf(move |query| {
                query.hub_id == HubId::new(hub_for_assert).unwrap()
                    && query.pagination.is_none()
                    && query.search.is_none()
            })
            .returning(|_| Ok((0, Vec::new())));

        repo.task_reader
            .expect_list_tasks()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.filters.hub_id, HubId::new(hub_for_assert).unwrap());
                assert_eq!(query.filters.search.as_deref(), Some("alp"));
                assert!(query.filters.hide_terminal_statuses);
                assert!(query.filters.track.is_none());
                assert!(query.filters.priority.is_none());
                assert!(query.filters.assignee_id.is_none());
                match &query.pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 2);
                        assert_eq!(pagination.per_page, DEFAULT_ITEMS_PER_PAGE);
                    }
                    None => panic!("expected pagination to be set"),
                }
                true
            })
            .returning(move |_| {
                Ok((
                    45,
                    vec![
                        sample_task(1, hub_for_return, "alpha"),
                        sample_task(2, hub_for_return, "beta"),
                    ],
                ))
            });

        repo.task_reader
            .expect_list_task_tracks()
            .times(1)
            .returning({
                let hub_for_tracks = expected_hub;
                let tracks_for_return = expected_tracks.clone();
                move |hub_id| {
                    assert_eq!(hub_id, HubId::new(hub_for_tracks).unwrap());
                    Ok(tracks_for_return.clone())
                }
            });

        repo.client_reader
            .expect_list_clients()
            .times(1)
            .returning(move |hub_id| {
                assert_eq!(hub_id, HubId::new(expected_hub).unwrap());
                Ok(Vec::new())
            });

        let result = load_index_page(query, &user, &repo);

        let data = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(data.filters.search.as_deref(), Some("alp"));
        assert!(data.filters.track.is_none());
        assert!(data.filters.priority.is_none());
        assert!(data.filters.assignee.is_none());
        assert!(data.users.is_empty());
        assert!(data.recently_updated_task_ids.is_empty());
        assert_eq!(data.tracks, expected_tracks);

        let serialized = match serde_json::to_value(&data.tasks) {
            Ok(value) => value,
            Err(err) => panic!("serialization failed: {err}"),
        };

        let page_value = match serialized.get("page") {
            Some(value) => value,
            None => panic!("missing page field"),
        };
        assert_eq!(page_value.as_u64(), Some(2));

        let items = match serialized.get("items") {
            Some(value) => match value.as_array() {
                Some(items) => items,
                None => panic!("items field is not an array"),
            },
            None => panic!("missing items field"),
        };
        assert_eq!(items.len(), 2);

        let first_title = items
            .first()
            .and_then(|item| item.as_object())
            .and_then(|map| map.get("task"))
            .and_then(|task| task.get("title"))
            .and_then(Value::as_str);
        assert_eq!(first_title, Some("alpha"));
    }

    #[test]
    fn load_index_page_applies_filters_to_query() {
        let mut repo = TaskReaderUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = IndexQuery {
            search: Some("project".to_string()),
            page: Some(1),
            status: Some("Completed".to_string()),
            track: Some("Activation".to_string()),
            assignee: Some(24),
            priority: Some("High".to_string()),
            updated_after: Some("2024-05-01".to_string()),
            updated_before: Some("2024-05-31".to_string()),
            client: None,
        };

        let expected_email = user.email.clone();
        let expected_hub_id = user.hub_id;
        let expected_name = user.name.clone();
        let expected_email = UserEmail::new(expected_email).unwrap();
        let expected_name = UserName::new(expected_name).unwrap();
        let expected_user = sample_user_record(
            7,
            expected_hub_id,
            expected_email.as_str(),
            expected_name.as_str(),
        );
        let assignee_user =
            sample_user_record(24, expected_hub_id, "owner@example.com", "Task Owner");
        let expected_tracks = vec![TaskTrack::new("Activation".to_string()).unwrap()];

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning({
                let expected_email = expected_email.clone();
                let expected_name = expected_name.clone();
                let expected_user = expected_user.clone();
                move |new_user| {
                    assert_eq!(new_user.hub_id, expected_user.hub_id);
                    assert_eq!(new_user.email, expected_email);
                    assert_eq!(new_user.name, expected_name);
                    Ok(expected_user.clone())
                }
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning({
                let expected_user = expected_user.clone();
                move |user_id, hub_id| {
                    assert_eq!(user_id, expected_user.id);
                    assert_eq!(hub_id, expected_user.hub_id);
                    Ok(())
                }
            });

        repo.user_reader
            .expect_list_users()
            .times(1)
            .withf(move |query| {
                query.hub_id == HubId::new(expected_hub_id).unwrap()
                    && query.pagination.is_none()
                    && query.search.is_none()
            })
            .returning({
                let expected_user = expected_user.clone();
                let assignee_user = assignee_user.clone();
                move |_| Ok((2, vec![expected_user.clone(), assignee_user.clone()]))
            });

        let expected_after_date =
            NaiveDate::from_ymd_opt(2024, 5, 1).expect("valid after date provided");
        let expected_before_date =
            NaiveDate::from_ymd_opt(2024, 5, 31).expect("valid before date provided");
        let expected_after_ts =
            start_of_day(expected_after_date).expect("start of day should be available");
        let expected_before_ts =
            end_of_day(expected_before_date).expect("end of day should be available");

        repo.task_reader
            .expect_list_tasks()
            .times(1)
            .returning(move |query| {
                let TaskListQuery {
                    filters,
                    pagination,
                } = query;

                assert_eq!(filters.hub_id, HubId::new(expected_hub_id).unwrap());
                assert_eq!(filters.search.as_deref(), Some("project"));
                assert_eq!(filters.status, Some(TaskStatus::Completed));
                assert_eq!(
                    filters.track.as_ref().map(|track| track.as_str()),
                    Some("Activation")
                );
                assert_eq!(filters.priority, Some(TaskPriority::High));
                assert_eq!(filters.assignee_id, Some(UserId::new(24).unwrap()));
                assert_eq!(filters.updated_after, Some(expected_after_ts));
                assert_eq!(filters.updated_before, Some(expected_before_ts));
                assert!(!filters.hide_terminal_statuses);

                match pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 1);
                        assert_eq!(pagination.per_page, DEFAULT_ITEMS_PER_PAGE);
                    }
                    None => panic!("expected pagination to be provided"),
                }

                Ok((0, Vec::new()))
            });

        repo.task_reader
            .expect_list_task_tracks()
            .times(1)
            .returning({
                let expected_tracks = expected_tracks.clone();
                move |hub_id| {
                    assert_eq!(hub_id, HubId::new(expected_hub_id).unwrap());
                    Ok(expected_tracks.clone())
                }
            });

        repo.client_reader
            .expect_list_clients()
            .times(1)
            .returning(move |hub_id| {
                assert_eq!(hub_id, HubId::new(expected_hub_id).unwrap());
                Ok(Vec::new())
            });

        let result = load_index_page(query, &user, &repo).expect("expected success");
        assert_eq!(result.filters.status, Some(TaskStatus::Completed));
        assert_eq!(
            result.filters.track,
            Some(TaskTrack::new("Activation".to_string()).unwrap())
        );
        assert_eq!(result.filters.assignee, Some(UserId::new(24).unwrap()));
        assert_eq!(result.filters.priority, Some(TaskPriority::High));
        assert_eq!(result.filters.updated_after, Some(expected_after_date));
        assert_eq!(result.filters.updated_before, Some(expected_before_date));
        assert_eq!(result.users.len(), 2);
        assert_eq!(result.users[1].id, UserId::new(24).unwrap());
        assert_eq!(result.tracks, expected_tracks);
    }

    #[test]
    fn load_index_page_marks_recently_updated_tasks() {
        let mut repo = TaskReaderUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = IndexQuery {
            search: None,
            page: None,
            ..Default::default()
        };

        let visited_at = fixed_datetime();
        let expected_email = UserEmail::new(user.email.clone()).unwrap();
        let expected_hub_id = user.hub_id;
        let expected_name = UserName::new(user.name.clone()).unwrap();
        let user_record = sample_user_record(
            42,
            expected_hub_id,
            expected_email.as_str(),
            expected_name.as_str(),
        );

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning({
                let expected_email = expected_email.clone();
                let expected_name = expected_name.clone();
                let user_record = user_record.clone();
                move |new_user| {
                    assert_eq!(new_user.hub_id, user_record.hub_id);
                    assert_eq!(new_user.email, expected_email);
                    assert_eq!(new_user.name, expected_name);
                    Ok(user_record.clone())
                }
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning({
                let user_record = user_record.clone();
                move |user_id, hub_id| {
                    assert_eq!(user_id, user_record.id);
                    assert_eq!(hub_id, user_record.hub_id);
                    Ok(())
                }
            });

        repo.user_reader
            .expect_list_users()
            .times(1)
            .withf(move |query| {
                query.hub_id == HubId::new(expected_hub_id).unwrap()
                    && query.pagination.is_none()
                    && query.search.is_none()
            })
            .returning(|_| Ok((0, Vec::new())));

        let fresh_task_id = TaskId::new(2).unwrap();
        let hub_id_for_tasks = user.hub_id;

        repo.task_reader.expect_list_tasks().times(1).returning({
            let visited_at_for_tasks = visited_at;
            move |_| {
                let mut stale_task = sample_task(1, hub_id_for_tasks, "stale");
                stale_task.updated_at = visited_at_for_tasks;

                let mut fresh_task = sample_task(fresh_task_id.get(), hub_id_for_tasks, "fresh");
                fresh_task.updated_at = visited_at_for_tasks + Duration::hours(1);

                Ok((2, vec![stale_task, fresh_task]))
            }
        });

        repo.task_reader
            .expect_list_task_tracks()
            .times(1)
            .returning({
                move |hub_id| {
                    assert_eq!(hub_id, HubId::new(expected_hub_id).unwrap());
                    Ok(Vec::new())
                }
            });

        repo.client_reader
            .expect_list_clients()
            .times(1)
            .returning(move |hub_id| {
                assert_eq!(hub_id, HubId::new(expected_hub_id).unwrap());
                Ok(Vec::new())
            });

        let result = load_index_page(query, &user, &repo).expect("expected success");

        assert!(result.users.is_empty());
        assert_eq!(result.recently_updated_task_ids, vec![fresh_task_id]);
        assert!(result.tracks.is_empty());
    }

    #[test]
    fn add_task_returns_unauthorized_when_role_missing() {
        let repo = TaskWriterUserRepo::new();
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[]);
        let form = AddTaskForm {
            title: "alpha".to_string(),
            message: None,
            track: None,
            priority: "Middle".to_string(),
            assignee: assignee_selection_form_none(),
        };

        let result = add_task(form, &user, &repo, &zmq, &zmq);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn add_task_returns_form_error_on_validation_failure() {
        let mut repo = TaskWriterUserRepo::new();
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: String::new(),
            message: None,
            track: None,
            priority: "Middle".to_string(),
            assignee: assignee_selection_form_none(),
        };

        let expected_hub = user.hub_id;
        let expected_email = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(7, expected_hub, &expected_email, &expected_name);

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .return_once(move |_| Ok(author));
        repo.user_writer.expect_touch_visited_at().never();
        repo.task_writer.expect_create_task().never();

        let result = add_task(form, &user, &repo, &zmq, &zmq);

        match result {
            Err(ServiceError::Form(message)) => {
                assert!(message.starts_with("validation errors:"));
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn add_task_persists_new_record_on_success() {
        let mut repo = TaskWriterUserRepo::new();
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: "alpha".to_string(),
            message: None,
            track: None,
            priority: "Middle".to_string(),
            assignee: assignee_selection_form_none(),
        };

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(7, expected_hub, &expected_email_lower, &expected_name);
        let expected_author_id = author.id;
        let hub_for_return = expected_hub;

        let expected_hub_id = HubId::new(expected_hub).unwrap();
        let expected_email_for_create = UserEmail::new(expected_email_lower.clone()).unwrap();
        let expected_name_for_create = UserName::new(expected_name.clone()).unwrap();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub_id);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, expected_author_id);
                assert_eq!(hub_id, expected_hub_id);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(1)
            .withf(move |task| {
                assert_eq!(task.hub_id, expected_hub_id);
                assert_eq!(task.title.as_str(), "alpha");
                assert_eq!(task.description, None);
                assert_eq!(task.status, TaskStatus::Pending);
                assert!(task.due_date.is_none());
                assert!(task.assigned_to.is_none());
                assert_eq!(task.author_id, expected_author_id);
                true
            })
            .returning(move |_| Ok(sample_task(1, hub_for_return, "alpha")));

        let result = add_task(form, &user, &repo, &zmq, &zmq);

        let created = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(created.hub_id, HubId::new(expected_hub).unwrap());
        assert_eq!(created.title.as_str(), "alpha");
    }

    #[test]
    fn add_task_assigns_to_selected_user() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let assignee_email = "assignee@example.com".to_string();
        let assignee_name = "Assigned User".to_string();
        let form = AddTaskForm {
            title: "alpha".to_string(),
            message: None,
            track: None,
            priority: "Middle".to_string(),
            assignee: AssigneeSelectionForm {
                email: Some(assignee_email.clone()),
                name: Some(assignee_name.clone()),
            },
        };

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(7, expected_hub, &expected_email_lower, &expected_name);
        let expected_author_id = author.id;
        let hub_for_return = expected_hub;

        let assignee_record = sample_user_record(11, expected_hub, &assignee_email, &assignee_name);
        let expected_assignee_id = assignee_record.id;
        let expected_hub_id = HubId::new(expected_hub).unwrap();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(2)
            .returning({
                let expected_author_email = expected_email_lower.clone();
                let expected_author_name = expected_name.clone();
                let author_for_return = author.clone();
                let expected_assignee_email = assignee_email.clone();
                let expected_assignee_name = assignee_name.clone();
                let assignee_for_return = assignee_record.clone();
                move |new_user| {
                    assert_eq!(new_user.hub_id, expected_hub_id);

                    if new_user.email.as_str() == expected_author_email {
                        assert_eq!(new_user.name.as_str(), expected_author_name);
                        Ok(author_for_return.clone())
                    } else if new_user.email.as_str() == expected_assignee_email {
                        assert_eq!(new_user.name.as_str(), expected_assignee_name);
                        Ok(assignee_for_return.clone())
                    } else {
                        panic!(
                            "unexpected user payload received: {} / {}",
                            new_user.email.as_str(),
                            new_user.name.as_str()
                        );
                    }
                }
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, expected_author_id);
                assert_eq!(hub_id, expected_hub_id);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(1)
            .withf({
                let expected_hub_id = HubId::new(expected_hub).unwrap();
                let expected_author_id_capture = expected_author_id;
                let expected_assignee_id_capture = expected_assignee_id;
                move |task| {
                    assert_eq!(task.hub_id, expected_hub_id);
                    assert_eq!(task.title.as_str(), "alpha");
                    assert_eq!(task.description, None);
                    assert_eq!(task.status, TaskStatus::Pending);
                    assert!(task.due_date.is_none());
                    assert_eq!(task.assigned_to, Some(expected_assignee_id_capture));
                    assert_eq!(task.author_id, expected_author_id_capture);
                    true
                }
            })
            .returning(move |_| {
                let mut task = sample_task(1, hub_for_return, "alpha");
                task.assigned_to = Some(expected_assignee_id);
                Ok(task)
            });

        let zmq = MockZmqSender {};
        let result = add_task(form, &user, &repo, &zmq, &zmq);

        let created = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(created.hub_id, HubId::new(expected_hub).unwrap());
        assert_eq!(created.assigned_to, Some(expected_assignee_id));
    }

    #[test]
    fn add_task_notifies_assignee_via_email() {
        let mut repo = TaskWriterUserRepo::new();
        let zmq_email = RecordingZmqSender::default();
        let zmq_tasks = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let assignee_email = "assignee@example.com".to_string();
        let assignee_name = "Assigned User".to_string();

        let form = AddTaskForm {
            title: "New Task".to_string(),
            message: Some("Task details".to_string()),
            track: None,
            priority: "Middle".to_string(),
            assignee: AssigneeSelectionForm {
                email: Some(assignee_email.clone()),
                name: Some(assignee_name.clone()),
            },
        };

        let expected_hub = user.hub_id;
        let author_record =
            sample_user_record(5, expected_hub, &user.email.to_lowercase(), &user.name);
        let assignee_record = sample_user_record(9, expected_hub, &assignee_email, &assignee_name);

        repo.user_reader.expect_get_user_by_email().never();

        let mut seq = Sequence::new();
        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .in_sequence(&mut seq)
            .return_once({
                let author_record = author_record.clone();
                move |new_user| {
                    assert_eq!(new_user.email, author_record.email);
                    Ok(author_record.clone())
                }
            });
        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .in_sequence(&mut seq)
            .return_once({
                let assignee_record = assignee_record.clone();
                move |new_user| {
                    assert_eq!(new_user.email, assignee_record.email);
                    Ok(assignee_record.clone())
                }
            });
        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning({
                let author_record = author_record.clone();
                move |user_id, hub_id| {
                    assert_eq!(user_id, author_record.id);
                    assert_eq!(hub_id, author_record.hub_id);
                    Ok(())
                }
            });

        let created_task = Task {
            id: TaskId::new(51).unwrap(),
            hub_id: HubId::new(expected_hub).unwrap(),
            title: TaskTitle::new("New Task").unwrap(),
            description: Some(TaskDescription::new("Task details").unwrap()),
            track: None,
            priority: TaskPriority::Middle,
            status: TaskStatus::Pending,
            due_date: None,
            assigned_to: Some(assignee_record.id),
            author_id: author_record.id,
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            completed_at: None,
            client_id: None,
            public_id: None,
        };

        repo.task_writer.expect_create_task().times(1).returning({
            let assignee_record = assignee_record.clone();
            let created_task = created_task.clone();
            move |new_task| {
                assert_eq!(new_task.hub_id, created_task.hub_id);
                assert_eq!(new_task.title, created_task.title);
                assert_eq!(new_task.description, created_task.description);
                assert_eq!(new_task.assigned_to, Some(assignee_record.id));
                Ok(created_task.clone())
            }
        });

        let outcome =
            add_task(form, &user, &repo, &zmq_email, &zmq_tasks).expect("should create task");

        assert_eq!(outcome.id, created_task.id);

        let payloads = zmq_email.messages();
        assert_eq!(payloads.len(), 1);

        let envelope: ZMQSendEmailMessage =
            serde_json::from_slice(&payloads[0]).expect("valid email payload");

        match envelope {
            ZMQSendEmailMessage::NewEmail(message) => {
                let (actor, email) = *message;
                assert_eq!(actor.email, user.email);
                assert_eq!(
                    email.hub_id,
                    pushkind_emailer::domain::types::HubId::new(user.hub_id).unwrap()
                );
                assert_eq!(email.recipients.len(), 1);

                let recipient = &email.recipients[0];
                assert_eq!(recipient.address.as_str(), assignee_record.email.as_str());
                assert_eq!(recipient.name.as_str(), assignee_record.name.as_str());
                assert_eq!(
                    recipient
                        .fields
                        .get("notification_kind")
                        .map(String::as_str),
                    Some("task_created"),
                );
                let expected_task_id = created_task.id.to_string();
                assert_eq!(
                    recipient.fields.get("task_id").map(String::as_str),
                    Some(expected_task_id.as_str()),
                );
                assert_eq!(
                    email.subject.as_ref().map(|subject| subject.as_str()),
                    Some("Вам назначена задача: New Task"),
                );
                assert!(email.message.as_str().contains("New Task"));
            }
            _ => panic!("unexpected email payload variant"),
        }
    }

    #[test]
    fn add_task_creates_author_when_missing() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: "alpha".to_string(),
            message: None,
            track: None,
            priority: "Middle".to_string(),
            assignee: assignee_selection_form_none(),
        };

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let expected_email_for_create = UserEmail::new(expected_email_lower.clone()).unwrap();
        let expected_name_for_create = UserName::new(expected_name.clone()).unwrap();
        let created_author =
            sample_user_record(13, expected_hub, &expected_email_lower, &expected_name);
        let author_id = created_author.id;
        let expected_hub_id = HubId::new(expected_hub).unwrap();
        let hub_for_return = expected_hub;
        let created_author_for_create = created_author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub_id);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(created_author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, author_id);
                assert_eq!(hub_id, expected_hub_id);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(1)
            .withf(move |task| {
                assert_eq!(task.hub_id, expected_hub_id);
                assert_eq!(task.author_id, author_id);
                true
            })
            .returning(move |_| Ok(sample_task(1, hub_for_return, "alpha")));

        let zmq = MockZmqSender {};
        let result = add_task(form, &user, &repo, &zmq, &zmq);

        assert!(result.is_ok(), "expected task creation to succeed");
    }

    #[test]
    fn add_task_propagates_repository_errors() {
        let mut repo = TaskWriterUserRepo::new();
        let zmq = MockZmqSender {};
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: "alpha".to_string(),
            message: None,
            track: None,
            priority: "Middle".to_string(),
            assignee: assignee_selection_form_none(),
        };

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(8, expected_hub, &expected_email_lower, &expected_name);
        let expected_author_id = author.id;

        let expected_hub_id = HubId::new(expected_hub).unwrap();
        let expected_email_for_create = UserEmail::new(expected_email_lower.clone()).unwrap();
        let expected_name_for_create = UserName::new(expected_name.clone()).unwrap();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub_id);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer.expect_touch_visited_at().never();

        repo.task_writer
            .expect_create_task()
            .times(1)
            .withf(move |task| {
                assert_eq!(task.hub_id, expected_hub_id);
                assert_eq!(task.author_id, expected_author_id);
                true
            })
            .returning(|_| Err(RepositoryError::Unexpected("db write failed".to_string())));

        let result = add_task(form, &user, &repo, &zmq, &zmq);

        match result {
            Err(ServiceError::Repository(RepositoryError::Unexpected(message))) => {
                assert_eq!(message, "db write failed");
            }
            other => panic!("expected repository error, got {other:?}"),
        }
    }

    #[test]
    fn upload_tasks_returns_unauthorized_when_role_missing() {
        let repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[]);
        let form = upload_form(
            "title
alpha
",
        );

        let result = upload_tasks(form, &user, &repo);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn upload_tasks_returns_form_error_when_parse_fails() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = upload_form(
            "title
foo,bar
",
        );

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(9, expected_hub, &expected_email_lower, &expected_name);

        let expected_hub_id = HubId::new(expected_hub).unwrap();
        let expected_email_for_create = UserEmail::new(expected_email_lower.clone()).unwrap();
        let expected_name_for_create = UserName::new(expected_name.clone()).unwrap();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub_id);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer.expect_touch_visited_at().never();

        repo.task_writer.expect_create_task().never();

        let result = upload_tasks(form, &user, &repo);

        match result {
            Err(ServiceError::Form(message)) => {
                assert_eq!(message, "Ошибка при парсинге задач");
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn upload_tasks_persists_uploaded_records() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = upload_form(
            "title,description
alpha,
beta,
",
        );

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(10, expected_hub, &expected_email_lower, &expected_name);
        let expected_author_id = author.id;
        let captured_titles = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let titles_for_closure = std::sync::Arc::clone(&captured_titles);
        let hub_for_return = expected_hub;

        let expected_hub_id = HubId::new(expected_hub).unwrap();
        let expected_email_for_create = UserEmail::new(expected_email_lower.clone()).unwrap();
        let expected_name_for_create = UserName::new(expected_name.clone()).unwrap();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub_id);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, expected_author_id);
                assert_eq!(hub_id, expected_hub_id);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(2)
            .returning(move |task| {
                assert_eq!(task.hub_id, expected_hub_id);
                assert!(task.description.is_none());
                assert!(task.due_date.is_none());
                assert!(task.assigned_to.is_none());
                assert_eq!(task.author_id, expected_author_id);

                let mut titles = match titles_for_closure.lock() {
                    Ok(guard) => guard,
                    Err(err) => panic!("failed to lock titles mutex: {err}"),
                };

                titles.push(task.title.clone());
                let task_id = titles.len() as i32;

                Ok(sample_task(task_id, hub_for_return, task.title.as_str()))
            });

        let result = upload_tasks(form, &user, &repo);

        let created_count = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(created_count, 2);

        let titles = match captured_titles.lock() {
            Ok(guard) => guard.clone(),
            Err(err) => panic!("failed to read captured titles: {err}"),
        };

        assert_eq!(titles.len(), 2);
        assert!(titles.iter().any(|title| title.as_str() == "alpha"));
        assert!(titles.iter().any(|title| title.as_str() == "beta"));
    }

    #[test]
    fn upload_tasks_creates_author_when_missing() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = upload_form(
            "title,description
alpha,
",
        );

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let expected_email_for_create = UserEmail::new(expected_email_lower.clone()).unwrap();
        let expected_name_for_create = UserName::new(expected_name.clone()).unwrap();
        let created_author =
            sample_user_record(21, expected_hub, &expected_email_lower, &expected_name);
        let author_id = created_author.id;
        let hub_for_return = expected_hub;
        let expected_hub_id = HubId::new(expected_hub).unwrap();
        let created_author_for_create = created_author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub_id);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(created_author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, author_id);
                assert_eq!(hub_id, expected_hub_id);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(1)
            .returning(move |task| {
                assert_eq!(task.hub_id, expected_hub_id);
                assert_eq!(task.author_id, author_id);
                Ok(sample_task(1, hub_for_return, task.title.as_str()))
            });

        let result = upload_tasks(form, &user, &repo);

        assert!(
            result.is_ok(),
            "expected upload to succeed when author is created"
        );
    }
}
