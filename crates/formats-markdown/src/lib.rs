use comrak::nodes::{AstNode, ListType, NodeList, NodeValue};
use comrak::{parse_document, Arena, Options};
use docmodel::{PmDoc, PmMark, PmNode};
use formats_core::{
    image_alt, is_safe_link, node_text, DocumentCodec, FormatError, ImportedDocument, WarningSink,
    WrittenDocument,
};
use serde_json::json;

// Память на разбор ограничена сверху размером входа: app-core::files не читает больше 50 MiB.
const MAX_DEPTH: usize = 256;
const MAX_NODES: usize = 1_000_000;

struct Limits {
    max_nodes: usize,
    max_depth: usize,
}

impl Limits {
    const fn defaults() -> Self {
        Self {
            max_nodes: MAX_NODES,
            max_depth: MAX_DEPTH,
        }
    }
}

struct Budget {
    nodes: usize,
    limits: Limits,
}

impl Budget {
    fn new(limits: Limits) -> Self {
        Self { nodes: 0, limits }
    }

    fn enter(&mut self, depth: usize) -> Result<(), FormatError> {
        if depth > self.limits.max_depth {
            return Err(FormatError::Parse(format!(
                "превышена максимальная глубина вложенности Markdown {}",
                self.limits.max_depth
            )));
        }

        self.nodes += 1;

        if self.nodes > self.limits.max_nodes {
            return Err(FormatError::Parse(format!(
                "превышено максимальное число узлов Markdown {}",
                self.limits.max_nodes
            )));
        }

        Ok(())
    }
}

pub struct MarkdownCodec;

static MARKDOWN_CODEC: MarkdownCodec = MarkdownCodec;

pub fn codec() -> &'static dyn DocumentCodec {
    &MARKDOWN_CODEC
}

impl DocumentCodec for MarkdownCodec {
    fn read(&self, source: &str) -> Result<ImportedDocument, FormatError> {
        read_with_limits(source, Limits::defaults())
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

fn read_with_limits(source: &str, limits: Limits) -> Result<ImportedDocument, FormatError> {
    let arena = Arena::new();
    let root = parse_document(&arena, source, &options());
    let mut warnings = WarningSink::new();
    let mut budget = Budget::new(limits);
    let mut content = convert_blocks(root, 1, &mut budget, &mut warnings)?;

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

fn options() -> Options<'static> {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options
}

fn convert_blocks<'a>(
    parent: &'a AstNode<'a>,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<Vec<PmNode>, FormatError> {
    let mut blocks = Vec::new();

    for child in parent.children() {
        blocks.extend(convert_block(child, depth, budget, warnings)?);
    }

    Ok(blocks)
}

fn convert_block<'a>(
    node: &'a AstNode<'a>,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<Vec<PmNode>, FormatError> {
    budget.enter(depth)?;
    let value = node.data.borrow().value.clone();

    match value {
        NodeValue::Paragraph => Ok(vec![PmNode::with_content(
            "paragraph",
            convert_inline_children(node, &[], depth + 1, budget, warnings)?,
        )]),
        NodeValue::Heading(heading) => {
            let mut paragraph = PmNode::with_content(
                "heading",
                convert_inline_children(node, &[], depth + 1, budget, warnings)?,
            );
            paragraph.attrs = Some(json!({ "level": heading.level }));
            Ok(vec![paragraph])
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
            Ok(vec![block])
        }
        NodeValue::BlockQuote => {
            let mut content = convert_blocks(node, depth + 1, budget, warnings)?;

            if content.is_empty() {
                content.push(PmNode::paragraph());
            }

            Ok(vec![PmNode::with_content("blockquote", content)])
        }
        NodeValue::List(list) => Ok(vec![convert_list(
            node,
            &list,
            depth + 1,
            budget,
            warnings,
        )?]),
        NodeValue::Table(_) => Ok(vec![convert_table(node, depth + 1, budget, warnings)?]),
        NodeValue::ThematicBreak => Ok(vec![block_node("horizontalRule")]),
        NodeValue::HtmlBlock(block) => {
            if block.literal.trim() == "<!-- pagebreak -->" {
                Ok(vec![block_node("pageBreak")])
            } else {
                warnings.warn("HTML-блоки не поддерживаются и были пропущены.");
                Ok(Vec::new())
            }
        }
        NodeValue::FootnoteDefinition(_) => {
            warnings.warn("Сноски не поддерживаются и были пропущены.");
            Ok(Vec::new())
        }
        NodeValue::TaskItem(_) => Ok(vec![convert_task_item(node, depth + 1, budget, warnings)?]),
        NodeValue::Document => convert_blocks(node, depth + 1, budget, warnings),
        _ => {
            let blocks = convert_blocks(node, depth + 1, budget, warnings)?;

            if !blocks.is_empty() {
                return Ok(blocks);
            }

            let inline = convert_inline_children(node, &[], depth + 1, budget, warnings)?;

            if inline.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![PmNode::with_content("paragraph", inline)])
            }
        }
    }
}

fn convert_list<'a>(
    node: &'a AstNode<'a>,
    list: &NodeList,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<PmNode, FormatError> {
    let node_type = if list.is_task_list {
        "taskList"
    } else {
        match list.list_type {
            ListType::Bullet => "bulletList",
            ListType::Ordered => "orderedList",
        }
    };

    let mut items = Vec::new();

    for item in node.children() {
        let converted = if list.is_task_list {
            convert_task_item(item, depth, budget, warnings)?
        } else {
            convert_list_item(item, depth, budget, warnings)?
        };
        items.push(converted);
    }

    let mut pm_node = PmNode::with_content(node_type, items);

    if !list.is_task_list && matches!(list.list_type, ListType::Ordered) && list.start != 1 {
        pm_node.attrs = Some(json!({ "start": list.start }));
    }

    Ok(pm_node)
}

fn convert_list_item<'a>(
    node: &'a AstNode<'a>,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<PmNode, FormatError> {
    budget.enter(depth)?;
    let mut content = convert_blocks(node, depth + 1, budget, warnings)?;

    if content.first().map(|child| child.node_type.as_str()) != Some("paragraph") {
        content.insert(0, PmNode::paragraph());
    }

    Ok(PmNode::with_content("listItem", content))
}

fn convert_task_item<'a>(
    node: &'a AstNode<'a>,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<PmNode, FormatError> {
    budget.enter(depth)?;
    let checked = match &node.data.borrow().value {
        NodeValue::TaskItem(item) => item.symbol.is_some(),
        _ => false,
    };
    let mut content = convert_blocks(node, depth + 1, budget, warnings)?;

    if content.first().map(|child| child.node_type.as_str()) != Some("paragraph") {
        content.insert(0, PmNode::paragraph());
    }

    let mut item = PmNode::with_content("taskItem", content);
    item.attrs = Some(json!({ "checked": checked }));

    Ok(item)
}

fn convert_table<'a>(
    node: &'a AstNode<'a>,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<PmNode, FormatError> {
    let mut rows = Vec::new();

    for row in node.children() {
        budget.enter(depth)?;
        let is_header = matches!(&row.data.borrow().value, NodeValue::TableRow(true));
        let mut cells = Vec::new();

        for cell in row.children() {
            budget.enter(depth)?;
            cells.push(convert_table_cell(
                cell,
                is_header,
                depth + 1,
                budget,
                warnings,
            )?);
        }

        rows.push(PmNode::with_content("tableRow", cells));
    }

    Ok(PmNode::with_content("table", rows))
}

fn convert_table_cell<'a>(
    node: &'a AstNode<'a>,
    is_header: bool,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<PmNode, FormatError> {
    let inline = convert_inline_children(node, &[], depth, budget, warnings)?;
    let content = if inline.is_empty() {
        vec![PmNode::paragraph()]
    } else {
        vec![PmNode::with_content("paragraph", inline)]
    };

    let mut cell = PmNode::with_content(
        if is_header {
            "tableHeader"
        } else {
            "tableCell"
        },
        content,
    );
    cell.attrs = Some(json!({ "colspan": 1, "rowspan": 1, "colwidth": null }));
    Ok(cell)
}

fn convert_inline_children<'a>(
    node: &'a AstNode<'a>,
    marks: &[PmMark],
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<Vec<PmNode>, FormatError> {
    let mut output = Vec::new();

    for child in node.children() {
        convert_inline(child, marks, &mut output, depth, budget, warnings)?;
    }

    Ok(output)
}

fn convert_inline<'a>(
    node: &'a AstNode<'a>,
    marks: &[PmMark],
    output: &mut Vec<PmNode>,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<(), FormatError> {
    budget.enter(depth)?;
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
            convert_inline_children_into(node, &nested, output, depth + 1, budget, warnings)?;
        }
        NodeValue::Strong => {
            let mut nested = marks.to_vec();
            nested.push(PmMark::new("bold"));
            convert_inline_children_into(node, &nested, output, depth + 1, budget, warnings)?;
        }
        NodeValue::Strikethrough => {
            let mut nested = marks.to_vec();
            nested.push(PmMark::new("strike"));
            convert_inline_children_into(node, &nested, output, depth + 1, budget, warnings)?;
        }
        NodeValue::Link(link) => {
            if link.url.trim().is_empty() {
                convert_inline_children_into(node, marks, output, depth + 1, budget, warnings)?;
                return Ok(());
            }

            if !is_safe_link(&link.url) {
                warnings.warn("Ссылка с небезопасным адресом была преобразована в текст.");
                convert_inline_children_into(node, marks, output, depth + 1, budget, warnings)?;
                return Ok(());
            }

            let mut nested = marks.to_vec();
            let mut mark = PmMark::new("link");
            mark.attrs = Some(json!({
                "href": link.url,
                "target": null,
                "rel": null,
                "class": null
            }));
            nested.push(mark);
            convert_inline_children_into(node, &nested, output, depth + 1, budget, warnings)?;
        }
        NodeValue::Image(image) => {
            if image.url.trim().is_empty() {
                warnings.warn("Изображение без адреса было пропущено.");
                return Ok(());
            }

            if !is_safe_link(&image.url) {
                warnings.warn("Изображение с небезопасным адресом было пропущено.");
                return Ok(());
            }

            let alt = plain_text(node, depth + 1, budget)?;
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
        _ => convert_inline_children_into(node, marks, output, depth + 1, budget, warnings)?,
    }

    Ok(())
}

fn convert_inline_children_into<'a>(
    node: &'a AstNode<'a>,
    marks: &[PmMark],
    output: &mut Vec<PmNode>,
    depth: usize,
    budget: &mut Budget,
    warnings: &mut WarningSink,
) -> Result<(), FormatError> {
    for child in node.children() {
        convert_inline(child, marks, output, depth, budget, warnings)?;
    }

    Ok(())
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

fn plain_text<'a>(
    node: &'a AstNode<'a>,
    depth: usize,
    budget: &mut Budget,
) -> Result<String, FormatError> {
    let mut text = String::new();

    for child in node.children() {
        budget.enter(depth)?;
        let value = child.data.borrow().value.clone();

        match value {
            NodeValue::Text(value) => text.push_str(&value),
            NodeValue::SoftBreak | NodeValue::LineBreak => text.push(' '),
            _ => text.push_str(&plain_text(child, depth + 1, budget)?),
        }
    }

    Ok(text)
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

fn warn_paragraph_format(node: &PmNode, context: &mut WriteContext) {
    let Some(attrs) = node.attrs.as_ref().and_then(|attrs| attrs.as_object()) else {
        return;
    };

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

    if align != "left" {
        context
            .warnings
            .warn("Выравнивание абзацев не сохраняется в Markdown.");
    }

    if indent > 0 {
        context
            .warnings
            .warn("Отступы абзацев не сохраняются в Markdown.");
    }

    if !line_height.is_empty() || space_before > 0 || space_after > 0 {
        context
            .warnings
            .warn("Интервалы абзацев не сохраняются в Markdown.");
    }
}

fn write_block(node: &PmNode, output: &mut String, indent: usize, context: &mut WriteContext) {
    match node.node_type.as_str() {
        "paragraph" => {
            warn_paragraph_format(node, context);
            write_inline_children(node, output, context);
            output.push_str("\n\n");
        }
        "heading" => {
            warn_paragraph_format(node, context);
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
        "taskList" => write_task_list(node, output, indent, context),
        "table" => write_table(node, output, context),
        "pageBreak" => output.push_str("<!-- pagebreak -->\n\n"),
        "horizontalRule" => output.push_str("---\n\n"),
        "image" => {
            if let Some(markdown) = image_markdown(node, context) {
                output.push_str(&markdown);
                output.push_str("\n\n");
            }
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

                        if is_safe_link(href) {
                            rendered = format!("[{rendered}]({href})");
                        } else {
                            context
                                .warnings
                                .warn("Ссылка с небезопасным адресом не сохраняется в Markdown.");
                        }
                    }
                    "textStyle" => context
                        .warnings
                        .warn("Шрифт, размер и цвет текста не сохраняются в Markdown."),
                    "highlight" => context
                        .warnings
                        .warn("Выделение цветом не сохраняется в Markdown."),
                    "underline" => context
                        .warnings
                        .warn("Подчёркивание не сохраняется в Markdown."),
                    other => context.warnings.warn(format!(
                        "Разметка «{other}» не поддерживается Markdown и была упрощена."
                    )),
                }
            }

            output.push_str(&rendered);
        }
        "hardBreak" => output.push_str("  \n"),
        "image" => {
            if let Some(markdown) = image_markdown(node, context) {
                output.push_str(&markdown);
            }
        }
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
                "taskList" => write_task_list(child, output, indent + 1, context),
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

fn write_task_list(node: &PmNode, output: &mut String, indent: usize, context: &mut WriteContext) {
    let padding = "  ".repeat(indent);

    for item in &node.content {
        let checked = item
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("checked"))
            .and_then(|checked| checked.as_bool())
            .unwrap_or(false);
        let marker = if checked { "- [x] " } else { "- [ ] " };
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
                "taskList" => write_task_list(child, output, indent + 1, context),
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
    if node
        .content
        .iter()
        .flat_map(|row| row.content.iter())
        .any(has_cell_span)
    {
        context
            .warnings
            .warn("Объединённые ячейки не сохраняются в Markdown.");
    }

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

fn has_cell_span(cell: &PmNode) -> bool {
    let Some(attrs) = cell.attrs.as_ref().and_then(|attrs| attrs.as_object()) else {
        return false;
    };
    let colspan = attrs
        .get("colspan")
        .and_then(|value| value.as_u64())
        .unwrap_or(1);
    let rowspan = attrs
        .get("rowspan")
        .and_then(|value| value.as_u64())
        .unwrap_or(1);

    colspan > 1 || rowspan > 1
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

fn image_markdown(node: &PmNode, context: &mut WriteContext) -> Option<String> {
    let source = node
        .attrs
        .as_ref()
        .and_then(|attrs| attrs.get("src"))
        .and_then(|src| src.as_str())
        .unwrap_or_default();

    if !is_safe_link(source) {
        context
            .warnings
            .warn("Изображение с небезопасным адресом не сохраняется в Markdown.");

        let alt = image_alt(node);

        return (!alt.trim().is_empty()).then(|| escape_text(&alt));
    }

    Some(format!("![{}]({source})", image_alt(node)))
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
    use formats_core::DocumentFormat;

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
    fn parses_task_lists() {
        let imported = read("- [ ] первая\n- [x] вторая\n");

        let list = &imported.document.content[0];
        let first = &list.content[0];
        let second = &list.content[1];

        assert_eq!(list.node_type, "taskList");
        assert_eq!(first.node_type, "taskItem");
        assert_eq!(first.attrs, Some(json!({ "checked": false })));
        assert_eq!(second.attrs, Some(json!({ "checked": true })));
        assert_eq!(node_text(first), "первая");
        assert_eq!(node_text(second), "вторая");
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn parses_nested_task_lists() {
        let imported = read("- [ ] родитель\n  - [x] дочерняя\n");

        let list = &imported.document.content[0];
        let parent = &list.content[0];
        let nested = &parent.content[1];

        assert_eq!(list.node_type, "taskList");
        assert_eq!(nested.node_type, "taskList");
        assert_eq!(nested.content[0].attrs, Some(json!({ "checked": true })));
        assert_eq!(node_text(&nested.content[0]), "дочерняя");
    }

    #[test]
    fn task_list_round_trip() {
        let source = "- [ ] первая\n- [x] вторая\n";
        let original = read(source);
        let written = write(&original.document);

        assert!(written.text.contains("- [ ] первая"));
        assert!(written.text.contains("- [x] вторая"));

        let restored = read(&written.text);
        assert_eq!(restored.document, original.document);
    }

    #[test]
    fn parses_gfm_tables_with_header() {
        let imported = read("| Имя | Значение |\n| --- | --- |\n| a | 1 |\n");
        let table = &imported.document.content[0];

        assert_eq!(table.node_type, "table");
        assert_eq!(table.content.len(), 2);
        assert_eq!(table.content[0].content[0].node_type, "tableHeader");
        assert_eq!(table.content[1].content[0].node_type, "tableCell");
        assert_eq!(node_text(&table.content[0].content[0]), "Имя");
        assert_eq!(node_text(&table.content[0].content[1]), "Значение");
        assert_eq!(node_text(&table.content[1].content[0]), "a");
        assert_eq!(node_text(&table.content[1].content[1]), "1");
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
    fn warns_about_paragraph_formatting() {
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

        let written = write(&document);

        assert!(written.text.contains("текст"));
        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("Выравнивание")));
        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("Отступы")));
        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("Интервалы")));
    }

    #[test]
    fn warns_about_text_style_mark() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode {
                    marks: vec![PmMark {
                        attrs: Some(json!({ "fontSize": "14pt" })),
                        ..PmMark::new("textStyle")
                    }],
                    ..PmNode::text("текст")
                }],
            )],
        };

        let written = write(&document);

        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("Шрифт")));
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
        assert_eq!(
            node_text(&restored.document.content[0].content[1].content[0]),
            "1"
        );
        assert_eq!(
            node_text(&restored.document.content[0].content[1].content[1]),
            "2"
        );
    }

    #[test]
    fn warns_about_merged_cells() {
        fn cell(node_type: &str, text: &str, colspan: u64) -> PmNode {
            let mut cell = PmNode::with_content(
                node_type,
                vec![PmNode::with_content("paragraph", vec![PmNode::text(text)])],
            );
            cell.attrs = Some(json!({ "colspan": colspan, "rowspan": 1 }));
            cell
        }

        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "table",
                vec![
                    PmNode::with_content(
                        "tableRow",
                        vec![cell("tableHeader", "A", 1), cell("tableHeader", "B", 1)],
                    ),
                    PmNode::with_content("tableRow", vec![cell("tableCell", "объединено", 2)]),
                ],
            )],
        };

        let written = write(&document);

        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("Объединённые")));
    }

    #[test]
    fn reports_supported_format() {
        assert_eq!(
            DocumentFormat::from_path(std::path::Path::new("README.markdown")),
            Some(DocumentFormat::Markdown)
        );
    }

    #[test]
    fn rejects_documents_deeper_than_limit() {
        let source = format!("{}текст\n", "> ".repeat(MAX_DEPTH + 44));
        let error = codec().read(&source).expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("глубина"));
    }

    #[test]
    fn accepts_normal_nested_documents() {
        let source = format!("{}текст\n", "> ".repeat(10));
        let imported = read(&source);

        assert!(node_text(&imported.document.content[0]).contains("текст"));
    }

    #[test]
    fn rejects_documents_over_node_limit() {
        let limits = Limits {
            max_nodes: 4,
            max_depth: MAX_DEPTH,
        };
        let error = read_with_limits("один\n\nдва\n\nтри\n\nчетыре\n", limits)
            .expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("узлов"));
    }

    #[test]
    fn drops_unsafe_links_and_images() {
        let imported = read("[опасная](javascript:alert(1))\n\n![картинка](javascript:alert(2))\n");

        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("Ссылка")));
        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("Изображение")));

        let paragraph = &imported.document.content[0];
        assert!(paragraph
            .content
            .iter()
            .all(|node| node.marks.iter().all(|mark| mark.mark_type != "link")));
        assert_eq!(node_text(paragraph), "опасная");
        assert!(imported.document.content[1]
            .content
            .iter()
            .all(|node| node.node_type != "image"));
    }

    #[test]
    fn keeps_empty_links_as_text_without_warning() {
        for source in ["[x]()\n", "[x](<>)\n", "[x]( )\n"] {
            let imported = read(source);

            assert!(
                imported.warnings.is_empty(),
                "{source}: {:?}",
                imported.warnings
            );
            assert_eq!(node_text(&imported.document.content[0]), "x");
            assert!(imported.document.content[0]
                .content
                .iter()
                .all(|node| node.marks.iter().all(|mark| mark.mark_type != "link")));
        }
    }

    #[test]
    fn warns_about_images_without_address() {
        let imported = read("![картинка]()\n");

        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("без адреса")));
        assert!(imported
            .warnings
            .iter()
            .all(|warning| !warning.contains("небезопасн")));
        assert!(imported.document.content[0]
            .content
            .iter()
            .all(|node| node.node_type != "image"));
    }

    fn link_mark(href: &str) -> PmMark {
        let mut mark = PmMark::new("link");
        mark.attrs = Some(json!({ "href": href }));
        mark
    }

    #[test]
    fn drops_unsafe_links_and_images_on_write() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![
                PmNode::with_content(
                    "paragraph",
                    vec![
                        PmNode {
                            marks: vec![link_mark("javascript:alert(1)")],
                            ..PmNode::text("опасная")
                        },
                        PmNode::text(" "),
                        PmNode {
                            marks: vec![link_mark("https://example.com")],
                            ..PmNode::text("безопасная")
                        },
                    ],
                ),
                PmNode {
                    attrs: Some(json!({ "src": "javascript:alert(2)", "alt": "подпись" })),
                    ..PmNode::with_content("image", Vec::new())
                },
            ],
        };

        let written = write(&document);

        assert!(!written.text.contains("javascript:"));
        assert!(written.text.contains("опасная"));
        assert!(written.text.contains("[безопасная](https://example.com)"));
        assert!(written.text.contains("подпись"));
        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("Ссылка")));
        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("Изображение")));
    }

    #[test]
    fn writes_safe_links_without_warnings() {
        let safe_links = [
            "http://example.com",
            "https://example.com",
            "mailto:user@example.com",
            "asset:image-1",
            "tel:+123456",
            "image.png",
        ];
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: safe_links
                .iter()
                .map(|href| {
                    PmNode::with_content(
                        "paragraph",
                        vec![PmNode {
                            marks: vec![link_mark(href)],
                            ..PmNode::text("ссылка")
                        }],
                    )
                })
                .collect(),
        };

        let written = write(&document);

        for href in safe_links {
            assert!(
                written.text.contains(&format!("]({href})")),
                "{href}: {}",
                written.text
            );
        }

        assert!(written
            .warnings
            .iter()
            .all(|warning| !warning.contains("небезопасн")));
    }
}
