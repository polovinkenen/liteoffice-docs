use wlite_docmodel::{PmDoc, PmMark, PmNode};
use wlite_formats_core::{node_text, FormatError, WarningSink, WrittenDocument};

use crate::style::{is_safe_css_value, is_safe_link, paragraph_style, text_style};

const MARK_ORDER: [&str; 8] = [
    "link",
    "textStyle",
    "highlight",
    "bold",
    "italic",
    "underline",
    "strike",
    "code",
];

pub fn write_document(document: &PmDoc) -> Result<WrittenDocument, FormatError> {
    let mut writer = Writer::new();

    writer.output.push_str(
        "<!DOCTYPE html>\n<html lang=\"ru\">\n<head>\n<meta charset=\"utf-8\">\n<title>WLite</title>\n</head>\n<body>\n",
    );

    for block in &document.content {
        writer.write_block(block);
    }

    writer.output.push_str("</body>\n</html>\n");

    Ok(WrittenDocument {
        text: writer.output,
        warnings: writer.warnings.into_warnings(),
    })
}

struct Writer {
    output: String,
    warnings: WarningSink,
}

impl Writer {
    fn new() -> Self {
        Self {
            output: String::new(),
            warnings: WarningSink::new(),
        }
    }

    fn write_block(&mut self, node: &PmNode) {
        match node.node_type.as_str() {
            "paragraph" => {
                self.output.push_str("<p");
                self.output
                    .push_str(&style_attribute(&paragraph_style(node.attrs.as_ref())));
                self.output.push('>');
                self.write_inline_children(node);
                self.output.push_str("</p>\n");
            }
            "heading" => {
                let level = node
                    .attrs
                    .as_ref()
                    .and_then(|attrs| attrs.get("level"))
                    .and_then(|level| level.as_u64())
                    .unwrap_or(1)
                    .clamp(1, 6);
                self.output.push_str(&format!("<h{level}"));
                self.output
                    .push_str(&style_attribute(&paragraph_style(node.attrs.as_ref())));
                self.output.push('>');
                self.write_inline_children(node);
                self.output.push_str(&format!("</h{level}>\n"));
            }
            "codeBlock" => {
                let language = node
                    .attrs
                    .as_ref()
                    .and_then(|attrs| attrs.get("language"))
                    .and_then(|language| language.as_str())
                    .filter(|language| {
                        !language.is_empty()
                            && language.chars().all(|character| {
                                character.is_ascii_alphanumeric()
                                    || matches!(character, '-' | '_' | '+')
                            })
                    });
                let literal = node_text(node);

                self.output.push_str("<pre><code");
                if let Some(language) = language {
                    self.output
                        .push_str(&format!(" class=\"language-{}\"", escape_attr(language)));
                }
                self.output.push('>');
                self.output.push_str(&escape_text(&literal));
                self.output.push_str("</code></pre>\n");
            }
            "blockquote" => {
                self.output.push_str("<blockquote>\n");
                self.write_blocks(&node.content);
                self.output.push_str("</blockquote>\n");
            }
            "bulletList" => self.write_list(node, false),
            "orderedList" => self.write_list(node, true),
            "taskList" => self.write_task_list(node),
            "table" => self.write_table(node),
            "pageBreak" => self
                .output
                .push_str("<div data-page-break=\"true\"></div>\n"),
            "horizontalRule" => self.output.push_str("<hr>\n"),
            "image" => {
                self.output.push_str("<p>");
                self.write_image(node);
                self.output.push_str("</p>\n");
            }
            _ => {
                self.warnings
                    .warn(format!("Узел «{}» не поддерживается HTML.", node.node_type));
                self.output.push_str("<p>");
                self.output.push_str(&escape_text(&node_text(node)));
                self.output.push_str("</p>\n");
            }
        }
    }

    fn write_blocks(&mut self, blocks: &[PmNode]) {
        for block in blocks {
            self.write_block(block);
        }
    }

    fn write_list(&mut self, node: &PmNode, ordered: bool) {
        let tag = if ordered { "ol" } else { "ul" };
        let start = node
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("start"))
            .and_then(|start| start.as_u64())
            .unwrap_or(1);

        self.output.push_str(&format!("<{tag}"));
        if ordered && start > 1 {
            self.output.push_str(&format!(" start=\"{start}\""));
        }
        self.output.push_str(">\n");

        for item in &node.content {
            self.output.push_str("<li>");
            self.write_blocks(&item.content);
            self.output.push_str("</li>\n");
        }

        self.output.push_str(&format!("</{tag}>\n"));
    }

    fn write_task_list(&mut self, node: &PmNode) {
        self.output.push_str("<ul data-type=\"taskList\">\n");

        for item in &node.content {
            let checked = item
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("checked"))
                .and_then(|checked| checked.as_bool())
                .unwrap_or(false);

            self.output.push_str(&format!(
                "<li data-type=\"taskItem\" data-checked=\"{checked}\">"
            ));
            self.write_blocks(&item.content);
            self.output.push_str("</li>\n");
        }

        self.output.push_str("</ul>\n");
    }

    fn write_table(&mut self, node: &PmNode) {
        self.output.push_str("<table>\n<tbody>\n");

        for row in &node.content {
            self.output.push_str("<tr>");

            for cell in &row.content {
                let tag = if cell.node_type == "tableHeader" {
                    "th"
                } else {
                    "td"
                };
                self.output.push_str(&format!("<{tag}"));
                self.write_span(cell, "colspan");
                self.write_span(cell, "rowspan");
                self.output.push('>');
                self.write_blocks(&cell.content);
                self.output.push_str(&format!("</{tag}>"));
            }

            self.output.push_str("</tr>\n");
        }

        self.output.push_str("</tbody>\n</table>\n");
    }

    fn write_span(&mut self, cell: &PmNode, name: &str) {
        let value = cell
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get(name))
            .and_then(|value| value.as_u64())
            .unwrap_or(1);

        if value > 1 {
            self.output.push_str(&format!(" {name}=\"{value}\""));
        }
    }

    fn write_inline_children(&mut self, node: &PmNode) {
        for child in &node.content {
            self.write_inline(child);
        }
    }

    fn write_inline(&mut self, node: &PmNode) {
        match node.node_type.as_str() {
            "text" => {
                let text = node.text.as_deref().unwrap_or_default();
                let tags = self.mark_tags(&node.marks);

                for (open, _) in &tags {
                    self.output.push_str(open);
                }

                self.output.push_str(&escape_text(text));

                for (_, close) in tags.iter().rev() {
                    self.output.push_str(close);
                }
            }
            "hardBreak" => self.output.push_str("<br>"),
            "image" => self.write_image(node),
            _ => {
                self.warnings.warn(format!(
                    "Встроенный узел «{}» не поддерживается HTML и был упрощён.",
                    node.node_type
                ));
                self.write_inline_children(node);
            }
        }
    }

    fn mark_tags(&mut self, marks: &[PmMark]) -> Vec<(String, String)> {
        let mut tags = Vec::new();

        for mark_type in MARK_ORDER {
            let Some(mark) = marks.iter().find(|mark| mark.mark_type == mark_type) else {
                continue;
            };

            match mark.mark_type.as_str() {
                "bold" => tags.push(("<strong>".to_owned(), "</strong>".to_owned())),
                "italic" => tags.push(("<em>".to_owned(), "</em>".to_owned())),
                "underline" => tags.push(("<u>".to_owned(), "</u>".to_owned())),
                "strike" => tags.push(("<s>".to_owned(), "</s>".to_owned())),
                "code" => tags.push(("<code>".to_owned(), "</code>".to_owned())),
                "link" => {
                    let href = mark
                        .attrs
                        .as_ref()
                        .and_then(|attrs| attrs.get("href"))
                        .and_then(|href| href.as_str())
                        .unwrap_or_default()
                        .trim();

                    if href.is_empty() {
                        continue;
                    }

                    if !is_safe_link(href) {
                        self.warnings
                            .warn("Ссылка с небезопасным адресом не сохраняется в HTML.");
                        continue;
                    }

                    tags.push((
                        format!("<a href=\"{}\">", escape_attr(href)),
                        "</a>".to_owned(),
                    ));
                }
                "textStyle" => {
                    let style = declaration_attribute(&text_style(mark.attrs.as_ref()));

                    if !style.is_empty() {
                        tags.push((format!("<span{style}>"), "</span>".to_owned()));
                    }
                }
                "highlight" => {
                    let color = mark
                        .attrs
                        .as_ref()
                        .and_then(|attrs| attrs.get("color"))
                        .and_then(|color| color.as_str())
                        .filter(|color| is_safe_css_value(color));

                    match color {
                        Some(color) => tags.push((
                            format!("<mark style=\"background-color: {}\">", escape_attr(color)),
                            "</mark>".to_owned(),
                        )),
                        None => tags.push(("<mark>".to_owned(), "</mark>".to_owned())),
                    }
                }
                other => self
                    .warnings
                    .warn(format!("Разметка «{other}» не поддерживается HTML.")),
            }
        }

        for mark in marks {
            if !MARK_ORDER.contains(&mark.mark_type.as_str()) {
                self.warnings.warn(format!(
                    "Разметка «{}» не поддерживается HTML и была упрощена.",
                    mark.mark_type
                ));
            }
        }

        tags
    }

    fn write_image(&mut self, node: &PmNode) {
        let source = node
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("src"))
            .and_then(|src| src.as_str())
            .unwrap_or_default()
            .trim();

        if source.is_empty() || !is_safe_link(source) {
            self.warnings
                .warn("Изображение с небезопасным адресом не сохраняется в HTML.");
            return;
        }

        let alt = string_attr(node, "alt").unwrap_or_default();
        let title = string_attr(node, "title");

        self.output
            .push_str(&format!("<img src=\"{}\"", escape_attr(source)));

        if !alt.is_empty() || node.attrs.is_some() {
            self.output
                .push_str(&format!(" alt=\"{}\"", escape_attr(alt)));
        }

        if let Some(title) = title.filter(|title| !title.is_empty()) {
            self.output
                .push_str(&format!(" title=\"{}\"", escape_attr(title)));
        }

        self.output.push('>');
    }
}

fn string_attr<'a>(node: &'a PmNode, name: &str) -> Option<&'a str> {
    node.attrs
        .as_ref()
        .and_then(|attrs| attrs.get(name))
        .and_then(|value| value.as_str())
}

fn style_attribute(style: &[(String, String)]) -> String {
    declaration_attribute(style)
}

fn declaration_attribute(style: &[(String, String)]) -> String {
    let declarations: Vec<String> = style
        .iter()
        .map(|(property, value)| format!("{property}: {value}"))
        .collect();

    if declarations.is_empty() {
        String::new()
    } else {
        format!(" style=\"{}\"", escape_attr(&declarations.join("; ")))
    }
}

fn escape_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            _ => output.push(character),
        }
    }

    output
}

fn escape_attr(value: &str) -> String {
    let mut output = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(character),
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write(document: &PmDoc) -> WrittenDocument {
        write_document(document).expect("запись HTML")
    }

    fn read(source: &str) -> PmDoc {
        crate::codec().read(source).expect("чтение HTML").document
    }

    #[test]
    fn writes_and_reads_formatted_document() {
        let document = read(
            r#"<h2 style="text-align: center">Заголовок</h2>
            <p><strong>жирный</strong> <em>курсив</em> <u>подчёркнутый</u>
               <s>зачёркнутый</s> <code>код</code>
               <a href="https://example.com">ссылка</a>
               <span style="color: #ff0000">красный</span></p>"#,
        );
        let written = write(&document);

        assert!(written.text.starts_with("<!DOCTYPE html>"));
        assert!(written.text.contains("<h2 style=\"text-align: center\">"));
        assert!(written.text.contains("<strong>жирный</strong>"));
        assert!(written.text.contains("<a href=\"https://example.com\">"));

        let reopened = crate::codec()
            .read(&written.text)
            .expect("повторное чтение");

        assert_eq!(reopened.document, document);
    }

    #[test]
    fn writes_every_supported_block() {
        let document = read(
            r#"<ul><li>Пункт</li></ul>
            <ol start="4"><li>Шаг</li></ol>
            <ul data-type="taskList">
              <li data-type="taskItem" data-checked="true">Готово</li>
            </ul>
            <blockquote><p>Цитата</p></blockquote>
            <pre><code class="language-rust">fn main() {}</code></pre>
            <hr>
            <div data-page-break="true"></div>
            <table><tr><th colspan="2">A</th></tr><tr><td>B</td><td>C</td></tr></table>"#,
        );
        let written = write(&document);

        assert!(written.text.contains("<ol start=\"4\">"));
        assert!(written.text.contains("data-type=\"taskList\""));
        assert!(written.text.contains("data-checked=\"true\""));
        assert!(written.text.contains("<pre><code class=\"language-rust\">"));
        assert!(written.text.contains("data-page-break=\"true\""));
        assert!(written.text.contains("<th colspan=\"2\">"));

        let reopened = crate::codec()
            .read(&written.text)
            .expect("повторное чтение");
        assert_eq!(reopened.document, document);
    }

    #[test]
    fn escapes_html_in_text() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode::text("a < b & c > d")],
            )],
        };
        let written = write(&document);

        assert!(written.text.contains("a &lt; b &amp; c &gt; d"));
        assert_eq!(read(&written.text), document);
    }

    #[test]
    fn drops_unsafe_links_and_images_on_export() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![
                    PmNode {
                        marks: vec![{
                            let mut mark = PmMark::new("link");
                            mark.attrs = Some(json!({ "href": "javascript:alert(1)" }));
                            mark
                        }],
                        ..PmNode::text("опасно")
                    },
                    PmNode {
                        attrs: Some(json!({ "src": "javascript:alert(2)", "alt": null })),
                        ..PmNode::with_content("image", Vec::new())
                    },
                ],
            )],
        };
        let written = write(&document);

        assert!(!written.text.contains("javascript:"));
        assert!(!written.text.contains("<img"));
        assert_eq!(written.warnings.len(), 2);
    }

    #[test]
    fn warns_about_unknown_nodes() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "unknownBlock",
                vec![PmNode::text("текст")],
            )],
        };
        let written = write(&document);

        assert!(written.text.contains("<p>текст</p>"));
        assert_eq!(written.warnings.len(), 1);
    }
}
