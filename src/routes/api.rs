//! JSON API routes used for React-owned shell, page-data, and lookup contracts.

use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::services::errors::ServiceError;

use crate::dto::api::{ClientLookupQueryDto, LookupQueryDto};
use crate::dto::main::IndexQuery;
use crate::forms::main::{AddTaskForm, AddTaskPayload, UploadTasksForm};
use crate::forms::task::{
    QuickTaskStatusForm, QuickTaskStatusPayload, TaskCommentForm, TaskCommentPayload,
    UpdateTaskForm, UpdateTaskPayload,
};
use crate::models::config::AppConfig;
use crate::models::config::ZmqSenders;
use crate::repository::DieselRepository;
use crate::routes::{form_error_response, mutation_error_response, mutation_success_response};
use crate::services::api as api_service;
use crate::services::main as main_service;
use crate::services::task as task_service;

#[get("/v1/iam")]
/// Return typed shell data for React-owned ToDo pages.
pub async fn api_v1_iam(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    match api_service::get_shell_data(&user, common_config.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(err) => {
            log::error!("Failed to load ToDo shell data: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/no-access")]
/// Return typed page data for the React-owned ToDo no-access page.
pub async fn api_v1_no_access(
    user: AuthenticatedUser,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    HttpResponse::Ok().json(api_service::get_no_access_data(
        &user,
        common_config.get_ref(),
    ))
}

#[get("/v1/tasks")]
/// Return the canonical JSON task collection for the React list page.
pub async fn api_v1_tasks(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::get_task_collection_data(params.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load task collection: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/tasks/{task_id}")]
/// Return the canonical JSON task details payload for the React task page.
pub async fn api_v1_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    app_config: web::Data<AppConfig>,
) -> impl Responder {
    match api_service::get_task_details_data(task_id.into_inner(), &user, repo.get_ref()) {
        Ok(mut response) => {
            if let Some(client) = response.client.as_mut() {
                client.url = Some(format!(
                    "{}/?public_id={}",
                    app_config.crm_service_url, client.public_id
                ));
            }

            HttpResponse::Ok().json(response)
        }
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(ServiceError::NotFound) => HttpResponse::NotFound().finish(),
        Err(err) => {
            log::error!("Failed to load task details: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/users")]
/// Return lookup items for assignee selectors.
pub async fn api_v1_users(
    params: web::Query<LookupQueryDto>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::list_users(params.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load users lookup: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/clients")]
/// Return lookup items for client selectors.
pub async fn api_v1_clients(
    params: web::Query<ClientLookupQueryDto>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::list_clients(params.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load clients lookup: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/v1/tracks")]
/// Return distinct task tracks for selectors and filters.
pub async fn api_v1_tracks(
    params: web::Query<LookupQueryDto>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match api_service::list_tracks(params.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to load tracks lookup: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/v1/tasks")]
/// Create a task from the React-owned task list page.
pub async fn api_v1_create_task(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    web::Form(form): web::Form<AddTaskForm>,
) -> impl Responder {
    let payload = match AddTaskPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => return form_error_response(&err),
    };

    let zmq_senders = zmq_senders.get_ref();
    match main_service::add_task(
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
        &zmq_senders.tasks,
    ) {
        Ok(_) => mutation_success_response("Задача добавлена.", None),
        Err(err @ ServiceError::Unauthorized)
        | Err(err @ ServiceError::Form(_))
        | Err(err @ ServiceError::TypeConstraint(_))
        | Err(err @ ServiceError::Conflict)
        | Err(err @ ServiceError::NotFound) => mutation_error_response(&err),
        Err(err) => {
            log::error!("Failed to create task from API: {err}");
            mutation_error_response(&err)
        }
    }
}

#[post("/v1/tasks/upload")]
/// Upload tasks in CSV format from the React-owned task list page.
pub async fn api_v1_upload_tasks(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(form): MultipartForm<UploadTasksForm>,
) -> impl Responder {
    let payload = match form.try_into_payload() {
        Ok(payload) => payload,
        Err(err) => return form_error_response(&err),
    };

    match main_service::upload_tasks(payload, &user, repo.get_ref()) {
        Ok(created_count) => {
            mutation_success_response(format!("Добавлено задач: {created_count}"), None)
        }
        Err(err @ ServiceError::Unauthorized)
        | Err(err @ ServiceError::Form(_))
        | Err(err @ ServiceError::TypeConstraint(_))
        | Err(err @ ServiceError::Conflict)
        | Err(err @ ServiceError::NotFound) => mutation_error_response(&err),
        Err(err) => {
            log::error!("Failed to upload tasks from API: {err}");
            mutation_error_response(&err)
        }
    }
}

#[post("/v1/tasks/{task_id}/update")]
/// Update a task from the React-owned task details page.
pub async fn api_v1_update_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    web::Form(form): web::Form<UpdateTaskForm>,
) -> impl Responder {
    let task_id = task_id.into_inner();
    let payload = match UpdateTaskPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => return form_error_response(&err),
    };

    let zmq_senders = zmq_senders.get_ref();
    match task_service::update_task(
        task_id,
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
        &zmq_senders.tasks,
    ) {
        Ok(_) => mutation_success_response("Задача обновлена.", None),
        Err(err @ ServiceError::Unauthorized)
        | Err(err @ ServiceError::Form(_))
        | Err(err @ ServiceError::TypeConstraint(_))
        | Err(err @ ServiceError::Conflict)
        | Err(err @ ServiceError::NotFound) => mutation_error_response(&err),
        Err(err) => {
            log::error!("Failed to update task from API: {err}");
            mutation_error_response(&err)
        }
    }
}

#[post("/v1/tasks/{task_id}/status")]
/// Apply a quick status transition from the React-owned task details page.
pub async fn api_v1_update_task_status(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    web::Form(form): web::Form<QuickTaskStatusForm>,
) -> impl Responder {
    let task_id = task_id.into_inner();
    let payload = match QuickTaskStatusPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => return form_error_response(&err),
    };

    let zmq_senders = zmq_senders.get_ref();
    match task_service::transition_task_status(
        task_id,
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
        &zmq_senders.tasks,
    ) {
        Ok(_) => mutation_success_response("Статус задачи обновлён.", None),
        Err(err @ ServiceError::Unauthorized)
        | Err(err @ ServiceError::Form(_))
        | Err(err @ ServiceError::TypeConstraint(_))
        | Err(err @ ServiceError::Conflict)
        | Err(err @ ServiceError::NotFound) => mutation_error_response(&err),
        Err(err) => {
            log::error!("Failed to update task status from API: {err}");
            mutation_error_response(&err)
        }
    }
}

#[post("/v1/tasks/{task_id}/comments")]
/// Add a task comment from the React-owned task details page.
pub async fn api_v1_add_task_comment(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    zmq_senders: web::Data<ZmqSenders>,
    web::Form(form): web::Form<TaskCommentForm>,
) -> impl Responder {
    let task_id = task_id.into_inner();
    let payload = match TaskCommentPayload::try_from(form) {
        Ok(payload) => payload,
        Err(err) => return form_error_response(&err),
    };

    let zmq_senders = zmq_senders.get_ref();
    match task_service::add_task_comment(
        task_id,
        payload,
        &user,
        repo.get_ref(),
        &zmq_senders.emailer,
    ) {
        Ok(_) => mutation_success_response("Комментарий добавлен.", None),
        Err(err @ ServiceError::Unauthorized)
        | Err(err @ ServiceError::Form(_))
        | Err(err @ ServiceError::TypeConstraint(_))
        | Err(err @ ServiceError::Conflict)
        | Err(err @ ServiceError::NotFound) => mutation_error_response(&err),
        Err(err) => {
            log::error!("Failed to add task comment from API: {err}");
            mutation_error_response(&err)
        }
    }
}

#[post("/v1/tasks/{task_id}/delete")]
/// Delete a task from the React-owned task details page.
pub async fn api_v1_delete_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    let task_id = task_id.into_inner();

    match task_service::delete_task(task_id, &user, repo.get_ref()) {
        Ok(()) => mutation_success_response("Задача удалена.", Some("/".to_string())),
        Err(err @ ServiceError::Unauthorized)
        | Err(err @ ServiceError::Form(_))
        | Err(err @ ServiceError::TypeConstraint(_))
        | Err(err @ ServiceError::Conflict)
        | Err(err @ ServiceError::NotFound) => mutation_error_response(&err),
        Err(err) => {
            log::error!("Failed to delete task from API: {err}");
            mutation_error_response(&err)
        }
    }
}
