use std::io::{Read, Seek};

use actix_multipart::form::{MultipartForm, tempfile::TempFile};
use csv::Trim;
use pushkind_common::routes::empty_string_as_none;
use serde::Deserialize;
use thiserror::Error;
use validator::Validate;

use crate::domain::template::NewTemplate;

#[derive(Deserialize, Validate)]
pub struct AddTemplateForm {
    #[validate(length(min = 1))]
    #[serde(deserialize_with = "empty_string_as_none")]
    pub value: Option<String>,
}

impl AddTemplateForm {
    pub fn to_new_template(self, hub_id: i32) -> NewTemplate {
        NewTemplate::new(self.value, hub_id)
    }
}

#[derive(MultipartForm)]
/// Multipart form for uploading a CSV file with new templates.
pub struct UploadTemplatesForm {
    #[multipart(limit = "10MB")]
    /// Uploaded CSV file containing template data.
    pub csv: TempFile,
}

#[derive(Debug, Error)]
/// Errors that can occur while parsing an uploaded templates CSV file.
pub enum UploadTemplatesFormError {
    #[error("Error reading csv file")]
    FileReadError,
    #[error("Error parsing csv file")]
    CsvParseError,
}

impl From<std::io::Error> for UploadTemplatesFormError {
    fn from(_: std::io::Error) -> Self {
        UploadTemplatesFormError::FileReadError
    }
}

impl From<csv::Error> for UploadTemplatesFormError {
    fn from(_: csv::Error) -> Self {
        UploadTemplatesFormError::CsvParseError
    }
}

impl UploadTemplatesForm {
    /// Parse the uploaded CSV file into a list of [`NewTemplate`] records.
    pub fn parse(&mut self, hub_id: i32) -> Result<Vec<NewTemplate>, UploadTemplatesFormError> {
        self.csv.file.rewind()?;
        parse_templates(self.csv.file.by_ref(), hub_id)
    }
}

#[derive(Deserialize)]
struct TemplateCsvRow {
    #[serde(default, deserialize_with = "empty_string_as_none")]
    value: Option<String>,
}

fn parse_templates<R: Read>(
    reader: R,
    hub_id: i32,
) -> Result<Vec<NewTemplate>, UploadTemplatesFormError> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(Trim::All)
        .from_reader(reader);

    let mut templates = Vec::new();

    for row in csv_reader.deserialize::<TemplateCsvRow>() {
        let record = row?;

        if let Some(value) = record.value {
            templates.push(NewTemplate::new(Some(value), hub_id));
        }
    }

    Ok(templates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_templates_returns_records_with_values() {
        let csv = "value\nhello\nworld\n";
        let templates = parse_templates(Cursor::new(csv), 42).expect("should parse");

        assert_eq!(templates.len(), 2);
        assert_eq!(templates[0].hub_id, 42);
        assert_eq!(templates[0].value.as_deref(), Some("hello"));
        assert_eq!(templates[1].value.as_deref(), Some("world"));
    }

    #[test]
    fn parse_templates_skips_empty_or_missing_values() {
        let csv = "value\n\n  \nfoo\n";
        let templates = parse_templates(Cursor::new(csv), 7).expect("should parse");

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].hub_id, 7);
        assert_eq!(templates[0].value.as_deref(), Some("foo"));
    }

    #[test]
    fn parse_templates_propagates_csv_errors() {
        let csv = "value\nfoo,bar\n";

        match parse_templates(Cursor::new(csv), 1) {
            Err(UploadTemplatesFormError::CsvParseError) => {}
            Err(other) => panic!("expected csv parse error, got {:?}", other),
            Ok(templates) => panic!(
                "expected csv parse error but parsed {} rows",
                templates.len()
            ),
        }
    }
}
