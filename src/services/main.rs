use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::Deserialize;
use validator::Validate;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::task::Task;
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
}

/// Data required to render the main index tasks page.
pub struct IndexPageData {
    /// Paginated list of tasks to show in the table.
    pub tasks: Paginated<Task>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}

/// Loads the tasks list for the main index page.
pub fn load_index_page<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: IndexQuery,
) -> ServiceResult<IndexPageData>
where
    R: TaskReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let page = query.page.unwrap_or(1);
    let mut list_query = TaskListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let Some(value) = query.search.as_ref()
        && !value.trim().is_empty()
    {
        list_query.filters_mut().search = Some(value.clone());
    }

    let (total, tasks) = repo.list_tasks(list_query).map_err(ServiceError::from)?;

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let tasks = Paginated::new(tasks, page, total_pages);

    Ok(IndexPageData {
        tasks,
        search: query.search,
    })
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

    Ok(RedirectSuccess {
        message: "Задачи добавлены.".to_string(),
        redirect_to: "/".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_multipart::form::tempfile::TempFile;
    use chrono::{NaiveDate, NaiveDateTime};
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
        let repo = MockTaskReader::new();
        let user = user_with_roles(&[]);

        let result = load_index_page(&repo, &user, IndexQuery::default());

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_index_page_returns_paginated_data() {
        let mut repo = MockTaskReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = IndexQuery {
            search: Some("alp".to_string()),
            page: Some(2),
        };

        let expected_hub = user.hub_id;
        let hub_for_assert = expected_hub;
        let hub_for_return = expected_hub;

        repo.expect_list_tasks()
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
