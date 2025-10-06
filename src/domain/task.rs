use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

/// Status assigned to a task as it moves through its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task has been created but no work has started yet.
    Pending,
    /// Work is actively happening on the task.
    InProgress,
    /// Task is blocked and cannot move forward until the blocker is resolved.
    Blocked,
    /// Task has been completed successfully.
    Completed,
    /// Task is archived and should no longer be modified.
    Archived,
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl TaskStatus {
    /// Whether the status represents a terminal state where no additional work is required.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Archived)
    }
}

/// Domain representation of a task managed by the service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier of the task.
    pub id: i32,
    /// Hub the task belongs to.
    pub hub_id: i32,
    /// Short summary describing the task.
    pub title: String,
    /// Optional detailed description for additional context.
    pub description: Option<String>,
    /// Current status for the task.
    pub status: TaskStatus,
    /// Optional due date for completing the task.
    pub due_date: Option<NaiveDate>,
    /// Identifier of the user assigned to the task, if any.
    pub assigned_to: Option<i32>,
    /// Identifier of the user who created the task.
    pub author_id: i32,
    /// Timestamp for when the task was created.
    pub created_at: NaiveDateTime,
    /// Timestamp for the most recent update.
    pub updated_at: NaiveDateTime,
    /// When the task was completed, if it has been finished.
    pub completed_at: Option<NaiveDateTime>,
}

impl Task {
    /// Returns `true` when the task is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

/// Parameters required to create a new task.
#[derive(Debug, Clone)]
pub struct NewTask {
    /// Hub that should own the new task.
    pub hub_id: i32,
    /// Title for the task.
    pub title: String,
    /// Optional description providing more context.
    pub description: Option<String>,
    /// Desired status for the task upon creation.
    pub status: TaskStatus,
    /// Optional due date.
    pub due_date: Option<NaiveDate>,
    /// Optional identifier for the assignee.
    pub assigned_to: Option<i32>,
    /// Identifier of the user who created the task.
    pub author_id: i32,
    /// Creation timestamp captured at the moment of building the payload.
    pub created_at: NaiveDateTime,
    /// Update timestamp captured at the moment of building the payload.
    pub updated_at: NaiveDateTime,
}

impl NewTask {
    /// Create a new task payload for the provided hub with the supplied title.
    pub fn new(hub_id: i32, author_id: i32, title: impl Into<String>) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            title: title.into(),
            description: None,
            status: TaskStatus::Pending,
            due_date: None,
            assigned_to: None,
            author_id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set a description for the task.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the desired status for the task on creation.
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    /// Set a due date for the task.
    pub fn due_date(mut self, due_date: NaiveDate) -> Self {
        self.due_date = Some(due_date);
        self
    }

    /// Assign the task to a specific user during creation.
    pub fn assign_to(mut self, assignee_id: i32) -> Self {
        self.assigned_to = Some(assignee_id);
        self
    }
}

/// Domain payload describing a task assignment event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Identifier of the task being assigned.
    pub task_id: i32,
    /// Hub that owns the task.
    pub hub_id: i32,
    /// Identifier of the assignee.
    pub assignee_id: i32,
    /// Timestamp when the assignment occurred.
    pub assigned_at: NaiveDateTime,
}

impl TaskAssignment {
    /// Create a new assignment payload using the current timestamp.
    pub fn new(task_id: i32, hub_id: i32, assignee_id: i32) -> Self {
        Self {
            task_id,
            hub_id,
            assignee_id,
            assigned_at: chrono::Local::now().naive_utc(),
        }
    }
}

impl From<TaskStatus> for &'static str {
    fn from(value: TaskStatus) -> Self {
        match value {
            TaskStatus::Pending => "Pending",
            TaskStatus::InProgress => "InProgress",
            TaskStatus::Blocked => "Blocked",
            TaskStatus::Completed => "Completed",
            TaskStatus::Archived => "Archived",
        }
    }
}

impl From<&str> for TaskStatus {
    fn from(value: &str) -> Self {
        match value {
            "Pending" => TaskStatus::Pending,
            "InProgress" => TaskStatus::InProgress,
            "Blocked" => TaskStatus::Blocked,
            "Completed" => TaskStatus::Completed,
            "Archived" => TaskStatus::Archived,
            _ => TaskStatus::Pending,
        }
    }
}

/// Filters applied when listing tasks.
#[derive(Debug, Clone)]
pub struct TaskListFilters {
    /// Hub that scopes the query.
    pub hub_id: i32,
    /// Optional filter to only return tasks assigned to a user.
    pub assignee_id: Option<i32>,
    /// Optional filter to limit results to a specific status.
    pub status: Option<TaskStatus>,
    /// Optional search term matching task titles or descriptions.
    pub search: Option<String>,
    /// Only return tasks due on or before this date.
    pub due_before: Option<NaiveDate>,
    /// Only return tasks due on or after this date.
    pub due_after: Option<NaiveDate>,
}

impl TaskListFilters {
    /// Build a new filter set scoped to the provided hub.
    pub fn new(hub_id: i32) -> Self {
        Self {
            hub_id,
            assignee_id: None,
            status: None,
            search: None,
            due_before: None,
            due_after: None,
        }
    }

    /// Restrict the results to a specific assignee.
    pub fn for_assignee(mut self, assignee_id: i32) -> Self {
        self.assignee_id = Some(assignee_id);
        self
    }

    /// Restrict the results to a particular status.
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Apply a free-text search filter.
    pub fn search(mut self, term: impl Into<String>) -> Self {
        self.search = Some(term.into());
        self
    }

    /// Only return tasks due on or before the provided date.
    pub fn due_before(mut self, date: NaiveDate) -> Self {
        self.due_before = Some(date);
        self
    }

    /// Only return tasks due on or after the provided date.
    pub fn due_after(mut self, date: NaiveDate) -> Self {
        self.due_after = Some(date);
        self
    }
}

/// Patch payload used when updating an existing task.
#[derive(Debug, Clone)]
pub struct UpdateTask {
    /// Optional new title.
    pub title: Option<String>,
    /// Optional new description (inner `None` clears the description).
    pub description: Option<Option<String>>,
    /// Optional new status value.
    pub status: Option<TaskStatus>,
    /// Optional due date change (inner `None` clears the due date).
    pub due_date: Option<Option<NaiveDate>>,
    /// Optional change to the assigned user (inner `None` unassigns the task).
    pub assigned_to: Option<Option<i32>>,
    /// Optional change to the completion timestamp (inner `None` clears the timestamp).
    pub completed_at: Option<Option<NaiveDateTime>>,
    /// Timestamp when the update payload was constructed.
    pub updated_at: NaiveDateTime,
}

impl UpdateTask {
    /// Construct an empty update payload with the current timestamp.
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            status: None,
            due_date: None,
            assigned_to: None,
            completed_at: None,
            updated_at: chrono::Local::now().naive_utc(),
        }
    }

    fn touch(&mut self) {
        self.updated_at = chrono::Local::now().naive_utc();
    }

    /// Replace the task title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.touch();
        self.title = Some(title.into());
        self
    }

    /// Update the description for the task.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.touch();
        self.description = Some(Some(description.into()));
        self
    }

    /// Clear the description field.
    pub fn clear_description(mut self) -> Self {
        self.touch();
        self.description = Some(None);
        self
    }

    /// Change the task status.
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.touch();
        self.status = Some(status);
        self
    }

    /// Set or update the due date.
    pub fn due_date(mut self, due_date: NaiveDate) -> Self {
        self.touch();
        self.due_date = Some(Some(due_date));
        self
    }

    /// Remove the due date from the task.
    pub fn clear_due_date(mut self) -> Self {
        self.touch();
        self.due_date = Some(None);
        self
    }

    /// Assign the task to a specific user.
    pub fn assign_to(mut self, assignee_id: i32) -> Self {
        self.touch();
        self.assigned_to = Some(Some(assignee_id));
        self
    }

    /// Remove the assignee from the task.
    pub fn unassign(mut self) -> Self {
        self.touch();
        self.assigned_to = Some(None);
        self
    }

    /// Mark the task as completed at a specific time.
    pub fn completed_at(mut self, timestamp: NaiveDateTime) -> Self {
        self.touch();
        self.completed_at = Some(Some(timestamp));
        self
    }

    /// Remove any stored completion timestamp.
    pub fn clear_completed_at(mut self) -> Self {
        self.touch();
        self.completed_at = Some(None);
        self
    }
}

impl Default for UpdateTask {
    fn default() -> Self {
        Self::new()
    }
}
