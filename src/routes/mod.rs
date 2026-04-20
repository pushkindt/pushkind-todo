//! Route module combining HTTP handlers for both the UI and JSON API.
use actix_web::{HttpResponse, http::StatusCode};
use pushkind_common::dto::mutation::{ApiMutationErrorDto, ApiMutationSuccessDto};
use pushkind_common::services::errors::ServiceError;

use crate::forms::FormError;

pub mod api;
pub mod aux;
pub mod main;
pub mod task;

#[allow(dead_code)]
pub(crate) fn mutation_error_status(err: &ServiceError) -> StatusCode {
    match err {
        ServiceError::Form(_) | ServiceError::TypeConstraint(_) => StatusCode::BAD_REQUEST,
        ServiceError::Unauthorized => StatusCode::UNAUTHORIZED,
        ServiceError::NotFound => StatusCode::NOT_FOUND,
        ServiceError::Conflict => StatusCode::CONFLICT,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[allow(dead_code)]
fn mutation_error_dto(err: &ServiceError) -> ApiMutationErrorDto {
    match err {
        ServiceError::Form(message) | ServiceError::TypeConstraint(message) => {
            ApiMutationErrorDto {
                message: message.clone(),
                field_errors: Vec::new(),
            }
        }
        ServiceError::Unauthorized => ApiMutationErrorDto {
            message: "Недостаточно прав.".to_string(),
            field_errors: Vec::new(),
        },
        ServiceError::NotFound => ApiMutationErrorDto {
            message: "Ресурс не найден.".to_string(),
            field_errors: Vec::new(),
        },
        ServiceError::Conflict => ApiMutationErrorDto {
            message: "Конфликт данных.".to_string(),
            field_errors: Vec::new(),
        },
        _ => ApiMutationErrorDto {
            message: "Внутренняя ошибка сервиса.".to_string(),
            field_errors: Vec::new(),
        },
    }
}

#[allow(dead_code)]
pub(crate) fn mutation_error_response(err: &ServiceError) -> HttpResponse {
    HttpResponse::build(mutation_error_status(err)).json(mutation_error_dto(err))
}

#[allow(dead_code)]
pub(crate) fn form_error_response(err: &FormError) -> HttpResponse {
    HttpResponse::BadRequest().json(ApiMutationErrorDto::from(err))
}

#[allow(dead_code)]
pub(crate) fn mutation_success_response(
    message: impl Into<String>,
    redirect_to: Option<String>,
) -> HttpResponse {
    HttpResponse::Ok().json(ApiMutationSuccessDto {
        message: message.into(),
        redirect_to,
    })
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use serde_json::{Value, json};

    use super::*;
    use crate::forms::FormError;

    #[test]
    fn mutation_error_status_maps_service_errors() {
        assert_eq!(
            mutation_error_status(&ServiceError::Form("invalid".to_string())),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            mutation_error_status(&ServiceError::Unauthorized),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            mutation_error_status(&ServiceError::NotFound),
            StatusCode::NOT_FOUND
        );
    }

    #[actix_web::test]
    async fn form_error_response_keeps_field_errors() {
        let response = form_error_response(&FormError::InvalidPriority);

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body())
            .await
            .expect("body should load");
        let payload: Value = serde_json::from_slice(&body).expect("body should be json");

        assert_eq!(payload["message"], json!("Ошибка валидации формы."));
        assert_eq!(
            payload["field_errors"],
            json!([{ "field": "priority", "message": "Выберите приоритет задачи." }])
        );
    }

    #[actix_web::test]
    async fn mutation_success_response_uses_shared_shape() {
        let response = mutation_success_response("Задача добавлена.", Some("/task/5".to_string()));

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body())
            .await
            .expect("body should load");
        let payload: Value = serde_json::from_slice(&body).expect("body should be json");

        assert_eq!(
            payload,
            json!({
                "message": "Задача добавлена.",
                "redirect_to": "/task/5"
            })
        );
    }
}
