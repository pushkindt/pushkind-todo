//! JSON API routes used for React-owned shell, page-data, and lookup contracts.

use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;

use crate::dto::api::{ClientLookupQueryDto, LookupQueryDto};
use crate::dto::main::IndexQuery;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, api as api_service};

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
) -> impl Responder {
    match api_service::get_task_details_data(task_id.into_inner(), &user, repo.get_ref()) {
        Ok(response) => HttpResponse::Ok().json(response),
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
