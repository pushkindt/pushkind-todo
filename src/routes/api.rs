use actix_web::{HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::dto::main::IndexQuery;
use crate::repository::DieselRepository;
use crate::services::{ServiceError, main as main_service};

#[get("/v1/tasks")]
/// Return a JSON list of tasks with optional search and pagination.
///
/// Users without the role stored in `crate::SERVICE_ACCESS_ROLE` receive a `401 Unauthorized` response.
pub async fn api_v1_tasks(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match main_service::load_index_page(repo.get_ref(), &user, params.0) {
        Ok(response) => HttpResponse::Ok().json(response.tasks),
        Err(ServiceError::Unauthorized) => HttpResponse::Unauthorized().finish(),
        Err(err) => {
            log::error!("Failed to list tasks: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
