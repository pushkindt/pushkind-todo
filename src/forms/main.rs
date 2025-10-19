use std::io::{Read, Seek};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv::Trim;
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::{domain::task::NewTask, forms::task::AssigneeSelectionForm};

/// Build a [`NewTask`] payload with sanitized content shared across forms and services.
pub(crate) fn build_new_task_payload(
    hub_id: i32,
    author_id: i32,
    title: String,
    description: Option<String>,
) -> NewTask {
    let mut new_task = NewTask::new(hub_id, author_id, title);

    if let Some(description) = description {
        let sanitized = ammonia::clean(&description);

        if !sanitized.trim().is_empty() {
            new_task = new_task.description(sanitized);
        }
    }

    new_task
}

#[derive(Deserialize, Validate)]
pub struct AddTaskForm {
    #[validate(length(min = 1))]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "empty_string_as_none")]
    pub message: Option<String>,
    /// Assignee data captured by the modal.
    #[serde(flatten, default)]
    pub assignee: AssigneeSelectionForm,
}

impl AddTaskForm {
    /// Convert the validated form into a [`NewTask`] payload.
    pub fn into_new_task(self, hub_id: i32, author_id: i32) -> Option<NewTask> {
        let title = self.title?;

        Some(build_new_task_payload(
            hub_id,
            author_id,
            title,
            self.message,
        ))
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
            let task = build_new_task_payload(hub_id, author_id, title, record.description);

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
        assert!(tasks.iter().all(|task| task.author_id == author_id));

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].hub_id, 42);
        assert_eq!(tasks[0].title, "hello");
        assert_eq!(tasks[0].description.as_deref(), Some("first"));
        assert_eq!(tasks[1].title, "world");
    }

    #[test]
    fn parse_tasks_skips_empty_or_missing_titles() {
        let csv = "title\n\n  \nfoo\n";
        let tasks = parse_tasks(Cursor::new(csv), 7, 9).expect("should parse");

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].hub_id, 7);
        assert_eq!(tasks[0].title, "foo");
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
        let task = build_new_task_payload(1, 2, "title".to_string(), Some("   ".to_string()));

        assert!(task.description.is_none());
    }

    #[test]
    fn parse_tasks_discards_blank_descriptions() {
        let csv = "title,description\nfoo,   \n";
        let tasks = parse_tasks(Cursor::new(csv), 5, 7).expect("should parse");

        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].description.is_none());
    }
}
