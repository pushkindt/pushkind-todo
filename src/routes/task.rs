use std::sync::Arc;

use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use pushkind_common::zmq::ZmqSender;
use tera::{Context, Tera};

use crate::forms::task::{NewTaskCommentForm, UpdateTaskForm};
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

#[post("/task/modal/{task_id}")]
pub async fn task_modal(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match task_service::load_task_modal(repo.get_ref(), &user, task_id.into_inner()) {
        Ok(data) => {
            let mut context = Context::new();
            context.insert("task", &data.task);
            context.insert("users", &data.users);
            context.insert("assignee", &data.assignee);
            context.insert("home_url", &server_config.auth_service_url);
            render_template(&tera, "task/modal_body.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            log::error!("Unauthorized to load task modal.");
            HttpResponse::Unauthorized().finish()
        }
        Err(ServiceError::NotFound) => HttpResponse::InternalServerError().finish(),
        Err(err) => {
            log::error!("Failed to load task modal: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/task/{task_id}/update")]
pub async fn update_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_sender: web::Data<Arc<ZmqSender>>,
    web::Form(form): web::Form<UpdateTaskForm>,
) -> impl Responder {
    let task_id = task_id.into_inner();

    match task_service::update_task(repo.get_ref(), &user, task_id, form) {
        Ok(updated_task) => {
            FlashMessage::success("Задача обновлена.").send();
            redirect(&format!("/task/{}", updated_task.id))
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Задача не найдена.").send();
            redirect("/")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(&format!("/task/{task_id}"))
        }
        Err(err) => {
            log::error!("Failed to update task {task_id}: {err}");
            FlashMessage::error("Не удалось обновить задачу.").send();
            redirect(&format!("/task/{task_id}"))
        }
    }
}

#[post("/task/{task_id}/comments")]
pub async fn add_task_comment(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<NewTaskCommentForm>,
) -> impl Responder {
    let task_id = task_id.into_inner();

    match task_service::add_task_comment(repo.get_ref(), &user, task_id, form) {
        Ok(_) => {
            FlashMessage::success("Комментарий добавлен.").send();
            redirect(&format!("/task/{}", task_id))
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Задача не найдена.").send();
            redirect("/")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect(&format!("/task/{}", task_id))
        }
        Err(err) => {
            log::error!("Failed to add comment for task {task_id}: {err}");
            FlashMessage::error("Не удалось добавить комментарий.").send();
            redirect(&format!("/task/{}", task_id))
        }
    }
}

#[post("/task/{task_id}/delete")]
pub async fn delete_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let task_id = task_id.into_inner();

    match task_service::delete_task(repo.get_ref(), &user, task_id) {
        Ok(()) => {
            FlashMessage::success("Задача удалена.").send();
            redirect("/")
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::NotFound) => {
            FlashMessage::error("Задача не найдена.").send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to delete task {task_id}: {err}");
            FlashMessage::error("Не удалось удалить задачу.").send();
            redirect("/")
        }
    }
}
