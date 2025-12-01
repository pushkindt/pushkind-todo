use crate::domain::task::Task;
use crate::domain::user::User;
use pushkind_common::pagination::Paginated;
use serde::{Deserialize, Serialize};

/// Query parameters accepted by the index page service.
#[derive(Debug, Default, Deserialize)]
pub struct IndexQuery {
    /// Optional search string entered by the user.
    pub search: Option<String>,
    /// Page number requested by the user interface.
    pub page: Option<usize>,
    /// Optional status filter provided by the user.
    pub status: Option<String>,
    /// Optional track filter provided by the user.
    pub track: Option<String>,
    /// Optional assignee identifier filter provided by the user.
    pub assignee: Option<String>,
    /// Optional priority filter provided by the user.
    pub priority: Option<String>,
    /// Only return tasks updated on or after this date (YYYY-MM-DD).
    pub updated_after: Option<String>,
    /// Only return tasks updated on or before this date (YYYY-MM-DD).
    pub updated_before: Option<String>,
}

/// Data required to render the main index tasks page.
#[derive(Debug, Serialize)]
pub struct IndexPageFilters {
    /// Search query echoed back to the template when present.
    pub search: Option<String>,
    /// Status filter echoed back to the template when present.
    pub status: Option<String>,
    /// Track filter echoed back to the template when present.
    pub track: Option<String>,
    /// Assignee filter echoed back to the template when present.
    pub assignee: Option<String>,
    /// Priority filter echoed back to the template when present.
    pub priority: Option<String>,
    /// Updated-after filter echoed back to the template when present.
    pub updated_after: Option<String>,
    /// Updated-before filter echoed back to the template when present.
    pub updated_before: Option<String>,
}

pub struct IndexPageData {
    /// Paginated list of tasks to show in the table.
    pub tasks: Paginated<IndexTask>,
    /// Filters currently applied to the task list.
    pub filters: IndexPageFilters,
    /// Users available in the current hub.
    pub users: Vec<User>,
    /// Task identifiers that were updated after the user's last visit.
    pub recently_updated_task_ids: Vec<i32>,
    /// Available task tracks to use for hints
    pub tracks: Vec<String>,
}

/// Task metadata displayed on the index page alongside assignee info.
#[derive(Debug, Serialize)]
pub struct IndexTask {
    /// Task details shown in the list.
    pub task: Task,
    /// Assignee of the task when available in the current hub.
    pub assignee: Option<User>,
}
