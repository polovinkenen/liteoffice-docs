use std::path::{Path, PathBuf};

use serde::Serialize;
use wordlite_app_core::files::{read_text, write_text_atomic, MAX_TEXT_FILE_BYTES};
use wordlite_app_core::{AppError, AppResult};
use wordlite_docmodel::PmDoc;
use wordlite_formats::{codec_for, DocumentFormat};

use super::run_blocking;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocument {
    pub path: String,
    pub format: String,
    pub document: PmDoc,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDocument {
    pub path: String,
    pub format: String,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn open_document(path: String) -> AppResult<OpenedDocument> {
    run_blocking(move || open_document_sync(path)).await
}

#[tauri::command]
pub async fn save_document(path: String, document: serde_json::Value) -> AppResult<SavedDocument> {
    let document: PmDoc = serde_json::from_value(document)
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    run_blocking(move || save_document_sync(path, document)).await
}

pub(crate) fn open_document_sync(path: String) -> AppResult<OpenedDocument> {
    let path_buf = PathBuf::from(&path);
    let format = format_for(&path_buf)?;
    let source = read_text(&path_buf, MAX_TEXT_FILE_BYTES)?;
    let imported = codec_for(format)
        .read(&source)
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    imported
        .document
        .validate()
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    tracing::info!(
        path = %path_buf.display(),
        format = format.id(),
        warnings = imported.warnings.len(),
        "документ открыт"
    );

    Ok(OpenedDocument {
        path,
        format: format.id().to_owned(),
        document: imported.document,
        warnings: imported.warnings,
    })
}

pub(crate) fn save_document_sync(path: String, document: PmDoc) -> AppResult<SavedDocument> {
    let path_buf = PathBuf::from(&path);
    let format = format_for(&path_buf)?;

    document
        .validate()
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    let written = codec_for(format)
        .write(&document)
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    write_text_atomic(&path_buf, &written.text)?;

    tracing::info!(
        path = %path_buf.display(),
        format = format.id(),
        warnings = written.warnings.len(),
        "документ сохранён"
    );

    Ok(SavedDocument {
        path,
        format: format.id().to_owned(),
        warnings: written.warnings,
    })
}

fn format_for(path: &Path) -> AppResult<DocumentFormat> {
    DocumentFormat::from_path(path).ok_or_else(|| {
        AppError::UnsupportedFormat(format!(
            "{} (поддерживаются: {})",
            path.display(),
            DocumentFormat::supported_extensions().join(", ")
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn saves_and_reopens_markdown_document() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let path = directory.path().join("заметка.md");
        let path_string = path.to_string_lossy().into_owned();
        let document: PmDoc = serde_json::from_value(json!({
            "type": "doc",
            "content": [{
                "type": "heading",
                "attrs": { "level": 1 },
                "content": [{ "type": "text", "text": "Заголовок" }]
            }]
        }))
        .expect("документ");

        let saved = save_document_sync(path_string.clone(), document.clone()).expect("сохранение");

        assert_eq!(saved.format, "md");
        assert_eq!(
            std::fs::read_to_string(&path).expect("чтение"),
            "# Заголовок\n"
        );

        let opened = open_document_sync(path_string).expect("открытие");

        assert_eq!(opened.format, "md");
        assert_eq!(opened.document, document);
        assert!(opened.warnings.is_empty());
    }

    #[test]
    fn rejects_unsupported_extension() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let path = directory.path().join("документ.odt");

        let error = save_document_sync(path.to_string_lossy().into_owned(), PmDoc::empty())
            .expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "unsupported_format");
    }
}
