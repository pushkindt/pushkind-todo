//! Configuration model loaded from external sources.

use pushkind_common::zmq::ZmqSender;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
/// Top-level settings structure shared with the executable entrypoint.
pub struct Settings {
    pub server: ServerConfig,
    pub app: AppConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppConfig {
    pub domain: String,
    pub database_url: String,
    pub zmq_emailer_pub: String,
    pub zmq_tasks_pub: String,
    pub secret: String,
    pub auth_service_url: String,
    pub crm_service_url: String,
    pub files_service_url: String,
}

/// Collection of ZeroMQ senders used by various services.
pub struct ZmqSenders {
    pub emailer: ZmqSender,
    pub tasks: ZmqSender,
}
