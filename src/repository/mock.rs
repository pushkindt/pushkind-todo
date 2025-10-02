use mockall::mock;

use super::{
    TaskListQuery, TaskReader, TaskWriter, TemplateListQuery, TemplateReader, TemplateWriter,
    UserListQuery, UserReader, UserWriter,
};
use crate::domain::{
    task::{NewTask, Task, TaskAssignment, UpdateTask},
    template::{NewTemplate, Template, UpdateTemplate},
    user::{NewUser, UpdateUser, User},
};
use pushkind_common::repository::errors::RepositoryResult;

mock! {
    pub TemplateReader {}

    impl TemplateReader for TemplateReader {
        fn get_template_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Template>>;
        fn list_templates(&self, query: TemplateListQuery) -> RepositoryResult<(usize, Vec<Template>)>;
    }
}

mock! {
    pub TemplateWriter {}

    impl TemplateWriter for TemplateWriter {
        fn create_templates(&self, new_templates: &[NewTemplate]) -> RepositoryResult<usize>;
        fn update_template(&self, template_id: i32, hub_id: i32, updates: &UpdateTemplate) -> RepositoryResult<Template>;
        fn delete_template(&self, template_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}

mock! {
    pub UserReader {}

    impl UserReader for UserReader {
        fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>>;
        fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<User>>;
        fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)>;
    }
}

mock! {
    pub UserWriter {}

    impl UserWriter for UserWriter {
        fn create_user(&self, new_user: &NewUser) -> RepositoryResult<User>;
        fn update_user(&self, user_id: i32, hub_id: i32, updates: &UpdateUser) -> RepositoryResult<User>;
        fn delete_user(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}

mock! {
    pub TaskReader {}

    impl TaskReader for TaskReader {
        fn get_task_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Task>>;
        fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)>;
        fn list_assignments_for_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<Vec<TaskAssignment>>;
    }
}

mock! {
    pub TaskWriter {}

    impl TaskWriter for TaskWriter {
        fn create_task(&self, new_task: &NewTask) -> RepositoryResult<Task>;
        fn update_task(&self, task_id: i32, hub_id: i32, updates: &UpdateTask) -> RepositoryResult<Task>;
        fn delete_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<()>;
        fn record_assignment(&self, assignment: &TaskAssignment) -> RepositoryResult<()>;
        fn remove_assignment(&self, task_id: i32, hub_id: i32, assignee_id: i32) -> RepositoryResult<()>;
    }
}
