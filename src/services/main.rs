use chrono::{NaiveDate, NaiveDateTime};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::Deserialize;
use validator::Validate;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::task::{Task, TaskStatus};
use crate::forms::main::{AddTaskForm, UploadTasksForm};
use crate::repository::{TaskListQuery, TaskReader, TaskWriter, UserReader, UserWriter};
use crate::services::{RedirectSuccess, ServiceError, ServiceResult};

/// Query parameters accepted by the index page service.
#[derive(Debug, Default, Deserialize)]
pub struct IndexQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
    /// Optional status filter provided by the user.
    pub status: Option<String>,
    /// Only return tasks updated on or after this date (YYYY-MM-DD).
    pub updated_after: Option<String>,
    /// Only return tasks updated on or before this date (YYYY-MM-DD).
    pub updated_before: Option<String>,
}

/// Data required to render the main index tasks page.
pub struct IndexPageData {
    /// Paginated list of tasks to show in the table.
    pub tasks: Paginated<Task>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
    /// Status filter echoed back to the template when present.
    pub status: Option<String>,
    /// Updated-after filter echoed back to the template when present.
    pub updated_after: Option<String>,
    /// Updated-before filter echoed back to the template when present.
    pub updated_before: Option<String>,
    /// Task identifiers that were updated after the user's last visit.
    pub recently_updated_task_ids: Vec<i32>,
}

/// Loads the tasks list for the main index page.
pub fn load_index_page<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: IndexQuery,
) -> ServiceResult<IndexPageData>
where
    R: TaskReader + UserReader + UserWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let page = query.page.unwrap_or(1);
    let mut list_query = TaskListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    let mut status_filter_text = None;
    if let Some(status_value) = query.status.as_deref().and_then(parse_status_filter) {
        list_query.filters_mut().status = Some(status_value);
        status_filter_text = Some((<&str>::from(status_value)).to_string());
    }

    let mut updated_after_text = None;
    if let Some(updated_after_value) = query.updated_after.as_deref().and_then(parse_date_filter)
        && let Some(timestamp) = start_of_day(updated_after_value)
    {
        list_query.filters_mut().updated_after = Some(timestamp);
        updated_after_text = Some(updated_after_value.format("%Y-%m-%d").to_string());
    }

    let mut updated_before_text = None;
    if let Some(updated_before_value) = query.updated_before.as_deref().and_then(parse_date_filter)
        && let Some(timestamp) = end_of_day(updated_before_value)
    {
        list_query.filters_mut().updated_before = Some(timestamp);
        updated_before_text = Some(updated_before_value.format("%Y-%m-%d").to_string());
    }

    if let Some(value) = query.search.as_ref()
        && !value.trim().is_empty()
    {
        list_query.filters_mut().search = Some(value.clone());
    }

    let new_user = user.into();
    let user = repo.create_or_update_user(&new_user)?;
    let visited_at = user.visited_at;

    repo.touch_visited_at(user.id, user.hub_id)?;

    let (total, tasks) = repo.list_tasks(list_query).map_err(ServiceError::from)?;

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
    let tasks = Paginated::new(tasks, page, total_pages);

    Ok(IndexPageData {
        tasks,
        search: query.search,
        status: status_filter_text,
        updated_after: updated_after_text,
        updated_before: updated_before_text,
        recently_updated_task_ids,
    })
}

fn parse_status_filter(input: &str) -> Option<TaskStatus> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    match trimmed {
        "Pending" => Some(TaskStatus::Pending),
        "InProgress" => Some(TaskStatus::InProgress),
        "Blocked" => Some(TaskStatus::Blocked),
        "Completed" => Some(TaskStatus::Completed),
        "Archived" => Some(TaskStatus::Archived),
        _ => None,
    }
}

fn parse_date_filter(input: &str) -> Option<NaiveDate> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok()
}

fn start_of_day(date: NaiveDate) -> Option<NaiveDateTime> {
    date.and_hms_opt(0, 0, 0)
}

fn end_of_day(date: NaiveDate) -> Option<NaiveDateTime> {
    date.and_hms_micro_opt(23, 59, 59, 999_999)
        .or_else(|| date.and_hms_opt(23, 59, 59))
}

/// Validates the add-task form and persists a new task record.
pub fn add_task<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddTaskForm,
) -> ServiceResult<RedirectSuccess>
where
    R: TaskWriter + UserReader + UserWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if let Err(err) = form.validate() {
        log::error!("Failed to validate form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let new_user = user.into();
    let author = repo.create_or_update_user(&new_user)?;

    let new_task = match form.into_new_task(user.hub_id, author.id) {
        Some(task) => task,
        None => {
            log::error!("Validated task form missing title value");
            return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
        }
    };

    repo.create_task(&new_task).map_err(|err| {
        log::error!("Failed to add a task: {err}");
        err
    })?;

    repo.touch_visited_at(author.id, author.hub_id)?;

    Ok(RedirectSuccess {
        message: "Задача добавлена.".to_string(),
        redirect_to: "/".to_string(),
    })
}

/// Parses the uploaded CSV file and creates task records in bulk.
pub fn upload_tasks<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: &mut UploadTasksForm,
) -> ServiceResult<RedirectSuccess>
where
    R: TaskWriter + UserReader + UserWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let new_user = user.into();
    let author = repo.create_or_update_user(&new_user)?;

    let new_tasks = form.parse(user.hub_id, author.id).map_err(|err| {
        log::error!("Failed to parse tasks: {err}");
        ServiceError::Form("Ошибка при парсинге задач".to_string())
    })?;

    for new_task in new_tasks {
        repo.create_task(&new_task).map_err(|err| {
            log::error!("Failed to add a task: {err}");
            err
        })?;
    }

    repo.touch_visited_at(author.id, author.hub_id)?;

    Ok(RedirectSuccess {
        message: "Задачи добавлены.".to_string(),
        redirect_to: "/".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_multipart::form::tempfile::TempFile;
    use chrono::{Duration, NaiveDate, NaiveDateTime};
    use pushkind_common::repository::errors::RepositoryError;
    use serde_json::Value;
    use tempfile::NamedTempFile;

    use crate::SERVICE_ACCESS_ROLE;
    use crate::domain::task::{
        NewTask as DomainNewTask, Task, TaskAssignment as DomainTaskAssignment, TaskStatus,
        UpdateTask as DomainUpdateTask,
    };
    use crate::domain::user::User;
    use crate::repository::mock::{MockTaskReader, MockTaskWriter, MockUserReader, MockUserWriter};
    use crate::repository::{TaskWriter, UserListQuery, UserReader, UserWriter};

    use std::io::Write;

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_task(id: i32, hub_id: i32, title: &str) -> Task {
        Task {
            id,
            hub_id,
            title: title.to_string(),
            description: None,
            status: TaskStatus::Pending,
            due_date: None,
            assigned_to: None,
            author_id: 1,
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
            completed_at: None,
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
            id,
            hub_id,
            name: name.to_string(),
            email: email.to_string(),
            visited_at: Some(fixed_datetime()),
        }
    }

    struct TaskReaderUserRepo {
        pub task_reader: MockTaskReader,
        pub user_reader: MockUserReader,
        pub user_writer: MockUserWriter,
    }

    impl TaskReaderUserRepo {
        fn new() -> Self {
            Self {
                task_reader: MockTaskReader::new(),
                user_reader: MockUserReader::new(),
                user_writer: MockUserWriter::new(),
            }
        }
    }

    impl TaskReader for TaskReaderUserRepo {
        fn get_task_by_id(
            &self,
            id: i32,
            hub_id: i32,
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
            task_id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<Vec<DomainTaskAssignment>>
        {
            self.task_reader.list_assignments_for_task(task_id, hub_id)
        }
    }

    impl UserReader for TaskReaderUserRepo {
        fn get_user_by_id(
            &self,
            id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(id, hub_id)
        }

        fn get_user_by_email(
            &self,
            email: &str,
            hub_id: i32,
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

    impl UserWriter for TaskReaderUserRepo {
        fn create_or_update_user(
            &self,
            new_user: &crate::domain::user::NewUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.create_or_update_user(new_user)
        }

        fn update_user(
            &self,
            user_id: i32,
            hub_id: i32,
            updates: &crate::domain::user::UpdateUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.update_user(user_id, hub_id, updates)
        }

        fn delete_user(
            &self,
            user_id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.user_writer.delete_user(user_id, hub_id)
        }

        fn touch_visited_at(
            &self,
            user_id: i32,
            hub_id: i32,
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
            task_id: i32,
            hub_id: i32,
            updates: &DomainUpdateTask,
        ) -> pushkind_common::repository::errors::RepositoryResult<Task> {
            self.task_writer.update_task(task_id, hub_id, updates)
        }

        fn delete_task(
            &self,
            task_id: i32,
            hub_id: i32,
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
            task_id: i32,
            hub_id: i32,
            assignee_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.task_writer
                .remove_assignment(task_id, hub_id, assignee_id)
        }
    }

    impl UserReader for TaskWriterUserRepo {
        fn get_user_by_id(
            &self,
            id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<Option<User>> {
            self.user_reader.get_user_by_id(id, hub_id)
        }

        fn get_user_by_email(
            &self,
            email: &str,
            hub_id: i32,
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
            new_user: &crate::domain::user::NewUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.create_or_update_user(new_user)
        }

        fn update_user(
            &self,
            user_id: i32,
            hub_id: i32,
            updates: &crate::domain::user::UpdateUser,
        ) -> pushkind_common::repository::errors::RepositoryResult<User> {
            self.user_writer.update_user(user_id, hub_id, updates)
        }

        fn delete_user(
            &self,
            user_id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.user_writer.delete_user(user_id, hub_id)
        }

        fn touch_visited_at(
            &self,
            user_id: i32,
            hub_id: i32,
        ) -> pushkind_common::repository::errors::RepositoryResult<()> {
            self.user_writer.touch_visited_at(user_id, hub_id)
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

        let result = load_index_page(&repo, &user, IndexQuery::default());

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

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning({
                let expected_hub_id = expected_hub;
                let expected_email = expected_email.clone();
                let expected_name = expected_name.clone();
                let expected_user = expected_user.clone();
                move |new_user| {
                    assert_eq!(new_user.hub_id, expected_hub_id);
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

        repo.task_reader
            .expect_list_tasks()
            .times(1)
            .withf(move |query| {
                assert_eq!(query.filters.hub_id, hub_for_assert);
                assert_eq!(query.filters.search.as_deref(), Some("alp"));
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

        let result = load_index_page(&repo, &user, query);

        let data = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(data.search.as_deref(), Some("alp"));
        assert!(data.recently_updated_task_ids.is_empty());

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
            .and_then(|map| map.get("title"))
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
            updated_after: Some("2024-05-01".to_string()),
            updated_before: Some("2024-05-31".to_string()),
        };

        let expected_email = user.email.clone();
        let expected_hub_id = user.hub_id;
        let expected_name = user.name.clone();
        let expected_user = sample_user_record(7, expected_hub_id, &expected_email, &expected_name);

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

                assert_eq!(filters.hub_id, expected_hub_id);
                assert_eq!(filters.search.as_deref(), Some("project"));
                assert_eq!(filters.status, Some(TaskStatus::Completed));
                assert_eq!(filters.updated_after, Some(expected_after_ts));
                assert_eq!(filters.updated_before, Some(expected_before_ts));

                match pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 1);
                        assert_eq!(pagination.per_page, DEFAULT_ITEMS_PER_PAGE);
                    }
                    None => panic!("expected pagination to be provided"),
                }

                Ok((0, Vec::new()))
            });

        let result = load_index_page(&repo, &user, query).expect("expected success");
        assert_eq!(result.status.as_deref(), Some("Completed"));
        assert_eq!(result.updated_after.as_deref(), Some("2024-05-01"));
        assert_eq!(result.updated_before.as_deref(), Some("2024-05-31"));
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
        let expected_email = user.email.clone();
        let expected_hub_id = user.hub_id;
        let expected_name = user.name.clone();
        let user_record = sample_user_record(42, expected_hub_id, &expected_email, &expected_name);

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

        let fresh_task_id = 2;
        let hub_id_for_tasks = user.hub_id;

        repo.task_reader.expect_list_tasks().times(1).returning({
            let visited_at_for_tasks = visited_at;
            move |_| {
                let mut stale_task = sample_task(1, hub_id_for_tasks, "stale");
                stale_task.updated_at = visited_at_for_tasks;

                let mut fresh_task = sample_task(fresh_task_id, hub_id_for_tasks, "fresh");
                fresh_task.updated_at = visited_at_for_tasks + Duration::hours(1);

                Ok((2, vec![stale_task, fresh_task]))
            }
        });

        let result = load_index_page(&repo, &user, query).expect("expected success");

        assert_eq!(result.recently_updated_task_ids, vec![fresh_task_id]);
    }

    #[test]
    fn add_task_returns_unauthorized_when_role_missing() {
        let repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[]);
        let form = AddTaskForm {
            title: Some("alpha".to_string()),
            message: None,
        };

        let result = add_task(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn add_task_returns_form_error_on_validation_failure() {
        let repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: Some(String::new()),
            message: None,
        };

        let result = add_task(&repo, &user, form);

        match result {
            Err(ServiceError::Form(message)) => {
                assert_eq!(message, "Ошибка валидации формы");
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn add_task_persists_new_record_on_success() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: Some("alpha".to_string()),
            message: None,
        };

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(7, expected_hub, &expected_email_lower, &expected_name);
        let expected_author_id = author.id;
        let hub_for_return = expected_hub;

        let expected_email_for_create = expected_email_lower.clone();
        let expected_name_for_create = expected_name.clone();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, expected_author_id);
                assert_eq!(hub_id, expected_hub);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(1)
            .withf(move |task| {
                assert_eq!(task.hub_id, expected_hub);
                assert_eq!(task.title, "alpha");
                assert_eq!(task.description, None);
                assert_eq!(task.status, TaskStatus::Pending);
                assert!(task.due_date.is_none());
                assert!(task.assigned_to.is_none());
                assert_eq!(task.author_id, expected_author_id);
                true
            })
            .returning(move |_| Ok(sample_task(1, hub_for_return, "alpha")));

        let result = add_task(&repo, &user, form);

        let redirect = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(redirect.message, "Задача добавлена.");
        assert_eq!(redirect.redirect_to, "/");
    }

    #[test]
    fn add_task_creates_author_when_missing() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: Some("alpha".to_string()),
            message: None,
        };

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_email_for_create = expected_email_lower.clone();
        let expected_name = user.name.clone();
        let expected_name_for_create = expected_name.clone();
        let created_author =
            sample_user_record(13, expected_hub, &expected_email_lower, &expected_name);
        let author_id = created_author.id;
        let hub_for_return = expected_hub;
        let created_author_for_create = created_author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(created_author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, author_id);
                assert_eq!(hub_id, expected_hub);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(1)
            .withf(move |task| {
                assert_eq!(task.hub_id, expected_hub);
                assert_eq!(task.author_id, author_id);
                true
            })
            .returning(move |_| Ok(sample_task(1, hub_for_return, "alpha")));

        let result = add_task(&repo, &user, form);

        assert!(result.is_ok(), "expected task creation to succeed");
    }

    #[test]
    fn add_task_propagates_repository_errors() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTaskForm {
            title: Some("alpha".to_string()),
            message: None,
        };

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(8, expected_hub, &expected_email_lower, &expected_name);
        let expected_author_id = author.id;

        let expected_email_for_create = expected_email_lower.clone();
        let expected_name_for_create = expected_name.clone();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer.expect_touch_visited_at().never();

        repo.task_writer
            .expect_create_task()
            .times(1)
            .withf(move |task| {
                assert_eq!(task.hub_id, expected_hub);
                assert_eq!(task.author_id, expected_author_id);
                true
            })
            .returning(|_| Err(RepositoryError::Unexpected("db write failed".to_string())));

        let result = add_task(&repo, &user, form);

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
        let mut form = upload_form(
            "title
alpha
",
        );

        let result = upload_tasks(&repo, &user, &mut form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn upload_tasks_returns_form_error_when_parse_fails() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let mut form = upload_form(
            "title
foo,bar
",
        );

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_name = user.name.clone();
        let author = sample_user_record(9, expected_hub, &expected_email_lower, &expected_name);

        let expected_email_for_create = expected_email_lower.clone();
        let expected_name_for_create = expected_name.clone();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer.expect_touch_visited_at().never();

        repo.task_writer.expect_create_task().never();

        let result = upload_tasks(&repo, &user, &mut form);

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
        let mut form = upload_form(
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

        let expected_email_for_create = expected_email_lower.clone();
        let expected_name_for_create = expected_name.clone();
        let author_for_create = author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, expected_author_id);
                assert_eq!(hub_id, expected_hub);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(2)
            .returning(move |task| {
                assert_eq!(task.hub_id, hub_for_return);
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

                Ok(sample_task(task_id, hub_for_return, &task.title))
            });

        let result = upload_tasks(&repo, &user, &mut form);

        let redirect = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(redirect.message, "Задачи добавлены.");
        assert_eq!(redirect.redirect_to, "/");

        let titles = match captured_titles.lock() {
            Ok(guard) => guard.clone(),
            Err(err) => panic!("failed to read captured titles: {err}"),
        };

        assert_eq!(titles.len(), 2);
        assert!(titles.iter().any(|title| title == "alpha"));
        assert!(titles.iter().any(|title| title == "beta"));
    }

    #[test]
    fn upload_tasks_creates_author_when_missing() {
        let mut repo = TaskWriterUserRepo::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let mut form = upload_form(
            "title,description
alpha,
",
        );

        let expected_hub = user.hub_id;
        let expected_email_lower = user.email.to_lowercase();
        let expected_email_for_create = expected_email_lower.clone();
        let expected_name = user.name.clone();
        let expected_name_for_create = expected_name.clone();
        let created_author =
            sample_user_record(21, expected_hub, &expected_email_lower, &expected_name);
        let author_id = created_author.id;
        let hub_for_return = expected_hub;
        let created_author_for_create = created_author.clone();

        repo.user_reader.expect_get_user_by_email().never();

        repo.user_writer
            .expect_create_or_update_user()
            .times(1)
            .returning(move |new_user| {
                assert_eq!(new_user.hub_id, expected_hub);
                assert_eq!(new_user.name, expected_name_for_create);
                assert_eq!(new_user.email, expected_email_for_create);
                Ok(created_author_for_create.clone())
            });

        repo.user_writer
            .expect_touch_visited_at()
            .times(1)
            .returning(move |user_id, hub_id| {
                assert_eq!(user_id, author_id);
                assert_eq!(hub_id, expected_hub);
                Ok(())
            });

        repo.task_writer
            .expect_create_task()
            .times(1)
            .returning(move |task| {
                assert_eq!(task.hub_id, hub_for_return);
                assert_eq!(task.author_id, author_id);
                Ok(sample_task(1, hub_for_return, &task.title))
            });

        let result = upload_tasks(&repo, &user, &mut form);

        assert!(
            result.is_ok(),
            "expected upload to succeed when author is created"
        );
    }
}
