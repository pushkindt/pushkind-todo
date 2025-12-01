use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use super::types::{HubId, SearchTerm, TaskDescription, TaskId, TaskTitle, TaskTrack, UserId};

/// Status assigned to a task as it moves through its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskStatus {
    /// Task has been created but no work has started yet.
    #[default]
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

impl TaskStatus {
    /// Whether the status represents a terminal state where no additional work is required.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Archived)
    }
}

/// Priority assigned to a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum TaskPriority {
    /// Task is low priority and can be handled after other work.
    Low,
    /// Task is middle priority and should be addressed in a timely manner.
    #[default]
    Middle,
    /// Task is high priority and requires immediate attention.
    High,
}

impl From<TaskPriority> for &'static str {
    fn from(value: TaskPriority) -> Self {
        match value {
            TaskPriority::Low => "Low",
            TaskPriority::Middle => "Middle",
            TaskPriority::High => "High",
        }
    }
}

impl From<&str> for TaskPriority {
    fn from(value: &str) -> Self {
        match value {
            "Low" => TaskPriority::Low,
            "High" => TaskPriority::High,
            _ => TaskPriority::Middle,
        }
    }
}

/// Domain representation of a task managed by the service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier of the task.
    pub id: TaskId,
    /// Hub the task belongs to.
    pub hub_id: HubId,
    /// Short summary describing the task.
    pub title: TaskTitle,
    /// Optional detailed description for additional context.
    pub description: Option<TaskDescription>,
    /// Optional track categorization that the task belongs to.
    pub track: Option<TaskTrack>,
    /// Priority level assigned to the task.
    pub priority: TaskPriority,
    /// Current status for the task.
    pub status: TaskStatus,
    /// Optional due date for completing the task.
    pub due_date: Option<NaiveDate>,
    /// Identifier of the user assigned to the task, if any.
    pub assigned_to: Option<UserId>,
    /// Identifier of the user who created the task.
    pub author_id: UserId,
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
    pub hub_id: HubId,
    /// Title for the task.
    pub title: TaskTitle,
    /// Optional description providing more context.
    pub description: Option<TaskDescription>,
    /// Optional track categorization that the task belongs to.
    pub track: Option<TaskTrack>,
    /// Priority level for the task.
    pub priority: TaskPriority,
    /// Desired status for the task upon creation.
    pub status: TaskStatus,
    /// Optional due date.
    pub due_date: Option<NaiveDate>,
    /// Optional identifier for the assignee.
    pub assigned_to: Option<UserId>,
    /// Identifier of the user who created the task.
    pub author_id: UserId,
    /// Creation timestamp captured at the moment of building the payload.
    pub created_at: NaiveDateTime,
    /// Update timestamp captured at the moment of building the payload.
    pub updated_at: NaiveDateTime,
}

impl NewTask {
    /// Create a new task payload for the provided hub with the supplied title.
    pub fn new(hub_id: HubId, author_id: UserId, title: TaskTitle) -> Self {
        let now = chrono::Local::now().naive_utc();
        Self {
            hub_id,
            title,
            description: None,
            track: None,
            priority: TaskPriority::default(),
            status: TaskStatus::Pending,
            due_date: None,
            assigned_to: None,
            author_id,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set a description for the task.
    pub fn description(mut self, description: TaskDescription) -> Self {
        self.description = Some(description);
        self
    }

    /// Set a track for the task.
    pub fn track(mut self, track: TaskTrack) -> Self {
        self.track = Some(track);
        self
    }

    /// Remove the track information from the task.
    pub fn clear_track(mut self) -> Self {
        self.track = None;
        self
    }

    /// Set the priority for the task.
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
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
    pub fn assign_to(mut self, assignee_id: UserId) -> Self {
        self.assigned_to = Some(assignee_id);
        self
    }
}

/// Domain payload describing a task assignment event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Identifier of the task being assigned.
    pub task_id: TaskId,
    /// Hub that owns the task.
    pub hub_id: HubId,
    /// Identifier of the assignee.
    pub assignee_id: UserId,
    /// Timestamp when the assignment occurred.
    pub assigned_at: NaiveDateTime,
}

impl TaskAssignment {
    /// Create a new assignment payload using the current timestamp.
    pub fn new(task_id: TaskId, hub_id: HubId, assignee_id: UserId) -> Self {
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
    pub hub_id: HubId,
    /// Optional filter to only return tasks assigned to a user.
    pub assignee_id: Option<UserId>,
    /// Optional filter restricting tasks to a specific track.
    pub track: Option<TaskTrack>,
    /// Optional filter to limit results to a specific status.
    pub status: Option<TaskStatus>,
    /// Optional filter to restrict results to a priority level.
    pub priority: Option<TaskPriority>,
    /// Optional search term matching task titles or descriptions.
    pub search: Option<SearchTerm>,
    /// Only return tasks due on or before this date.
    pub due_before: Option<NaiveDate>,
    /// Only return tasks due on or after this date.
    pub due_after: Option<NaiveDate>,
    /// Only return tasks updated on or before this timestamp.
    pub updated_before: Option<NaiveDateTime>,
    /// Only return tasks updated on or after this timestamp.
    pub updated_after: Option<NaiveDateTime>,
}

impl TaskListFilters {
    /// Build a new filter set scoped to the provided hub.
    pub fn new(hub_id: HubId) -> Self {
        Self {
            hub_id,
            assignee_id: None,
            track: None,
            status: None,
            priority: None,
            search: None,
            due_before: None,
            due_after: None,
            updated_before: None,
            updated_after: None,
        }
    }

    /// Restrict the results to a specific assignee.
    pub fn for_assignee(mut self, assignee_id: UserId) -> Self {
        self.assignee_id = Some(assignee_id);
        self
    }

    /// Restrict the results to a specific track name.
    pub fn with_track(mut self, track: TaskTrack) -> Self {
        self.track = Some(track);
        self
    }

    /// Restrict the results to a particular status.
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Restrict the results to a particular priority level.
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Apply a free-text search filter.
    pub fn search(mut self, term: SearchTerm) -> Self {
        self.search = Some(term);
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

    /// Only return tasks updated on or before the provided timestamp.
    pub fn updated_before(mut self, timestamp: NaiveDateTime) -> Self {
        self.updated_before = Some(timestamp);
        self
    }

    /// Only return tasks updated on or after the provided timestamp.
    pub fn updated_after(mut self, timestamp: NaiveDateTime) -> Self {
        self.updated_after = Some(timestamp);
        self
    }
}

/// Patch payload used when updating an existing task.
#[derive(Debug, Clone)]
pub struct UpdateTask {
    /// New title value for the task.
    pub title: TaskTitle,
    /// New description content for the task.
    pub description: Option<TaskDescription>,
    /// Updated track information for the task.
    pub track: Option<TaskTrack>,
    /// Updated priority value.
    pub priority: TaskPriority,
    /// Updated status value.
    pub status: TaskStatus,
    /// Updated due date for the task.
    pub due_date: Option<NaiveDate>,
    /// Updated assignee for the task.
    pub assigned_to: Option<UserId>,
    /// Updated completion timestamp for the task.
    pub completed_at: Option<NaiveDateTime>,
    /// Timestamp when the update payload was constructed.
    pub updated_at: NaiveDateTime,
}

impl UpdateTask {
    /// Construct an update payload seeded with the current state of a task.
    pub fn from_task(task: &Task) -> Self {
        Self {
            title: task.title.clone(),
            description: task.description.clone(),
            track: task.track.clone(),
            priority: task.priority,
            status: task.status,
            due_date: task.due_date,
            assigned_to: task.assigned_to,
            completed_at: task.completed_at,
            updated_at: chrono::Local::now().naive_utc(),
        }
    }

    fn touch(&mut self) {
        self.updated_at = chrono::Local::now().naive_utc();
    }

    /// Replace the task title.
    pub fn title(mut self, title: TaskTitle) -> Self {
        self.touch();
        self.title = title;
        self
    }

    /// Update the description for the task.
    pub fn description(mut self, description: TaskDescription) -> Self {
        self.touch();
        self.description = Some(description);
        self
    }

    /// Clear the description field.
    pub fn clear_description(mut self) -> Self {
        self.touch();
        self.description = None;
        self
    }

    /// Update the track for the task.
    pub fn track(mut self, track: TaskTrack) -> Self {
        self.touch();
        self.track = Some(track);
        self
    }

    /// Remove the track information from the task.
    pub fn clear_track(mut self) -> Self {
        self.touch();
        self.track = None;
        self
    }

    /// Update the priority level for the task.
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.touch();
        self.priority = priority;
        self
    }

    /// Change the task status.
    pub fn status(mut self, status: TaskStatus) -> Self {
        self.touch();
        self.status = status;
        self
    }

    /// Set or update the due date.
    pub fn due_date(mut self, due_date: NaiveDate) -> Self {
        self.touch();
        self.due_date = Some(due_date);
        self
    }

    /// Remove the due date from the task.
    pub fn clear_due_date(mut self) -> Self {
        self.touch();
        self.due_date = None;
        self
    }

    /// Assign the task to a specific user.
    pub fn assign_to(mut self, assignee_id: UserId) -> Self {
        self.touch();
        self.assigned_to = Some(assignee_id);
        self
    }

    /// Remove the assignee from the task.
    pub fn unassign(mut self) -> Self {
        self.touch();
        self.assigned_to = None;
        self
    }

    /// Mark the task as completed at a specific time.
    pub fn completed_at(mut self, timestamp: NaiveDateTime) -> Self {
        self.touch();
        self.completed_at = Some(timestamp);
        self
    }

    /// Remove any stored completion timestamp.
    pub fn clear_completed_at(mut self) -> Self {
        self.touch();
        self.completed_at = None;
        self
    }
}
