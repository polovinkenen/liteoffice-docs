use std::collections::HashSet;

use base64::Engine;
use serde_json::Value;
use wlite_docmodel::{PmDoc, PmMark, PmNode};

use crate::escape::{escape_markup, page_text_markup, typst_string};
use crate::fonts::{resolve_family, FamilyResolution, SERIF_FAMILY};
use crate::{PdfError, PdfOptions};

#[derive(Debug)]
pub struct Conversion {
    pub source: String,
    pub files: Vec<(String, Vec<u8>)>,
    pub warnings: Vec<String>,
}

pub fn convert(
    document: &PmDoc,
    options: &PdfOptions,
    title: Option<&str>,
) -> Result<Conversion, PdfError> {
    options.validate()?;

    let mut converter = Converter {
        files: Vec::new(),
        warnings: Vec::new(),
    };

    let mut body = converter.blocks(&document.content);

    if body.trim().is_empty() {
        body.push_str("#par[]");
    }

    let mut source = converter.preamble(options, title);
    source.push('\n');
    source.push_str(&body);
    source.push('\n');

    Ok(Conversion {
        source,
        files: converter.files,
        warnings: converter.warnings,
    })
}

struct Converter {
    files: Vec<(String, Vec<u8>)>,
    warnings: Vec<String>,
}

impl Converter {
    fn preamble(&mut self, options: &PdfOptions, title: Option<&str>) -> String {
        let mut out = String::new();

        if let Some(title) = title.map(str::trim).filter(|value| !value.is_empty()) {
            let title = title.replace(['\n', '\r'], " ");
            out.push_str(&format!("#set document(title: {})\n", typst_string(&title)));
        }

        let (top, right, bottom, left) = (
            options.margins.top,
            options.margins.right,
            options.margins.bottom,
            options.margins.left,
        );
        out.push_str("#set page(\n");
        out.push_str(&format!("  paper: \"{}\",\n", options.paper.typst_name()));
        if options.orientation.is_landscape() {
            out.push_str("  flipped: true,\n");
        }
        out.push_str(&format!(
            "  margin: (top: {}mm, right: {}mm, bottom: {}mm, left: {}mm),\n",
            trim_number(top),
            trim_number(right),
            trim_number(bottom),
            trim_number(left)
        ));

        if let Some(header) = self.page_slot(options.header.as_deref(), false) {
            out.push_str(&format!("  header: {header},\n"));
        }

        if let Some(footer) = self.page_slot(options.footer.as_deref(), options.page_numbers) {
            out.push_str(&format!("  footer: {footer},\n"));
        }

        out.push_str(")\n");
        out.push_str(&format!(
            "#set text(font: (\"{SERIF_FAMILY}\",), size: 12pt, lang: \"ru\")\n"
        ));
        out.push_str("#set par(justify: false, leading: 0.65em, spacing: 1.15em)\n");
        out.push_str("#set table(stroke: 0.5pt, inset: (x: 6pt, y: 4pt))\n");

        out
    }

    fn page_slot(&mut self, text: Option<&str>, page_numbers: bool) -> Option<String> {
        let text = text.map(str::trim).filter(|value| !value.is_empty());
        let has_counter = text
            .map(|value| value.contains("{page}") || value.contains("{pages}"))
            .unwrap_or(false);
        let mut parts = Vec::new();

        if let Some(text) = text {
            parts.push(page_text_markup(text));
        }

        if page_numbers && !has_counter {
            parts.push("#context counter(page).display()".to_owned());
        }

        if parts.is_empty() {
            return None;
        }

        let combined = parts.join("#linebreak()");

        Some(format!(
            "align(center)[#text(size: 9pt, fill: rgb(\"#5a5a5a\"))[{combined}]]"
        ))
    }

    fn warn(&mut self, message: impl Into<String>) {
        let message = message.into();

        if !self.warnings.contains(&message) {
            self.warnings.push(message);
        }
    }

    fn blocks(&mut self, nodes: &[PmNode]) -> String {
        let mut parts = Vec::new();

        for node in nodes {
            let rendered = self.block(node);

            if !rendered.is_empty() {
                parts.push(rendered);
            }
        }

        parts.join("\n\n")
    }

    fn block(&mut self, node: &PmNode) -> String {
        match node.node_type.as_str() {
            "doc" => self.blocks(&node.content),
            "paragraph" => self.paragraph(node),
            "heading" => self.heading(node),
            "bulletList" => self.list(node, false, false),
            "orderedList" => self.list(node, true, false),
            "taskList" => self.list(node, false, true),
            "blockquote" => self.blockquote(node),
            "codeBlock" => self.code_block(node),
            "table" => self.table(node),
            "horizontalRule" => "#line(length: 100%, stroke: 0.6pt)".to_owned(),
            "pageBreak" => "#pagebreak()".to_owned(),
            "image" => self.image(node),
            "text" => format!("#par[{}]", self.inline(std::slice::from_ref(node))),
            "hardBreak" => "#par[#linebreak()]".to_owned(),
            "listItem" | "taskItem" => self.list(node, false, node.node_type == "taskItem"),
            other => {
                self.warn(format!(
                    "Элемент «{other}» не поддерживается в PDF и пропущен"
                ));

                if node.content.is_empty() {
                    let text = node.text.as_deref().unwrap_or_default();
                    format!("#par[{}]", escape_markup(text))
                } else {
                    self.blocks(&node.content)
                }
            }
        }
    }

    fn inline(&mut self, nodes: &[PmNode]) -> String {
        let mut out = String::new();

        for node in nodes {
            match node.node_type.as_str() {
                "text" => out.push_str(&self.text_run(node)),
                "hardBreak" => out.push_str("#linebreak()"),
                "image" => out.push_str(&self.image(node)),
                other => {
                    self.warn(format!(
                        "Элемент «{other}» внутри абзаца не поддерживается в PDF"
                    ));
                    out.push_str(&self.inline(&node.content));
                }
            }
        }

        out
    }

    fn text_run(&mut self, node: &PmNode) -> String {
        let text = node.text.as_deref().unwrap_or_default();

        if text.is_empty() {
            return String::new();
        }

        if node.marks.iter().any(|mark| mark.mark_type == "code") {
            return format!("#raw({})", typst_string(text));
        }

        let mut out = escape_markup(text);

        for mark in &node.marks {
            out = self.mark(mark, out);
        }

        out
    }

    fn mark(&mut self, mark: &PmMark, content: String) -> String {
        match mark.mark_type.as_str() {
            "bold" => format!("*{content}*"),
            "italic" => format!("_{content}_"),
            "underline" => format!("#underline[{content}]"),
            "strike" => format!("#strike[{content}]"),
            "highlight" => {
                let color = mark
                    .attrs
                    .as_ref()
                    .and_then(|attrs| attrs.get("color"))
                    .and_then(Value::as_str)
                    .and_then(typst_color)
                    .unwrap_or_else(|| "rgb(\"#fff2a8\")".to_owned());

                format!("#highlight(fill: {color})[{content}]")
            }
            "link" => {
                let Some(href) = mark
                    .attrs
                    .as_ref()
                    .and_then(|attrs| attrs.get("href"))
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                else {
                    return content;
                };

                if !safe_link(href) {
                    self.warn(format!("Ссылка «{href}» не сохранена в PDF"));
                    return content;
                }

                format!("#link({})[{content}]", typst_string(href))
            }
            "textStyle" => self.text_style(mark, content),
            other => {
                self.warn(format!("Форматирование «{other}» не поддерживается в PDF"));
                content
            }
        }
    }

    fn text_style(&mut self, mark: &PmMark, content: String) -> String {
        let Some(attrs) = mark.attrs.as_ref().and_then(Value::as_object) else {
            return content;
        };
        let mut out = content;

        if let Some(family) = attrs.get("fontFamily").and_then(Value::as_str) {
            if !family.trim().is_empty() {
                let resolved = match resolve_family(family) {
                    FamilyResolution::Known(family) => family,
                    FamilyResolution::Fallback(family) => {
                        let name = family
                            .split(',')
                            .next()
                            .unwrap_or(family)
                            .trim()
                            .trim_matches(['"', '\'']);
                        self.warn(format!("Шрифт «{name}» заменён на «{family}» в PDF"));
                        family
                    }
                };

                out = format!("#text(font: \"{resolved}\")[{out}]");
            }
        }

        if let Some(size) = attrs.get("fontSize").and_then(Value::as_str) {
            match typst_size(size) {
                Some(size) => out = format!("#text(size: {size})[{out}]"),
                None => self.warn(format!("Размер шрифта «{size}» не сохранён в PDF")),
            }
        }

        if let Some(color) = attrs.get("color").and_then(Value::as_str) {
            match typst_color(color) {
                Some(color) => out = format!("#text(fill: {color})[{out}]"),
                None => self.warn(format!("Цвет текста «{color}» не сохранён в PDF")),
            }
        }

        out
    }

    fn paragraph(&mut self, node: &PmNode) -> String {
        let mut out = self.inline(&node.content);

        if out.trim().is_empty() {
            out = "#par[]".to_owned();
        } else {
            out = self.paragraph_format(node, out);
        }

        out
    }

    fn heading(&mut self, node: &PmNode) -> String {
        let level = attr_u64(node, "level").unwrap_or(1);

        if !(1..=6).contains(&level) {
            self.warn(format!("Уровень заголовка {level} заменён на 1 в PDF"));
        }

        let level = level.clamp(1, 6);
        let inner = self.inline(&node.content);
        let heading = format!("#heading(level: {level})[{inner}]");

        self.paragraph_format(node, heading)
    }

    fn paragraph_format(&mut self, node: &PmNode, content: String) -> String {
        let mut out = content;
        let align = attr_str(node, "textAlign").unwrap_or("left");
        let justify = align == "justify";

        if let Some(line_height) = attr_str(node, "lineHeight").and_then(parse_number) {
            if line_height > 0.0 {
                let leading = (line_height - 1.0).max(0.0);
                out = format!("#par(leading: {}em)[{out}]", trim_number(leading));
            }
        }

        if justify {
            out = format!("#par(justify: true)[{out}]");
        }

        match align {
            "center" | "right" => out = format!("#align({align})[{out}]"),
            "left" | "justify" => {}
            other => self.warn(format!("Выравнивание «{other}» не сохранено в PDF")),
        }

        let indent = attr_u64(node, "indent").unwrap_or(0).min(8);

        if indent > 0 {
            let width = indent * 18;
            out = format!("#block(inset: (left: {width}pt))[{out}]");
        }

        let before = attr_u64(node, "spaceBefore").unwrap_or(0).min(200);
        let after = attr_u64(node, "spaceAfter").unwrap_or(0).min(200);

        if before > 0 || after > 0 {
            out = format!("#block(above: {before}pt, below: {after}pt)[{out}]");
        }

        out
    }

    fn blockquote(&mut self, node: &PmNode) -> String {
        let body = self.blocks(&node.content);

        format!("#block(inset: (left: 12pt), stroke: (left: 2pt + rgb(\"#d0d0d0\")))[{body}]")
    }

    fn code_block(&mut self, node: &PmNode) -> String {
        let text = node_text(node);

        format!("#raw({}, block: true, lang: \"text\")", typst_string(&text))
    }

    fn list(&mut self, node: &PmNode, ordered: bool, task: bool) -> String {
        let mut out = String::new();

        for item in &node.content {
            if !out.is_empty() {
                out.push('\n');
            }

            let checked = attr_bool(item, "checked").unwrap_or(false);
            let marker = if task {
                if checked {
                    "- #sym.square.filled"
                } else {
                    "- #sym.square.stroked"
                }
            } else if ordered {
                "+"
            } else {
                "-"
            };

            out.push_str(marker);
            out.push(' ');
            out.push_str(&self.list_item(item));
        }

        out
    }

    fn list_item(&mut self, item: &PmNode) -> String {
        let mut out = String::new();
        let mut first = true;

        for child in &item.content {
            match child.node_type.as_str() {
                "paragraph" => {
                    let text = self.inline(&child.content);

                    if first {
                        out.push_str(&text);
                        first = false;
                    } else {
                        out.push_str("\n\n");
                        out.push_str(&indent_lines(&text, 2));
                    }
                }
                "bulletList" | "orderedList" | "taskList" => {
                    let nested = self.block(child);

                    if !first {
                        out.push('\n');
                    }

                    out.push_str(&indent_lines(&nested, 2));
                    first = false;
                }
                other => {
                    let block = self.block(child);

                    if first {
                        out.push_str(&block);
                        first = false;
                    } else {
                        out.push_str("\n\n");
                        out.push_str(&indent_lines(&block, 2));
                    }

                    let _ = other;
                }
            }
        }

        out
    }

    fn table(&mut self, node: &PmNode) -> String {
        let rows: Vec<&PmNode> = node
            .content
            .iter()
            .filter(|row| row.node_type == "tableRow")
            .collect();

        if rows.is_empty() {
            self.warn("Пустая таблица пропущена в PDF");
            return String::new();
        }

        let columns = grid_columns(&rows);
        let mut entries: Vec<String> = Vec::new();

        for row in &rows {
            let header_row = row
                .content
                .iter()
                .any(|cell| cell.node_type == "tableHeader");
            let mut cells = Vec::new();

            for cell in &row.content {
                let mut content = self.blocks(&cell.content);

                if header_row {
                    content = format!("*{content}*");
                }

                let colspan = attr_u64(cell, "colspan").unwrap_or(1).clamp(1, 100);
                let rowspan = attr_u64(cell, "rowspan").unwrap_or(1).clamp(1, 100);

                if colspan > 1 || rowspan > 1 {
                    cells.push(format!(
                        "table.cell(colspan: {colspan}, rowspan: {rowspan})[{content}]"
                    ));
                } else {
                    cells.push(format!("[{content}]"));
                }
            }

            if header_row {
                entries.push(format!("table.header({})", cells.join(", ")));
            } else {
                entries.extend(cells);
            }
        }

        format!(
            "#table(\n  columns: (1fr,) * {columns},\n  align: left + top,\n  {}\n)",
            entries.join(", ")
        )
    }

    fn image(&mut self, node: &PmNode) -> String {
        let Some(source) = attr_str(node, "src").filter(|value| !value.is_empty()) else {
            self.warn("Изображение без источника пропущено в PDF");
            return String::new();
        };

        let Some(path) = self.register_image(source) else {
            return String::new();
        };

        let alt = attr_str(node, "alt").unwrap_or_default();

        format!(
            "#image({}, alt: {}, width: 100%)",
            typst_string(&path),
            typst_string(alt)
        )
    }

    fn register_image(&mut self, source: &str) -> Option<String> {
        let Some(data) = source.strip_prefix("data:") else {
            if source.starts_with("asset:") {
                self.warn("Изображение из кэша не найдено и пропущено в PDF");
            } else {
                self.warn("Внешнее изображение не встроено в PDF");
            }

            return None;
        };
        let Some((meta, payload)) = data.split_once(',') else {
            self.warn("Некорректное изображение пропущено в PDF");
            return None;
        };
        let extension = if meta.starts_with("image/png") {
            "png"
        } else if meta.starts_with("image/jpeg") {
            "jpg"
        } else if meta.starts_with("image/gif") {
            "gif"
        } else if meta.starts_with("image/webp") {
            "webp"
        } else {
            self.warn("Формат изображения не поддерживается в PDF");
            return None;
        };
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(payload) else {
            self.warn("Не удалось прочитать изображение для PDF");
            return None;
        };

        let path = format!("/assets/image-{}.{}", self.files.len() + 1, extension);
        self.files.push((path.clone(), bytes));

        Some(path)
    }
}

fn attr_str<'a>(node: &'a PmNode, key: &str) -> Option<&'a str> {
    node.attrs
        .as_ref()
        .and_then(|attrs| attrs.get(key))
        .and_then(Value::as_str)
}

fn attr_u64(node: &PmNode, key: &str) -> Option<u64> {
    node.attrs
        .as_ref()
        .and_then(|attrs| attrs.get(key))
        .and_then(Value::as_u64)
}

fn attr_bool(node: &PmNode, key: &str) -> Option<bool> {
    node.attrs
        .as_ref()
        .and_then(|attrs| attrs.get(key))
        .and_then(Value::as_bool)
}

fn parse_number(value: &str) -> Option<f64> {
    value.trim().parse().ok()
}

fn trim_number(value: f64) -> String {
    let rounded = (value * 100.0).round() / 100.0;

    if (rounded - rounded.trunc()).abs() < f64::EPSILON {
        format!("{rounded:.0}")
    } else {
        format!("{rounded}")
    }
}

fn typst_color(value: &str) -> Option<String> {
    let value = value.trim();
    let hex = value.strip_prefix('#')?;

    if !matches!(hex.len(), 3 | 4 | 6 | 8) || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    Some(format!("rgb(\"{value}\")"))
}

fn typst_size(value: &str) -> Option<String> {
    let value = value.trim().to_ascii_lowercase();
    let (number, factor) = if let Some(number) = value.strip_suffix("pt") {
        (number, 1.0)
    } else {
        (value.strip_suffix("px")?, 0.75)
    };
    let number: f64 = number.trim().parse().ok()?;

    if !(1.0..=200.0).contains(&number) {
        return None;
    }

    Some(format!("{}pt", trim_number(number * factor)))
}

fn safe_link(href: &str) -> bool {
    if href.chars().any(char::is_control) {
        return false;
    }

    let lower = href.to_ascii_lowercase();

    !lower.starts_with("javascript:") && !lower.starts_with("data:") && !lower.starts_with("file:")
}

fn node_text(node: &PmNode) -> String {
    if let Some(text) = &node.text {
        return text.clone();
    }

    match node.node_type.as_str() {
        "hardBreak" => "\n".to_owned(),
        _ => node.content.iter().map(node_text).collect(),
    }
}

fn grid_columns(rows: &[&PmNode]) -> usize {
    let mut occupied: HashSet<(usize, usize)> = HashSet::new();
    let mut columns = 1;

    for (row_index, row) in rows.iter().enumerate() {
        let mut column = 0;

        for cell in &row.content {
            while occupied.contains(&(row_index, column)) {
                column += 1;
            }

            let colspan = attr_u64(cell, "colspan").unwrap_or(1).clamp(1, 100) as usize;
            let rowspan = attr_u64(cell, "rowspan").unwrap_or(1).clamp(1, 100) as usize;

            for offset_row in 0..rowspan {
                for offset_column in 0..colspan {
                    occupied.insert((row_index + offset_row, column + offset_column));
                }
            }

            column += colspan;
            columns = columns.max(column);
        }
    }

    columns
}

fn indent_lines(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);

    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn paragraph(text: &str) -> PmNode {
        PmNode::with_content("paragraph", vec![PmNode::text(text)])
    }

    fn document(content: Vec<PmNode>) -> PmDoc {
        PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content,
        }
    }

    #[test]
    fn renders_paragraph_with_marks() {
        let doc = document(vec![PmNode::with_content(
            "paragraph",
            vec![PmNode {
                marks: vec![PmMark::new("bold")],
                ..PmNode::text("жирный")
            }],
        )]);
        let output = convert(&doc, &PdfOptions::default(), None).expect("конверсия");

        assert!(output.source.contains("*жирный*"), "{}", output.source);
    }

    #[test]
    fn renders_heading_and_page_break() {
        let doc = document(vec![
            PmNode {
                attrs: Some(json!({ "level": 2 })),
                ..PmNode::with_content("heading", vec![PmNode::text("Заголовок")])
            },
            PmNode {
                node_type: "pageBreak".to_owned(),
                attrs: None,
                content: Vec::new(),
                marks: Vec::new(),
                text: None,
            },
        ]);
        let output = convert(&doc, &PdfOptions::default(), None).expect("конверсия");

        assert!(output.source.contains("#heading(level: 2)[Заголовок]"));
        assert!(output.source.contains("#pagebreak()"));
    }

    #[test]
    fn renders_task_list_markers() {
        let doc = document(vec![PmNode::with_content(
            "taskList",
            vec![
                PmNode {
                    attrs: Some(json!({ "checked": true })),
                    ..PmNode::with_content("taskItem", vec![paragraph("готово")])
                },
                PmNode {
                    attrs: Some(json!({ "checked": false })),
                    ..PmNode::with_content("taskItem", vec![paragraph("сделать")])
                },
            ],
        )]);
        let output = convert(&doc, &PdfOptions::default(), None).expect("конверсия");

        assert!(output.source.contains("- #sym.square.filled готово"));
        assert!(output.source.contains("- #sym.square.stroked сделать"));
    }

    #[test]
    fn renders_table_with_span_and_header() {
        let doc = document(vec![PmNode::with_content(
            "table",
            vec![
                PmNode::with_content(
                    "tableRow",
                    vec![PmNode {
                        attrs: Some(json!({ "colspan": 2, "rowspan": 1 })),
                        ..PmNode::with_content("tableHeader", vec![paragraph("Шапка")])
                    }],
                ),
                PmNode::with_content(
                    "tableRow",
                    vec![
                        PmNode {
                            attrs: Some(json!({ "colspan": 1, "rowspan": 1 })),
                            ..PmNode::with_content("tableCell", vec![paragraph("раз")])
                        },
                        PmNode {
                            attrs: Some(json!({ "colspan": 1, "rowspan": 1 })),
                            ..PmNode::with_content("tableCell", vec![paragraph("два")])
                        },
                    ],
                ),
            ],
        )]);
        let output = convert(&doc, &PdfOptions::default(), None).expect("конверсия");

        assert!(
            output.source.contains("columns: (1fr,) * 2"),
            "{}",
            output.source
        );
        assert!(output.source.contains("table.header("), "{}", output.source);
        assert!(output.source.contains("table.cell(colspan: 2, rowspan: 1)"));
    }

    #[test]
    fn embeds_data_url_image_and_reports_asset_image() {
        let doc = document(vec![PmNode::with_content(
            "paragraph",
            vec![
                PmNode {
                    attrs: Some(json!({
                        "src": "data:image/png;base64,iVBORw0KGgo=",
                        "alt": "схема"
                    })),
                    ..PmNode::with_content("image", Vec::new())
                },
                PmNode {
                    attrs: Some(json!({ "src": "asset:missing" })),
                    ..PmNode::with_content("image", Vec::new())
                },
            ],
        )]);
        let output = convert(&doc, &PdfOptions::default(), None).expect("конверсия");

        assert_eq!(output.files.len(), 1);
        assert_eq!(output.files[0].0, "/assets/image-1.png");
        assert!(output
            .source
            .contains("#image(\"/assets/image-1.png\", alt: \"схема\""));
        assert!(output
            .warnings
            .iter()
            .any(|warning| warning.contains("кэша")));
    }

    #[test]
    fn replaces_unknown_font_and_warns() {
        let doc = document(vec![PmNode::with_content(
            "paragraph",
            vec![PmNode {
                marks: vec![PmMark {
                    mark_type: "textStyle".to_owned(),
                    attrs: Some(json!({ "fontFamily": "Comic Sans MS" })),
                }],
                ..PmNode::text("текст")
            }],
        )]);
        let output = convert(&doc, &PdfOptions::default(), None).expect("конверсия");

        assert!(output.source.contains("#text(font: \"Liberation Serif\")"));
        assert!(output
            .warnings
            .iter()
            .any(|warning| warning.contains("заменён")));
    }

    #[test]
    fn renders_header_footer_and_page_numbers() {
        let options = PdfOptions {
            header: Some("Отчёт {page}".to_owned()),
            footer: Some("{pages}".to_owned()),
            page_numbers: true,
            ..PdfOptions::default()
        };
        let doc = document(vec![paragraph("текст")]);
        let output = convert(&doc, &PdfOptions::default(), None).expect("конверсия");
        assert!(!output.source.contains("header:"));

        let output = convert(&doc, &options, None).expect("конверсия");

        assert!(
            output.source.contains("header: align(center)"),
            "{}",
            output.source
        );
        assert!(output.source.contains("#context counter(page).display()"));
        assert!(output
            .source
            .contains("#context counter(page).final().first()"));
    }

    #[test]
    fn rejects_invalid_margins() {
        let options = PdfOptions {
            margins: crate::Margins::uniform(120.0),
            ..PdfOptions::default()
        };
        let doc = document(vec![paragraph("текст")]);

        let error = convert(&doc, &options, None).expect_err("ожидалась ошибка");

        assert!(matches!(error, PdfError::InvalidOptions(_)));
    }
}
