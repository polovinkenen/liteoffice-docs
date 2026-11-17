use tauri::{AppHandle, Manager};
use wlite_app_core::recent::{RecentEntry, RecentStore};
use wlite_app_core::{AppError, AppResult};

use super::run_blocking;

fn store_for(app: &AppHandle) -> AppResult<RecentStore> {
    let directory = app.path().app_data_dir().map_err(|error| {
        AppError::Internal(format!("не удалось определить каталог данных: {error}"))
    })?;

    Ok(RecentStore::new(directory))
}

fn refresh_menu(app: &AppHandle) {
    if let Err(error) = crate::menu::refresh_menu(app) {
        tracing::warn!(%error, "не удалось обновить меню недавних документов");
    }
}

#[tauri::command]
pub async fn list_recent(app: AppHandle) -> AppResult<Vec<RecentEntry>> {
    let store = store_for(&app)?;

    run_blocking(move || store.list()).await
}

#[tauri::command]
pub async fn add_recent(app: AppHandle, path: String) -> AppResult<Vec<RecentEntry>> {
    let store = store_for(&app)?;
    let entries = run_blocking(move || store.add(&path)).await?;

    refresh_menu(&app);

    Ok(entries)
}

#[tauri::command]
pub async fn remove_recent(app: AppHandle, path: String) -> AppResult<Vec<RecentEntry>> {
    let store = store_for(&app)?;
    let entries = run_blocking(move || store.remove(&path)).await?;

    refresh_menu(&app);

    Ok(entries)
}

#[tauri::command]
pub async fn clear_recent(app: AppHandle) -> AppResult<Vec<RecentEntry>> {
    let store = store_for(&app)?;
    let entries = run_blocking(move || store.clear()).await?;

    refresh_menu(&app);

    Ok(entries)
}
