pub mod document;

use serde::Serialize;
use wordlite_app_core::{AppError, AppResult};
use wordlite_docmodel::PmDoc;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub platform: &'static str,
    pub arch: &'static str,
}

#[tauri::command]
pub fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "Wordlite",
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        arch: std::env::consts::ARCH,
    }
}

#[tauri::command]
pub fn validate_document(doc: serde_json::Value) -> AppResult<()> {
    let parsed: PmDoc = serde_json::from_value(doc)
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    parsed
        .validate()
        .map_err(|error| AppError::InvalidDocument(error.to_string()))
}
