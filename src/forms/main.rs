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
    forms::task::AssigneeSelectionForm,
};

/// Build a [`NewTask`] payload with sanitized content shared across forms and services.
pub(crate) fn build_new_task_payload(
    hub_id: i32,
    author_id: i32,
    title: String,
    description: Option<String>,
    track: Option<String>,
    priority: Option<TaskPriority>,
) -> Result<NewTask, TypeConstraintError> {
    let title = TaskTitle::new(title)?;
    let mut new_task = NewTask::new(HubId::new(hub_id)?, UserId::new(author_id)?, title);

    if let Some(description) = description {
        let sanitized = ammonia::clean(&description);

        if !sanitized.trim().is_empty() {
            new_task = new_task.description(TaskDescription::from(sanitized));
        }
    }

    if let Some(track) = track
        && let Ok(track) = TaskTrack::new(track.trim())
    {
        new_task = new_task.track(track);
    }

    if let Some(priority) = priority {
        new_task = new_task.priority(priority);
    }

    Ok(new_task)
}

#[derive(Deserialize, Validate)]
pub struct AddTaskForm {
    #[validate(length(min = 1))]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub track: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub priority: Option<String>,
    /// Assignee data captured by the modal.
    #[serde(flatten, default)]
    pub assignee: AssigneeSelectionForm,
}

impl AddTaskForm {
    /// Convert the validated form into a [`NewTask`] payload.
    pub fn into_new_task(
        self,
        hub_id: i32,
        author_id: i32,
    ) -> Result<Option<NewTask>, TypeConstraintError> {
        let title = match self.title {
            Some(title) => title,
            None => return Ok(None),
        };
        let priority = Self::parse_priority(self.priority);

        build_new_task_payload(hub_id, author_id, title, self.message, self.track, priority)
            .map(Some)
    }

    pub(crate) fn parse_priority(priority: Option<String>) -> Option<TaskPriority> {
        priority.and_then(|value| {
            let trimmed = value.trim();

            if trimmed.is_empty() {
                return None;
            }

            let priority = TaskPriority::from(trimmed);
            let priority_text: &str = <&str>::from(priority);

            (priority_text == trimmed).then_some(priority)
        })
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
        hub_id: i32,
        author_id: i32,
    ) -> Result<Vec<NewTask>, UploadTasksFormError> {
        self.csv.file.rewind()?;
        parse_tasks(self.csv.file.by_ref(), hub_id, author_id)
    }
}

#[derive(Deserialize)]
struct TaskCsvRow {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    title: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    description: Option<String>,
}

fn parse_tasks<R: Read>(
    reader: R,
    hub_id: i32,
    author_id: i32,
) -> Result<Vec<NewTask>, UploadTasksFormError> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    let mut tasks = Vec::new();

    for row in csv_reader.deserialize::<TaskCsvRow>() {
        let record = row?;

        if let Some(title) = record.title {
            let task =
                build_new_task_payload(hub_id, author_id, title, record.description, None, None)?;

            tasks.push(task);
        }
    }

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_tasks_returns_records_with_titles() {
        let csv = "title,description\nhello,first\nworld,second\n";
        let author_id = 5;
        let tasks = parse_tasks(Cursor::new(csv), 42, author_id).expect("should parse");
        assert!(
            tasks
                .iter()
                .all(|task| task.author_id == UserId::new(author_id).unwrap())
        );

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].hub_id, HubId::new(42).unwrap());
        assert_eq!(tasks[0].title.as_str(), "hello");
        assert_eq!(
            tasks[0].description.as_ref().map(|d| d.as_str()),
            Some("first")
        );
        assert_eq!(tasks[1].title.as_str(), "world");
    }

    #[test]
    fn parse_tasks_skips_empty_or_missing_titles() {
        let csv = "title\n\n  \nfoo\n";
        let tasks = parse_tasks(Cursor::new(csv), 7, 9).expect("should parse");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].hub_id, HubId::new(7).unwrap());
        assert_eq!(tasks[0].title.as_str(), "foo");
    }

    #[test]
    fn parse_tasks_propagates_csv_errors() {
        let csv = "title\nfoo,bar\n";

        match parse_tasks(Cursor::new(csv), 1, 3) {
            Err(UploadTasksFormError::CsvParseError) => {}
            Err(other) => panic!("expected csv parse error, got {:?}", other),
            Ok(tasks) => panic!("expected csv parse error but parsed {} rows", tasks.len()),
        }
    }

    #[test]
    fn build_new_task_payload_discards_empty_descriptions() {
        let task = build_new_task_payload(
            1,
            2,
            "title".to_string(),
            Some("   ".to_string()),
            None,
            None,
        )
        .expect("payload should build");

        assert!(task.description.is_none());
    }

    #[test]
    fn build_new_task_payload_sets_track_and_priority() {
        let task = build_new_task_payload(
            1,
            2,
            "title".to_string(),
            None,
            Some("Alpha Track".to_string()),
            Some(TaskPriority::High),
        )
        .expect("payload should build");

        assert_eq!(task.track.as_ref().map(|t| t.as_str()), Some("Alpha Track"));
        assert_eq!(task.priority, TaskPriority::High);
    }

    #[test]
    fn parse_priority_returns_none_for_invalid_values() {
        assert_eq!(
            AddTaskForm::parse_priority(Some("Unknown".to_string())),
            None
        );
    }

    #[test]
    fn parse_tasks_discards_blank_descriptions() {
        let csv = "title,description\nfoo,   \n";
        let tasks = parse_tasks(Cursor::new(csv), 5, 7).expect("should parse");

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].description.is_none());
    }
}
