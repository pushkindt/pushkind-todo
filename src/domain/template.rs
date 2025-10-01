use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

/// Domain representation of a template record that belongs to a hub.
#[derive(Serialize, Deserialize, Clone)]
pub struct Template {
    pub id: i32,
    pub hub_id: i32,
    pub value: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Parameters required to insert a new template into the database.
pub struct NewTemplate {
    pub hub_id: i32,
    pub value: Option<String>,
    pub updated_at: NaiveDateTime,
}

impl NewTemplate {
    /// Build a new template payload for the given hub, capturing the current timestamp.
    pub fn new(value: Option<String>, hub_id: i32) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            value,
            updated_at: now,
        }
    }
}

/// Patch data applied when updating an existing template.
pub struct UpdateTemplate {
    pub value: Option<String>,
    pub updated_at: NaiveDateTime,
}

impl UpdateTemplate {
    /// Build an update payload with the new value and current timestamp.
    pub fn new(value: Option<String>) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            value,
            updated_at: now,
        }
    }
}
