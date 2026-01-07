//! DTO definitions for the index page queries and view payloads.
use chrono::NaiveDate;
use pushkind_common::pagination::Paginated;
use pushkind_common::routes::empty_string_as_none_fromstr;
use serde::{Deserialize, Serialize};

use crate::domain::client::Client;
use crate::domain::task::{TaskPriority, TaskStatus};
use crate::domain::types::{ClientId, TaskId, UserId};
use crate::domain::user::User;
use crate::domain::{task::Task, types::TaskTrack};

/// Query parameters accepted by the index page service.
#[derive(Debug, Default, Deserialize)]
pub struct IndexQuery {
    /// Optional search string entered by the user.
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub search: Option<String>,
    /// Page number requested by the user interface.
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub page: Option<usize>,
    /// Optional status filter provided by the user.
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub status: Option<String>,
    /// Optional track filter provided by the user.
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub track: Option<String>,
    /// Optional assignee identifier filter provided by the user.
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub assignee: Option<i32>,
    /// Optional client identifier filter provided by the user.
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub client: Option<i32>,
    /// Optional priority filter provided by the user.
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub priority: Option<String>,
    /// Only return tasks updated on or after this date (YYYY-MM-DD).
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub updated_after: Option<String>,
    /// Only return tasks updated on or before this date (YYYY-MM-DD).
    #[serde(default, deserialize_with = "empty_string_as_none_fromstr")]
    pub updated_before: Option<String>,
}

/// Data required to render the main index tasks page.
#[derive(Debug, Serialize)]
pub struct IndexPageFilters {
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
    /// Status filter echoed back to the template when present.
    pub status: Option<TaskStatus>,
    /// Track filter echoed back to the template when present.
    pub track: Option<TaskTrack>,
    /// Assignee filter echoed back to the template when present.
    pub assignee: Option<UserId>,
    /// Client filter echoed back to the template when present.
    pub client: Option<ClientId>,
    /// Priority filter echoed back to the template when present.
    pub priority: Option<TaskPriority>,
    /// Updated-after filter echoed back to the template when present.
    pub updated_after: Option<NaiveDate>,
    /// Updated-before filter echoed back to the template when present.
    pub updated_before: Option<NaiveDate>,
}

pub struct IndexPageData {
    /// Paginated list of tasks to show in the table.
    pub tasks: Paginated<IndexTask>,
    /// Filters currently applied to the task list.
    pub filters: IndexPageFilters,
    /// Users available in the current hub.
    pub users: Vec<User>,
    /// Task identifiers that were updated after the user's last visit.
    pub recently_updated_task_ids: Vec<TaskId>,
    /// Available task tracks to use for hints
    pub tracks: Vec<TaskTrack>,
    /// Clients available in the current hub.
    pub clients: Vec<Client>,
}

/// Task metadata displayed on the index page alongside assignee info.
#[derive(Debug, Serialize)]
pub struct IndexTask {
    /// Task details shown in the list.
    pub task: Task,
    /// Assignee of the task when available in the current hub.
    pub assignee: Option<User>,
}
