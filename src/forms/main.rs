//! Helpers and form definitions powering the index page and task import flows.
use std::io::{Read, Seek};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv::Trim;
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::{
    domain::{
        task::{NewTask, TaskPriority},
        types::{HubId, TaskDescription, TaskTitle, TaskTrack, TypeConstraintError, UserId},
    },
    forms::{
        FormError,
        task::{AssigneeSelectionForm, AssigneeSelectionPayload},
    },
};

#[derive(Deserialize, Validate)]
pub struct AddTaskForm {
    #[validate(length(min = 1))]
    pub title: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub track: Option<String>,
    #[validate(length(min = 1))]
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
    pub fn into_domain(
        self,
        author_id: UserId,
        hub_id: HubId,
    ) -> Result<NewTask, TypeConstraintError> {
        let mut new_task = NewTask::new(hub_id, author_id, self.title);
        new_task = new_task.priority(self.priority);

        if let Some(description) = self.description {
            new_task = new_task.description(description);
        }

        if let Some(track) = self.track {
            new_task = new_task.track(track);
        }

        Ok(new_task)
    }
}

#[derive(MultipartForm)]
/// Multipart form for uploading a CSV file with new tasks.
pub struct UploadTasksForm {
    #[multipart(limit = "10MB")]
    /// Uploaded CSV file containing task data.
    pub csv: TempFile,
}

#[derive(Debug, Error)]
/// Errors that can occur while parsing an uploaded tasks CSV file.
pub enum UploadTasksFormError {
    #[error("Error reading csv file")]
    FileReadError,
    #[error("Error parsing csv file")]
    CsvParseError,
    #[error("Invalid task data: {0}")]
    InvalidTaskData(String),
}

impl From<std::io::Error> for UploadTasksFormError {
    fn from(_: std::io::Error) -> Self {
        UploadTasksFormError::FileReadError
    }
}

impl From<csv::Error> for UploadTasksFormError {
    fn from(_: csv::Error) -> Self {
        UploadTasksFormError::CsvParseError
    }
}

impl From<TypeConstraintError> for UploadTasksFormError {
    fn from(err: TypeConstraintError) -> Self {
        UploadTasksFormError::InvalidTaskData(err.to_string())
    }
}

impl UploadTasksForm {
    /// Parse the uploaded CSV file into a list of [`NewTask`] records.
    pub fn parse(
        &mut self,
        author_id: UserId,
        hub_id: HubId,
    ) -> Result<Vec<NewTask>, UploadTasksFormError> {
        self.csv.file.rewind()?;
        parse_tasks(author_id, hub_id, self.csv.file.by_ref())
    }
}

#[derive(Deserialize)]
struct TaskCsvRow {
    #[serde(default)]
    title: String,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    description: Option<String>,
}

fn parse_tasks<R: Read>(
    author_id: UserId,
    hub_id: HubId,
    reader: R,
) -> Result<Vec<NewTask>, UploadTasksFormError> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    let mut tasks = Vec::new();

    for row in csv_reader.deserialize::<TaskCsvRow>() {
        let TaskCsvRow { title, description } = row?;
        let title = title.trim();

        if title.is_empty() {
            continue;
        }

        let mut task = NewTask::try_new(hub_id.get(), author_id.get(), title)?;

        if let Some(description) = description {
            task = task.description(TaskDescription::new(description)?);
        }

        tasks.push(task);
    }

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_tasks_skips_rows_without_titles() {
        let author_id = UserId::new(1).unwrap();
        let hub_id = HubId::new(1).unwrap();
        let csv = "title,description\nalpha,\n,\nbeta,\n";

        let tasks = parse_tasks(author_id, hub_id, Cursor::new(csv)).expect("parse should succeed");

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].title.as_str(), "alpha");
        assert_eq!(tasks[1].title.as_str(), "beta");
    }

    #[test]
    fn parse_tasks_allows_missing_title_header() {
        let author_id = UserId::new(1).unwrap();
        let hub_id = HubId::new(1).unwrap();
        let csv = "description\nsomething\n";

        let tasks = parse_tasks(author_id, hub_id, Cursor::new(csv)).expect("parse should succeed");

        assert!(tasks.is_empty());
    }
}
