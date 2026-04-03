//! Service layer root re-exporting shared error helpers and service submodules.
pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

pub mod api;
pub mod main;
pub mod mock;
pub mod notifications;
pub mod task;
