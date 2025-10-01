use diesel::prelude::*;
use pushkind_common::repository::errors::{RepositoryError, RepositoryResult};

use crate::{
    domain::template::{NewTemplate, Template, UpdateTemplate},
    models::template::{
        NewTemplate as DbNewTemplate, Template as DbTemplate, UpdateTemplate as DbUpdateTemplate,
    },
    repository::{DieselRepository, TemplateListQuery, TemplateReader, TemplateWriter},
};

impl TemplateReader for DieselRepository {
    fn get_template_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Template>> {
        use crate::schema::templates;

        let mut conn = self.conn()?;
        let template = templates::table
            .find(id)
            .filter(templates::hub_id.eq(hub_id))
            .first::<DbTemplate>(&mut conn)
            .optional()?;
        let template = match template {
            Some(template) => template,
            None => return Ok(None),
        };

        Ok(Some(template.into()))
    }

    fn list_templates(&self, query: TemplateListQuery) -> RepositoryResult<(usize, Vec<Template>)> {
        use crate::schema::templates;

        let mut conn = self.conn()?;

        let query_builder = || {
            // Start with boxed query on templates
            let mut items = templates::table
                .filter(templates::hub_id.eq(query.hub_id))
                .into_boxed::<diesel::sqlite::Sqlite>();

            if let Some(value) = &query.value {
                items = items.filter(templates::value.eq(value));
            }
            items
        };

        // Get the total count before applying pagination
        let total = query_builder().count().get_result::<i64>(&mut conn)? as usize;

        let mut items = query_builder();

        // Apply pagination if requested
        if let Some(pagination) = &query.pagination {
            let offset = ((pagination.page.max(1) - 1) * pagination.per_page) as i64;
            let limit = pagination.per_page as i64;
            items = items.offset(offset).limit(limit);
        }

        // Final load
        let db_templates = items
            .order(templates::id.asc())
            .load::<DbTemplate>(&mut conn)?;

        if db_templates.is_empty() {
            return Ok((total, Vec::new()));
        }

        Ok((total, db_templates.into_iter().map(Into::into).collect()))
    }
}

impl TemplateWriter for DieselRepository {
    fn create_templates(&self, new_templates: &[NewTemplate]) -> RepositoryResult<usize> {
        use crate::schema::templates;

        let mut conn = self.conn()?;

        conn.transaction::<usize, RepositoryError, _>(|conn| {
            let mut count_inserted: usize = 0;

            for new in new_templates {
                let db_new: DbNewTemplate = new.into();

                diesel::insert_into(templates::table)
                    .values(&db_new)
                    .execute(conn)?;
                count_inserted += 1;
            }

            Ok(count_inserted)
        })
    }

    fn update_template(
        &self,
        template_id: i32,
        hub_id: i32,
        updates: &UpdateTemplate,
    ) -> RepositoryResult<Template> {
        use crate::schema::templates;

        let mut conn = self.conn()?;
        let db_updates: DbUpdateTemplate = updates.into();

        let target = templates::table
            .filter(templates::id.eq(template_id))
            .filter(templates::hub_id.eq(hub_id));

        let updated: Template = diesel::update(target)
            .set(&db_updates)
            .get_result::<DbTemplate>(&mut conn)?
            .into();

        Ok(updated)
    }

    fn delete_template(&self, template_id: i32, hub_id: i32) -> RepositoryResult<()> {
        use crate::schema::templates;

        let mut conn = self.conn()?;

        let target = templates::table
            .filter(templates::id.eq(template_id))
            .filter(templates::hub_id.eq(hub_id));

        let deleted = diesel::delete(target).execute(&mut conn)?;
        if deleted == 0 {
            return Err(RepositoryError::NotFound);
        }

        Ok(())
    }
}
