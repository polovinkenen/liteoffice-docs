use ego_tree::NodeRef;
use scraper::node::Node;
use scraper::Html;
use serde_json::{json, Value};
use wlite_docmodel::{PmDoc, PmMark, PmNode};
use wlite_formats_core::{FormatError, ImportedDocument, WarningSink};

use crate::style::{is_safe_link, merge_marks, paragraph_attrs, parse_declarations, text_marks};

const MAX_DEPTH: usize = 256;
const MAX_NODES: usize = 1_000_000;

const BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "center",
    "dd",
    "details",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "html",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "tr",
    "ul",
];

const CONTAINER_TAGS: &[&str] = &[
    "address", "article", "aside", "body", "center", "details", "div", "dl", "fieldset", "figure",
    "footer", "header", "hgroup", "html", "main", "nav", "section", "summary",
];

const PARAGRAPH_TAGS: &[&str] = &["address", "dd", "dt", "figcaption"];

const DROPPED_TAGS: &[&str] = &[
    "applet", "audio", "base", "button", "canvas", "embed", "form", "iframe", "input", "link",
    "math", "meta", "noscript", "object", "script", "select", "source", "style", "svg", "template",
    "textarea", "track", "video",
];

const INLINE_TAGS: &[&str] = &[
    "abbr", "b", "bdi", "bdo", "big", "br", "cite", "code", "data", "del", "dfn", "em", "font",
    "i", "img", "ins", "kbd", "label", "mark", "q", "s", "samp", "small", "span", "strike",
    "strong", "sub", "sup", "time", "tt", "u", "var",
];

pub fn read_document(source: &str) -> Result<ImportedDocument, FormatError> {
    let html = Html::parse_document(source);
    let root = html.tree.root();
    let body = find_element(root, "body").unwrap_or(root);
    let mut context = ReadContext::new();
    let mut content = context.convert_blocks(body, 0)?;

    if content.is_empty() {
        content.push(PmNode::paragraph());
    }

    Ok(ImportedDocument {
        document: PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content,
        },
        warnings: context.warnings.into_warnings(),
    })
}

fn find_element<'a>(node: NodeRef<'a, Node>, name: &str) -> Option<NodeRef<'a, Node>> {
    for child in node.children() {
        if child
            .value()
            .as_element()
            .is_some_and(|element| element.name() == name)
        {
            return Some(child);
        }

        if let Some(found) = find_element(child, name) {
            return Some(found);
        }
    }

    None
}

struct ReadContext {
    warnings: WarningSink,
    nodes: usize,
}

impl ReadContext {
    fn new() -> Self {
        Self {
            warnings: WarningSink::new(),
            nodes: 0,
        }
    }

    fn enter(&mut self, depth: usize) -> Result<(), FormatError> {
        if depth > MAX_DEPTH {
            return Err(FormatError::Parse(
                "превышена максимальная глубина вложенности HTML".to_owned(),
            ));
        }

        self.nodes += 1;

        if self.nodes > MAX_NODES {
            return Err(FormatError::Parse(
                "превышено максимальное число узлов HTML".to_owned(),
            ));
        }

        Ok(())
    }

    fn warn(&mut self, message: impl Into<String>) {
        self.warnings.warn(message);
    }

    fn convert_blocks(
        &mut self,
        parent: NodeRef<'_, Node>,
        depth: usize,
    ) -> Result<Vec<PmNode>, FormatError> {
        let mut blocks = Vec::new();
        let mut pending = Vec::new();

        for child in parent.children() {
            self.enter(depth)?;

            match child.value() {
                Node::Element(element) => {
                    let name = element.name();

                    if DROPPED_TAGS.contains(&name) {
                        self.warn(format!(
                            "Тег «{name}» не поддерживается и был пропущен вместе с содержимым."
                        ));
                        continue;
                    }

                    if is_block_tag(name) {
                        flush_inline(&mut pending, &mut blocks);
                        blocks.extend(self.convert_block(child, depth + 1)?);
                    } else {
                        self.convert_inline(child, &[], &mut pending, depth + 1)?;
                    }
                }
                Node::Text(text) => {
                    push_text(&mut pending, text, &[]);
                }
                Node::Comment(_) | Node::Doctype(_) | Node::ProcessingInstruction(_) => {}
                Node::Document | Node::Fragment => {
                    flush_inline(&mut pending, &mut blocks);
                    blocks.extend(self.convert_blocks(child, depth + 1)?);
                }
            }
        }

        flush_inline(&mut pending, &mut blocks);

        Ok(blocks)
    }

    fn convert_block(
        &mut self,
        node: NodeRef<'_, Node>,
        depth: usize,
    ) -> Result<Vec<PmNode>, FormatError> {
        let Some(element) = node.value().as_element() else {
            return Ok(Vec::new());
        };
        let name = element.name().to_owned();

        if name == "hr" {
            return Ok(vec![atom("horizontalRule")]);
        }

        if name == "table" {
            return Ok(vec![self.convert_table(node, depth)?]);
        }

        if name == "pre" {
            return Ok(vec![self.convert_code_block(node)]);
        }

        if name == "ul" || name == "ol" {
            return Ok(vec![self.convert_list(node, depth)?]);
        }

        if name == "blockquote" {
            let mut content = self.convert_blocks(node, depth + 1)?;

            if content.is_empty() {
                content.push(PmNode::paragraph());
            }

            return Ok(vec![PmNode::with_content("blockquote", content)]);
        }

        if let Some(level) = heading_level(&name) {
            let attrs = merge_paragraph_attrs(element, Some(json!({ "level": level })));
            let content = self.convert_inline_children(node, element, depth + 1)?;
            let mut heading = PmNode::with_content("heading", content);
            heading.attrs = attrs;
            return Ok(vec![heading]);
        }

        if name == "div" && element.attr("data-page-break").is_some() {
            return Ok(vec![atom("pageBreak")]);
        }

        if name == "p" || PARAGRAPH_TAGS.contains(&name.as_str()) {
            return Ok(vec![self.paragraph(node, element, depth)?]);
        }

        if CONTAINER_TAGS.contains(&name.as_str()) || name == "li" {
            if has_block_children(node) {
                let mut content = self.convert_blocks(node, depth + 1)?;
                if content.is_empty() {
                    content.push(PmNode::paragraph());
                }
                return Ok(content);
            }

            return Ok(vec![self.paragraph(node, element, depth)?]);
        }

        let mut content = self.convert_blocks(node, depth + 1)?;
        if content.is_empty() {
            content.push(self.paragraph(node, element, depth)?);
        }

        Ok(content)
    }

    fn paragraph(
        &mut self,
        node: NodeRef<'_, Node>,
        element: &scraper::node::Element,
        depth: usize,
    ) -> Result<PmNode, FormatError> {
        let attrs = merge_paragraph_attrs(element, None);
        let content = self.convert_inline_children(node, element, depth + 1)?;
        let mut paragraph = PmNode::with_content("paragraph", content);
        paragraph.attrs = attrs;
        Ok(paragraph)
    }

    fn convert_inline_children(
        &mut self,
        node: NodeRef<'_, Node>,
        element: &scraper::node::Element,
        depth: usize,
    ) -> Result<Vec<PmNode>, FormatError> {
        let marks = text_marks(&parse_declarations(style_attr(element)));
        let mut content = Vec::new();

        for child in node.children() {
            self.enter(depth)?;
            self.convert_inline(child, &marks, &mut content, depth + 1)?;
        }

        trim_inline(&mut content);

        Ok(content)
    }

    fn convert_inline(
        &mut self,
        node: NodeRef<'_, Node>,
        marks: &[PmMark],
        output: &mut Vec<PmNode>,
        depth: usize,
    ) -> Result<(), FormatError> {
        match node.value() {
            Node::Text(text) => push_text(output, text, marks),
            Node::Comment(_) | Node::Doctype(_) | Node::ProcessingInstruction(_) => {}
            Node::Document | Node::Fragment => {
                for child in node.children() {
                    self.enter(depth)?;
                    self.convert_inline(child, marks, output, depth + 1)?;
                }
            }
            Node::Element(element) => {
                let name = element.name();

                if DROPPED_TAGS.contains(&name) {
                    self.warn(format!(
                        "Тег «{name}» не поддерживается и был пропущен вместе с содержимым."
                    ));
                    return Ok(());
                }

                if is_block_tag(name) {
                    for child in node.children() {
                        self.enter(depth)?;
                        self.convert_inline(child, marks, output, depth + 1)?;
                    }
                    return Ok(());
                }

                if name == "br" {
                    output.push(atom("hardBreak"));
                    return Ok(());
                }

                if name == "img" {
                    if let Some(image) = self.convert_image(element) {
                        output.push(image);
                    }
                    return Ok(());
                }

                let mut nested = marks.to_vec();

                if let Some(mark) = self.tag_mark(element) {
                    nested = merge_marks(&nested, mark);
                }

                for mark in text_marks(&parse_declarations(style_attr(element))) {
                    nested = merge_marks(&nested, mark);
                }

                if !INLINE_TAGS.contains(&name) {
                    self.warn(format!(
                        "Тег «{name}» не поддерживается, его содержимое сохранено."
                    ));
                }

                for child in node.children() {
                    self.enter(depth)?;
                    self.convert_inline(child, &nested, output, depth + 1)?;
                }
            }
        }

        Ok(())
    }

    fn tag_mark(&mut self, element: &scraper::node::Element) -> Option<PmMark> {
        let name = element.name();

        match name {
            "strong" | "b" => Some(PmMark::new("bold")),
            "em" | "i" => Some(PmMark::new("italic")),
            "u" | "ins" => Some(PmMark::new("underline")),
            "s" | "del" | "strike" => Some(PmMark::new("strike")),
            "code" => Some(PmMark::new("code")),
            "mark" => {
                let mut mark = PmMark::new("highlight");
                let color = parse_declarations(style_attr(element))
                    .into_iter()
                    .find(|(property, _)| property == "background-color")
                    .map(|(_, value)| value);

                if let Some(color) = color {
                    mark.attrs = Some(json!({ "color": color }));
                }

                Some(mark)
            }
            "a" => {
                let href = element.attr("href").unwrap_or_default().trim();

                if !href.is_empty() && !is_safe_link(href) {
                    self.warn("Ссылка с небезопасным адресом была преобразована в текст.");
                    return None;
                }

                if href.is_empty() {
                    return None;
                }

                let mut mark = PmMark::new("link");
                mark.attrs = Some(json!({
                    "href": href,
                    "target": null,
                    "rel": null,
                    "class": null
                }));
                Some(mark)
            }
            _ => None,
        }
    }

    fn convert_image(&mut self, element: &scraper::node::Element) -> Option<PmNode> {
        let source = element.attr("src").unwrap_or_default().trim();

        if !is_supported_image_source(source) {
            self.warn("Внешние изображения не загружаются; изображение было пропущено.");
            return None;
        }

        let alt = element.attr("alt").filter(|value| !value.is_empty());
        let title = element.attr("title").filter(|value| !value.is_empty());

        let mut image = atom("image");
        image.attrs = Some(json!({
            "src": source,
            "alt": alt,
            "title": title
        }));

        Some(image)
    }

    fn convert_code_block(&mut self, node: NodeRef<'_, Node>) -> PmNode {
        let language = find_element(node, "code").and_then(|code| {
            let element = code.value().as_element()?;

            element
                .attr("class")?
                .split_whitespace()
                .find_map(|class| class.strip_prefix("language-").map(str::to_owned))
        });

        let mut literal = raw_text(node);
        if literal.starts_with('\n') {
            literal.remove(0);
        }
        let literal = literal.trim_end_matches('\n').to_owned();

        let mut block = PmNode::with_content("codeBlock", vec![PmNode::text(literal)]);

        if let Some(language) = language {
            block.attrs = Some(json!({ "language": language }));
        }

        block
    }

    fn convert_list(
        &mut self,
        node: NodeRef<'_, Node>,
        depth: usize,
    ) -> Result<PmNode, FormatError> {
        let Some(element) = node.value().as_element() else {
            return Ok(PmNode::with_content("bulletList", Vec::new()));
        };
        let ordered = element.name() == "ol";
        let items: Vec<_> = node
            .children()
            .filter(|child| {
                child
                    .value()
                    .as_element()
                    .is_some_and(|element| element.name() == "li")
            })
            .collect();
        let task_list = element.attr("data-type") == Some("taskList")
            || items.iter().any(|item| {
                item.value()
                    .as_element()
                    .is_some_and(|element| element.attr("data-type") == Some("taskItem"))
            });

        if task_list {
            let mut content = Vec::new();

            for item in items {
                let checked = item
                    .value()
                    .as_element()
                    .and_then(|element| element.attr("data-checked"))
                    .is_some_and(|value| value == "true");
                let mut blocks = self.convert_blocks(item, depth + 1)?;
                ensure_leading_paragraph(&mut blocks);

                let mut task_item = PmNode::with_content("taskItem", blocks);
                task_item.attrs = Some(json!({ "checked": checked }));
                content.push(task_item);
            }

            return Ok(PmNode::with_content("taskList", content));
        }

        let mut content = Vec::new();

        for item in items {
            let mut blocks = self.convert_blocks(item, depth + 1)?;
            ensure_leading_paragraph(&mut blocks);
            content.push(PmNode::with_content("listItem", blocks));
        }

        let node_type = if ordered { "orderedList" } else { "bulletList" };
        let mut list = PmNode::with_content(node_type, content);

        if ordered {
            let start = element
                .attr("start")
                .and_then(|value| value.trim().parse::<u64>().ok())
                .unwrap_or(1);

            if start > 1 {
                list.attrs = Some(json!({ "start": start }));
            }
        }

        Ok(list)
    }

    fn convert_table(
        &mut self,
        node: NodeRef<'_, Node>,
        depth: usize,
    ) -> Result<PmNode, FormatError> {
        let mut rows = Vec::new();

        for row in descendants(node, "tr") {
            let mut cells = Vec::new();

            for cell in row.children() {
                let Some(element) = cell.value().as_element() else {
                    continue;
                };

                if element.name() != "td" && element.name() != "th" {
                    continue;
                }

                let header = element.name() == "th";
                let mut content = self.convert_blocks(cell, depth + 1)?;
                ensure_leading_paragraph(&mut content);

                let mut table_cell =
                    PmNode::with_content(if header { "tableHeader" } else { "tableCell" }, content);
                table_cell.attrs = Some(json!({
                    "colspan": span(element, "colspan"),
                    "rowspan": span(element, "rowspan"),
                    "colwidth": null
                }));
                cells.push(table_cell);
            }

            rows.push(PmNode::with_content("tableRow", cells));
        }

        Ok(PmNode::with_content("table", rows))
    }
}

fn is_block_tag(name: &str) -> bool {
    BLOCK_TAGS.contains(&name)
}

fn heading_level(name: &str) -> Option<u64> {
    let level = name.strip_prefix('h')?;
    let level: u64 = level.parse().ok()?;

    if (1..=6).contains(&level) {
        Some(level)
    } else {
        None
    }
}

fn style_attr(element: &scraper::node::Element) -> &str {
    element.attr("style").unwrap_or_default()
}

fn merge_paragraph_attrs(element: &scraper::node::Element, extra: Option<Value>) -> Option<Value> {
    let mut attrs = paragraph_attrs(&parse_declarations(style_attr(element)))
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    if let Some(align) = element.attr("align") {
        if matches!(align, "left" | "right" | "center" | "justify") && align != "left" {
            attrs.insert("textAlign".to_owned(), json!(align));
        }
    }

    if let Some(Value::Object(extra)) = extra {
        for (key, value) in extra {
            attrs.insert(key, value);
        }
    }

    if attrs.is_empty() {
        None
    } else {
        Some(Value::Object(attrs))
    }
}

fn span(element: &scraper::node::Element, name: &str) -> u64 {
    element
        .attr(name)
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(1)
        .clamp(1, 1000)
}

fn has_block_children(node: NodeRef<'_, Node>) -> bool {
    node.children().any(|child| {
        child
            .value()
            .as_element()
            .is_some_and(|element| is_block_tag(element.name()))
    })
}

fn descendants<'a>(node: NodeRef<'a, Node>, name: &str) -> Vec<NodeRef<'a, Node>> {
    let mut found = Vec::new();

    for child in node.children() {
        if child
            .value()
            .as_element()
            .is_some_and(|element| element.name() == name)
        {
            found.push(child);
        } else if child.value().is_element() {
            found.extend(descendants(child, name));
        }
    }

    found
}

fn raw_text(node: NodeRef<'_, Node>) -> String {
    let mut text = String::new();

    for child in node.children() {
        match child.value() {
            Node::Text(value) => text.push_str(value),
            Node::Element(element) if element.name() == "br" => text.push('\n'),
            Node::Element(_) | Node::Document | Node::Fragment => {
                text.push_str(&raw_text(child));
            }
            Node::Comment(_) | Node::Doctype(_) | Node::ProcessingInstruction(_) => {}
        }
    }

    text
}

fn is_supported_image_source(source: &str) -> bool {
    let lowered = source.trim().to_ascii_lowercase();

    lowered.starts_with("asset:")
        || lowered.starts_with("data:image/png;")
        || lowered.starts_with("data:image/jpeg;")
        || lowered.starts_with("data:image/jpg;")
        || lowered.starts_with("data:image/gif;")
        || lowered.starts_with("data:image/webp;")
}

fn ensure_leading_paragraph(blocks: &mut Vec<PmNode>) {
    if blocks
        .first()
        .map(|node| node.node_type.as_str())
        .is_none_or(|node_type| node_type != "paragraph")
    {
        blocks.insert(0, PmNode::paragraph());
    }
}

fn atom(node_type: &str) -> PmNode {
    PmNode::with_content(node_type, Vec::new())
}

fn push_text(output: &mut Vec<PmNode>, text: &str, marks: &[PmMark]) {
    let text = normalize_text(text);

    if text.is_empty() {
        return;
    }

    if let Some(last) = output.last_mut() {
        if last.node_type == "text" && last.marks == marks {
            if let Some(existing) = last.text.as_mut() {
                existing.push_str(&text);
                return;
            }
        }
    }

    output.push(PmNode {
        node_type: "text".to_owned(),
        attrs: None,
        content: Vec::new(),
        marks: marks.to_vec(),
        text: Some(text),
    });
}

fn normalize_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut whitespace = false;

    for character in text.chars() {
        if character.is_whitespace() {
            whitespace = true;
            continue;
        }

        if whitespace {
            output.push(' ');
            whitespace = false;
        }

        output.push(character);
    }

    if whitespace && !output.is_empty() {
        output.push(' ');
    }

    output
}

fn trim_inline(content: &mut Vec<PmNode>) {
    if let Some(first) = content.first_mut() {
        if first.node_type == "text" {
            if let Some(text) = first.text.as_mut() {
                let trimmed = text.trim_start().to_owned();
                *text = trimmed;
            }
        }
    }

    if let Some(last) = content.last_mut() {
        if last.node_type == "text" {
            if let Some(text) = last.text.as_mut() {
                let trimmed = text.trim_end().to_owned();
                *text = trimmed;
            }
        }
    }

    content.retain(|node| {
        node.node_type != "text" || node.text.as_deref().is_some_and(|text| !text.is_empty())
    });
}

fn flush_inline(pending: &mut Vec<PmNode>, blocks: &mut Vec<PmNode>) {
    if pending.is_empty() {
        return;
    }

    trim_inline(pending);
    let content = std::mem::take(pending);

    if !content.is_empty() {
        blocks.push(PmNode::with_content("paragraph", content));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(source: &str) -> ImportedDocument {
        read_document(source).expect("чтение HTML")
    }

    fn types(document: &PmDoc) -> Vec<&str> {
        document
            .content
            .iter()
            .map(|node| node.node_type.as_str())
            .collect()
    }

    #[test]
    fn parses_blocks_and_inline_marks() {
        let imported = read(
            r#"<h1>Заголовок</h1>
            <p>Текст <strong>жирный</strong> и <em>курсив</em>, <code>код</code>.</p>
            <blockquote><p>Цитата</p></blockquote>
            <hr>
            <div data-page-break="true"></div>"#,
        );

        assert_eq!(
            types(&imported.document),
            vec![
                "heading",
                "paragraph",
                "blockquote",
                "horizontalRule",
                "pageBreak"
            ]
        );

        let marks: Vec<&str> = imported.document.content[1]
            .content
            .iter()
            .flat_map(|node| node.marks.iter())
            .map(|mark| mark.mark_type.as_str())
            .collect();
        assert!(marks.contains(&"bold"));
        assert!(marks.contains(&"italic"));
        assert!(marks.contains(&"code"));
        assert!(imported.warnings.is_empty());
    }

    #[test]
    fn parses_lists_and_tasks() {
        let imported = read(
            r#"<ul><li>Первый</li><li>Второй<ul><li>Вложенный</li></ul></li></ul>
            <ol start="3"><li>Третий</li></ol>
            <ul data-type="taskList">
              <li data-type="taskItem" data-checked="true"><p>Сделано</p></li>
              <li data-type="taskItem" data-checked="false">Не сделано</li>
            </ul>"#,
        );

        assert_eq!(
            types(&imported.document),
            vec!["bulletList", "orderedList", "taskList"]
        );
        assert_eq!(
            imported.document.content[1].attrs.as_ref().unwrap()["start"],
            3
        );
        assert_eq!(
            imported.document.content[2].content[0]
                .attrs
                .as_ref()
                .unwrap()["checked"],
            true
        );
        assert_eq!(
            imported.document.content[2].content[1]
                .attrs
                .as_ref()
                .unwrap()["checked"],
            false
        );
    }

    #[test]
    fn parses_table_cells_with_spans() {
        let imported = read(
            r#"<table>
                <thead><tr><th colspan="2">Заголовок</th></tr></thead>
                <tbody><tr><td rowspan="2">A</td><td>B</td></tr><tr><td>C</td></tr></tbody>
            </table>"#,
        );

        assert_eq!(types(&imported.document), vec!["table"]);
        let rows = &imported.document.content[0].content;
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].content[0].node_type, "tableHeader");
        assert_eq!(rows[0].content[0].attrs.as_ref().unwrap()["colspan"], 2);
        assert_eq!(rows[1].content[0].attrs.as_ref().unwrap()["rowspan"], 2);
    }

    #[test]
    fn parses_paragraph_and_text_styles() {
        let imported = read(
            r#"<p style="text-align: center; margin-left: 2em; line-height: 1.5; margin-bottom: 4pt">
                <span style="font-family: Georgia; font-size: 14pt; color: #ff0000">Текст</span>
            </p>"#,
        );
        let paragraph = &imported.document.content[0];

        assert_eq!(paragraph.attrs.as_ref().unwrap()["textAlign"], "center");
        assert_eq!(paragraph.attrs.as_ref().unwrap()["indent"], 1);
        assert_eq!(paragraph.attrs.as_ref().unwrap()["lineHeight"], "1.5");
        assert_eq!(paragraph.attrs.as_ref().unwrap()["spaceAfter"], 4);

        let mark = &paragraph.content[0].marks[0];
        assert_eq!(mark.mark_type, "textStyle");
        assert_eq!(mark.attrs.as_ref().unwrap()["fontFamily"], "Georgia");
    }

    #[test]
    fn sanitizes_dangerous_markup() {
        let imported = read(
            r#"<html><head><style>body { color: red }</style><script>alert(1)</script></head>
            <body onload="alert(2)">
              <p onclick="alert(3)">Текст <a href="javascript:alert(4)">опасная</a>
                <img src="https://example.com/x.png" alt="внешняя">
                <img src="data:image/png;base64,AAAA" alt="встроенная"></p>
              <iframe src="https://example.com"></iframe>
              <form><input value="x"></form>
            </body></html>"#,
        );

        assert_eq!(types(&imported.document), vec!["paragraph"]);
        assert!(!imported.warnings.is_empty());

        let paragraph = &imported.document.content[0];
        let text: String = paragraph
            .content
            .iter()
            .filter_map(|node| node.text.clone())
            .collect();
        assert_eq!(text, "Текст опасная");

        let images: Vec<_> = paragraph
            .content
            .iter()
            .filter(|node| node.node_type == "image")
            .collect();
        assert_eq!(images.len(), 1);
        assert_eq!(
            images[0].attrs.as_ref().unwrap()["src"],
            "data:image/png;base64,AAAA"
        );

        let json = serde_json::to_string(&imported.document).expect("json");
        assert!(!json.contains("javascript:"));
        assert!(!json.contains("onclick"));
        assert!(!json.contains("iframe"));
    }

    #[test]
    fn rejects_too_deep_markup() {
        let mut source = String::new();
        for _ in 0..300 {
            source.push_str("<div>");
        }
        source.push_str("текст");
        for _ in 0..300 {
            source.push_str("</div>");
        }

        let error = read_document(&source).expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("глубина"));
    }

    #[test]
    fn empty_document_keeps_paragraph() {
        let imported = read("");

        assert_eq!(types(&imported.document), vec!["paragraph"]);
        assert!(imported.document.content[0].content.is_empty());
    }
}
