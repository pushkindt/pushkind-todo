//! Repository layer definitions and Diesel-backed implementations used throughout the services.
use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::{
    task::{NewTask, Task, TaskAssignment, TaskListFilters, UpdateTask},
    task_event::{NewTaskEvent, TaskEvent},
    user::{NewUser, UpdateUser, User},
};

pub mod task;
pub mod task_event;
pub mod user;

#[cfg(test)]
pub mod mock;

#[derive(Clone)]
/// Diesel-backed repository implementation that wraps an r2d2 pool.
pub struct DieselRepository {
    pool: DbPool, // r2d2::Pool is cheap to clone
}

impl DieselRepository {
    /// Create a new repository using the provided connection pool.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Borrow a pooled database connection for repository operations.
    fn conn(&self) -> RepositoryResult<DbConnection> {
        Ok(self.pool.get()?)
    }
}

#[derive(Debug, Clone)]
/// Query definition used to filter and paginate users for a hub.
pub struct UserListQuery {
    pub hub_id: i32,
    pub search: Option<String>,
    pub pagination: Option<Pagination>,
}

impl UserListQuery {
    /// Construct a query scoped to the provided hub.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            search: None,
            pagination: None,
        }
    }

    /// Apply a free-text search filter to the query.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Apply pagination to the query with the given page number and size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

#[derive(Debug, Clone)]
/// Query definition used to filter and paginate tasks for a hub.
pub struct TaskListQuery {
    pub filters: TaskListFilters,
    pub pagination: Option<Pagination>,
}

impl TaskListQuery {
    /// Construct a query scoped to the given hub using default filters.
    pub fn new(hub_id: i32) -> Result<Self, crate::domain::types::TypeConstraintError> {
        let hub_id = crate::domain::types::HubId::new(hub_id)?;
        Ok(Self {
            filters: TaskListFilters::new(hub_id),
            pagination: None,
        })
    }

    /// Replace the filters used by the query.
    pub fn with_filters(mut self, filters: TaskListFilters) -> Self {
        self.filters = filters;
        self
    }

    /// Apply pagination to the query with the given page number and size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }

    /// Gain mutable access to the inner filters for incremental adjustments.
    pub fn filters_mut(&mut self) -> &mut TaskListFilters {
        &mut self.filters
    }
}

/// Read-only operations over user records.
pub trait UserReader {
    /// Get a user by identifier within the hub.
    fn get_user_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<User>>;
    /// Lookup a user by email scoped to the hub.
    fn get_user_by_email(&self, email: &str, hub_id: i32) -> RepositoryResult<Option<User>>;
    /// List users matching the query filters.
    fn list_users(&self, query: UserListQuery) -> RepositoryResult<(usize, Vec<User>)>;
}

/// Write operations over user records.
pub trait UserWriter {
    /// Insert or refresh a user record based on unique constraints.
    fn create_or_update_user(&self, new_user: &NewUser) -> RepositoryResult<User>;
    /// Update mutable fields for an existing user.
    fn update_user(
        &self,
        user_id: i32,
        hub_id: i32,
        updates: &UpdateUser,
    ) -> RepositoryResult<User>;
    /// Delete a user record by identifier.
    fn delete_user(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()>;
    /// Touch the user's `visited_at` timestamp.
    fn touch_visited_at(&self, user_id: i32, hub_id: i32) -> RepositoryResult<()>;
}

/// Read-only operations over task records.
pub trait TaskReader {
    /// Fetch a task by id within the hub.
    fn get_task_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Task>>;
    /// List tasks with filtering/pagination support.
    fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<Task>)>;
    /// Read all assignments for a task.
    fn list_assignments_for_task(
        &self,
        task_id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Vec<TaskAssignment>>;
    /// Return available task tracks for a hub.
    fn list_task_tracks(&self, hub_id: i32) -> RepositoryResult<Vec<String>>;
}

/// Write operations over task records.
pub trait TaskWriter {
    /// Insert a new task record.
    fn create_task(&self, new_task: &NewTask) -> RepositoryResult<Task>;
    /// Persist updates to a task.
    fn update_task(
        &self,
        task_id: i32,
        hub_id: i32,
        updates: &UpdateTask,
    ) -> RepositoryResult<Task>;
    /// Delete a task from the hub.
    fn delete_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<()>;
    /// Save a new assignment entry for auditing.
    fn record_assignment(&self, assignment: &TaskAssignment) -> RepositoryResult<()>;
    /// Remove an assignment snapshot.
    fn remove_assignment(
        &self,
        task_id: i32,
        hub_id: i32,
        assignee_id: i32,
    ) -> RepositoryResult<()>;
}

/// Read-only operations over task event records.
pub trait TaskEventReader {
    /// Stream events tied to the provided task.
    fn list_events_for_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<Vec<TaskEvent>>;
    /// Lookup a task event by id and hub.
    fn get_event_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<TaskEvent>>;
}

/// Write operations over task event records.
pub trait TaskEventWriter {
    /// Persist a new task event entry.
    fn record_event(&self, event: &NewTaskEvent) -> RepositoryResult<TaskEvent>;
    /// Delete a task event by id/hub.
    fn delete_event(&self, id: i32, hub_id: i32) -> RepositoryResult<()>;
}
