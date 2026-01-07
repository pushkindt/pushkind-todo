//! Routes focused on task-specific views, updates, and modal interactions.
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::{Context, Tera};

use crate::forms::task::{QuickTaskStatusForm, TaskCommentForm, UpdateTaskForm};
use crate::models::config::ServerConfig;
use crate::models::config::ZmqSenders;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, task as task_service};

/// Display a task’s detail page with events, assignee, and author info.
#[get("/task/{task_id}")]
pub async fn show_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<ServerConfig>,
    common_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    let task_id = task_id.into_inner();

    match task_service::load_task_details(task_id, &user, repo.get_ref()) {
        Ok(details) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "task",
                &common_config.auth_service_url,
            );
            context.insert("task", &details.task);
            context.insert("author", &details.author);
            context.insert("assignee", &details.assignee);
            context.insert("client", &details.client);
            context.insert("events", &details.events);
            context.insert("crm_service_url", &server_config.crm_service_url);
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

/// Render the modal payload used for editing a task via AJAX.
#[post("/task/{task_id}/modal")]
pub async fn task_modal(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match task_service::load_task_modal(task_id.into_inner(), &user, repo.get_ref()) {
        Ok(data) => {
            let mut context = Context::new();
            context.insert("task", &data.task);
            context.insert("assignee", &data.assignee);
            context.insert("client", &data.client);
            context.insert("home_url", &server_config.auth_service_url);
            context.insert("tracks", &data.tracks);
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

/// Apply edits submitted from the task modal and redirect back.
#[post("/task/{task_id}/update")]
pub async fn update_task(
    task_id: web::Path<i32>,
    web::Form(form): web::Form<UpdateTaskForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
) -> impl Responder {
    let task_id = task_id.into_inner();
    let zmq_senders = zmq_senders.get_ref();
    match task_service::update_task(task_id, form, &user, repo.get_ref(), &zmq_senders.emailer) {
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
            log::info!("Form error updating task {task_id}: {message}");
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

/// Quickly transition task status from the task list controls.
#[post("/task/{task_id}/status")]
pub async fn quick_update_task_status(
    task_id: web::Path<i32>,
    web::Form(form): web::Form<QuickTaskStatusForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
) -> impl Responder {
    let task_id = task_id.into_inner();
    let zmq_senders = zmq_senders.get_ref();

    match task_service::transition_task_status(
        task_id,
        form,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
    ) {
        Ok(_) => {
            FlashMessage::success("Статус задачи обновлён.").send();
            redirect(&format!("/task/{task_id}"))
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
            log::error!("Failed to update task status {task_id}: {err}");
            FlashMessage::error("Не удалось обновить статус задачи.").send();
            redirect(&format!("/task/{task_id}"))
        }
    }
}

/// Record a new comment for a task and refresh the task view.
#[post("/task/{task_id}/comments")]
pub async fn add_task_comment(
    task_id: web::Path<i32>,
    web::Form(form): web::Form<TaskCommentForm>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
) -> impl Responder {
    let task_id = task_id.into_inner();
    let zmq_senders = zmq_senders.get_ref();

    match task_service::add_task_comment(task_id, form, &user, repo.get_ref(), &zmq_senders.emailer)
    {
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

/// Delete the specified task, sending flash feedback on success/failure.
#[post("/task/{task_id}/delete")]
pub async fn delete_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let task_id = task_id.into_inner();

    match task_service::delete_task(task_id, &user, repo.get_ref()) {
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
