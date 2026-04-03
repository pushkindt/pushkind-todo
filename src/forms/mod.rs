//! Forms entrypoint re-exporting the main entry and task-related form modules.
//!
//! The [`FormError`] enum provides a shared set of field identifiers for
//! reporting validation failures consistently across different form payloads.
use std::borrow::Cow;

use thiserror::Error;
use validator::{ValidationError, ValidationErrors};

pub mod main;
pub mod task;

#[derive(Clone, Debug, PartialEq, Eq)]
/// Field-level validation error emitted by the form layer.
pub struct FormFieldError {
    pub field: Cow<'static, str>,
    pub message: Cow<'static, str>,
}

#[derive(Debug, Error)]
/// Errors that can occur when processing form data.
pub enum FormError {
    #[error("{}", validation_errors_display(.0))]
    Validation(#[from] ValidationErrors),
    #[error("Название задачи заполнено некорректно.")]
    InvalidTitle,
    #[error("Описание задачи заполнено некорректно.")]
    InvalidDescription,
    #[error("Укажите корректную дату.")]
    InvalidDueDate,
    #[error("Выберите статус задачи.")]
    InvalidStatus,
    #[error("Трек задачи заполнен некорректно.")]
    InvalidTrack,
    #[error("Выберите приоритет задачи.")]
    InvalidPriority,
    #[error("Укажите имя исполнителя.")]
    InvalidAssigneeName,
    #[error("Укажите корректный email исполнителя.")]
    InvalidAssigneeEmail,
    #[error("Введите комментарий.")]
    InvalidCommentMessage,
    #[error("Комментарий заполнен некорректно.")]
    InvalidQuickComment,
    #[error("Параметр назначения заполнен некорректно.")]
    InvalidAssignSelf,
    #[error("Не удалось обработать CSV-файл.")]
    InvalidCsv,
    #[error("Укажите клиента.")]
    InvalidClientName,
    #[error("Укажите корректный идентификатор клиента.")]
    InvalidClientPublicId,
}

impl FormError {
    pub(crate) fn field_errors(&self) -> Vec<FormFieldError> {
        match self {
            Self::Validation(errors) => collect_validation_errors(errors),
            _ => self
                .field()
                .map(|field| vec![field_error(field, self.to_string())])
                .unwrap_or_default(),
        }
    }

    fn field(&self) -> Option<&'static str> {
        match self {
            Self::Validation(_) => None,
            Self::InvalidTitle => Some("title"),
            Self::InvalidDescription => Some("message"),
            Self::InvalidDueDate => Some("due_date"),
            Self::InvalidStatus => Some("status"),
            Self::InvalidTrack => Some("track"),
            Self::InvalidPriority => Some("priority"),
            Self::InvalidAssigneeName => Some("name"),
            Self::InvalidAssigneeEmail => Some("email"),
            Self::InvalidCommentMessage => Some("message"),
            Self::InvalidQuickComment => Some("comment"),
            Self::InvalidAssignSelf => Some("assign_self"),
            Self::InvalidCsv => Some("csv"),
            Self::InvalidClientName => Some("client_name"),
            Self::InvalidClientPublicId => Some("client_public_id"),
        }
    }
}

fn collect_validation_errors(errors: &ValidationErrors) -> Vec<FormFieldError> {
    errors
        .field_errors()
        .iter()
        .flat_map(|(field, field_errors)| {
            field_errors.iter().map(|error| FormFieldError {
                field: field.clone(),
                message: validation_error_message(error),
            })
        })
        .collect()
}

fn validation_error_message(error: &ValidationError) -> Cow<'static, str> {
    error
        .message
        .clone()
        .unwrap_or(Cow::Borrowed("Поле заполнено некорректно."))
}

fn validation_errors_display(errors: &ValidationErrors) -> String {
    let messages = collect_validation_errors(errors)
        .into_iter()
        .map(|error| error.message.into_owned())
        .collect::<Vec<_>>();

    if messages.is_empty() {
        "Ошибка валидации формы.".to_string()
    } else {
        format!("Ошибка валидации формы: {}", messages.join("; "))
    }
}

fn field_error(field: &'static str, message: impl Into<Cow<'static, str>>) -> FormFieldError {
    FormFieldError {
        field: Cow::Borrowed(field),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::FormError;
    use crate::forms::main::AddTaskForm;
    use crate::forms::task::{AssigneeSelectionForm, QuickTaskStatusForm};
    use validator::Validate;

    fn field_errors(error: &FormError) -> Vec<(String, String)> {
        let mut field_errors = error
            .field_errors()
            .into_iter()
            .map(|error| (error.field.to_string(), error.message.into_owned()))
            .collect::<Vec<_>>();
        field_errors.sort();
        field_errors
    }

    #[test]
    fn validation_errors_use_messages_declared_by_forms() {
        let form = AddTaskForm {
            title: String::new(),
            message: None,
            track: None,
            priority: String::new(),
            assignee: AssigneeSelectionForm::default(),
        };

        let error = FormError::from(form.validate().expect_err("form should be invalid"));

        assert_eq!(
            field_errors(&error),
            vec![
                (
                    "priority".to_string(),
                    "Выберите приоритет задачи.".to_string()
                ),
                ("title".to_string(), "Укажите название задачи.".to_string(),),
            ]
        );
    }

    #[test]
    fn conversion_errors_keep_field_names_in_forms_layer() {
        assert_eq!(
            field_errors(&FormError::InvalidCsv),
            vec![(
                "csv".to_string(),
                "Не удалось обработать CSV-файл.".to_string(),
            )]
        );
    }

    #[test]
    fn form_error_display_is_localized() {
        let validation_error = FormError::from(
            QuickTaskStatusForm {
                status: String::new(),
                comment: None,
                assign_self: false,
            }
            .validate()
            .expect_err("form should be invalid"),
        );

        let message = validation_error.to_string();
        assert!(message.contains("Ошибка валидации формы:"));
        assert!(message.contains("Выберите статус задачи."));
        assert_eq!(
            FormError::InvalidAssigneeEmail.to_string(),
            "Укажите корректный email исполнителя."
        );
    }
}
