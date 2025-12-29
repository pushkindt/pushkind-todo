//! Domain user structures representing persisted users and creation/update payloads.
use pushkind_common::domain::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

use super::types::{HubId, TypeConstraintError, UserEmail, UserId, UserName};

/// Domain representation of a user belonging to a specific hub.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct User {
    /// Unique identifier for the user.
    pub id: UserId,
    /// Hub the user is associated with.
    pub hub_id: HubId,
    /// Display name of the user.
    pub name: UserName,
    /// Primary email address used for authentication and notifications.
    pub email: UserEmail,
    /// Timestamp of last activity by the user.
    pub visited_at: Option<chrono::NaiveDateTime>,
}

/// Parameters required to create a new user.
#[derive(Clone, Debug, Deserialize)]
pub struct NewUser {
    /// Hub that should own the new user.
    pub hub_id: HubId,
    /// Display name for the new user.
    pub name: UserName,
    /// Email address used to identify and contact the user.
    pub email: UserEmail,
}

impl NewUser {
    /// Creates a new user payload from validated domain values.
    #[must_use]
    pub fn new(hub_id: HubId, name: UserName, email: UserEmail) -> Self {
        Self {
            hub_id,
            name,
            email,
        }
    }

    /// Attempts to construct a new user payload from raw input values.
    pub fn try_new<N, E>(hub_id: i32, name: N, email: E) -> Result<Self, TypeConstraintError>
    where
        N: Into<String>,
        E: Into<String>,
    {
        let hub_id = HubId::new(hub_id)?;
        let name = UserName::new(name)?;
        let email = UserEmail::new(email)?;

        Ok(Self::new(hub_id, name, email))
    }
}

/// Payload for updating mutable user fields.
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateUser {
    /// Updated display name for the user.
    pub name: UserName,
}

impl TryFrom<&AuthenticatedUser> for NewUser {
    type Error = TypeConstraintError;

    fn try_from(value: &AuthenticatedUser) -> Result<Self, Self::Error> {
        NewUser::try_new(value.hub_id, &value.name, &value.email)
    }
}
