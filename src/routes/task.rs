use actix_web::{HttpResponse, Responder, get, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::repository::DieselRepository;
use crate::services::{ServiceError, task as task_service};

#[get("/task/{task_id}")]
pub async fn show_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let task_id = task_id.into_inner();

    match task_service::load_task_details(repo.get_ref(), &user, task_id) {
        Ok(details) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "task",
                &server_config.auth_service_url,
            );
            context.insert("task", &details.task);
            context.insert("author", &details.author);
            context.insert("assignee", &details.assignee);
            context.insert("events", &details.events);
            render_template(&tera, "task/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load task {task_id}: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
