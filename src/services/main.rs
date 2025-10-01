use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::pagination::{DEFAULT_ITEMS_PER_PAGE, Paginated};
use pushkind_common::routes::check_role;
use serde::Deserialize;
use validator::Validate;

use crate::SERVICE_ACCESS_ROLE;
use crate::domain::template::Template;
use crate::forms::main::{AddTemplateForm, UploadTemplatesForm};
use crate::repository::{TemplateListQuery, TemplateReader, TemplateWriter};
use crate::services::{RedirectSuccess, ServiceError, ServiceResult};

/// Query parameters accepted by the index page service.
#[derive(Debug, Default, Deserialize)]
pub struct IndexQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
}

/// Data required to render the main index template.
pub struct IndexPageData {
    /// Paginated list of templates to show in the table.
    pub templates: Paginated<Template>,
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
}

/// Loads the templates list for the main index page.
pub fn load_index_page<R>(
    repo: &R,
    user: &AuthenticatedUser,
    query: IndexQuery,
) -> ServiceResult<IndexPageData>
where
    R: TemplateReader + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let page = query.page.unwrap_or(1);
    let mut list_query = TemplateListQuery::new(user.hub_id).paginate(page, DEFAULT_ITEMS_PER_PAGE);

    if let Some(value) = query.search.as_ref() {
        list_query = list_query.value(value);
    }

    let (total, templates) = repo
        .list_templates(list_query)
        .map_err(ServiceError::from)?;

    let total_pages = total.div_ceil(DEFAULT_ITEMS_PER_PAGE);
    let templates = Paginated::new(templates, page, total_pages);

    Ok(IndexPageData {
        templates,
        search: query.search,
    })
}

/// Validates the add-template form and persists a new template record.
pub fn add_template<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: AddTemplateForm,
) -> ServiceResult<RedirectSuccess>
where
    R: TemplateWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    if let Err(err) = form.validate() {
        log::error!("Failed to validate form: {err}");
        return Err(ServiceError::Form("Ошибка валидации формы".to_string()));
    }

    let new_template = form.to_new_template(user.hub_id);

    repo.create_templates(&[new_template]).map_err(|err| {
        log::error!("Failed to add a template: {err}");
        err
    })?;

    Ok(RedirectSuccess {
        message: "Шаблон добавлен.".to_string(),
        redirect_to: "/".to_string(),
    })
}

/// Parses the uploaded CSV file and creates template records in bulk.
pub fn upload_templates<R>(
    repo: &R,
    user: &AuthenticatedUser,
    form: &mut UploadTemplatesForm,
) -> ServiceResult<RedirectSuccess>
where
    R: TemplateWriter + ?Sized,
{
    if !check_role(SERVICE_ACCESS_ROLE, &user.roles) {
        return Err(ServiceError::Unauthorized);
    }

    let new_templates = form.parse(user.hub_id).map_err(|err| {
        log::error!("Failed to parse templates: {err}");
        ServiceError::Form("Ошибка при парсинге шаблонов".to_string())
    })?;

    repo.create_templates(&new_templates).map_err(|err| {
        log::error!("Failed to add a templates: {err}");
        err
    })?;

    Ok(RedirectSuccess {
        message: "Шаблоны добавлены.".to_string(),
        redirect_to: "/".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_multipart::form::tempfile::TempFile;
    use chrono::{NaiveDate, NaiveDateTime};
    use pushkind_common::repository::errors::RepositoryError;
    use serde_json::Value;
    use tempfile::NamedTempFile;

    use crate::SERVICE_ACCESS_ROLE;
    use crate::domain::template::NewTemplate;
    use crate::repository::mock::{MockTemplateReader, MockTemplateWriter};

    use std::io::Write;

    fn fixed_datetime() -> NaiveDateTime {
        match NaiveDate::from_ymd_opt(2024, 1, 1) {
            Some(date) => date.and_hms_opt(0, 0, 0).unwrap_or_default(),
            None => NaiveDateTime::default(),
        }
    }

    fn sample_template(id: i32, hub_id: i32, value: &str) -> Template {
        Template {
            id,
            hub_id,
            value: Some(value.to_string()),
            created_at: fixed_datetime(),
            updated_at: fixed_datetime(),
        }
    }

    fn user_with_roles(roles: &[&str]) -> AuthenticatedUser {
        AuthenticatedUser {
            sub: "user-1".to_string(),
            email: "user@example.com".to_string(),
            hub_id: 99,
            name: "Tester".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
            exp: 0,
        }
    }

    fn upload_form(contents: &str) -> UploadTemplatesForm {
        let mut file = match NamedTempFile::new() {
            Ok(file) => file,
            Err(err) => panic!("failed to create temp file: {err}"),
        };

        if let Err(err) = file.write_all(contents.as_bytes()) {
            panic!("failed to write csv contents: {err}");
        }

        UploadTemplatesForm {
            csv: TempFile {
                file,
                content_type: None,
                file_name: Some("upload.csv".to_string()),
                size: contents.len(),
            },
        }
    }

    #[test]
    fn load_index_page_returns_unauthorized_when_role_missing() {
        let repo = MockTemplateReader::new();
        let user = user_with_roles(&[]);

        let result = load_index_page(&repo, &user, IndexQuery::default());

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn load_index_page_returns_paginated_data() {
        let mut repo = MockTemplateReader::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let query = IndexQuery {
            search: Some("alp".to_string()),
            page: Some(2),
        };

        let expected_hub = user.hub_id;

        repo.expect_list_templates()
            .times(1)
            .withf(|query| {
                assert_eq!(query.value.as_deref(), Some("alp"));
                match &query.pagination {
                    Some(pagination) => {
                        assert_eq!(pagination.page, 2);
                        assert_eq!(pagination.per_page, DEFAULT_ITEMS_PER_PAGE);
                    }
                    None => panic!("expected pagination to be set"),
                }
                true
            })
            .returning(move |_| {
                Ok((
                    45,
                    vec![
                        sample_template(1, expected_hub, "alpha"),
                        sample_template(2, expected_hub, "beta"),
                    ],
                ))
            });

        let result = load_index_page(&repo, &user, query);

        let data = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(data.search.as_deref(), Some("alp"));

        let serialized = match serde_json::to_value(&data.templates) {
            Ok(value) => value,
            Err(err) => panic!("serialization failed: {err}"),
        };

        let page_value = match serialized.get("page") {
            Some(value) => value,
            None => panic!("missing page field"),
        };
        assert_eq!(page_value.as_u64(), Some(2));

        let items = match serialized.get("items") {
            Some(value) => match value.as_array() {
                Some(items) => items,
                None => panic!("items field is not an array"),
            },
            None => panic!("missing items field"),
        };
        assert_eq!(items.len(), 2);

        let first_value = items
            .first()
            .and_then(|item| item.as_object())
            .and_then(|map| map.get("value"))
            .and_then(Value::as_str);
        assert_eq!(first_value, Some("alpha"));
    }

    #[test]
    fn add_template_returns_unauthorized_when_role_missing() {
        let repo = MockTemplateWriter::new();
        let user = user_with_roles(&[]);
        let form = AddTemplateForm {
            value: Some("alpha".to_string()),
        };

        let result = add_template(&repo, &user, form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn add_template_returns_form_error_on_validation_failure() {
        let repo = MockTemplateWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTemplateForm {
            value: Some(String::new()),
        };

        let result = add_template(&repo, &user, form);

        match result {
            Err(ServiceError::Form(message)) => {
                assert_eq!(message, "Ошибка валидации формы");
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn add_template_persists_new_record_on_success() {
        let mut repo = MockTemplateWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTemplateForm {
            value: Some("alpha".to_string()),
        };

        let expected_hub = user.hub_id;

        repo.expect_create_templates()
            .times(1)
            .withf(move |templates: &[NewTemplate]| {
                assert_eq!(templates.len(), 1);
                let template = &templates[0];
                assert_eq!(template.hub_id, expected_hub);
                assert_eq!(template.value.as_deref(), Some("alpha"));
                true
            })
            .returning(|templates| Ok(templates.len()));

        let result = add_template(&repo, &user, form);

        let redirect = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(redirect.message, "Шаблон добавлен.");
        assert_eq!(redirect.redirect_to, "/");
    }

    #[test]
    fn add_template_propagates_repository_errors() {
        let mut repo = MockTemplateWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let form = AddTemplateForm {
            value: Some("alpha".to_string()),
        };

        repo.expect_create_templates()
            .times(1)
            .returning(|_| Err(RepositoryError::Unexpected("db write failed".to_string())));

        let result = add_template(&repo, &user, form);

        match result {
            Err(ServiceError::Repository(RepositoryError::Unexpected(message))) => {
                assert_eq!(message, "db write failed");
            }
            other => panic!("expected repository error, got {other:?}"),
        }
    }

    #[test]
    fn upload_templates_returns_unauthorized_when_role_missing() {
        let repo = MockTemplateWriter::new();
        let user = user_with_roles(&[]);
        let mut form = upload_form(
            "value
alpha
",
        );

        let result = upload_templates(&repo, &user, &mut form);

        assert!(matches!(result, Err(ServiceError::Unauthorized)));
    }

    #[test]
    fn upload_templates_returns_form_error_when_parse_fails() {
        let mut repo = MockTemplateWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let mut form = upload_form(
            "value
foo,bar
",
        );

        repo.expect_create_templates().never();

        let result = upload_templates(&repo, &user, &mut form);

        match result {
            Err(ServiceError::Form(message)) => {
                assert_eq!(message, "Ошибка при парсинге шаблонов");
            }
            other => panic!("expected form error, got {other:?}"),
        }
    }

    #[test]
    fn upload_templates_persists_uploaded_records() {
        let mut repo = MockTemplateWriter::new();
        let user = user_with_roles(&[SERVICE_ACCESS_ROLE]);
        let mut form = upload_form(
            "value
alpha
beta
",
        );

        let expected_hub = user.hub_id;

        repo.expect_create_templates()
            .times(1)
            .withf(move |templates: &[NewTemplate]| {
                assert_eq!(templates.len(), 2);
                assert!(
                    templates
                        .iter()
                        .all(|template| template.hub_id == expected_hub)
                );
                assert_eq!(templates[0].value.as_deref(), Some("alpha"));
                assert_eq!(templates[1].value.as_deref(), Some("beta"));
                true
            })
            .returning(|templates| Ok(templates.len()));

        let result = upload_templates(&repo, &user, &mut form);

        let redirect = match result {
            Ok(value) => value,
            Err(err) => panic!("expected success, got error: {err}"),
        };

        assert_eq!(redirect.message, "Шаблоны добавлены.");
        assert_eq!(redirect.redirect_to, "/");
    }
}
