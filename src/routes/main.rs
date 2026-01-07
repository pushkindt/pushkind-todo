//! UI routes handling the main index, task creation, and file upload workflows.
use std::sync::Arc;

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::ZmqSender;
use tera::Tera;

use crate::dto::main::IndexQuery;
use crate::forms::main::{AddTaskForm, UploadTasksForm};
use crate::repository::DieselRepository;
use crate::services::{ServiceError, main as main_service};

/// Display the main index page showing tasks, filters, and flash messages.
#[get("/")]
pub async fn show_index(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match main_service::load_index_page(params.into_inner(), &user, repo.get_ref()) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );
            context.insert("tasks", &data.tasks);
            context.insert("templates", &data.tasks); // temporary alias while templates migrate
            context.insert("filters", &data.filters);
            context.insert("users", &data.users);
            context.insert("clients", &data.clients);
            context.insert("recently_updated_task_ids", &data.recently_updated_task_ids);
            context.insert("tracks", &data.tracks);
            render_template(&tera, "main/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list tasks: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

/// Handle task creation submissions and provide flash feedback.
#[post("/task/add")]
pub async fn add_task(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    web::Form(form): web::Form<AddTaskForm>,
) -> impl Responder {
    let zmq_sender = zmq_sender.get_ref().as_ref();
    match main_service::add_task(form, &user, repo.get_ref(), zmq_sender) {
        Ok(_) => {
            FlashMessage::success("Задача добавлена.").send();
            redirect("/")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to add a task: {err}");
            FlashMessage::error("Ошибка при добавлении задачи").send();
            redirect("/")
        }
    }
}

/// Accept a CSV upload of tasks, process it, and flash results.
#[post("/tasks/upload")]
pub async fn tasks_upload(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(form): MultipartForm<UploadTasksForm>,
) -> impl Responder {
    match main_service::upload_tasks(form, &user, repo.get_ref()) {
        Ok(created_count) => {
            FlashMessage::success(format!("Добавлено задач: {created_count}")).send();
            redirect("/")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to add tasks: {err}");
            FlashMessage::error("Ошибка при добавлении задач").send();
            redirect("/")
        }
    }
}
