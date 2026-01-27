//! DTO module exposing index and task payload helpers used by services and handlers.
#[cfg(feature = "server")]
pub mod main;
#[cfg(feature = "server")]
pub mod task;
pub mod zmq;
