//! Route module combining HTTP handlers for both the UI and JSON API.
use actix_web::{HttpResponse, http::StatusCode};

use crate::dto::api::ApiMutationErrorDto;
use crate::services::ServiceError;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
