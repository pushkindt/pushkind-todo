//! Helpers for loading compiled frontend assets and opening built HTML documents.

use std::path::Path;

use actix_files::NamedFile;
use thiserror::Error;

/// Root directory for built frontend artifacts emitted by Vite.
pub const FRONTEND_DIST_DIR: &str = "assets/dist";

/// Relative path of the Vite manifest inside [`FRONTEND_DIST_DIR`].
pub const FRONTEND_MANIFEST_PATH: &str = "manifest.json";

/// Built HTML document that will eventually back `GET /`.
pub const FRONTEND_INDEX_DOCUMENT: &str = "app/index.html";

/// Built HTML document that will eventually back `GET /task/{task_id}`.
pub const FRONTEND_TASK_DOCUMENT: &str = "app/task.html";

/// Built HTML document that will eventually back `GET /na`.
pub const FRONTEND_NO_ACCESS_DOCUMENT: &str = "app/no-access.html";

/// Errors raised while reading frontedn_assets.
#[derive(Debug, Error)]
pub enum FrontendAssetError {
    #[error("failed to read frontend asset: {0}")]
    Read(#[from] std::io::Error),
}

/// Open a Vite-built HTML document for a React-owned route.
pub async fn open_frontend_html(path: impl AsRef<Path>) -> Result<NamedFile, FrontendAssetError> {
    let file = NamedFile::open_async(path).await?;
    Ok(file.use_last_modified(true).prefer_utf8(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn can_open_existing_file() {
        let dir = tempdir().expect("tempdit should be created");
        let html_path = dir.path().join("index.html");
        std::fs::write(&html_path, "<!doctype html><title>ok<title>")
            .expect("html file should be written");

        let result = actix_web::rt::System::new().block_on(open_frontend_html(&html_path));
        assert!(result.is_ok());
    }

    #[test]
    fn missing_document_returns_read_error() {
        let error = actix_web::rt::System::new()
            .block_on(open_frontend_html("assets/dist/does-not-exist.html"))
            .expect_err("missing file should return an error");
        assert!(matches!(error, FrontendAssetError::Read(_)));
    }
}
