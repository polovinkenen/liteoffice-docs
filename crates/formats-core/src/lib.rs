use std::path::Path;

use wordlite_docmodel::{PmDoc, PmNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    Txt,
    Markdown,
    Html,
    Rtf,
}

impl DocumentFormat {
    pub const ALL: [Self; 4] = [Self::Txt, Self::Markdown, Self::Html, Self::Rtf];

    pub fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();

        Self::ALL
            .into_iter()
            .find(|format| format.extensions().contains(&extension.as_str()))
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Rtf => "rtf",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Txt => "Текст",
            Self::Markdown => "Markdown",
            Self::Html => "HTML",
            Self::Rtf => "RTF",
        }
    }

    pub fn default_extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Rtf => "rtf",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Txt => &["txt"],
            Self::Markdown => &["md", "markdown"],
            Self::Html => &["html", "htm"],
            Self::Rtf => &["rtf"],
        }
    }

    pub fn supported_extensions() -> Vec<&'static str> {
        let mut extensions = Vec::new();

        for format in Self::ALL {
            for extension in format.extensions() {
                if !extensions.contains(extension) {
                    extensions.push(*extension);
                }
            }
        }

        extensions
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedDocument {
    pub document: PmDoc,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenDocument {
    pub text: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("не удалось прочитать документ: {0}")]
    Parse(String),
    #[error("не удалось записать документ: {0}")]
    Write(String),
}

pub trait DocumentCodec: Send + Sync {
    fn read(&self, source: &str) -> Result<ImportedDocument, FormatError>;
    fn write(&self, document: &PmDoc) -> Result<WrittenDocument, FormatError>;

    fn decode(&self, bytes: &[u8]) -> Result<String, FormatError> {
        let source = std::str::from_utf8(bytes).map_err(|error| {
            FormatError::Parse(format!("файл должен быть в кодировке UTF-8: {error}"))
        })?;

        Ok(source.strip_prefix('\u{feff}').unwrap_or(source).to_owned())
    }
}

pub fn node_text(node: &PmNode) -> String {
    if let Some(text) = &node.text {
        return text.clone();
    }

    match node.node_type.as_str() {
        "hardBreak" => "\n".to_owned(),
        "image" => image_alt(node),
        _ => node.content.iter().map(node_text).collect(),
    }
}

pub fn image_alt(node: &PmNode) -> String {
    node.attrs
        .as_ref()
        .and_then(|attrs| attrs.get("alt"))
        .and_then(|alt| alt.as_str())
        .unwrap_or_default()
        .to_owned()
}

pub fn has_formatting(document: &PmDoc) -> bool {
    document.content.iter().any(node_has_formatting)
}

fn node_has_formatting(node: &PmNode) -> bool {
    if !node.marks.is_empty() {
        return true;
    }

    if node.node_type == "paragraph" {
        if let Some(attrs) = node.attrs.as_ref().and_then(|attrs| attrs.as_object()) {
            let align = attrs
                .get("textAlign")
                .and_then(|value| value.as_str())
                .unwrap_or("left");
            let indent = attrs
                .get("indent")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let line_height = attrs
                .get("lineHeight")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            let space_before = attrs
                .get("spaceBefore")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);
            let space_after = attrs
                .get("spaceAfter")
                .and_then(|value| value.as_u64())
                .unwrap_or(0);

            if align != "left"
                || indent > 0
                || !line_height.is_empty()
                || space_before > 0
                || space_after > 0
            {
                return true;
            }
        }
    }

    match node.node_type.as_str() {
        "text" | "paragraph" | "hardBreak" => {}
        _ => return true,
    }

    node.content.iter().any(node_has_formatting)
}

pub struct WarningSink {
    warnings: Vec<String>,
}

impl WarningSink {
    pub fn new() -> Self {
        Self {
            warnings: Vec::new(),
        }
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();

        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    pub fn into_warnings(self) -> Vec<String> {
        self.warnings
    }
}

impl Default for WarningSink {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wordlite_docmodel::PmMark;

    #[test]
    fn detects_format_by_extension_case_insensitively() {
        assert_eq!(
            DocumentFormat::from_path(Path::new("Заметка.MD")),
            Some(DocumentFormat::Markdown)
        );
        assert_eq!(
            DocumentFormat::from_path(Path::new("письмо.txt")),
            Some(DocumentFormat::Txt)
        );
        assert_eq!(
            DocumentFormat::from_path(Path::new("Страница.HTM")),
            Some(DocumentFormat::Html)
        );
        assert_eq!(
            DocumentFormat::from_path(Path::new("Письмо.RTF")),
            Some(DocumentFormat::Rtf)
        );
        assert_eq!(DocumentFormat::from_path(Path::new("документ.odt")), None);
    }

    #[test]
    fn lists_supported_extensions() {
        assert_eq!(
            DocumentFormat::supported_extensions(),
            vec!["txt", "md", "markdown", "html", "htm", "rtf"]
        );
    }

    #[test]
    fn plain_document_has_no_formatting() {
        let document = PmDoc::empty();

        assert!(!has_formatting(&document));
    }

    #[test]
    fn paragraph_formatting_is_detected() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode {
                attrs: Some(json!({
                    "textAlign": "center",
                    "indent": 1,
                    "lineHeight": "1.5"
                })),
                ..PmNode::with_content("paragraph", vec![PmNode::text("текст")])
            }],
        };

        assert!(has_formatting(&document));
    }

    #[test]
    fn heading_and_marks_are_treated_as_formatting() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode {
                attrs: Some(json!({ "level": 1 })),
                ..PmNode::with_content("heading", vec![PmNode::text("Заголовок")])
            }],
        };

        assert!(has_formatting(&document));

        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode {
                    marks: vec![PmMark::new("bold")],
                    ..PmNode::text("жирный")
                }],
            )],
        };

        assert!(has_formatting(&document));
    }

    #[test]
    fn text_extraction_handles_breaks_and_images() {
        let paragraph = PmNode::with_content(
            "paragraph",
            vec![
                PmNode::text("до"),
                PmNode {
                    node_type: "hardBreak".to_owned(),
                    attrs: None,
                    content: Vec::new(),
                    marks: Vec::new(),
                    text: None,
                },
                PmNode::text("после"),
            ],
        );

        assert_eq!(node_text(&paragraph), "до\nпосле");
    }
}
