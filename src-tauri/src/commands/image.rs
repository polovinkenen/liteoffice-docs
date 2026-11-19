use std::path::Path;

use app_core::assets::{AssetStore, StoredAsset, MAX_ASSET_BYTES};
use app_core::{AppError, AppResult};
use tauri::{AppHandle, Manager};

use super::run_blocking;

fn store_for(app: &AppHandle) -> AppResult<AssetStore> {
    let directory = app.path().app_data_dir().map_err(|error| {
        AppError::Internal(format!("не удалось определить каталог данных: {error}"))
    })?;

    Ok(AssetStore::new(directory))
}

#[tauri::command]
pub async fn import_image(app: AppHandle, path: String) -> AppResult<StoredAsset> {
    let store = store_for(&app)?;

    run_blocking(move || store.store_image(Path::new(&path), MAX_ASSET_BYTES)).await
}

#[tauri::command]
pub async fn load_asset(app: AppHandle, asset_id: String) -> AppResult<String> {
    let store = store_for(&app)?;

    run_blocking(move || store.load_data_url(&asset_id)).await
}
