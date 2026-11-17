use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use wlite_docmodel::PmDoc;

use crate::files::{read_text, write_text_atomic};
use crate::{AppError, AppResult};

pub const AUTOSAVE_DIR: &str = "autosave";
pub const MAX_AUTOSAVE_BYTES: usize = 20 * 1024 * 1024;
pub const MAX_DOC_ID_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutosaveRecord {
    pub doc_id: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    pub file_name: String,
    pub updated_at: u64,
    pub document: PmDoc,
}

impl AutosaveRecord {
    pub fn summary(&self) -> AutosaveSummary {
        AutosaveSummary {
            doc_id: self.doc_id.clone(),
            path: self.path.clone(),
            format: self.format.clone(),
            file_name: self.file_name.clone(),
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutosaveSummary {
    pub doc_id: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    pub file_name: String,
    pub updated_at: u64,
}

pub struct AutosaveStore {
    root: PathBuf,
}

impl AutosaveStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn dir(&self) -> PathBuf {
        self.root.join(AUTOSAVE_DIR)
    }

    pub fn save(
        &self,
        doc_id: &str,
        path: Option<String>,
        format: Option<String>,
        file_name: String,
        document: PmDoc,
    ) -> AppResult<AutosaveSummary> {
        self.save_with_updated_at(doc_id, path, format, file_name, document, now_millis())
    }

    fn save_with_updated_at(
        &self,
        doc_id: &str,
        path: Option<String>,
        format: Option<String>,
        file_name: String,
        document: PmDoc,
        updated_at: u64,
    ) -> AppResult<AutosaveSummary> {
        let file_path = self.path_for(doc_id)?;

        document
            .validate()
            .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

        let record = AutosaveRecord {
            doc_id: doc_id.to_owned(),
            path,
            format,
            file_name,
            updated_at,
            document,
        };
        let payload = serde_json::to_string(&record)
            .map_err(|error| AppError::Internal(error.to_string()))?;

        if payload.len() > MAX_AUTOSAVE_BYTES {
            return Err(AppError::FileTooLarge {
                path: file_path.display().to_string(),
                size: payload.len() as u64,
                max_bytes: MAX_AUTOSAVE_BYTES as u64,
            });
        }

        std::fs::create_dir_all(self.dir())?;
        write_text_atomic(&file_path, &payload)?;

        Ok(record.summary())
    }

    pub fn list(&self) -> AppResult<Vec<AutosaveSummary>> {
        let entries = match std::fs::read_dir(self.dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(AppError::Io(error)),
        };

        let mut summaries = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }

            let Ok(text) = read_text(&path, MAX_AUTOSAVE_BYTES as u64) else {
                continue;
            };
            let Ok(summary) = serde_json::from_str::<AutosaveSummary>(&text) else {
                continue;
            };
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };

            if stem != summary.doc_id {
                continue;
            }

            summaries.push(summary);
        }

        summaries.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.doc_id.cmp(&right.doc_id))
        });

        Ok(summaries)
    }

    pub fn read(&self, doc_id: &str) -> AppResult<AutosaveRecord> {
        let path = self.path_for(doc_id)?;
        let text = read_text(&path, MAX_AUTOSAVE_BYTES as u64)?;
        let record: AutosaveRecord = serde_json::from_str(&text).map_err(|error| {
            AppError::InvalidDocument(format!("не удалось прочитать автосохранение: {error}"))
        })?;

        if record.doc_id != doc_id {
            return Err(AppError::InvalidDocument(
                "идентификатор автосохранения не совпадает с именем файла".to_owned(),
            ));
        }

        record
            .document
            .validate()
            .map_err(|error| AppError::InvalidDocument(error.to_string()))?;

        Ok(record)
    }

    pub fn remove(&self, doc_id: &str) -> AppResult<()> {
        let path = self.path_for(doc_id)?;

        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AppError::Io(error)),
        }
    }

    pub fn clear(&self) -> AppResult<()> {
        match std::fs::remove_dir_all(self.dir()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AppError::Io(error)),
        }
    }

    fn path_for(&self, doc_id: &str) -> AppResult<PathBuf> {
        if !is_valid_doc_id(doc_id) {
            return Err(AppError::InvalidArgument(
                "некорректный идентификатор документа".to_owned(),
            ));
        }

        Ok(self.dir().join(format!("{doc_id}.json")))
    }
}

pub fn is_valid_doc_id(doc_id: &str) -> bool {
    !doc_id.is_empty()
        && doc_id.len() <= MAX_DOC_ID_LENGTH
        && doc_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
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
    use wlite_docmodel::PmNode;

    fn document(text: &str) -> PmDoc {
        PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content("paragraph", vec![PmNode::text(text)])],
        }
    }

    #[test]
    fn saves_lists_and_restores() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AutosaveStore::new(directory.path());
        let original = document("черновик");

        let summary = store
            .save(
                "doc-1",
                Some("/tmp/заметка.md".to_owned()),
                Some("md".to_owned()),
                "заметка.md".to_owned(),
                original.clone(),
            )
            .expect("автосохранение");

        assert_eq!(summary.doc_id, "doc-1");
        assert_eq!(summary.file_name, "заметка.md");

        let listed = store.list().expect("список");
        assert_eq!(listed, vec![summary]);

        let restored = store.read("doc-1").expect("восстановление");
        assert_eq!(restored.document, original);
        assert_eq!(restored.path.as_deref(), Some("/tmp/заметка.md"));
    }

    #[test]
    fn orders_summaries_by_update_time() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AutosaveStore::new(directory.path());

        store
            .save_with_updated_at("old", None, None, "old".to_owned(), document("старый"), 100)
            .expect("старое автосохранение");
        store
            .save_with_updated_at("new", None, None, "new".to_owned(), document("новый"), 200)
            .expect("новое автосохранение");

        let listed = store.list().expect("список");
        let ids: Vec<&str> = listed.iter().map(|entry| entry.doc_id.as_str()).collect();

        assert_eq!(ids, vec!["new", "old"]);
    }

    #[test]
    fn removes_and_clears() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AutosaveStore::new(directory.path());

        store
            .save(
                "doc-1",
                None,
                None,
                "Без названия".to_owned(),
                document("текст"),
            )
            .expect("автосохранение");
        store.remove("doc-1").expect("удаление");
        assert!(store.list().expect("список").is_empty());

        store
            .save(
                "doc-2",
                None,
                None,
                "Без названия".to_owned(),
                document("текст"),
            )
            .expect("автосохранение");
        store.clear().expect("очистка");
        assert!(store.list().expect("список").is_empty());
        store.remove("missing").expect("удаление отсутствующего");
    }

    #[test]
    fn ignores_corrupt_files() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AutosaveStore::new(directory.path());

        std::fs::create_dir_all(store.dir()).expect("каталог автосохранений");
        std::fs::write(store.dir().join("broken.json"), "не json").expect("запись");
        std::fs::write(store.dir().join("other.txt"), "текст").expect("запись");

        assert!(store.list().expect("список").is_empty());
    }

    #[test]
    fn rejects_invalid_doc_id() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AutosaveStore::new(directory.path());

        for doc_id in ["", "../escape", "a/b", "a.b", &"x".repeat(65)] {
            let error = store
                .save(doc_id, None, None, "имя".to_owned(), document("текст"))
                .expect_err("ожидалась ошибка");

            assert_eq!(error.code(), "invalid_argument");
        }
    }

    #[test]
    fn rejects_invalid_document() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AutosaveStore::new(directory.path());
        let invalid = PmDoc {
            node_type: "paragraph".to_owned(),
            attrs: None,
            content: Vec::new(),
        };

        let error = store
            .save("doc-1", None, None, "имя".to_owned(), invalid)
            .expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_document");
    }
}
