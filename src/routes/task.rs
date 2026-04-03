//! UI route serving the task-details React document.
use std::path::Path;

use actix_web::{Either, HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::redirect;
use pushkind_common::services::errors::ServiceError;

use crate::frontend::{FRONTEND_DIST_DIR, FRONTEND_TASK_DOCUMENT, open_frontend_html};
use crate::repository::DieselRepository;
use crate::services::task as task_service;

/// Display the React-backed task page after access checks succeed.
#[get("/task/{task_id}")]
pub async fn show_task(
    task_id: web::Path<i32>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
) -> impl Responder {
    match task_service::verify_task_page_access(task_id.into_inner(), &user, repo.get_ref()) {
        Ok(_) => {}
        Err(ServiceError::Unauthorized) => {
            return Either::Right(redirect("/na"));
        }
        Err(ServiceError::NotFound) => {
            return Either::Right(HttpResponse::NotFound().finish());
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
