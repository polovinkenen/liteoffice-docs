use tauri::{AppHandle, Manager};
use wlite_app_core::autosave::{AutosaveRecord, AutosaveStore, AutosaveSummary};
use wlite_app_core::{AppError, AppResult};
use wlite_docmodel::PmDoc;

use super::run_blocking;

fn store_for(app: &AppHandle) -> AppResult<AutosaveStore> {
    let directory = app.path().app_data_dir().map_err(|error| {
        AppError::Internal(format!("не удалось определить каталог данных: {error}"))
    })?;

    Ok(AutosaveStore::new(directory))
}

#[tauri::command]
pub async fn autosave_document(
    app: AppHandle,
    doc_id: String,
    path: Option<String>,
    format: Option<String>,
    file_name: String,
    document: serde_json::Value,
) -> AppResult<AutosaveSummary> {
    let document: PmDoc = serde_json::from_value(document)
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    run_blocking(move || {
        let store = store_for(&app)?;
        store.save(&doc_id, path, format, file_name, document)
    })
    .await
}

#[tauri::command]
pub async fn list_autosaves(app: AppHandle) -> AppResult<Vec<AutosaveSummary>> {
    run_blocking(move || {
        let store = store_for(&app)?;
        store.list()
    })
    .await
}

#[tauri::command]
pub async fn restore_autosave(app: AppHandle, doc_id: String) -> AppResult<AutosaveRecord> {
    run_blocking(move || {
        let store = store_for(&app)?;
        store.read(&doc_id)
    })
    .await
}

#[tauri::command]
pub async fn discard_autosave(app: AppHandle, doc_id: String) -> AppResult<()> {
    run_blocking(move || {
        let store = store_for(&app)?;
        store.remove(&doc_id)
    })
    .await
}

#[tauri::command]
pub async fn clear_autosaves(app: AppHandle) -> AppResult<()> {
    run_blocking(move || {
        let store = store_for(&app)?;
        store.clear()
    })
    .await
}
