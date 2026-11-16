use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::files::{read_text, write_text_atomic};
use crate::{AppError, AppResult};

pub const RECENT_FILE: &str = "recent.json";
pub const MAX_RECENT_ENTRIES: usize = 15;
pub const MAX_RECENT_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentEntry {
    pub path: String,
    pub file_name: String,
    pub opened_at: u64,
}

pub struct RecentStore {
    root: PathBuf,
}

impl RecentStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(RECENT_FILE)
    }

    pub fn list(&self) -> AppResult<Vec<RecentEntry>> {
        let path = self.path();

        match std::fs::metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(AppError::Io(error)),
        }

        let text = read_text(&path, MAX_RECENT_BYTES)?;

        serde_json::from_str(&text).map_err(|error| {
            AppError::InvalidDocument(format!("не удалось прочитать недавние документы: {error}"))
        })
    }

    pub fn add(&self, path: &str) -> AppResult<Vec<RecentEntry>> {
        self.add_with_opened_at(path, now_millis())
    }

    fn add_with_opened_at(&self, path: &str, opened_at: u64) -> AppResult<Vec<RecentEntry>> {
        let path = normalized_path(path)?;
        let file_name = PathBuf::from(&path)
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| path.clone());
        let mut entries = self.load_or_empty()?;

        entries.retain(|entry| entry.path != path);
        entries.insert(
            0,
            RecentEntry {
                path,
                file_name,
                opened_at,
            },
        );
        entries.truncate(MAX_RECENT_ENTRIES);
        self.save(&entries)?;

        Ok(entries)
    }

    pub fn remove(&self, path: &str) -> AppResult<Vec<RecentEntry>> {
        let path = normalized_path(path)?;
        let mut entries = self.load_or_empty()?;

        entries.retain(|entry| entry.path != path);
        self.save(&entries)?;

        Ok(entries)
    }

    pub fn clear(&self) -> AppResult<Vec<RecentEntry>> {
        match std::fs::remove_file(self.path()) {
            Ok(()) => Ok(Vec::new()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(AppError::Io(error)),
        }
    }

    fn load_or_empty(&self) -> AppResult<Vec<RecentEntry>> {
        match self.list() {
            Ok(entries) => Ok(entries),
            Err(AppError::InvalidDocument(_)) => Ok(Vec::new()),
            Err(error) => Err(error),
        }
    }

    fn save(&self, entries: &[RecentEntry]) -> AppResult<()> {
        let path = self.path();
        let payload = serde_json::to_string_pretty(entries)
            .map_err(|error| AppError::Internal(error.to_string()))?;

        if payload.len() as u64 > MAX_RECENT_BYTES {
            return Err(AppError::FileTooLarge {
                path: path.display().to_string(),
                size: payload.len() as u64,
                max_bytes: MAX_RECENT_BYTES,
            });
        }

        std::fs::create_dir_all(&self.root)?;
        write_text_atomic(&path, &payload)
    }
}

fn normalized_path(path: &str) -> AppResult<String> {
    if path.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "путь документа не может быть пустым".to_owned(),
        ));
    }

    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path));

    Ok(canonical.to_string_lossy().into_owned())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(directory: &std::path::Path, name: &str) -> String {
        let path = directory.join(name);
        std::fs::write(&path, "текст").expect("создание файла");
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn missing_file_returns_empty_list() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());

        assert!(store.list().expect("список").is_empty());
    }

    #[test]
    fn adds_entries_newest_first() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());
        let first = touch(directory.path(), "первый.md");
        let second = touch(directory.path(), "второй.txt");

        store.add(&first).expect("первый");
        let entries = store.add(&second).expect("второй");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file_name, "второй.txt");
        assert_eq!(entries[1].file_name, "первый.md");
    }

    #[test]
    fn deduplicates_by_path_and_moves_to_top() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());
        let first = touch(directory.path(), "первый.md");
        let second = touch(directory.path(), "второй.md");

        store.add_with_opened_at(&first, 10).expect("первый");
        store.add_with_opened_at(&second, 20).expect("второй");
        let entries = store
            .add_with_opened_at(&first, 30)
            .expect("повторный первый");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, normalized_path(&first).expect("путь"));
        assert_eq!(entries[0].opened_at, 30);
    }

    #[test]
    fn limits_entries() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());

        for index in 0..(MAX_RECENT_ENTRIES + 3) {
            let path = touch(directory.path(), &format!("файл-{index}.md"));
            store.add(&path).expect("добавление");
        }

        let entries = store.list().expect("список");

        assert_eq!(entries.len(), MAX_RECENT_ENTRIES);
        assert_eq!(
            entries[0].file_name,
            format!("файл-{}.md", MAX_RECENT_ENTRIES + 2)
        );
    }

    #[test]
    fn removes_and_clears_entries() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());
        let first = touch(directory.path(), "первый.md");
        let second = touch(directory.path(), "второй.md");

        store.add(&first).expect("первый");
        store.add(&second).expect("второй");
        let entries = store.remove(&first).expect("удаление");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, normalized_path(&second).expect("путь"));
        assert!(store.clear().expect("очистка").is_empty());
        assert!(store.list().expect("список").is_empty());
        assert!(store.clear().expect("повторная очистка").is_empty());
    }

    #[test]
    fn rejects_empty_path() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());

        let error = store.add("   ").expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_argument");
    }

    #[test]
    fn reports_corrupt_file() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());
        std::fs::write(store.path(), "не json").expect("запись");

        let error = store.list().expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_document");
    }

    #[test]
    fn recovers_from_corrupt_file_on_add() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = RecentStore::new(directory.path());
        let path = touch(directory.path(), "документ.md");
        std::fs::write(store.path(), "не json").expect("запись");

        let entries = store.add(&path).expect("добавление");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].file_name, "документ.md");
    }
}
