pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

impl From<crate::domain::types::TypeConstraintError> for ServiceError {
    fn from(_: crate::domain::types::TypeConstraintError) -> Self {
        ServiceError::Internal
    }
}

mod notifications;

pub mod main;
pub mod mock;
pub mod task;
