use actix_multipart::form::MultipartForm;
use actix_web::{HttpResponse, Responder, get, post, web};
use actix_web_flash_messages::{FlashMessage, IncomingFlashMessages};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{base_context, redirect, render_template};
use tera::Tera;

use crate::forms::main::{AddTemplateForm, UploadTemplatesForm};
use crate::repository::DieselRepository;
use crate::services::main::IndexQuery;
use crate::services::{ServiceError, main as main_service};

#[get("/")]
pub async fn show_index(
    params: web::Query<IndexQuery>,
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    flash_messages: IncomingFlashMessages,
    server_config: web::Data<CommonServerConfig>,
    tera: web::Data<Tera>,
) -> impl Responder {
    match main_service::load_index_page(repo.get_ref(), &user, params.0) {
        Ok(data) => {
            let mut context = base_context(
                &flash_messages,
                &user,
                "index",
                &server_config.auth_service_url,
            );
            context.insert("templates", &data.templates);
            context.insert("search", &data.search);
            render_template(&tera, "main/index.html", &context)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(err) => {
            log::error!("Failed to list templates: {err}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[post("/template/add")]
pub async fn add_template(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    web::Form(form): web::Form<AddTemplateForm>,
) -> impl Responder {
    match main_service::add_template(repo.get_ref(), &user, form) {
        Ok(outcome) => {
            FlashMessage::success(outcome.message).send();
            redirect(&outcome.redirect_to)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to add a template: {err}");
            FlashMessage::error("Ошибка при добавлении шаблона").send();
            redirect("/")
        }
    }
}

#[post("/templates/upload")]
pub async fn templates_upload(
    user: AuthenticatedUser,
    repo: web::Data<DieselRepository>,
    MultipartForm(mut form): MultipartForm<UploadTemplatesForm>,
) -> impl Responder {
    match main_service::upload_templates(repo.get_ref(), &user, &mut form) {
        Ok(outcome) => {
            FlashMessage::success(outcome.message).send();
            redirect(&outcome.redirect_to)
        }
        Err(ServiceError::Unauthorized) => {
            FlashMessage::error("Недостаточно прав.").send();
            redirect("/na")
        }
        Err(ServiceError::Form(message)) => {
            FlashMessage::error(message).send();
            redirect("/")
        }
        Err(err) => {
            log::error!("Failed to add templates: {err}");
            FlashMessage::error("Ошибка при добавлении шаблонов").send();
            redirect("/")
        }
    }
}
