use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::template::{
    NewTemplate as DomainNewTemplate, Template as DomainTemplate,
    UpdateTemplate as DomainUpdateTemplate,
};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::templates)]
pub struct Template {
    pub id: i32,
    pub hub_id: i32,
    pub value: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::templates)]
pub struct NewTemplate<'a> {
    pub hub_id: i32,
    pub value: Option<&'a str>,
    pub updated_at: NaiveDateTime,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::schema::templates)]
pub struct UpdateTemplate<'a> {
    pub value: Option<&'a str>,
    pub updated_at: NaiveDateTime,
}

impl From<Template> for DomainTemplate {
    fn from(value: Template) -> Self {
        Self {
            id: value.id,
            hub_id: value.hub_id,
            value: value.value,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl<'a> From<&'a DomainNewTemplate> for NewTemplate<'a> {
    fn from(value: &'a DomainNewTemplate) -> Self {
        Self {
            hub_id: value.hub_id,
            value: value.value.as_deref(),
            updated_at: value.updated_at,
        }
    }
}

impl<'a> From<&'a DomainUpdateTemplate> for UpdateTemplate<'a> {
    fn from(value: &'a DomainUpdateTemplate) -> Self {
        Self {
            value: value.value.as_deref(),
            updated_at: value.updated_at,
        }
    }
}
