use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use wlite_app_core::files::write_bytes_atomic;
use wlite_app_core::{AppError, AppResult};
use wlite_docmodel::PmDoc;
use wlite_export_pdf::{render, PdfOptions};

use super::document::asset_store;
use super::run_blocking;

const PREVIEW_KEEP: usize = 5;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfResult {
    pub path: String,
    pub pages: usize,
    pub bytes: usize,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn export_pdf(
    app: AppHandle,
    document: serde_json::Value,
    options: PdfOptions,
    out_path: Option<String>,
    title: Option<String>,
) -> AppResult<PdfResult> {
    let mut document: PmDoc = serde_json::from_value(document)
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    document
        .validate()
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    let assets = asset_store(&app)?;
    let target = match out_path {
        Some(path) => path,
        None => preview_target(&app, title.as_deref())?,
    };

    run_blocking(move || {
        assets.resolve_document(&mut document)?;
        export_pdf_to(&document, &options, title.as_deref(), Path::new(&target))
    })
    .await
}

#[tauri::command]
pub fn open_pdf_preview(app: AppHandle, path: String) -> AppResult<()> {
    let directory = preview_dir(&app)?;
    std::fs::create_dir_all(&directory)?;
    let canonical_path = validate_preview_path(Path::new(&path), &directory)?;

    tauri_plugin_opener::open_path(&canonical_path, None::<&str>)
        .map_err(|error| AppError::Internal(format!("не удалось открыть PDF: {error}")))
}

fn validate_preview_path(path: &Path, directory: &Path) -> AppResult<PathBuf> {
    let canonical_dir = directory.canonicalize()?;
    let canonical_path = path.canonicalize()?;

    if !canonical_path.starts_with(&canonical_dir) || !canonical_path.is_file() {
        return Err(AppError::InvalidArgument(
            "предпросмотр PDF доступен только для файлов WLite".to_owned(),
        ));
    }

    Ok(canonical_path)
}

pub(crate) fn export_pdf_to(
    document: &PmDoc,
    options: &PdfOptions,
    title: Option<&str>,
    target: &Path,
) -> AppResult<PdfResult> {
    validate_out_path(target)?;
    options
        .validate()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;

    let rendered = render(document, options, title)
        .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

    write_bytes_atomic(target, &rendered.bytes)?;

    tracing::info!(
        path = %target.display(),
        pages = rendered.pages,
        bytes = rendered.bytes.len(),
        warnings = rendered.warnings.len(),
        "PDF экспортирован"
    );

    Ok(PdfResult {
        path: target.to_string_lossy().into_owned(),
        pages: rendered.pages,
        bytes: rendered.bytes.len(),
        warnings: rendered.warnings,
    })
}

fn validate_out_path(path: &Path) -> AppResult<()> {
    let is_pdf = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);

    if !is_pdf {
        return Err(AppError::InvalidArgument(
            "файл PDF должен иметь расширение .pdf".to_owned(),
        ));
    }

    Ok(())
}

fn preview_dir(app: &AppHandle) -> AppResult<PathBuf> {
    let directory = app
        .path()
        .app_cache_dir()
        .map_err(|error| {
            AppError::Internal(format!("не удалось определить кэш приложения: {error}"))
        })?
        .join("pdf-preview");

    Ok(directory)
}

fn preview_target(app: &AppHandle, title: Option<&str>) -> AppResult<String> {
    let directory = preview_dir(app)?;
    std::fs::create_dir_all(&directory)?;

    let slug = preview_slug(title);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let path = directory.join(format!("{slug}-{timestamp}.pdf"));

    prune_previews(&directory, PREVIEW_KEEP);

    Ok(path.to_string_lossy().into_owned())
}

fn preview_slug(title: Option<&str>) -> String {
    let mut slug: String = title
        .unwrap_or("документ")
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .take(48)
        .collect();

    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }

    let slug = slug.trim_matches('-');

    if slug.is_empty() {
        "документ".to_owned()
    } else {
        slug.to_owned()
    }
}

fn prune_previews(directory: &Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut previews: Vec<(SystemTime, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();

            if path.extension().and_then(|value| value.to_str()) != Some("pdf") {
                return None;
            }

            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect();

    if previews.len() <= keep {
        return;
    }

    previews.sort_by_key(|(modified, _)| *modified);
    let remove = previews.len() - keep;

    for (_, path) in previews.into_iter().take(remove) {
        if let Err(error) = std::fs::remove_file(&path) {
            tracing::warn!(path = %path.display(), %error, "не удалось удалить старый предпросмотр PDF");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn document() -> PmDoc {
        serde_json::from_value(json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [{ "type": "text", "text": "Проверка PDF" }]
            }]
        }))
        .expect("документ")
    }

    #[test]
    fn exports_pdf_file() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let path = directory.path().join("документ.pdf");

        let result = export_pdf_to(&document(), &PdfOptions::default(), Some("Проверка"), &path)
            .expect("экспорт");

        assert_eq!(result.pages, 1);
        assert!(result.bytes > 0);
        let bytes = std::fs::read(&path).expect("чтение");
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn rejects_non_pdf_target() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let path = directory.path().join("документ.txt");

        let error = export_pdf_to(&document(), &PdfOptions::default(), None, &path)
            .expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_argument");
    }

    #[test]
    fn allows_only_preview_directory_files() {
        let root = tempfile::tempdir().expect("временный каталог");
        let preview = root.path().join("preview");
        std::fs::create_dir_all(&preview).expect("каталог");
        let inside = preview.join("документ.pdf");
        let outside = root.path().join("документ.pdf");
        std::fs::write(&inside, b"%PDF-").expect("запись");
        std::fs::write(&outside, b"%PDF-").expect("запись");

        let allowed = validate_preview_path(&inside, &preview).expect("свой файл");
        assert_eq!(allowed, inside.canonicalize().expect("путь"));

        let error = validate_preview_path(&outside, &preview).expect_err("чужой файл");
        assert_eq!(error.code(), "invalid_argument");

        let directory_error = validate_preview_path(&preview, &preview).expect_err("каталог");
        assert_eq!(directory_error.code(), "invalid_argument");
    }

    #[test]
    fn builds_preview_slug_and_keeps_files() {
        assert_eq!(preview_slug(Some("Отчёт 2026 / тест")), "Отчёт-2026-тест");
        assert_eq!(preview_slug(Some("***")), "документ");
        assert_eq!(preview_slug(None), "документ");

        let directory = tempfile::tempdir().expect("временный каталог");

        for index in 0..8 {
            std::fs::write(directory.path().join(format!("{index}.pdf")), b"%PDF-")
                .expect("запись");
        }

        std::fs::write(directory.path().join("заметка.txt"), b"txt").expect("запись");
        prune_previews(directory.path(), 3);
        let remaining = std::fs::read_dir(directory.path()).expect("чтение").count();

        assert_eq!(remaining, 4);
    }
}
