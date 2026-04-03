//! Core Actix-Web application wiring the repository, routes, and notification services into a runnable server.

#[cfg(feature = "server")]
use actix_files::Files;
#[cfg(feature = "server")]
use actix_identity::IdentityMiddleware;
#[cfg(feature = "server")]
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
#[cfg(feature = "server")]
use actix_web::cookie::Key;
#[cfg(feature = "server")]
use actix_web::{App, HttpServer, middleware, web};
#[cfg(feature = "server")]
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
#[cfg(feature = "server")]
use pushkind_common::db::establish_connection_pool;
#[cfg(feature = "server")]
use pushkind_common::middleware::RedirectUnauthorized;
#[cfg(feature = "server")]
use pushkind_common::models::config::CommonServerConfig;
#[cfg(feature = "server")]
use pushkind_common::routes::logout;
#[cfg(feature = "server")]
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
#[cfg(feature = "server")]
use tera::Tera;

#[cfg(feature = "server")]
use crate::models::config::{ServerConfig, ZmqSenders};
#[cfg(feature = "server")]
use crate::repository::DieselRepository;
#[cfg(feature = "server")]
use crate::routes::api::{
    api_v1_add_task_comment, api_v1_clients, api_v1_create_task, api_v1_delete_task, api_v1_iam,
    api_v1_no_access, api_v1_task, api_v1_tasks, api_v1_tracks, api_v1_update_task,
    api_v1_update_task_status, api_v1_upload_tasks, api_v1_users,
};
#[cfg(feature = "server")]
use crate::routes::aux::not_assigned;
#[cfg(feature = "server")]
use crate::routes::main::{add_task, show_index, tasks_upload};
#[cfg(feature = "server")]
use crate::routes::task::{
    add_task_comment, delete_task, quick_update_task_status, show_task, task_modal, update_task,
};

#[cfg(feature = "data")]
pub mod domain;
#[cfg(feature = "data")]
pub mod dto;
#[cfg(feature = "server")]
pub mod error_conversions;
#[cfg(feature = "server")]
pub mod forms;
#[cfg(feature = "server")]
pub mod frontend;
#[cfg(feature = "data")]
pub mod models;
#[cfg(feature = "server")]
pub mod repository;
#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "data")]
pub mod schema;
#[cfg(feature = "server")]
pub mod services;

#[cfg(feature = "server")]
pub const SERVICE_ACCESS_ROLE: &str = "todo";

#[cfg(feature = "server")]
pub async fn run(server_config: ServerConfig) -> std::io::Result<()> {
    let common_config = CommonServerConfig {
        auth_service_url: server_config.auth_service_url.clone(),
        secret: server_config.secret.clone(),
    };

    // Start background ZeroMQ senders used for outbound notifications and integration events.
    let emailer_sender = ZmqSender::start(ZmqSenderOptions::pub_default(
        &server_config.zmq_emailer_pub,
    ))
    .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ email sender: {e}")))?;

    let task_sender = ZmqSender::start(ZmqSenderOptions::pub_default(&server_config.zmq_tasks_pub))
        .map_err(|e| {
            std::io::Error::other(format!("Failed to start ZMQ task-events sender: {e}"))
        })?;

    let zmq_senders = web::Data::new(ZmqSenders {
        emailer: emailer_sender,
        tasks: task_sender,
    });

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&server_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity, sessions, and flash messages.
    let secret_key = Key::from(server_config.secret.as_bytes());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let tera = Tera::new(&server_config.templates_dir)
        .map_err(|e| std::io::Error::other(format!("Template parsing error(s): {e}")))?;

    let bind_address = (server_config.address.clone(), server_config.port);

    HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", server_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(not_assigned)
            .service(
                web::scope("/api")
                    .service(api_v1_iam)
                    .service(api_v1_no_access)
                    .service(api_v1_tasks)
                    .service(api_v1_create_task)
                    .service(api_v1_upload_tasks)
                    .service(api_v1_task)
                    .service(api_v1_update_task)
                    .service(api_v1_update_task_status)
                    .service(api_v1_add_task_comment)
                    .service(api_v1_delete_task)
                    .service(api_v1_users)
                    .service(api_v1_clients)
                    .service(api_v1_tracks),
            )
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(show_index)
                    .service(add_task)
                    .service(tasks_upload)
                    .service(show_task)
                    .service(task_modal)
                    .service(add_task_comment)
                    .service(update_task)
                    .service(quick_update_task_status)
                    .service(delete_task)
                    .service(logout),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(server_config.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(zmq_senders.clone())
    })
    .bind(bind_address)?
    .run()
    .await
}
