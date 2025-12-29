//! Diesel repository implementation for task event creation and retrieval.
use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::task_event::{NewTaskEvent as DomainNewTaskEvent, TaskEvent as DomainTaskEvent},
    domain::types::{HubId, TaskEventId, TaskId},
    models::task_event::{
        NewTaskEvent as DbNewTaskEvent, TaskEvent as DbTaskEvent, TaskEventModelError,
    },
    repository::{DieselRepository, TaskEventReader, TaskEventWriter},
};

/// Map model-level validation failures into repository validation errors.
fn model_error_as_validation(err: TaskEventModelError) -> RepositoryError {
    RepositoryError::ValidationError(err.to_string())
}

/// Treat unexpected model errors as repository unexpected failures.
fn model_error_as_unexpected(err: TaskEventModelError) -> RepositoryError {
    RepositoryError::Unexpected(err.to_string())
}

impl TaskEventReader for DieselRepository {
    /// Load the streams of events linked to a task, ordered by recency.
    fn list_events_for_task(
        &self,
        task_id: TaskId,
        hub_id: HubId,
    ) -> RepositoryResult<Vec<DomainTaskEvent>> {
        use crate::schema::{task_events, tasks};

        let mut conn = self.conn()?;
        let task_id = i32::from(task_id);
        let hub_id = i32::from(hub_id);

        let db_events = task_events::table
            .inner_join(tasks::table)
            .filter(task_events::task_id.eq(task_id))
            .filter(tasks::hub_id.eq(hub_id))
            .order(task_events::created_at.desc())
            .select(DbTaskEvent::as_select())
            .load::<DbTaskEvent>(&mut conn)?;

        db_events
            .into_iter()
            .map(|event| event.try_into())
            .collect::<Result<Vec<_>, _>>()
            .map_err(model_error_as_unexpected)
    }
}

impl TaskEventWriter for DieselRepository {
    /// Persist a new task event and update the parent task timestamp.
    fn record_event(&self, event: &DomainNewTaskEvent) -> RepositoryResult<DomainTaskEvent> {
        use crate::schema::{task_events, tasks};

        let mut conn = self.conn()?;
        let db_new = DbNewTaskEvent::try_from(event).map_err(model_error_as_validation)?;

        conn.transaction::<DomainTaskEvent, RepositoryError, _>(|conn| {
            let inserted = diesel::insert_into(task_events::table)
                .values(&db_new)
                .returning(DbTaskEvent::as_returning())
                .get_result::<DbTaskEvent>(conn)?;

            let task_id = inserted.task_id;
            let event_created_at = inserted.created_at;

            let updated = diesel::update(tasks::table.filter(tasks::id.eq(task_id)))
                .set(tasks::updated_at.eq(event_created_at))
                .execute(conn)?;

            if updated == 0 {
                return Err(RepositoryError::Unexpected(
                    "Recorded task event but failed to update task timestamp".to_string(),
                ));
            }

            inserted.try_into().map_err(model_error_as_unexpected)
        })
    }

    /// Remove a recorded task event by id, enforcing hub scope.
    fn delete_event(&self, id: TaskEventId, hub_id: HubId) -> RepositoryResult<()> {
        use crate::schema::{task_events, tasks};

        let mut conn = self.conn()?;
        let id = i32::from(id);
        let hub_id = i32::from(hub_id);

        conn.transaction::<(), RepositoryError, _>(|conn| {
            let exists = task_events::table
                .inner_join(tasks::table)
                .filter(task_events::id.eq(id))
                .filter(tasks::hub_id.eq(hub_id))
                .select(task_events::id)
                .first::<i32>(conn)
                .optional()?;

            let Some(_) = exists else {
                return Err(RepositoryError::NotFound);
            };

            diesel::delete(task_events::table.filter(task_events::id.eq(id))).execute(conn)?;

            Ok(())
        })?;

        Ok(())
    }
}
