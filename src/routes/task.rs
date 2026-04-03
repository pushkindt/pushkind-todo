//! UI route serving the task-details React document.
use std::path::Path;

use actix_web::{Either, HttpResponse, Responder, get, web};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{ensure_role, redirect};

use crate::SERVICE_ACCESS_ROLE;
use crate::frontend::{FRONTEND_DIST_DIR, FRONTEND_TASK_DOCUMENT, open_frontend_html};
use crate::services::ServiceError;

/// Display the React-backed task page after access checks succeed.
#[get("/task/{task_id}")]
pub async fn show_task(_task_id: web::Path<i32>, user: AuthenticatedUser) -> impl Responder {
    match ensure_role(&user, SERVICE_ACCESS_ROLE) {
        Ok(_) => {}
        Err(ServiceError::Unauthorized) => {
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
