//! Helpers and form definitions powering the index page and task import flows.
use std::io::{Read, Seek};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv::Trim;
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use validator::Validate;

use crate::{
    domain::{
        task::{NewTask, TaskPriority},
        types::{HubId, TaskDescription, TaskTitle, TaskTrack, UserId},
    },
    forms::{
        FormError,
        task::{AssigneeSelectionForm, AssigneeSelectionPayload},
    },
};

#[derive(Deserialize, Validate)]
pub struct AddTaskForm {
    #[serde(default)]
    #[validate(length(min = 1, message = "Укажите название задачи."))]
    pub title: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub track: Option<String>,
    #[serde(default)]
    #[validate(length(min = 1, message = "Выберите приоритет задачи."))]
    pub priority: String,
    /// Assignee data captured by the modal.
    #[serde(flatten, default)]
    pub assignee: AssigneeSelectionForm,
}

#[derive(Debug)]
/// Normalized and strongly typed data captured from the add-task form.
pub struct AddTaskPayload {
    /// Task title.
    pub title: TaskTitle,
    /// Optional sanitized HTML description.
    pub description: Option<TaskDescription>,
    /// Optional track categorization.
    pub track: Option<TaskTrack>,
    /// Optional priority selection.
    pub priority: TaskPriority,
    /// Optional assignee selection.
    pub assignee: Option<AssigneeSelectionPayload>,
}

impl TryFrom<AddTaskForm> for AddTaskPayload {
    type Error = FormError;

    fn try_from(form: AddTaskForm) -> Result<Self, Self::Error> {
        form.validate().map_err(FormError::Validation)?;

        let AddTaskForm {
            title,
            message,
            track,
            priority,
            assignee,
        } = form;

        Ok(Self {
            title: TaskTitle::new(title).map_err(|_| FormError::InvalidTitle)?,
            description: message
                .map(TaskDescription::new)
                .transpose()
                .map_err(|_| FormError::InvalidDescription)?,
            track: track
                .map(TaskTrack::new)
                .transpose()
                .map_err(|_| FormError::InvalidTrack)?,
            priority: TaskPriority::try_from(priority.as_str())
                .map_err(|_| FormError::InvalidPriority)?,
            assignee: assignee.try_into()?,
        })
    }
}

impl AddTaskPayload {
    /// Convert the payload into a [`NewTask`] for the given hub and author.
    pub fn into_domain(self, author_id: UserId, hub_id: HubId) -> NewTask {
        let mut new_task = NewTask::new(hub_id, author_id, self.title);
        new_task = new_task.priority(self.priority);

        if let Some(description) = self.description {
            new_task = new_task.description(description);
        }

        if let Some(track) = self.track {
            new_task = new_task.track(track);
        }

        new_task
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A normalized task row parsed from the uploaded CSV file.
pub struct UploadTaskRowPayload {
    pub title: TaskTitle,
    pub description: Option<TaskDescription>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsed CSV payload detached from the multipart transport details.
pub struct UploadTasksPayload {
    pub tasks: Vec<UploadTaskRowPayload>,
}

impl UploadTasksPayload {
    /// Convert the normalized rows into domain tasks for the current author.
    pub fn into_domain(self, author_id: UserId, hub_id: HubId) -> Vec<NewTask> {
        self.tasks
            .into_iter()
            .map(|task| {
                let mut new_task = NewTask::new(hub_id, author_id, task.title);
                if let Some(description) = task.description {
                    new_task = new_task.description(description);
                }
                new_task
            })
            .collect()
    }
}

#[derive(MultipartForm)]
/// Multipart form for uploading a CSV file with new tasks.
pub struct UploadTasksForm {
    #[multipart(limit = "10MB")]
    /// Uploaded CSV file containing task data.
    pub csv: TempFile,
}

impl UploadTasksForm {
    /// Parse the uploaded CSV file into a normalized payload.
    pub fn try_into_payload(mut self) -> Result<UploadTasksPayload, FormError> {
        self.csv.file.rewind().map_err(|_| FormError::InvalidCsv)?;
        parse_tasks(self.csv.file.by_ref()).map_err(|_| FormError::InvalidCsv)
    }
}

#[derive(Deserialize)]
struct TaskCsvRow {
    #[serde(default)]
    title: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    description: Option<String>,
}

fn parse_tasks<R: Read>(reader: R) -> Result<UploadTasksPayload, FormError> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    let mut tasks = Vec::new();

    for row in csv_reader.deserialize::<TaskCsvRow>() {
        let TaskCsvRow { title, description } = row.map_err(|_| FormError::InvalidCsv)?;
        let title = title.trim();

        if title.is_empty() {
            continue;
        }

        tasks.push(UploadTaskRowPayload {
            title: TaskTitle::new(title).map_err(|_| FormError::InvalidCsv)?,
            description: description
                .map(TaskDescription::new)
                .transpose()
                .map_err(|_| FormError::InvalidCsv)?,
        });
    }

    Ok(UploadTasksPayload { tasks })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_tasks_skips_rows_without_titles() {
        let csv = "title,description\nalpha,\n,\nbeta,\n";

        let payload = parse_tasks(Cursor::new(csv)).expect("parse should succeed");

        assert_eq!(payload.tasks.len(), 2);
        assert_eq!(payload.tasks[0].title.as_str(), "alpha");
        assert_eq!(payload.tasks[1].title.as_str(), "beta");
    }

    #[test]
    fn parse_tasks_allows_missing_title_header() {
        let csv = "description\nsomething\n";

        let payload = parse_tasks(Cursor::new(csv)).expect("parse should succeed");

        assert!(payload.tasks.is_empty());
    }

    #[test]
    fn add_task_payload_uses_localized_priority_error() {
        let error = AddTaskPayload::try_from(AddTaskForm {
            title: "Task".to_string(),
            message: None,
            track: None,
            priority: "urgent".to_string(),
            assignee: AssigneeSelectionForm::default(),
        })
        .expect_err("priority should be invalid");

        assert_eq!(error.to_string(), "Выберите приоритет задачи.");
        assert_eq!(error.field_errors()[0].field, "priority");
    }
}
