//! Application entry point building the Actix-Web server.
use std::env;

use config::Config;
use dotenvy::dotenv;

use pushkind_todo::{models::config::Settings, run};

#[actix_web::main]
async fn main() {
    // Load environment variables from `.env` in local development.
    dotenv().ok();
    // Initialize logger with default level INFO if not provided.
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    // Select config profile (defaults to `local`).
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "local".into());

    let settings = Config::builder()
        // Add `./config/default.yaml`
        .add_source(config::File::with_name("config/default"))
        // Add environment-specific overrides
        .add_source(config::File::with_name(&format!("config/{}", app_env)).required(false))
        // Add settings from the environment (with a prefix of APP)
        .add_source(config::Environment::with_prefix("APP"))
        .build();

    let settings = match settings {
        Ok(settings) => settings,
        Err(err) => {
            log::error!("Error loading settings: {}", err);
            std::process::exit(1);
        }
    };

    let settings = match settings.try_deserialize::<Settings>() {
        Ok(settings) => settings,
        Err(err) => {
            log::error!("Error loading server config: {}", err);
            std::process::exit(1);
        }
    };

    match run(settings).await {
        Ok(_) => log::info!("Server started"),
        Err(err) => {
            log::error!("Error starting server: {}", err);
            std::process::exit(1);
        }
    }
}
