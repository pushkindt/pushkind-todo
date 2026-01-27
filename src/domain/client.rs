//! Domain model describing CRM clients.

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::domain::types::{ClientId, ClientName, ClientPublicId, HubId, TypeConstraintError};

/// Represent a trusted CRM client stored in the system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Client {
    pub id: ClientId,
    pub public_id: ClientPublicId,
    pub hub_id: HubId,
    pub name: ClientName,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl Client {
    /// Create a trusted client from already validated domain values.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ClientId,
        public_id: ClientPublicId,
        hub_id: HubId,
        name: ClientName,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
    ) -> Self {
        Self {
            id,
            public_id,
            hub_id,
            name,
            created_at,
            updated_at,
        }
    }

    /// Create a client from raw values, validating identifiers and inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: i32,
        public_id: String,
        hub_id: i32,
        name: String,
        created_at: NaiveDateTime,
        updated_at: NaiveDateTime,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            ClientId::try_from(id)?,
            ClientPublicId::new(public_id)?,
            HubId::try_from(hub_id)?,
            ClientName::new(name)?,
            created_at,
            updated_at,
        ))
    }
}

/// Data required to persist a new client record.
#[derive(Clone, Debug, Deserialize)]
pub struct NewClient {
    pub public_id: ClientPublicId,
    pub hub_id: HubId,
    pub name: ClientName,
}

impl NewClient {
    /// Create a new client from already validated domain values.
    #[must_use]
    pub fn new(hub_id: HubId, name: ClientName, public_id: ClientPublicId) -> Self {
        Self {
            public_id,
            hub_id,
            name,
        }
    }

    /// Create a new client from raw inputs, validating identifiers and values.
    pub fn try_new(
        hub_id: i32,
        name: String,
        public_id: String,
    ) -> Result<Self, TypeConstraintError> {
        Ok(Self::new(
            HubId::try_from(hub_id)?,
            ClientName::new(name)?,
            ClientPublicId::new(public_id)?,
        ))
    }
}
