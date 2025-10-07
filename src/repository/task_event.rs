use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::task_event::{NewTaskEvent as DomainNewTaskEvent, TaskEvent as DomainTaskEvent},
    models::task_event::{
        NewTaskEvent as DbNewTaskEvent, TaskEvent as DbTaskEvent, TaskEventModelError,
    },
    repository::{DieselRepository, TaskEventReader, TaskEventWriter},
};

fn model_error_as_validation(err: TaskEventModelError) -> RepositoryError {
    RepositoryError::ValidationError(err.to_string())
}

fn model_error_as_unexpected(err: TaskEventModelError) -> RepositoryError {
    RepositoryError::Unexpected(err.to_string())
}

impl TaskEventReader for DieselRepository {
    fn list_events_for_task(
        &self,
        task_id: i32,
        hub_id: i32,
    ) -> RepositoryResult<Vec<DomainTaskEvent>> {
        use crate::schema::{task_events, tasks};

        let mut conn = self.conn()?;

        let db_events = task_events::table
            .inner_join(tasks::table)
            .filter(task_events::task_id.eq(task_id))
            .filter(tasks::hub_id.eq(hub_id))
            .order(task_events::created_at.desc())
            .select(DbTaskEvent::as_select())
            .load::<DbTaskEvent>(&mut conn)?;

        db_events
            .into_iter()
            .map(|event| event.try_into().map_err(model_error_as_unexpected))
            .collect()
    }

    fn get_event_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<DomainTaskEvent>> {
        use crate::schema::{task_events, tasks};

        let mut conn = self.conn()?;

        let event = task_events::table
            .inner_join(tasks::table)
            .filter(task_events::id.eq(id))
            .filter(tasks::hub_id.eq(hub_id))
            .select(DbTaskEvent::as_select())
            .first::<DbTaskEvent>(&mut conn)
            .optional()?;

        let Some(event) = event else {
            return Ok(None);
        };

        Ok(Some(event.try_into().map_err(model_error_as_unexpected)?))
    }
}

impl TaskEventWriter for DieselRepository {
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

    fn delete_event(&self, id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::{task_events, tasks};

        let mut conn = self.conn()?;

        let exists = task_events::table
            .inner_join(tasks::table)
            .filter(task_events::id.eq(id))
            .filter(tasks::hub_id.eq(hub_id))
            .select(task_events::id)
            .first::<i32>(&mut conn)
            .optional()?;

        let Some(_) = exists else {
            return Err(RepositoryError::NotFound);
        };

        diesel::delete(task_events::table.filter(task_events::id.eq(id))).execute(&mut conn)?;

        Ok(())
    }
}
