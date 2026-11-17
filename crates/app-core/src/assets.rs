use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wlite_docmodel::{PmDoc, PmNode};

use crate::files::{read_bytes, write_bytes_atomic};
use crate::{AppError, AppResult};

pub const ASSETS_DIR: &str = "assets";
pub const MAX_ASSET_BYTES: u64 = 15 * 1024 * 1024;

static ASSET_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAsset {
    pub asset_id: String,
    pub file_name: String,
    pub alt: String,
}

pub struct AssetStore {
    root: PathBuf,
}

impl AssetStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn dir(&self) -> PathBuf {
        self.root.join(ASSETS_DIR)
    }

    pub fn store_image(&self, path: &Path, max_bytes: u64) -> AppResult<StoredAsset> {
        let (extension, _) = image_format(path)?;
        let bytes = read_bytes(path, max_bytes)?;
        let asset_id = new_asset_id();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("изображение")
            .to_owned();
        let alt = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("изображение")
            .to_owned();

        std::fs::create_dir_all(self.dir())?;
        write_bytes_atomic(&self.dir().join(format!("{asset_id}.{extension}")), &bytes)?;

        Ok(StoredAsset {
            asset_id,
            file_name,
            alt,
        })
    }

    pub fn load_data_url(&self, asset_id: &str) -> AppResult<String> {
        if !is_valid_asset_id(asset_id) {
            return Err(AppError::InvalidArgument(
                "некорректный идентификатор изображения".to_owned(),
            ));
        }

        let entries = match std::fs::read_dir(self.dir()) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(AppError::InvalidDocument(format!(
                    "изображение {asset_id} не найдено"
                )));
            }
            Err(error) => return Err(AppError::Io(error)),
        };

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };

            if stem != asset_id {
                continue;
            }

            let (_, mime) = image_format(&path)?;
            let bytes = read_bytes(&path, MAX_ASSET_BYTES)?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);

            return Ok(format!("data:{mime};base64,{encoded}"));
        }

        Err(AppError::InvalidDocument(format!(
            "изображение {asset_id} не найдено"
        )))
    }

    pub fn resolve_document(&self, document: &mut PmDoc) -> AppResult<()> {
        for node in &mut document.content {
            self.resolve_node(node)?;
        }

        Ok(())
    }

    fn resolve_node(&self, node: &mut PmNode) -> AppResult<()> {
        if node.node_type == "image" {
            let asset_id = node
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("src"))
                .and_then(|src| src.as_str())
                .and_then(|src| src.strip_prefix("asset:"))
                .map(str::to_owned);

            if let Some(asset_id) = asset_id {
                let data_url = self.load_data_url(&asset_id)?;

                if let Some(attrs) = node.attrs.as_mut().and_then(Value::as_object_mut) {
                    attrs.insert("src".to_owned(), Value::String(data_url));
                }
            }
        }

        for child in &mut node.content {
            self.resolve_node(child)?;
        }

        Ok(())
    }
}

pub fn is_valid_asset_id(asset_id: &str) -> bool {
    !asset_id.is_empty()
        && asset_id.len() <= 64
        && asset_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn image_format(path: &Path) -> AppResult<(&'static str, &'static str)> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    match extension.as_deref() {
        Some("png") => Ok(("png", "image/png")),
        Some("jpg" | "jpeg") => Ok(("jpg", "image/jpeg")),
        Some("gif") => Ok(("gif", "image/gif")),
        Some("webp") => Ok(("webp", "image/webp")),
        _ => Err(AppError::UnsupportedFormat(format!(
            "{} (поддерживаются: png, jpg, jpeg, gif, webp)",
            path.display()
        ))),
    }
}

fn new_asset_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let sequence = ASSET_COUNTER.fetch_add(1, Ordering::Relaxed);

    format!("{timestamp:x}-{sequence:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn png(directory: &Path, name: &str) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, [0x89, b'P', b'N', b'G']).expect("запись");
        path
    }

    #[test]
    fn stores_and_loads_image() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AssetStore::new(directory.path());
        let path = png(directory.path(), "схема.png");

        let asset = store
            .store_image(&path, MAX_ASSET_BYTES)
            .expect("сохранение");

        assert!(is_valid_asset_id(&asset.asset_id));
        assert_eq!(asset.file_name, "схема.png");
        assert_eq!(asset.alt, "схема");

        let data_url = store.load_data_url(&asset.asset_id).expect("загрузка");

        assert!(data_url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn resolves_asset_sources_in_document() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AssetStore::new(directory.path());
        let path = png(directory.path(), "схема.png");
        let asset = store
            .store_image(&path, MAX_ASSET_BYTES)
            .expect("сохранение");
        let mut document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode {
                    attrs: Some(json!({
                        "src": format!("asset:{}", asset.asset_id),
                        "alt": "схема"
                    })),
                    ..PmNode {
                        node_type: "image".to_owned(),
                        attrs: None,
                        content: Vec::new(),
                        marks: Vec::new(),
                        text: None,
                    }
                }],
            )],
        };

        store.resolve_document(&mut document).expect("разрешение");

        let attrs = document.content[0].content[0]
            .attrs
            .as_ref()
            .expect("атрибуты");
        assert!(attrs["src"].as_str().unwrap().starts_with("data:image/png"));
    }

    #[test]
    fn rejects_unsupported_images() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AssetStore::new(directory.path());
        let path = directory.path().join("рисунок.svg");
        std::fs::write(&path, "<svg />").expect("запись");

        let error = store
            .store_image(&path, MAX_ASSET_BYTES)
            .expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "unsupported_format");
    }

    #[test]
    fn rejects_missing_asset() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AssetStore::new(directory.path());

        let error = store
            .load_data_url("missing")
            .expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_document");
    }

    #[test]
    fn rejects_invalid_asset_id() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = AssetStore::new(directory.path());

        let error = store
            .load_data_url("../escape")
            .expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_argument");
    }
}
