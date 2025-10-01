use pushkind_common::db::{DbConnection, DbPool};
use pushkind_common::pagination::Pagination;
use pushkind_common::repository::errors::RepositoryResult;

use crate::domain::template::{NewTemplate, Template, UpdateTemplate};

pub mod template;

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

    fn conn(&self) -> RepositoryResult<DbConnection> {
        Ok(self.pool.get()?)
    }
}

#[derive(Debug, Clone)]
/// Query definition used to filter and paginate templates for a hub.
pub struct TemplateListQuery {
    pub hub_id: i32,
    pub value: Option<String>,
    pub pagination: Option<Pagination>,
}

impl TemplateListQuery {
    /// Construct a query that targets all templates belonging to `hub_id`.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            value: None,
            pagination: None,
        }
    }

    /// Filter the results to templates matching the exact `value`.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Apply pagination to the query with the given page number and page size.
    pub fn paginate(mut self, page: usize, per_page: usize) -> Self {
        self.pagination = Some(Pagination { page, per_page });
        self
    }
}

/// Read-only operations over template records.
pub trait TemplateReader {
    fn get_template_by_id(&self, id: i32, hub_id: i32) -> RepositoryResult<Option<Template>>;
    fn list_templates(&self, query: TemplateListQuery) -> RepositoryResult<(usize, Vec<Template>)>;
}

/// Write operations over template records.
pub trait TemplateWriter {
    fn create_templates(&self, new_templates: &[NewTemplate]) -> RepositoryResult<usize>;
    fn update_template(
        &self,
        template_id: i32,
        hub_id: i32,
        updates: &UpdateTemplate,
    ) -> RepositoryResult<Template>;
    fn delete_template(&self, template_id: i32, hub_id: i32) -> RepositoryResult<()>;
}
