//! Error conversion glue for `data` feature consumers.
//!
//! The domain layer must not depend on service/repository error types, but
//! downstream crates using `pushkind-emailer` with only the `data` feature may
//! still want convenient conversions.
use pushkind_common::repository::errors::RepositoryError;
use pushkind_common::services::errors::ServiceError;

use crate::domain::types::TypeConstraintError;
use crate::forms::FormError;

impl From<TypeConstraintError> for ServiceError {
    fn from(val: TypeConstraintError) -> Self {
        ServiceError::TypeConstraint(val.to_string())
    }
}

impl From<FormError> for ServiceError {
    fn from(val: FormError) -> Self {
        ServiceError::Form(val.to_string())
    }
}

impl From<TypeConstraintError> for RepositoryError {
    fn from(val: TypeConstraintError) -> Self {
        RepositoryError::ValidationError(val.to_string())
    }
}
