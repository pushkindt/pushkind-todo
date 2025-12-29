//! Diesel repository implementation for task persistence and queries.
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::{
        task::{
            NewTask as DomainNewTask, Task as DomainTask, TaskAssignment as DomainTaskAssignment,
            TaskListFilters, TaskStatus, UpdateTask as DomainUpdateTask,
        },
        types::{HubId, TaskId, TaskTrack, TypeConstraintError, UserId},
    },
    models::task::{
        NewTask as DbNewTask, NewTaskAssignment as DbNewTaskAssignment, Task as DbTask,
        TaskAssignment as DbTaskAssignment, UpdateTask as DbUpdateTask,
    },
    repository::{DieselRepository, TaskListQuery, TaskReader, TaskWriter},
};

impl TaskReader for DieselRepository {
    /// Retrieve the distinct task tracks for a hub.
    fn list_task_tracks(&self, hub_id: HubId) -> RepositoryResult<Vec<TaskTrack>> {
        use crate::schema::tasks;

        let mut conn = self.conn()?;
        let hub_id = i32::from(hub_id);

        let tracks = tasks::table
            .filter(tasks::hub_id.eq(hub_id))
            .select(tasks::track)
            .distinct()
            .order(tasks::track)
            .load::<Option<String>>(&mut conn)?;
        Ok(tracks
            .into_iter()
            .flatten()
            .map(TaskTrack::new)
            .collect::<Result<Vec<TaskTrack>, TypeConstraintError>>()?)
    }

    /// Load a single task within the hub by its identifier.
    fn get_task_by_id(&self, id: TaskId, hub_id: HubId) -> RepositoryResult<Option<DomainTask>> {
        use crate::schema::tasks;

        let mut conn = self.conn()?;
        let id = i32::from(id);
        let hub_id = i32::from(hub_id);

        let task = tasks::table
            .filter(tasks::id.eq(id))
            .filter(tasks::hub_id.eq(hub_id))
            .select(DbTask::as_select())
            .first::<DbTask>(&mut conn)
            .optional()?;

        Ok(task.map(|t| t.try_into()).transpose()?)
    }

    /// Query for tasks matching the provided filters and pagination.
    fn list_tasks(&self, query: TaskListQuery) -> RepositoryResult<(usize, Vec<DomainTask>)> {
        use crate::schema::tasks;

        let mut conn = self.conn()?;

        let TaskListQuery {
            filters:
                TaskListFilters {
                    hub_id,
                    assignee_id,
                    track,
                    status,
                    priority,
                    search,
                    due_before,
                    due_after,
                    updated_before,
                    updated_after,
                    hide_terminal_statuses,
                },
            pagination,
        } = query;

        let status_text = status.map(<&'static str>::from);
        let priority_text = priority.map(<&'static str>::from);
        let search_pattern = search.as_ref().map(|term| format!("%{}%", term.as_str()));

        let query_builder = || {
            let mut items = tasks::table
                .filter(tasks::hub_id.eq(hub_id.get()))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(assignee_id) = assignee_id {
                items = items.filter(tasks::assigned_to.eq(Some(assignee_id.get())));
            }

            if let Some(track_value) = track.as_ref() {
                items = items.filter(tasks::track.eq(track_value.as_str()));
            }

            if let Some(status_text) = status_text {
                items = items.filter(tasks::status.eq(status_text));
            }

            if let Some(priority_text) = priority_text {
                items = items.filter(tasks::priority.eq(priority_text));
            }

            if let Some(due_before) = due_before {
                items = items.filter(tasks::due_date.le(due_before));
            }

            if let Some(due_after) = due_after {
                items = items.filter(tasks::due_date.ge(due_after));
            }

            if let Some(updated_before) = updated_before {
                items = items.filter(tasks::updated_at.le(updated_before));
            }

            if let Some(updated_after) = updated_after {
                items = items.filter(tasks::updated_at.ge(updated_after));
            }

            if let Some(pattern) = search_pattern.as_deref() {
                items = items.filter(
                    tasks::title
                        .like(pattern)
                        .or(tasks::description.like(pattern)),
                );
            }

            if hide_terminal_statuses {
                let completed = <&str>::from(TaskStatus::Completed);
                let archived = <&str>::from(TaskStatus::Archived);
                let blocked = <&str>::from(TaskStatus::Blocked);
                items = items
                    .filter(tasks::status.ne(completed))
                    .filter(tasks::status.ne(archived))
                    .filter(tasks::status.ne(blocked));
            }

            items
        };

        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();

        if let Some(pagination) = &pagination {
            let page = pagination.page.max(1);
            let offset = ((page - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        let db_tasks = items
            .order(tasks::updated_at.desc())
            .select(DbTask::as_select())
            .load::<DbTask>(&mut conn)?;

        Ok((
            total,
            db_tasks
                .into_iter()
                .map(|t| t.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    /// Read the history of assignments recorded for a task.
    fn list_assignments_for_task(
        &self,
        task_id: TaskId,
        hub_id: HubId,
    ) -> RepositoryResult<Vec<DomainTaskAssignment>> {
        use crate::schema::task_assignments;

        let mut conn = self.conn()?;
        let task_id = i32::from(task_id);
        let hub_id = i32::from(hub_id);

        let assignments = task_assignments::table
            .filter(task_assignments::task_id.eq(task_id))
            .filter(task_assignments::hub_id.eq(hub_id))
            .order(task_assignments::assigned_at.asc())
            .select(DbTaskAssignment::as_select())
            .load::<DbTaskAssignment>(&mut conn)?;

        Ok(assignments
            .into_iter()
            .map(|a| a.try_into())
            .collect::<Result<Vec<_>, _>>()?)
    }
}

impl TaskWriter for DieselRepository {
    /// Insert a new task record and return its domain representation.
    fn create_task(&self, new_task: &DomainNewTask) -> RepositoryResult<DomainTask> {
        use crate::schema::{tasks, users};

        let mut conn = self.conn()?;

        conn.transaction::<DomainTask, RepositoryError, _>(|conn| {
            let author_exists = users::table
                .filter(users::id.eq(new_task.author_id.get()))
                .filter(users::hub_id.eq(new_task.hub_id.get()))
                .select(users::id)
                .first::<i32>(conn)
                .optional()?;

            if author_exists.is_none() {
                return Err(RepositoryError::NotFound);
            }

            if let Some(assignee_id) = new_task.assigned_to {
                let assignee = users::table
                    .filter(users::id.eq(assignee_id.get()))
                    .filter(users::hub_id.eq(new_task.hub_id.get()))
                    .select(users::id)
                    .first::<i32>(conn)
                    .optional()?;

                if assignee.is_none() {
                    return Err(RepositoryError::NotFound);
                }
            }

            let db_new = DbNewTask::from(new_task);

            let created = diesel::insert_into(tasks::table)
                .values(&db_new)
                .returning(DbTask::as_returning())
                .get_result::<DbTask>(conn)?;

            created.try_into().map_err(RepositoryError::from)
        })
    }

    /// Persist updates to an existing task record.
    fn update_task(
        &self,
        task_id: TaskId,
        hub_id: HubId,
        updates: &DomainUpdateTask,
    ) -> RepositoryResult<DomainTask> {
        use crate::schema::{tasks, users};

        let mut conn = self.conn()?;
        let task_id = i32::from(task_id);
        let hub_id = i32::from(hub_id);

        conn.transaction::<DomainTask, RepositoryError, _>(|conn| {
            if let Some(assignee_id) = updates.assigned_to {
                let assignee = users::table
                    .filter(users::id.eq(assignee_id.get()))
                    .filter(users::hub_id.eq(hub_id))
                    .select(users::id)
                    .first::<i32>(conn)
                    .optional()?;

                if assignee.is_none() {
                    return Err(RepositoryError::NotFound);
                }
            }

            let db_updates = DbUpdateTask::from(updates);

            let target = tasks::table
                .filter(tasks::id.eq(task_id))
                .filter(tasks::hub_id.eq(hub_id));

            let updated = diesel::update(target)
                .set(&db_updates)
                .returning(DbTask::as_returning())
                .get_result::<DbTask>(conn)?;

            updated.try_into().map_err(RepositoryError::from)
        })
    }

    /// Remove a task belonging to the specified hub.
    fn delete_task(&self, task_id: TaskId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::tasks;

        let mut conn = self.conn()?;
        let task_id = i32::from(task_id);
        let hub_id = i32::from(hub_id);

        let target = tasks::table
            .filter(tasks::id.eq(task_id))
            .filter(tasks::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    /// Record a new assignment entry for auditing.
    fn record_assignment(&self, assignment: &DomainTaskAssignment) -> RepositoryResult<()> {
        use crate::schema::task_assignments;

        let mut conn = self.conn()?;
        let db_new = DbNewTaskAssignment::from(assignment);

        diesel::insert_into(task_assignments::table)
            .values(&db_new)
            .execute(&mut conn)?;

        Ok(())
    }

    /// Remove an existing assignment snapshot for a task.
    fn remove_assignment(
        &self,
        task_id: TaskId,
        hub_id: HubId,
        assignee_id: UserId,
    ) -> RepositoryResult<()> {
        use crate::schema::task_assignments;

        let mut conn = self.conn()?;
        let task_id = i32::from(task_id);
        let hub_id = i32::from(hub_id);
        let assignee_id = i32::from(assignee_id);

        let target = task_assignments::table
            .filter(task_assignments::task_id.eq(task_id))
            .filter(task_assignments::hub_id.eq(hub_id))
            .filter(task_assignments::assignee_id.eq(assignee_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
