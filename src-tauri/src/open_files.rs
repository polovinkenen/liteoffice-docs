use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use formats::DocumentFormat;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

pub const OPEN_FILES_EVENT: &str = "app:open-files";

#[derive(Default)]
struct PendingPaths {
    paths: Vec<String>,
    frontend_ready: bool,
}

#[derive(Default)]
pub struct PendingOpenFiles(Mutex<PendingPaths>);

impl PendingOpenFiles {
    pub(crate) fn push_all(&self, paths: Vec<String>) {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());

        for path in paths {
            if !pending.paths.contains(&path) {
                pending.paths.push(path);
            }
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .frontend_ready
    }

    pub(crate) fn take(&self) -> Vec<String> {
        let mut pending = self.0.lock().unwrap_or_else(|error| error.into_inner());
        pending.frontend_ready = true;
        std::mem::take(&mut pending.paths)
    }
}

fn normalize(raw: &OsStr, cwd: &Path) -> Option<String> {
    let text = raw.to_string_lossy();

    if text.is_empty() {
        return None;
    }

    let candidate = PathBuf::from(raw);
    let absolute = if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    };

    if text.starts_with('-') && !absolute.exists() {
        return None;
    }

    let canonical = std::fs::canonicalize(&absolute).ok()?;

    if !canonical.is_file() || DocumentFormat::from_path(&canonical).is_none() {
        return None;
    }

    Some(canonical.to_string_lossy().into_owned())
}

pub fn collect_paths<I, S>(args: I, cwd: &Path) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut paths: Vec<String> = Vec::new();

    for arg in args {
        if let Some(path) = normalize(arg.as_ref(), cwd) {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }

    paths
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn collect_urls(urls: &[tauri::Url]) -> Vec<String> {
    let paths: Vec<PathBuf> = urls
        .iter()
        .filter_map(|url| url.to_file_path().ok())
        .collect();

    collect_paths(paths, Path::new(""))
}

pub fn queue<R: Runtime>(app: &AppHandle<R>, paths: Vec<String>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }

    if paths.is_empty() {
        return;
    }

    let pending = app.state::<PendingOpenFiles>();

    if pending.is_ready() {
        let _ = app.emit(OPEN_FILES_EVENT, paths);
    } else {
        pending.push_all(paths);
    }
}

#[tauri::command]
pub fn take_opened_paths(state: State<'_, PendingOpenFiles>) -> Vec<String> {
    state.take()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::fs;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("временный каталог")
    }

    #[test]
    fn collects_supported_existing_files_and_skips_the_rest() {
        let directory = temp_dir();
        let root = directory.path();
        fs::create_dir(root.join("nested")).expect("каталог");
        fs::write(root.join("заметка.md"), "# Заголовок").expect("файл");
        fs::write(root.join("nested").join("текст.txt"), "текст").expect("файл");
        fs::write(root.join("данные.xyz"), "?").expect("файл");

        let args = vec![
            OsString::from("-psn_0_1"),
            OsString::from("заметка.md"),
            OsString::from("nested/текст.txt"),
            OsString::from("данные.xyz"),
            OsString::from("нет-такого.md"),
            OsString::from(""),
        ];
        let paths = collect_paths(args, root);

        assert_eq!(paths.len(), 2);
        assert!(paths[0].ends_with("заметка.md"));
        assert!(paths[1].ends_with("текст.txt"));
    }

    #[test]
    fn resolves_absolute_paths_without_cwd() {
        let directory = temp_dir();
        let file = directory.path().join("документ.docx");
        fs::write(&file, b"docx").expect("файл");

        let paths = collect_paths(vec![file.as_os_str().to_owned()], Path::new("/nonexistent"));

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("документ.docx"));
    }

    #[test]
    fn deduplicates_same_file() {
        let directory = temp_dir();
        let root = directory.path();
        fs::write(root.join("a.html"), "<p>a</p>").expect("файл");

        let paths = collect_paths(
            vec![OsString::from("a.html"), OsString::from("./a.html")],
            root,
        );

        assert_eq!(paths.len(), 1);
    }

    #[test]
    fn skips_unsupported_and_directories() {
        let directory = temp_dir();
        let root = directory.path();
        fs::create_dir(root.join("папка.md")).expect("каталог");
        fs::write(root.join("a.rtf"), "{\\rtf1}").expect("файл");

        let paths = collect_paths(
            vec![OsString::from("папка.md"), OsString::from("a.rtf")],
            root,
        );

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("a.rtf"));
    }

    #[test]
    fn collects_file_urls() {
        let directory = temp_dir();
        let file = directory.path().join("файл.md");
        fs::write(&file, "текст").expect("файл");
        let url = tauri::Url::from_file_path(&file).expect("url");
        let remote = tauri::Url::parse("https://example.com/doc.md").expect("url");

        let paths = collect_urls(&[url, remote]);

        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("файл.md"));
    }

    #[test]
    fn drains_pending_paths_once() {
        let pending = PendingOpenFiles::default();
        pending.push_all(vec!["/tmp/a.md".into(), "/tmp/a.md".into()]);

        assert_eq!(pending.take(), vec!["/tmp/a.md".to_string()]);
        assert!(pending.take().is_empty());
    }

    #[test]
    fn push_merges_into_existing_pending_list() {
        let pending = PendingOpenFiles::default();
        pending.push_all(vec!["/tmp/a.md".into()]);
        pending.push_all(vec!["/tmp/b.txt".into(), "/tmp/a.md".into()]);

        assert_eq!(
            pending.take(),
            vec!["/tmp/a.md".to_string(), "/tmp/b.txt".to_string()]
        );
    }

    #[test]
    fn marks_frontend_ready_after_drain() {
        let pending = PendingOpenFiles::default();
        assert!(!pending.is_ready());

        pending.take();

        assert!(pending.is_ready());
    }
}
