//! Forms entrypoint re-exporting the main entry and task-related form modules.
//!
//! The [`FormError`] enum provides a shared set of field identifiers for
//! reporting validation failures consistently across different form payloads.
use thiserror::Error;
use validator::ValidationErrors;

pub mod main;
pub mod task;

#[derive(Debug, Error)]
/// Errors that can occur when processing form data.
pub enum FormError {
    #[error("validation errors: {0}")]
    Validation(#[from] ValidationErrors),
    #[error("invalid task title")]
    InvalidTitle,
    #[error("invalid task description")]
    InvalidDescription,
    #[error("invalid task due date")]
    InvalidDueDate,
    #[error("invalid task status")]
    InvalidStatus,
    #[error("invalid task track")]
    InvalidTrack,
    #[error("invalid task priority")]
    InvalidPriority,
    #[error("invalid task assignee name")]
    InvalidAssigneeName,
    #[error("invalid task assignee email")]
    InvalidAssigneeEmail,
    #[error("invalid comment")]
    InvalidComment,
    #[error("invalid assign_self option")]
    InvalidAssignSelf,
    #[error("invalid csv file")]
    InvalidCsv,
}
