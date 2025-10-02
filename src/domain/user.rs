use pushkind_common::domain::auth::AuthenticatedUser;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
pub struct User {
    pub id: i32,
    pub hub_id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NewUser {
    pub hub_id: i32,
    pub name: String,
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

#[derive(Clone, Debug, Deserialize)]
pub struct UpdateUser {
    pub name: String,
}

impl From<&AuthenticatedUser> for NewUser {
    fn from(value: &AuthenticatedUser) -> Self {
        NewUser::new(value.hub_id, value.name.clone(), value.email.clone())
    }
}
