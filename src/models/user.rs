//! Diesel user models bridging persisted user records with domain conversions.
use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    types::{HubId, UserEmail, UserId, UserName},
    user::{NewUser as DomainNewUser, UpdateUser as DomainUpdateUser, User as DomainUser},
};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub visited_at: Option<NaiveDateTime>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub hub_id: i32,
    pub name: &'a str,
    pub email: &'a str,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = crate::schema::users)]
pub struct UpdateUser<'a> {
    pub name: Option<&'a str>,
    pub updated_at: NaiveDateTime,
}

impl TryFrom<User> for DomainUser {
    type Error = crate::domain::types::TypeConstraintError;

    fn try_from(value: User) -> Result<Self, Self::Error> {
        Ok(Self {
            id: UserId::new(value.id)?,
            hub_id: HubId::new(value.hub_id)?,
            name: UserName::new(value.name)?,
            email: UserEmail::new(value.email)?,
            visited_at: value.visited_at,
        })
    }
}

impl<'a> From<&'a DomainNewUser> for NewUser<'a> {
    fn from(value: &'a DomainNewUser) -> Self {
        Self {
            hub_id: value.hub_id.get(),
            name: value.name.as_str(),
            email: value.email.as_str(),
        }
    }
}

impl<'a> From<&'a DomainNewUser> for UpdateUser<'a> {
    fn from(value: &'a DomainNewUser) -> Self {
        Self {
            name: Some(value.name.as_str()),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}

impl<'a> From<&'a DomainUpdateUser> for UpdateUser<'a> {
    fn from(value: &'a DomainUpdateUser) -> Self {
        Self {
            name: Some(value.name.as_str()),
            updated_at: chrono::Utc::now().naive_utc(),
        }
    }
}
