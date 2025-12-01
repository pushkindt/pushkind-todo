//! Service layer root re-exporting shared error helpers and service submodules.
pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

impl From<crate::domain::types::TypeConstraintError> for ServiceError {
    /// Convert domain value-constraint failures into a generic service error.
    fn from(_: crate::domain::types::TypeConstraintError) -> Self {
        ServiceError::Internal
    }
}

mod notifications;

pub mod main;
pub mod mock;
pub mod task;
