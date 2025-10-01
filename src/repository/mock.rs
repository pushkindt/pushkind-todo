use mockall::mock;

use super::{TemplateListQuery, TemplateReader, TemplateWriter};
use crate::domain::template::{NewTemplate, Template, UpdateTemplate};
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
