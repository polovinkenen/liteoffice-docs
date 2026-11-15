use serde::ser::{SerializeStruct, Serializer};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("неподдерживаемый формат: {0}")]
    UnsupportedFormat(String),
    #[error("ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    #[error("некорректный документ: {0}")]
    InvalidDocument(String),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedFormat(_) => "unsupported_format",
            Self::Io(_) => "io_error",
            Self::InvalidDocument(_) => "invalid_document",
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
}
