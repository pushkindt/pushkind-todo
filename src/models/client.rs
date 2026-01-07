//! Diesel client models bridging persisted client records with domain conversions.
use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::domain::{
    client::{Client as DomainClient, NewClient as DomainNewClient},
    types::{ClientId, ClientName, HubId, PublicId, TypeConstraintError},
};

#[derive(Debug, Clone, Identifiable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::clients)]
pub struct Client {
    pub id: i32,
    pub public_id: String,
    pub hub_id: i32,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = crate::schema::clients)]
pub struct NewClient<'a> {
    pub public_id: &'a str,
    pub hub_id: i32,
    pub name: &'a str,
}

impl TryFrom<Client> for DomainClient {
    type Error = TypeConstraintError;

    fn try_from(value: Client) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ClientId::new(value.id)?,
            public_id: PublicId::new(value.public_id)?,
            hub_id: HubId::new(value.hub_id)?,
            name: ClientName::new(value.name)?,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
    }
}

impl<'a> From<&'a DomainNewClient> for NewClient<'a> {
    fn from(value: &'a DomainNewClient) -> Self {
        Self {
            public_id: value.public_id.as_str(),
            hub_id: value.hub_id.get(),
            name: value.name.as_str(),
        }
    }
}
