//! UI routes handling the main index, task creation, and file upload workflows.
use std::path::Path;

use actix_multipart::form::MultipartForm;
use actix_web::{Either, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::FlashMessage;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{ensure_role, redirect};

use crate::SERVICE_ACCESS_ROLE;
use crate::forms::main::{AddTaskForm, AddTaskPayload, UploadTasksForm};
use crate::frontend::{FRONTEND_DIST_DIR, FRONTEND_INDEX_DOCUMENT, open_frontend_html};
use crate::models::config::ZmqSenders;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, main as main_service};

/// Display the React-backed main index page after access checks succeed.
#[get("/")]
pub async fn show_index(user: AuthenticatedUser) -> impl Responder {
    match ensure_role(&user, SERVICE_ACCESS_ROLE) {
        Ok(_) => {}
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return Either::Right(redirect("/na"));
        }
        Err(err) => {
            log::error!("Failed to authorize list page access: {err}");
            return Either::Right(HttpResponse::InternalServerError().finish());
        }
    }

    let index_document = Path::new(FRONTEND_DIST_DIR).join(FRONTEND_INDEX_DOCUMENT);
    match open_frontend_html(&index_document).await {
        Ok(document) => Either::Left(document),
        Err(err) => {
            log::error!("Failed to open built index document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
        }
    }
}

/// Handle task creation submissions and provide flash feedback.
#[post("/task/add")]
pub async fn add_task(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    web::Form(form): web::Form<AddTaskForm>,
) -> impl Responder {
    let zmq_senders = zmq_senders.get_ref();
    let payload = match AddTaskPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => {
            FlashMessage::error(err.to_string()).send();
            return redirect("/");
        }
    };

    match main_service::add_task(
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
        &zmq_senders.tasks,
    ) {
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
    let payload = match form.try_into_payload() {
        Ok(payload) => payload,
        Err(err) => {
            FlashMessage::error(err.to_string()).send();
            return redirect("/");
        }
    };

    match main_service::upload_tasks(payload, &user, repo.get_ref()) {
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
