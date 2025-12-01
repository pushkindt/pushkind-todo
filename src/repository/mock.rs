//! Mock repository implementations generated with `mockall` for service tests.
use mockall::mock;

use super::{
    TaskEventReader, TaskEventWriter, TaskListQuery, TaskReader, TaskWriter, UserListQuery,
    UserReader, UserWriter,
};
use crate::domain::{
    task::{NewTask, Task, TaskAssignment, UpdateTask},
    task_event::{NewTaskEvent, TaskEvent},
    user::{NewUser, UpdateUser, User},
};
use pushkind_common::repository::errors::RepositoryResult;

mock! {
    pub UserReader {}

    impl UserReader for UserReader {
        /// Mocked lookup by id within the hub.
        fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>>;
        /// Mocked lookup by email within the hub.
        fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<User>>;
        /// Mocked listing of users.
        fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)>;
    }
}

mock! {
    pub UserWriter {}

    impl UserWriter for UserWriter {
        /// Mocked upsert that returns a user.
        fn create_or_update_user(&self, new_user: &NewUser) -> RepositoryResult<User>;
        /// Mocked user updates.
        fn update_user(&self, user_id: i32, hub_id: i32, updates: &UpdateUser) -> RepositoryResult<User>;
        /// Mocked user deletion.
        fn delete_user(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()>;
        /// Mocked visited-at touch.
        fn touch_visited_at(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}

mock! {
    pub TaskReader {}

    impl TaskReader for TaskReader {
        /// Mocked fetch by task id.
        fn get_task_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Task>>;
        /// Mocked task listing.
        fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)>;
        /// Mocked assignment history retrieval.
        fn list_assignments_for_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<Vec<TaskAssignment>>;
        /// Mocked track listing.
        fn list_task_tracks(&self, hub_id: i32) -> RepositoryResult<Vec<String>>;
    }
}

mock! {
    pub TaskWriter {}

    impl TaskWriter for TaskWriter {
        /// Mocked task creation.
        fn create_task(&self, new_task: &NewTask) -> RepositoryResult<Task>;
        /// Mocked task updates.
        fn update_task(&self, task_id: i32, hub_id: i32, updates: &UpdateTask) -> RepositoryResult<Task>;
        /// Mocked task deletion.
        fn delete_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<()>;
        /// Mocked assignment recording.
        fn record_assignment(&self, assignment: &TaskAssignment) -> RepositoryResult<()>;
        /// Mocked assignment removal.
        fn remove_assignment(&self, task_id: i32, hub_id: i32, assignee_id: i32) -> RepositoryResult<()>;
    }
}

mock! {
    pub TaskEventReader {}

    impl TaskEventReader for TaskEventReader {
        /// Mocked event listing.
        fn list_events_for_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<Vec<TaskEvent>>;
        /// Mocked event lookup by id.
        fn get_event_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<TaskEvent>>;
    }
}

mock! {
    pub TaskEventWriter {}

    impl TaskEventWriter for TaskEventWriter {
        /// Mocked event recording.
        fn record_event(&self, event: &NewTaskEvent) -> RepositoryResult<TaskEvent>;
        /// Mocked event deletion.
        fn delete_event(&self, id: i32, hub_id: i32) -> RepositoryResult<()>;
    }
}
