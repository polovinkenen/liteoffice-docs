#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PdfError {
    #[error("некорректные параметры PDF: {0}")]
    InvalidOptions(String),
    #[error("не удалось сформировать PDF: {0}")]
    Render(String),
}
