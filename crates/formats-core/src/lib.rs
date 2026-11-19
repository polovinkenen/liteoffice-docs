use std::path::Path;

use docmodel::{PmDoc, PmNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
    Txt,
    Markdown,
    Html,
    Rtf,
    Docx,
}

impl DocumentFormat {
    pub const ALL: [Self; 5] = [Self::Txt, Self::Markdown, Self::Html, Self::Rtf, Self::Docx];

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
            Self::Docx => "docx",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Txt => "Текст",
            Self::Markdown => "Markdown",
            Self::Html => "HTML",
            Self::Rtf => "RTF",
            Self::Docx => "DOCX",
        }
    }

    pub fn default_extension(self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Rtf => "rtf",
            Self::Docx => "docx",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Txt => &["txt"],
            Self::Markdown => &["md", "markdown"],
            Self::Html => &["html", "htm"],
            Self::Rtf => &["rtf"],
            Self::Docx => &["docx"],
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenFile {
    pub bytes: Vec<u8>,
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

    fn read_bytes(&self, bytes: &[u8]) -> Result<ImportedDocument, FormatError> {
        let source = self.decode(bytes)?;
        self.read(&source)
    }

    fn write_bytes(&self, document: &PmDoc) -> Result<WrittenFile, FormatError> {
        let written = self.write(document)?;

        Ok(WrittenFile {
            bytes: written.text.into_bytes(),
            warnings: written.warnings,
        })
    }
}

const SAFE_SCHEMES: &[&str] = &["http", "https", "mailto", "asset", "tel"];

const SAFE_DATA_MIME_TYPES: &[&str] = &[
    "image/png",
    "image/jpeg",
    "image/jpg",
    "image/gif",
    "image/webp",
];

pub fn is_safe_link(url: &str) -> bool {
    let decoded = decode_entities(url);
    let trimmed = decoded.trim();

    if trimmed.is_empty() {
        return true;
    }

    if trimmed.chars().any(char::is_control) {
        return false;
    }

    let cleaned: String = trimmed
        .chars()
        .filter(|character| !is_format_character(*character) && !character.is_whitespace())
        .collect();

    let Some((scheme, rest)) = split_scheme(&cleaned) else {
        return true;
    };

    match scheme.as_str() {
        "data" => is_safe_data_url(rest),
        scheme if SAFE_SCHEMES.contains(&scheme) => true,
        _ => false,
    }
}

fn is_format_character(character: char) -> bool {
    matches!(
        u32::from(character),
        0x00AD
            | 0x0600..=0x0605
            | 0x061C
            | 0x06DD
            | 0x070F
            | 0x0890..=0x0891
            | 0x08E2
            | 0x180E
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x2066..=0x206F
            | 0xFEFF
            | 0xFFF9..=0xFFFB
            | 0x110BD
            | 0x110CD
            | 0x13430..=0x1343F
            | 0x1BCA0..=0x1BCA3
            | 0x1D173..=0x1D17A
            | 0xE0001
            | 0xE0020..=0xE007F
    )
}

const MAX_ENTITY_ITERATIONS: usize = 5;

fn decode_entities(input: &str) -> String {
    let mut current = input.to_owned();

    for _ in 0..MAX_ENTITY_ITERATIONS {
        let decoded = decode_entities_once(&current);

        if decoded == current {
            break;
        }

        current = decoded;
    }

    current
}

fn decode_entities_once(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut rest = input;

    loop {
        let Some(index) = rest.find('&') else {
            output.push_str(rest);
            break;
        };

        output.push_str(&rest[..index]);
        let after = &rest[index + 1..];

        if let Some((character, consumed)) = decode_entity_at(after) {
            output.push(character);
            rest = &after[consumed..];
        } else {
            output.push('&');
            rest = after;
        }
    }

    output
}

fn decode_entity_at(input: &str) -> Option<(char, usize)> {
    if let Some(rest) = input.strip_prefix('#') {
        let (digits, radix, prefix_length) = match rest.strip_prefix(['x', 'X']) {
            Some(hex) => (hex, 16, 2),
            None => (rest, 10, 1),
        };
        let digit_length = digits
            .bytes()
            .take_while(|byte| {
                if radix == 16 {
                    byte.is_ascii_hexdigit()
                } else {
                    byte.is_ascii_digit()
                }
            })
            .count();

        if digit_length == 0 {
            return None;
        }

        let semicolon_length = usize::from(digits.as_bytes().get(digit_length) == Some(&b';'));
        let code = u32::from_str_radix(&digits[..digit_length], radix).ok()?;

        return char::from_u32(code)
            .map(|character| (character, prefix_length + digit_length + semicolon_length));
    }

    let name_length = input.bytes().take_while(u8::is_ascii_alphanumeric).count();

    if name_length == 0 || input.as_bytes().get(name_length) != Some(&b';') {
        return None;
    }

    let character = named_entity(&input[..name_length])?;

    Some((character, name_length + 1))
}

fn named_entity(entity: &str) -> Option<char> {
    if entity.eq_ignore_ascii_case("colon") {
        return Some(':');
    }

    match entity.to_ascii_lowercase().as_str() {
        "tab" => Some('\t'),
        "newline" => Some('\n'),
        "nbsp" => Some('\u{a0}'),
        "amp" => Some('&'),
        "sol" => Some('/'),
        "bsol" => Some('\\'),
        "lpar" => Some('('),
        "rpar" => Some(')'),
        _ => None,
    }
}

fn split_scheme(url: &str) -> Option<(String, &str)> {
    let colon = url.find(':')?;
    let scheme = &url[..colon];

    if scheme.is_empty() {
        return None;
    }

    let mut characters = scheme.chars();
    let first = characters.next()?;

    if !first.is_ascii_alphabetic() {
        return None;
    }

    if !characters
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
    {
        return None;
    }

    Some((scheme.to_ascii_lowercase(), &url[colon + 1..]))
}

fn is_safe_data_url(rest: &str) -> bool {
    let end = rest.find([';', ',']).unwrap_or(rest.len());
    let mime = rest[..end].trim().to_ascii_lowercase();

    SAFE_DATA_MIME_TYPES.contains(&mime.as_str())
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
    use docmodel::PmMark;
    use serde_json::json;

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
        assert_eq!(
            DocumentFormat::from_path(Path::new("Договор.Docx")),
            Some(DocumentFormat::Docx)
        );
        assert_eq!(DocumentFormat::from_path(Path::new("документ.odt")), None);
    }

    #[test]
    fn lists_supported_extensions() {
        assert_eq!(
            DocumentFormat::supported_extensions(),
            vec!["txt", "md", "markdown", "html", "htm", "rtf", "docx"]
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

    #[test]
    fn accepts_relative_urls_and_known_schemes() {
        for url in [
            "image.png",
            "./image.png",
            "/assets/image.png",
            "#anchor",
            "https://example.com",
            "HTTP://example.com",
            "http://example.com",
            "mailto:user@example.com",
            "asset:image-1",
            "tel:+123456",
            "//example.com/image.png",
            "",
            "  ",
        ] {
            assert!(is_safe_link(url), "{url}");
        }
    }

    #[test]
    fn accepts_only_supported_data_image_types() {
        for url in [
            "data:image/png;base64,AAAA",
            "data:image/jpeg;base64,AAAA",
            "data:image/jpg;base64,AAAA",
            "data:image/gif;base64,AAAA",
            "data:image/webp;base64,AAAA",
            "DATA:IMAGE/PNG;base64,AAAA",
        ] {
            assert!(is_safe_link(url), "{url}");
        }

        for url in [
            "data:text/html;base64,AAAA",
            "data:image/svg+xml;base64,AAAA",
            "data:application/pdf;base64,AAAA",
            "data:,",
        ] {
            assert!(!is_safe_link(url), "{url}");
        }
    }

    #[test]
    fn rejects_dangerous_schemes_and_control_characters() {
        for url in [
            "javascript:alert(1)",
            "JavaScript:alert(1)",
            "vbscript:msgbox(1)",
            "file:///etc/passwd",
            "blob:https://example.com/id",
            "ftp://example.com/file",
            "https://example.com/\tpath",
        ] {
            assert!(!is_safe_link(url), "{url}");
        }
    }

    #[test]
    fn decodes_html_entities_before_checking_scheme() {
        for url in [
            "javascript&colon;alert(1)",
            "JavaScript&#58;alert(1)",
            "JavaScript&COLON;alert(1)",
            "&#x6a;avascript:alert(1)",
            "javascript&amp;colon;alert(1)",
            "javascript&#000000000000000000000000000000058;alert(1)",
            "data&colon;text/html;base64,AAAA",
            "data:image/svg+xml;base64,AAAA",
            "java&#x09;script:alert(1)",
            "java&tab;script:alert(1)",
            "&nbsp;javascript&colon;alert(1)",
        ] {
            assert!(!is_safe_link(url), "{url}");
        }
    }

    #[test]
    fn accepts_safe_links_with_html_entities() {
        for url in [
            "data&colon;image/png;base64,AAAA",
            "https&colon;//example.com",
            "https&sol;&sol;example.com",
        ] {
            assert!(is_safe_link(url), "{url}");
        }
    }

    #[test]
    fn accepts_tel_links() {
        assert!(is_safe_link("tel:+123456"));
        assert!(is_safe_link("TEL:+1-234-567"));
    }

    #[test]
    fn decodes_numeric_entities_without_semicolon() {
        for url in [
            "javascript&#58alert(1)",
            "javascript&#x3A/alert(1)",
            "javascript&#0000058alert(1)",
        ] {
            assert!(!is_safe_link(url), "{url}");
        }

        assert!(is_safe_link("https&#58//example.com"));
    }

    #[test]
    fn ignores_format_characters_and_unicode_spaces_in_scheme() {
        for url in [
            "java\u{200B}script:alert(1)",
            "\u{FEFF}javascript:alert(1)",
            "java\u{00A0}script:alert(1)",
            "java\u{200E}script:alert(1)",
        ] {
            assert!(!is_safe_link(url), "{url}");
        }

        for url in [
            "https://example.com/\u{200B}path",
            "mailto:user@example.com",
            "tel:+123456",
            "image.png",
        ] {
            assert!(is_safe_link(url), "{url}");
        }
    }
}
