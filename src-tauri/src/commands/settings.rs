use tauri::{AppHandle, Manager};
use wlite_app_core::settings::{Settings, SettingsStore};
use wlite_app_core::{AppError, AppResult};

use super::run_blocking;

fn store_for(app: &AppHandle) -> AppResult<SettingsStore> {
    let directory = app.path().app_data_dir().map_err(|error| {
        AppError::Internal(format!("не удалось определить каталог данных: {error}"))
    })?;

    Ok(SettingsStore::new(directory))
}

#[tauri::command]
pub async fn load_settings(app: AppHandle) -> AppResult<Settings> {
    run_blocking(move || {
        let store = store_for(&app)?;
        store.load()
    })
    .await
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: Settings) -> AppResult<Settings> {
    run_blocking(move || {
        let store = store_for(&app)?;
        store.save(&settings)?;
        Ok(settings)
    })
    .await
}
