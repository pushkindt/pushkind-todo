pub mod main;

pub use pushkind_common::services::errors::{ServiceError, ServiceResult};

/// Successful service outcome that carries a flash message and redirect target.
#[derive(Debug, Clone)]
pub struct RedirectSuccess {
    /// Message displayed to the end user after the redirect.
    pub message: String,
    /// Target location for the subsequent redirect.
    pub redirect_to: String,
}
