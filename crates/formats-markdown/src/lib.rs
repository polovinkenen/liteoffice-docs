use comrak::nodes::{AstNode, ListType, NodeList, NodeValue};
use comrak::{parse_document, Arena, Options};
use serde_json::json;
use wordlite_docmodel::{PmDoc, PmMark, PmNode};
use wordlite_formats_core::{
    image_alt, node_text, DocumentCodec, FormatError, ImportedDocument, WarningSink,
    WrittenDocument,
};

pub struct MarkdownCodec;

static MARKDOWN_CODEC: MarkdownCodec = MarkdownCodec;

pub fn codec() -> &'static dyn DocumentCodec {
    &MARKDOWN_CODEC
}

impl DocumentCodec for MarkdownCodec {
    fn read(&self, source: &str) -> Result<ImportedDocument, FormatError> {
        let arena = Arena::new();
        let root = parse_document(&arena, source, &options());
        let mut warnings = WarningSink::new();
        let mut content = convert_blocks(root, &mut warnings);

        if content.is_empty() {
            content.push(PmNode::paragraph());
        }

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
        let mut context = WriteContext::new();
        let mut output = String::new();

        for node in &document.content {
            write_block(node, &mut output, 0, &mut context);
        }

        let trimmed = output.trim_end();

        Ok(WrittenDocument {
            text: if trimmed.is_empty() {
                String::new()
            } else {
                format!("{trimmed}\n")
            },
            warnings: context.warnings.into_warnings(),
        })
    }
}

fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options
}

fn convert_blocks<'a>(parent: &'a AstNode<'a>, warnings: &mut WarningSink) -> Vec<PmNode> {
    parent
        .children()
        .flat_map(|child| convert_block(child, warnings))
        .collect()
}

fn convert_block<'a>(node: &'a AstNode<'a>, warnings: &mut WarningSink) -> Vec<PmNode> {
    let value = node.data.borrow().value.clone();

    match value {
        NodeValue::Paragraph => vec![PmNode::with_content(
            "paragraph",
            convert_inline_children(node, &[], warnings),
        )],
        NodeValue::Heading(heading) => {
            let mut paragraph =
                PmNode::with_content("heading", convert_inline_children(node, &[], warnings));
            paragraph.attrs = Some(json!({ "level": heading.level }));
            vec![paragraph]
        }
        NodeValue::CodeBlock(code) => {
            let language = code
                .info
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned();
            let mut block = PmNode::with_content("codeBlock", vec![PmNode::text(code.literal)]);
            block.attrs = if language.is_empty() {
                None
            } else {
                Some(json!({ "language": language }))
            };
            vec![block]
        }
        NodeValue::BlockQuote => {
            let mut content = convert_blocks(node, warnings);

            if content.is_empty() {
                content.push(PmNode::paragraph());
            }

            vec![PmNode::with_content("blockquote", content)]
        }
        NodeValue::List(list) => vec![convert_list(node, &list, warnings)],
        NodeValue::Table(_) => vec![convert_table(node, warnings)],
        NodeValue::ThematicBreak => vec![block_node("horizontalRule")],
        NodeValue::HtmlBlock(block) => {
            if block.literal.trim() == "<!-- pagebreak -->" {
                vec![block_node("pageBreak")]
            } else {
                warnings.warn("HTML-блоки не поддерживаются и были пропущены.");
                Vec::new()
            }
        }
        NodeValue::FootnoteDefinition(_) => {
            warnings.warn("Сноски не поддерживаются и были пропущены.");
            Vec::new()
        }
        NodeValue::TaskItem(_) => {
            warnings.warn("Списки задач не поддерживаются Markdown-конвертером.");
            Vec::new()
        }
        NodeValue::Document => convert_blocks(node, warnings),
        _ => {
            let blocks = convert_blocks(node, warnings);

            if !blocks.is_empty() {
                return blocks;
            }

            let inline = convert_inline_children(node, &[], warnings);

            if inline.is_empty() {
                Vec::new()
            } else {
                vec![PmNode::with_content("paragraph", inline)]
            }
        }
    }
}

fn convert_list<'a>(node: &'a AstNode<'a>, list: &NodeList, warnings: &mut WarningSink) -> PmNode {
    let node_type = match list.list_type {
        ListType::Bullet => "bulletList",
        ListType::Ordered => "orderedList",
    };

    let items = node
        .children()
        .map(|item| convert_list_item(item, warnings))
        .collect();

    let mut pm_node = PmNode::with_content(node_type, items);

    if matches!(list.list_type, ListType::Ordered) && list.start != 1 {
        pm_node.attrs = Some(json!({ "start": list.start }));
    }

    pm_node
}

fn convert_list_item<'a>(node: &'a AstNode<'a>, warnings: &mut WarningSink) -> PmNode {
    let mut content = convert_blocks(node, warnings);

    if content.first().map(|child| child.node_type.as_str()) != Some("paragraph") {
        content.insert(0, PmNode::paragraph());
    }

    PmNode::with_content("listItem", content)
}

fn convert_table<'a>(node: &'a AstNode<'a>, warnings: &mut WarningSink) -> PmNode {
    let rows = node
        .children()
        .map(|row| {
            let is_header = matches!(&row.data.borrow().value, NodeValue::TableRow(true));
            let cells = row
                .children()
                .map(|cell| convert_table_cell(cell, is_header, warnings))
                .collect();

            PmNode::with_content("tableRow", cells)
        })
        .collect();

    PmNode::with_content("table", rows)
}

fn convert_table_cell<'a>(
    node: &'a AstNode<'a>,
    is_header: bool,
    warnings: &mut WarningSink,
) -> PmNode {
    let mut content = convert_blocks(node, warnings);

    if content.is_empty() {
        content.push(PmNode::paragraph());
    }

    let mut cell = PmNode::with_content(
        if is_header {
            "tableHeader"
        } else {
            "tableCell"
        },
        content,
    );
    cell.attrs = Some(json!({ "colspan": 1, "rowspan": 1, "colwidth": null }));
    cell
}

fn convert_inline_children<'a>(
    node: &'a AstNode<'a>,
    marks: &[PmMark],
    warnings: &mut WarningSink,
) -> Vec<PmNode> {
    let mut output = Vec::new();

    for child in node.children() {
        convert_inline(child, marks, &mut output, warnings);
    }

    output
}

fn convert_inline<'a>(
    node: &'a AstNode<'a>,
    marks: &[PmMark],
    output: &mut Vec<PmNode>,
    warnings: &mut WarningSink,
) {
    let value = node.data.borrow().value.clone();

    match value {
        NodeValue::Text(text) => push_text(output, &text, marks),
        NodeValue::SoftBreak => push_text(output, " ", marks),
        NodeValue::LineBreak => output.push(block_node("hardBreak")),
        NodeValue::Code(code) => {
            let mut code_marks = marks.to_vec();
            code_marks.push(PmMark::new("code"));
            push_text(output, &code.literal, &code_marks);
        }
        NodeValue::Emph => {
            let mut nested = marks.to_vec();
            nested.push(PmMark::new("italic"));
            convert_inline_children_into(node, &nested, output, warnings);
        }
        NodeValue::Strong => {
            let mut nested = marks.to_vec();
            nested.push(PmMark::new("bold"));
            convert_inline_children_into(node, &nested, output, warnings);
        }
        NodeValue::Strikethrough => {
            let mut nested = marks.to_vec();
            nested.push(PmMark::new("strike"));
            convert_inline_children_into(node, &nested, output, warnings);
        }
        NodeValue::Link(link) => {
            let mut nested = marks.to_vec();
            let mut mark = PmMark::new("link");
            mark.attrs = Some(json!({
                "href": link.url,
                "target": null,
                "rel": null,
                "class": null
            }));
            nested.push(mark);
            convert_inline_children_into(node, &nested, output, warnings);
        }
        NodeValue::Image(image) => {
            let alt = plain_text(node);
            let mut image_node = block_node("image");
            image_node.attrs = Some(json!({
                "src": image.url,
                "alt": if alt.is_empty() { serde_json::Value::Null } else { json!(alt) },
                "title": if image.title.is_empty() { serde_json::Value::Null } else { json!(image.title) }
            }));
            output.push(image_node);
        }
        NodeValue::HtmlInline(_) => {
            warnings.warn("Встроенный HTML не поддерживается и был удалён.");
        }
        NodeValue::FootnoteReference(_) => {
            warnings.warn("Ссылки на сноски не поддерживаются и были удалены.");
        }
        _ => convert_inline_children_into(node, marks, output, warnings),
    }
}

fn convert_inline_children_into<'a>(
    node: &'a AstNode<'a>,
    marks: &[PmMark],
    output: &mut Vec<PmNode>,
    warnings: &mut WarningSink,
) {
    for child in node.children() {
        convert_inline(child, marks, output, warnings);
    }
}

fn push_text(output: &mut Vec<PmNode>, text: &str, marks: &[PmMark]) {
    if text.is_empty() {
        return;
    }

    if let Some(last) = output.last_mut() {
        if last.node_type == "text" && last.marks == marks {
            if let Some(existing) = last.text.as_mut() {
                existing.push_str(text);
                return;
            }
        }
    }

    output.push(PmNode {
        node_type: "text".to_owned(),
        attrs: None,
        content: Vec::new(),
        marks: marks.to_vec(),
        text: Some(text.to_owned()),
    });
}

fn plain_text<'a>(node: &'a AstNode<'a>) -> String {
    let mut text = String::new();

    for child in node.children() {
        let value = child.data.borrow().value.clone();

        match value {
            NodeValue::Text(value) => text.push_str(&value),
            NodeValue::SoftBreak | NodeValue::LineBreak => text.push(' '),
            _ => text.push_str(&plain_text(child)),
        }
    }

    text
}

fn block_node(node_type: &str) -> PmNode {
    PmNode {
        node_type: node_type.to_owned(),
        attrs: None,
        content: Vec::new(),
        marks: Vec::new(),
        text: None,
    }
}

struct WriteContext {
    warnings: WarningSink,
}

impl WriteContext {
    fn new() -> Self {
        Self {
            warnings: WarningSink::new(),
        }
    }
}

fn write_block(node: &PmNode, output: &mut String, indent: usize, context: &mut WriteContext) {
    match node.node_type.as_str() {
        "paragraph" => {
            write_inline_children(node, output, context);
            output.push_str("\n\n");
        }
        "heading" => {
            let level = node
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("level"))
                .and_then(|level| level.as_u64())
                .unwrap_or(1)
                .clamp(1, 6) as usize;
            output.push_str(&"#".repeat(level));
            output.push(' ');
            write_inline_children(node, output, context);
            output.push_str("\n\n");
        }
        "codeBlock" => {
            let literal = node_text(node);
            let language = node
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("language"))
                .and_then(|language| language.as_str())
                .unwrap_or_default();
            let fence_length = (longest_backtick_run(&literal) + 1).max(3);
            let fence = "`".repeat(fence_length);
            let body = literal.trim_end_matches('\n');

            output.push_str(&format!("{fence}{language}\n{body}\n{fence}\n\n"));
        }
        "blockquote" => {
            let mut inner = String::new();

            for child in &node.content {
                write_block(child, &mut inner, indent, context);
            }

            for line in inner.trim_end().lines() {
                if line.is_empty() {
                    output.push_str(">\n");
                } else {
                    output.push_str(&format!("> {line}\n"));
                }
            }

            output.push('\n');
        }
        "bulletList" => write_list(node, output, false, indent, context),
        "orderedList" => write_list(node, output, true, indent, context),
        "table" => write_table(node, output, context),
        "pageBreak" => output.push_str("<!-- pagebreak -->\n\n"),
        "horizontalRule" => output.push_str("---\n\n"),
        "image" => {
            output.push_str(&image_markdown(node));
            output.push_str("\n\n");
        }
        other => {
            if other != "text" && other != "hardBreak" {
                context
                    .warnings
                    .warn(format!("Узел «{other}» не поддерживается Markdown."));
            }
            output.push_str(&node_text(node));
            output.push_str("\n\n");
        }
    }
}

fn write_inline_children(node: &PmNode, output: &mut String, context: &mut WriteContext) {
    for child in &node.content {
        write_inline(child, output, context);
    }
}

fn write_inline(node: &PmNode, output: &mut String, context: &mut WriteContext) {
    match node.node_type.as_str() {
        "text" => {
            let text = node.text.as_deref().unwrap_or_default();
            let mut rendered = if node.marks.iter().any(|mark| mark.mark_type == "code") {
                code_span(text)
            } else {
                escape_text(text)
            };

            for mark in &node.marks {
                match mark.mark_type.as_str() {
                    "code" => {}
                    "bold" => rendered = format!("**{rendered}**"),
                    "italic" => rendered = format!("*{rendered}*"),
                    "strike" => rendered = format!("~~{rendered}~~"),
                    "link" => {
                        let href = mark
                            .attrs
                            .as_ref()
                            .and_then(|attrs| attrs.get("href"))
                            .and_then(|href| href.as_str())
                            .unwrap_or_default();
                        rendered = format!("[{rendered}]({href})");
                    }
                    other => context.warnings.warn(format!(
                        "Разметка «{other}» не поддерживается Markdown и была упрощена."
                    )),
                }
            }

            output.push_str(&rendered);
        }
        "hardBreak" => output.push_str("  \n"),
        "image" => output.push_str(&image_markdown(node)),
        _ => write_inline_children(node, output, context),
    }
}

fn write_list(
    node: &PmNode,
    output: &mut String,
    ordered: bool,
    indent: usize,
    context: &mut WriteContext,
) {
    let start = if ordered {
        node.attrs
            .as_ref()
            .and_then(|attrs| attrs.get("start"))
            .and_then(|start| start.as_u64())
            .unwrap_or(1) as usize
    } else {
        1
    };
    let padding = "  ".repeat(indent);

    for (index, item) in node.content.iter().enumerate() {
        let marker = if ordered {
            format!("{}. ", start + index)
        } else {
            "- ".to_owned()
        };
        let mut first_written = false;

        for child in &item.content {
            match child.node_type.as_str() {
                "paragraph" => {
                    let mut text = String::new();
                    write_inline_children(child, &mut text, context);

                    if first_written {
                        output.push_str(&format!("{padding}  {text}\n"));
                    } else {
                        output.push_str(&format!("{padding}{marker}{text}\n"));
                        first_written = true;
                    }
                }
                "bulletList" => write_list(child, output, false, indent + 1, context),
                "orderedList" => write_list(child, output, true, indent + 1, context),
                _ => {
                    let mut nested = String::new();
                    write_block(child, &mut nested, indent + 1, context);

                    for line in nested.trim_end().lines() {
                        output.push_str(&format!("{padding}  {line}\n"));
                    }
                }
            }
        }

        if !first_written {
            output.push_str(&format!("{padding}{marker}\n"));
        }
    }

    output.push('\n');
}

fn write_table(node: &PmNode, output: &mut String, context: &mut WriteContext) {
    let rows: Vec<Vec<String>> = node
        .content
        .iter()
        .map(|row| {
            row.content
                .iter()
                .map(|cell| cell_markdown(cell, context))
                .collect()
        })
        .collect();

    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);

    if columns == 0 {
        return;
    }

    let empty = Vec::new();
    let header = rows.first().unwrap_or(&empty);
    write_table_row(header, columns, output);
    output.push_str(&format!("| {} |\n", vec!["---"; columns].join(" | ")));

    for row in rows.iter().skip(1) {
        write_table_row(row, columns, output);
    }

    output.push('\n');
}

fn write_table_row(cells: &[String], columns: usize, output: &mut String) {
    let mut padded = cells.to_vec();
    padded.resize(columns, String::new());
    output.push_str(&format!("| {} |\n", padded.join(" | ")));
}

fn cell_markdown(cell: &PmNode, context: &mut WriteContext) -> String {
    let mut parts = Vec::new();

    for child in &cell.content {
        if child.node_type == "paragraph" {
            let mut text = String::new();
            write_inline_children(child, &mut text, context);
            parts.push(text);
        } else {
            parts.push(node_text(child));
        }
    }

    parts.join(" ").replace('\n', " ").replace('|', "\\|")
}

fn image_markdown(node: &PmNode) -> String {
    let source = node
        .attrs
        .as_ref()
        .and_then(|attrs| attrs.get("src"))
        .and_then(|src| src.as_str())
        .unwrap_or_default();

    format!("![{}]({source})", image_alt(node))
}

fn escape_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());

    for character in text.chars() {
        if matches!(
            character,
            '\\' | '`'
                | '*'
                | '_'
                | '{'
                | '}'
                | '['
                | ']'
                | '<'
                | '>'
                | '#'
                | '+'
                | '-'
                | '!'
                | '|'
                | '~'
        ) {
            output.push('\\');
        }

        output.push(character);
    }

    output
}

fn code_span(text: &str) -> String {
    let fence = "`".repeat(longest_backtick_run(text) + 1);
    let needs_padding = text.starts_with('`')
        || text.ends_with('`')
        || text.starts_with(' ')
        || text.ends_with(' ');

    if needs_padding {
        format!("{fence} {text} {fence}")
    } else {
        format!("{fence}{text}{fence}")
    }
}

fn longest_backtick_run(text: &str) -> usize {
    text.split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordlite_formats_core::DocumentFormat;

    fn read(source: &str) -> ImportedDocument {
        codec().read(source).expect("чтение Markdown")
    }

    fn write(document: &PmDoc) -> WrittenDocument {
        codec().write(document).expect("запись Markdown")
    }

    fn types(document: &PmDoc) -> Vec<&str> {
        document
            .content
            .iter()
            .map(|node| node.node_type.as_str())
            .collect()
    }

    #[test]
    fn parses_headings_paragraphs_and_lists() {
        let imported = read("# Заголовок\n\nАбзац\n\n- один\n- два\n");

        assert_eq!(
            types(&imported.document),
            vec!["heading", "paragraph", "bulletList"]
        );
        assert_eq!(
            imported.document.content[0].attrs,
            Some(json!({ "level": 1 }))
        );
        assert_eq!(imported.document.content[2].content.len(), 2);
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn parses_inline_marks_and_links() {
        let imported = read(
            "Текст **жирный** и *курсив*, ~~зачёркнутый~~, `код`, [ссылка](https://example.com).\n",
        );
        let paragraph = &imported.document.content[0];

        let marks: Vec<&str> = paragraph
            .content
            .iter()
            .flat_map(|node| node.marks.iter())
            .map(|mark| mark.mark_type.as_str())
            .collect();

        assert!(marks.contains(&"bold"));
        assert!(marks.contains(&"italic"));
        assert!(marks.contains(&"strike"));
        assert!(marks.contains(&"code"));
        assert!(marks.contains(&"link"));
    }

    #[test]
    fn parses_nested_lists() {
        let imported = read("- родитель\n  - дочерний\n");

        let list = &imported.document.content[0];
        let item = &list.content[0];

        assert_eq!(item.node_type, "listItem");
        assert_eq!(item.content[1].node_type, "bulletList");
    }

    #[test]
    fn parses_gfm_tables_with_header() {
        let imported = read("| Имя | Значение |\n| --- | --- |\n| a | 1 |\n");
        let table = &imported.document.content[0];

        assert_eq!(table.node_type, "table");
        assert_eq!(table.content.len(), 2);
        assert_eq!(table.content[0].content[0].node_type, "tableHeader");
        assert_eq!(table.content[1].content[0].node_type, "tableCell");
    }

    #[test]
    fn parses_images_and_code_blocks() {
        let imported = read("![alt](image.png)\n\n```rust\nfn main() {}\n```\n");

        assert_eq!(imported.document.content[0].content[0].node_type, "image");
        assert_eq!(imported.document.content[1].node_type, "codeBlock");
        assert_eq!(
            imported.document.content[1].attrs,
            Some(json!({ "language": "rust" }))
        );
    }

    #[test]
    fn warns_about_inline_html() {
        let imported = read("до <b>жир</b> после\n");

        assert_eq!(imported.warnings.len(), 1);
        assert!(node_text(&imported.document.content[0]).contains("жир"));
    }

    #[test]
    fn imports_and_exports_page_break_marker() {
        let imported = read("до\n\n<!-- pagebreak -->\n\nпосле\n");

        assert_eq!(
            types(&imported.document),
            vec!["paragraph", "pageBreak", "paragraph"]
        );
        assert!(imported.warnings.is_empty());

        let written = write(&imported.document);
        assert!(written.text.contains("<!-- pagebreak -->"));

        let restored = read(&written.text);
        assert_eq!(restored.document, imported.document);
    }

    #[test]
    fn warns_about_other_html_blocks() {
        let imported = read("<div>блок</div>\n");

        assert_eq!(imported.warnings.len(), 1);
        assert!(imported
            .document
            .content
            .iter()
            .all(|node| node.node_type != "pageBreak"));
    }

    #[test]
    fn writes_markdown_and_round_trips() {
        let source = "# Заголовок\n\nАбзац с **жирным** и *курсивом*.\n\n- один\n- два\n";
        let original = read(source);
        let written = write(&original.document);
        let restored = read(&written.text);

        assert_eq!(restored.document, original.document);
    }

    #[test]
    fn escaping_preserves_plain_text() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode::text("1 * 2 = 2, [не ссылка] и # не заголовок")],
            )],
        };

        let written = write(&document);
        let restored = read(&written.text);

        assert_eq!(
            node_text(&restored.document.content[0]),
            "1 * 2 = 2, [не ссылка] и # не заголовок"
        );
    }

    #[test]
    fn table_round_trip_preserves_cells() {
        let source = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
        let original = read(source);
        let restored = read(&write(&original.document).text);

        assert_eq!(restored.document, original.document);
    }

    #[test]
    fn reports_supported_format() {
        assert_eq!(
            DocumentFormat::from_path(std::path::Path::new("README.markdown")),
            Some(DocumentFormat::Markdown)
        );
    }
}
