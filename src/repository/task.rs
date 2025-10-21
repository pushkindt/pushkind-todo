use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::task::{
        NewTask as DomainNewTask, Task as DomainTask, TaskAssignment as DomainTaskAssignment,
        TaskListFilters, UpdateTask as DomainUpdateTask,
    },
    models::task::{
        NewTask as DbNewTask, NewTaskAssignment as DbNewTaskAssignment, Task as DbTask,
        TaskAssignment as DbTaskAssignment, UpdateTask as DbUpdateTask,
    },
    repository::{DieselRepository, TaskListQuery, TaskReader, TaskWriter},
};

impl TaskReader for DieselRepository {
    fn list_task_tracks(&self, hub_id: i32) -> RepositoryResult<Vec<String>> {
        use crate::schema::tasks;

        let mut conn = self.conn()?;

        let tracks = tasks::table
            .filter(tasks::hub_id.eq(hub_id))
            .select(tasks::track)
            .distinct()
            .order(tasks::track)
            .load::<Option<String>>(&mut conn)?;
        Ok(tracks.into_iter().flatten().collect())
    }

    fn get_task_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainTask>> {
        use crate::schema::tasks;

        let mut conn = self.conn()?;

        let task = tasks::table
            .filter(tasks::id.eq(id))
            .filter(tasks::hub_id.eq(hub_id))
            .select(DbTask::as_select())
            .first::<DbTask>(&mut conn)
            .optional()?;

        Ok(task.map(Into::into))
    }

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
                },
            pagination,
        } = query;

        let status_text = status.map(<&'static str>::from);
        let priority_text = priority.map(<&'static str>::from);
        let search_pattern = search.as_ref().and_then(|term| {
            let trimmed = term.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(format!("%{}%", trimmed))
            }
        });

        let query_builder = || {
            let mut items = tasks::table
                .filter(tasks::hub_id.eq(hub_id))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(assignee_id) = assignee_id {
                items = items.filter(tasks::assigned_to.eq(Some(assignee_id)));
            }

            if let Some(track_value) = track
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                items = items.filter(tasks::track.eq(track_value));
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

        Ok((total, db_tasks.into_iter().map(Into::into).collect()))
    }

    fn list_assignments_for_task(
        &self,
        task_id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Vec<DomainTaskAssignment>> {
        use crate::schema::task_assignments;

        let mut conn = self.conn()?;

        let assignments = task_assignments::table
            .filter(task_assignments::task_id.eq(task_id))
            .filter(task_assignments::hub_id.eq(hub_id))
            .order(task_assignments::assigned_at.asc())
            .select(DbTaskAssignment::as_select())
            .load::<DbTaskAssignment>(&mut conn)?;

        Ok(assignments.into_iter().map(Into::into).collect())
    }
}

impl TaskWriter for DieselRepository {
    fn create_task(&self, new_task: &DomainNewTask) -> RepositoryResult<DomainTask> {
        use crate::schema::{tasks, users};

        let mut conn = self.conn()?;

        let author_exists = users::table
            .filter(users::id.eq(new_task.author_id))
            .filter(users::hub_id.eq(new_task.hub_id))
            .select(users::id)
            .first::<i32>(&mut conn)
            .optional()?;

        if author_exists.is_none() {
            return Err(RepositoryError::NotFound);
        }

        if let Some(assignee_id) = new_task.assigned_to {
            let assignee = users::table
                .filter(users::id.eq(assignee_id))
                .filter(users::hub_id.eq(new_task.hub_id))
                .select(users::id)
                .first::<i32>(&mut conn)
                .optional()?;

            if assignee.is_none() {
                return Err(RepositoryError::NotFound);
            }
        }

        let db_new = DbNewTask::from(new_task);

        let created = diesel::insert_into(tasks::table)
            .values(&db_new)
            .returning(DbTask::as_returning())
            .get_result::<DbTask>(&mut conn)?;

        Ok(created.into())
    }

    fn update_task(
        &self,
        task_id: i32,
        hub_id: i32,
        updates: &DomainUpdateTask,
    ) -> RepositoryResult<DomainTask> {
        use crate::schema::{tasks, users};

        let mut conn = self.conn()?;

        if let Some(assignee_id) = updates.assigned_to {
            let assignee = users::table
                .filter(users::id.eq(assignee_id))
                .filter(users::hub_id.eq(hub_id))
                .select(users::id)
                .first::<i32>(&mut conn)
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
            .get_result::<DbTask>(&mut conn)?;

        Ok(updated.into())
    }

    fn delete_task(&self, task_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::tasks;

        let mut conn = self.conn()?;

        let target = tasks::table
            .filter(tasks::id.eq(task_id))
            .filter(tasks::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }

    fn record_assignment(&self, assignment: &DomainTaskAssignment) -> RepositoryResult<()> {
        use crate::schema::task_assignments;

        let mut conn = self.conn()?;
        let db_new = DbNewTaskAssignment::from(assignment);

        diesel::insert_into(task_assignments::table)
            .values(&db_new)
            .execute(&mut conn)?;

        Ok(())
    }

    fn remove_assignment(
        &self,
        task_id: i32,
        hub_id: i32,
        assignee_id: i32,
    ) -> RepositoryResult<()> {
        use crate::schema::task_assignments;

        let mut conn = self.conn()?;

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
