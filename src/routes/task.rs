//! Routes focused on task-specific views, updates, and modal interactions.
use std::path::Path;

use actix_web::{Either, HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::FlashMessage;
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{ensure_role, redirect, render_template};
use tera::{Context, Tera};

use crate::SERVICE_ACCESS_ROLE;
use crate::forms::task::{
    QuickTaskStatusForm, QuickTaskStatusPayload, TaskCommentForm, TaskCommentPayload,
    UpdateTaskForm, UpdateTaskPayload,
};
use crate::frontend::{FRONTEND_DIST_DIR, FRONTEND_TASK_DOCUMENT, open_frontend_html};
use crate::models::config::ZmqSenders;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, task as task_service};

/// Display the React-backed task page after access checks succeed.
#[get("/task/{task_id}")]
pub async fn show_task(_task_id: web::Path<i32>, user: AuthenticatedUser) -> impl Responder {
    match ensure_role(&user, SERVICE_ACCESS_ROLE) {
        Ok(_) => {}
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            return Either::Right(redirect("/na"));
        }
        Err(err) => {
            log::error!("Failed to authorize task page access: {err}");
            return Either::Right(HttpResponse::InternalServerError().finish());
        }
    }

    let task_document = Path::new(FRONTEND_DIST_DIR).join(FRONTEND_TASK_DOCUMENT);
    match open_frontend_html(&task_document).await {
        Ok(document) => Either::Left(document),
        Err(err) => {
            log::error!("Failed to open built task document: {err}");
            Either::Right(HttpResponse::InternalServerError().finish())
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
    let payload = match UpdateTaskPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => {
            FlashMessage::error(err.to_string()).send();
            return redirect(&format!("/task/{task_id}"));
        }
    };

    match task_service::update_task(
        task_id,
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
        &zmq_senders.tasks,
    ) {
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
    let payload = match QuickTaskStatusPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => {
            FlashMessage::error(err.to_string()).send();
            return redirect(&format!("/task/{task_id}"));
        }
    };

    match task_service::transition_task_status(
        task_id,
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
        &zmq_senders.tasks,
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
    let payload = match TaskCommentPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => {
            FlashMessage::error(err.to_string()).send();
            return redirect(&format!("/task/{task_id}"));
        }
    };

    match task_service::add_task_comment(
        task_id,
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
    ) {
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
