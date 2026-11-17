use wlite_docmodel::{PmDoc, PmNode};
use wlite_formats_core::{
    has_formatting, image_alt, node_text, DocumentCodec, FormatError, ImportedDocument,
    WarningSink, WrittenDocument,
};

pub struct TxtCodec;

static TXT_CODEC: TxtCodec = TxtCodec;

pub fn codec() -> &'static dyn DocumentCodec {
    &TXT_CODEC
}

impl DocumentCodec for TxtCodec {
    fn read(&self, source: &str) -> Result<ImportedDocument, FormatError> {
        let normalized = source.replace("\r\n", "\n").replace('\r', "\n");
        let mut paragraphs: Vec<Vec<&str>> = Vec::new();
        let mut current: Vec<&str> = Vec::new();

        for line in normalized.split('\n') {
            if line.trim().is_empty() {
                if !current.is_empty() {
                    paragraphs.push(std::mem::take(&mut current));
                }
            } else {
                current.push(line.trim_end());
            }
        }

        if !current.is_empty() {
            paragraphs.push(current);
        }

        let warnings = WarningSink::new();
        let content = if paragraphs.is_empty() {
            vec![PmNode::paragraph()]
        } else {
            paragraphs
                .into_iter()
                .map(|lines| paragraph_from_lines(&lines))
                .collect()
        };

        Ok(ImportedDocument {
            document: PmDoc {
                node_type: "doc".to_owned(),
                attrs: None,
                content,
            },
            warnings: warnings.into_warnings(),
        })
    }

    fn write(&self, document: &PmDoc) -> Result<WrittenDocument, FormatError> {
        let mut warnings = WarningSink::new();

        if has_formatting(document) {
            warnings
                .warn("Формат TXT сохраняет только текст: разметка и структура будут потеряны.");
        }

        let text = document
            .content
            .iter()
            .map(block_text)
            .collect::<Vec<_>>()
            .join("\n\n");

        Ok(WrittenDocument {
            text: format!("{text}\n"),
            warnings: warnings.into_warnings(),
        })
    }
}

fn paragraph_from_lines(lines: &[&str]) -> PmNode {
    let mut content = Vec::with_capacity(lines.len() * 2);

    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            content.push(PmNode {
                node_type: "hardBreak".to_owned(),
                attrs: None,
                content: Vec::new(),
                marks: Vec::new(),
                text: None,
            });
        }

        content.push(PmNode::text(*line));
    }

    PmNode::with_content("paragraph", content)
}

fn block_text(node: &PmNode) -> String {
    match node.node_type.as_str() {
        "paragraph" | "heading" => node_text(node),
        "codeBlock" => node_text(node),
        "blockquote" => node
            .content
            .iter()
            .map(block_text)
            .collect::<Vec<_>>()
            .join("\n\n"),
        "bulletList" => list_text(node, false, 0),
        "orderedList" => list_text(node, true, 0),
        "table" => table_text(node),
        "pageBreak" | "horizontalRule" => "---".to_owned(),
        "image" => image_alt(node),
        _ => node_text(node),
    }
}

fn list_text(node: &PmNode, ordered: bool, indent: usize) -> String {
    let start = if ordered {
        node.attrs
            .as_ref()
            .and_then(|attrs| attrs.get("start"))
            .and_then(|value| value.as_u64())
            .unwrap_or(1) as usize
    } else {
        1
    };

    let mut output = String::new();

    for (index, item) in node.content.iter().enumerate() {
        let padding = "  ".repeat(indent);
        let marker = if ordered {
            format!("{}. ", start + index)
        } else {
            "- ".to_owned()
        };
        let mut first_written = false;

        for child in &item.content {
            match child.node_type.as_str() {
                "paragraph" => {
                    let text = node_text(child);

                    if first_written {
                        for line in text.lines() {
                            output.push_str(&format!("{padding}  {line}\n"));
                        }
                    } else {
                        output.push_str(&format!("{padding}{marker}{text}\n"));
                        first_written = true;
                    }
                }
                "bulletList" => output.push_str(&list_text(child, false, indent + 1)),
                "orderedList" => output.push_str(&list_text(child, true, indent + 1)),
                _ => {
                    for line in block_text(child).lines() {
                        output.push_str(&format!("{padding}  {line}\n"));
                    }
                }
            }
        }

        if !first_written {
            output.push_str(&format!("{padding}{marker}\n"));
        }
    }

    output.trim_end().to_owned()
}

fn table_text(node: &PmNode) -> String {
    node.content
        .iter()
        .map(|row| {
            row.content
                .iter()
                .map(node_text)
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wlite_formats_core::DocumentFormat;

    fn read(source: &str) -> PmDoc {
        codec().read(source).expect("чтение TXT").document
    }

    fn write(document: &PmDoc) -> WrittenDocument {
        codec().write(document).expect("запись TXT")
    }

    #[test]
    fn empty_file_becomes_empty_paragraph() {
        assert_eq!(read(""), PmDoc::empty());
    }

    #[test]
    fn splits_paragraphs_by_blank_lines_and_keeps_line_breaks() {
        let document = read("первая строка\nвторая строка\n\nновый абзац\n");

        assert_eq!(document.content.len(), 2);
        assert_eq!(
            node_text(&document.content[0]),
            "первая строка\nвторая строка"
        );
        assert_eq!(node_text(&document.content[1]), "новый абзац");
        assert_eq!(document.content[0].content[1].node_type, "hardBreak");
    }

    #[test]
    fn normalizes_windows_and_old_mac_line_endings() {
        let document = read("один\r\nдва\rтри");

        assert_eq!(node_text(&document.content[0]), "один\nдва\nтри");
    }

    #[test]
    fn writes_paragraphs_separated_by_blank_lines() {
        let document = read("первый\n\nвторой");

        assert_eq!(write(&document).text, "первый\n\nвторой\n");
    }

    #[test]
    fn round_trip_preserves_plain_text_document() {
        let original = read("альфа\nбета\n\nгамма");
        let restored = read(&write(&original).text);

        assert_eq!(restored, original);
    }

    #[test]
    fn flattens_structured_content_and_reports_warning() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode {
                attrs: Some(json!({ "level": 1 })),
                ..PmNode::with_content("heading", vec![PmNode::text("Заголовок")])
            }],
        };

        let written = write(&document);

        assert_eq!(written.text, "Заголовок\n");
        assert_eq!(written.warnings.len(), 1);
    }

    #[test]
    fn reports_supported_format() {
        assert_eq!(
            DocumentFormat::from_path(std::path::Path::new("a.txt")),
            Some(DocumentFormat::Txt)
        );
    }
}
