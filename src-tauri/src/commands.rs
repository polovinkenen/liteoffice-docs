pub mod autosave;
pub mod document;
pub mod image;
pub mod recent;
pub mod settings;

use std::sync::atomic::Ordering;

use serde::Serialize;
use tauri::State;
use wlite_app_core::{AppError, AppResult};
use wlite_docmodel::PmDoc;

use crate::AppState;

pub(crate) async fn run_blocking<T, F>(task: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| AppError::Internal(format!("фоновый поток завершился: {error}")))?
}

#[tauri::command]
pub fn set_document_dirty(state: State<'_, AppState>, dirty: bool) {
    state.dirty.store(dirty, Ordering::SeqCst);
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle, state: State<'_, AppState>) {
    state.allow_exit.store(true, Ordering::SeqCst);
    app.exit(0);
}

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
        name: "WLite",
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
