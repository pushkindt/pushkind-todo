//! Service layer root re-exporting shared error helpers and service submodules.
pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

mod notifications;

pub mod main;
pub mod mock;
pub mod task;
