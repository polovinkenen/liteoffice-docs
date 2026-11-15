use std::path::{Path, PathBuf};

use base64::Engine;
use serde::Serialize;
use wordlite_app_core::files::read_bytes;
use wordlite_app_core::{AppError, AppResult};

use super::run_blocking;

pub const MAX_IMAGE_BYTES: u64 = 15 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadedImage {
    pub data_url: String,
    pub name: String,
    pub alt: String,
}

#[tauri::command]
pub async fn load_image(path: String) -> AppResult<LoadedImage> {
    run_blocking(move || load_image_sync(path)).await
}

fn load_image_sync(path: String) -> AppResult<LoadedImage> {
    let path_buf = PathBuf::from(&path);
    let mime = image_mime(&path_buf)?;
    let bytes = read_bytes(&path_buf, MAX_IMAGE_BYTES)?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);

    let name = path_buf
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("изображение")
        .to_owned();
    let alt = path_buf
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("изображение")
        .to_owned();

    Ok(LoadedImage {
        data_url: format!("data:{mime};base64,{encoded}"),
        name,
        alt,
    })
}

fn image_mime(path: &Path) -> AppResult<&'static str> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("png") => Ok("image/png"),
        Some("jpg" | "jpeg") => Ok("image/jpeg"),
        Some("gif") => Ok("image/gif"),
        Some("webp") => Ok("image/webp"),
        _ => Err(AppError::UnsupportedFormat(format!(
            "{} (поддерживаются: png, jpg, jpeg, gif, webp)",
            path.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;

    #[test]
    fn builds_data_url_for_png() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let path = directory.path().join("схема.png");
        let content = [0x89, b'P', b'N', b'G'];
        std::fs::write(&path, content).expect("запись");

        let image = load_image_sync(path.to_string_lossy().into_owned()).expect("загрузка");

        assert!(image.data_url.starts_with("data:image/png;base64,"));
        assert_eq!(image.name, "схема.png");
        assert_eq!(image.alt, "схема");

        let encoded = image.data_url.split(',').nth(1).expect("payload");
        assert_eq!(STANDARD.decode(encoded).expect("base64"), content);
    }

    #[test]
    fn detects_jpeg_extension() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let path = directory.path().join("фото.JPEG");
        std::fs::write(&path, [0xff, 0xd8, 0xff]).expect("запись");

        let image = load_image_sync(path.to_string_lossy().into_owned()).expect("загрузка");

        assert!(image.data_url.starts_with("data:image/jpeg;base64,"));
    }

    #[test]
    fn rejects_unsupported_image_format() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let path = directory.path().join("рисунок.svg");
        std::fs::write(&path, "<svg />").expect("запись");

        let error =
            load_image_sync(path.to_string_lossy().into_owned()).expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "unsupported_format");
    }
}
