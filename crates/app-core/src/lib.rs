pub mod autosave;
pub mod files;
pub mod settings;

use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("неподдерживаемый формат: {0}")]
    UnsupportedFormat(String),
    #[error("некорректный аргумент: {0}")]
    InvalidArgument(String),
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("некорректный документ: {0}")]
    InvalidDocument(String),
    #[error("файл слишком большой: {path} ({size} байт, максимум {max_bytes})")]
    FileTooLarge {
        path: String,
        size: u64,
        max_bytes: u64,
    },
    #[error("внутренняя ошибка: {0}")]
    Internal(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::Io(_) => "io_error",
            Self::InvalidDocument(_) => "invalid_document",
            Self::FileTooLarge { .. } => "file_too_large",
            Self::Internal(_) => "internal_error",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_has_stable_code() {
        let error = AppError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "нет файла",
        ));

        assert_eq!(error.code(), "io_error");
        assert!(error.to_string().contains("нет файла"));
    }

    #[test]
    fn error_serializes_to_code_and_message() {
        let error = AppError::UnsupportedFormat("odt".to_owned());
        let json = serde_json::to_value(&error).expect("ошибка должна сериализоваться");

        assert_eq!(json["code"], "unsupported_format");
        assert_eq!(json["message"], "неподдерживаемый формат: odt");
    }

    #[test]
    fn invalid_argument_has_stable_code() {
        let error = AppError::InvalidArgument("пусто".to_owned());

        assert_eq!(error.code(), "invalid_argument");
    }
}
