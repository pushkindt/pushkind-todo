//! UI route serving the main React document.
use std::path::Path;

use actix_web::{Either, HttpResponse, Responder, get};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::routes::{ensure_role, redirect};

use crate::SERVICE_ACCESS_ROLE;
use crate::frontend::{FRONTEND_DIST_DIR, FRONTEND_INDEX_DOCUMENT, open_frontend_html};
use crate::services::ServiceError;

/// Display the React-backed main index page after access checks succeed.
#[get("/")]
pub async fn show_index(user: AuthenticatedUser) -> impl Responder {
    match ensure_role(&user, SERVICE_ACCESS_ROLE) {
        Ok(_) => {}
        Err(ServiceError::Unauthorized) => {
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
