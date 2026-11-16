use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::files::{read_text, write_text_atomic};
use crate::{AppError, AppResult};

pub const SETTINGS_FILE: &str = "settings.json";
pub const MAX_SETTINGS_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

fn default_autosave_enabled() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub theme: Theme,
    #[serde(default = "default_autosave_enabled")]
    pub autosave_enabled: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            autosave_enabled: true,
        }
    }
}

pub struct SettingsStore {
    root: PathBuf,
}

impl SettingsStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn path(&self) -> PathBuf {
        self.root.join(SETTINGS_FILE)
    }

    pub fn load(&self) -> AppResult<Settings> {
        let path = self.path();

        match std::fs::metadata(&path) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Settings::default());
            }
            Err(error) => return Err(AppError::Io(error)),
        }

        let text = read_text(&path, MAX_SETTINGS_BYTES)?;

        serde_json::from_str(&text).map_err(|error| {
            AppError::InvalidDocument(format!("не удалось прочитать настройки: {error}"))
        })
    }

    pub fn save(&self, settings: &Settings) -> AppResult<()> {
        let path = self.path();
        let payload = serde_json::to_string_pretty(settings)
            .map_err(|error| AppError::Internal(error.to_string()))?;

        if payload.len() as u64 > MAX_SETTINGS_BYTES {
            return Err(AppError::FileTooLarge {
                path: path.display().to_string(),
                size: payload.len() as u64,
                max_bytes: MAX_SETTINGS_BYTES,
            });
        }

        std::fs::create_dir_all(&self.root)?;
        write_text_atomic(&path, &payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_defaults() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = SettingsStore::new(directory.path());

        assert_eq!(store.load().expect("настройки"), Settings::default());
    }

    #[test]
    fn saves_and_loads_settings() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = SettingsStore::new(directory.path());
        let settings = Settings {
            theme: Theme::Dark,
            autosave_enabled: false,
        };

        store.save(&settings).expect("сохранение");

        assert_eq!(store.load().expect("загрузка"), settings);
        assert_eq!(
            std::fs::read_to_string(store.path()).expect("чтение"),
            "{\n  \"theme\": \"dark\",\n  \"autosaveEnabled\": false\n}"
        );
    }

    #[test]
    fn fills_missing_fields_with_defaults() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = SettingsStore::new(directory.path());
        std::fs::write(store.path(), r#"{"theme":"light"}"#).expect("запись");

        let settings = store.load().expect("загрузка");

        assert_eq!(settings.theme, Theme::Light);
        assert!(settings.autosave_enabled);
    }

    #[test]
    fn rejects_unknown_theme() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = SettingsStore::new(directory.path());
        std::fs::write(store.path(), r#"{"theme":"blue","autosaveEnabled":true}"#).expect("запись");

        let error = store.load().expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_document");
    }

    #[test]
    fn rejects_malformed_json() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = SettingsStore::new(directory.path());
        std::fs::write(store.path(), "не json").expect("запись");

        let error = store.load().expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_document");
    }

    #[test]
    fn overwrites_previous_settings_atomically() {
        let directory = tempfile::tempdir().expect("временный каталог");
        let store = SettingsStore::new(directory.path());

        store
            .save(&Settings {
                theme: Theme::Light,
                autosave_enabled: true,
            })
            .expect("первое сохранение");
        store
            .save(&Settings {
                theme: Theme::System,
                autosave_enabled: false,
            })
            .expect("второе сохранение");

        let settings = store.load().expect("загрузка");

        assert_eq!(settings.theme, Theme::System);
        assert!(!settings.autosave_enabled);
    }
}
