use pushkind_common::domain::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

/// Domain representation of a user belonging to a specific hub.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct User {
    /// Unique identifier for the user.
    pub id: i32,
    /// Hub the user is associated with.
    pub hub_id: i32,
    /// Display name of the user.
    pub name: String,
    /// Primary email address used for authentication and notifications.
    pub email: String,
    /// Timestamp of last activity by the user.
    pub visited_at: Option<chrono::NaiveDateTime>,
}

/// Parameters required to create a new user.
#[derive(Clone, Debug, Deserialize)]
pub struct NewUser {
    /// Hub that should own the new user.
    pub hub_id: i32,
    /// Display name for the new user.
    pub name: String,
    /// Email address used to identify and contact the user.
    pub email: String,
}

impl NewUser {
    #[must_use]
    pub fn new(hub_id: i32, name: String, email: String) -> Self {
        Self {
            hub_id,
            name,
            email: email.to_lowercase(),
        }
    }
}

/// Payload for updating mutable user fields.
#[derive(Clone, Debug, Deserialize)]
pub struct UpdateUser {
    /// Updated display name for the user.
    pub name: String,
}

impl From<&AuthenticatedUser> for NewUser {
    fn from(value: &AuthenticatedUser) -> Self {
        NewUser::new(value.hub_id, value.name.clone(), value.email.clone())
    }
}
