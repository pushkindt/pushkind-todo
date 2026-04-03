//! Auxiliary routes for React-owned frontend documents.

use std::path::Path;

use actix_web::{HttpRequest, HttpResponse, get};
use pushkind_common::domain::auth::AuthenticatedUser;

use crate::frontend::{
    FRONTEND_DIST_DIR, FRONTEND_NO_ACCESS_DOCUMENT, FrontendAssetError, open_frontend_html,
};

#[get("/na")]
pub async fn not_assigned(request: HttpRequest, _user: AuthenticatedUser) -> HttpResponse {
    let no_access_document = Path::new(FRONTEND_DIST_DIR).join(FRONTEND_NO_ACCESS_DOCUMENT);

    match open_frontend_html(&no_access_document).await {
        Ok(file) => file.into_response(&request),
        Err(FrontendAssetError::Read(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            HttpResponse::ServiceUnavailable()
                .body("ToDo frontend assets are not built yet. Run `cd frontend && npm run build`.")
        }
        Err(error) => {
            log::error!("Failed to open ToDo no-access frontend document: {error}");
            HttpResponse::InternalServerError().finish()
        }
    }
}
