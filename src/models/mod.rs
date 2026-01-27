//! Diesel model module grouping configuration, task, event, and user structures.
pub mod client;
#[cfg(feature = "server")]
pub mod config;
pub mod task;
pub mod task_event;
pub mod user;
