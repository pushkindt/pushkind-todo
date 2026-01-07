//! Configuration model loaded from external sources.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
/// Basic configuration shared across handlers.
pub struct ServerConfig {
    pub domain: String,
    pub address: String,
    pub port: u16,
    pub database_url: String,
    pub zmq_emailer_pub: String,
    pub templates_dir: String,
    pub secret: String,
    pub auth_service_url: String,
    pub crm_service_url: String,
}
