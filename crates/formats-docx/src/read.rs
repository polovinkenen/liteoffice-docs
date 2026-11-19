use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use docx_rs::{
    DocumentChild, Docx, DrawingData, HyperlinkData, Paragraph, ParagraphChild, ReadDocxOptions,
    RunChild, TableCellContent, TableChild, TableRowChild,
};
use serde_json::{json, Map, Value};
use wlite_docmodel::{PmDoc, PmMark, PmNode, MAX_DEPTH, MAX_NODES};
use wlite_formats_core::{is_safe_link, FormatError, ImportedDocument, WarningSink};

use crate::units::{
    canonical_font_family, font_size_points, format_color, line_height_from_twips, parse_color,
    value_bool, value_string, value_u64, DEFAULT_FONT_NAME, DEFAULT_FONT_SIZE_HALF_POINTS,
    TWIPS_PER_LEVEL, TWIPS_PER_POINT,
};

const MAX_IMAGE_BYTES: usize = 15 * 1024 * 1024;

#[derive(Clone, Copy)]
pub(crate) struct ReadLimits {
    pub(crate) max_nodes: usize,
    pub(crate) max_depth: usize,
}

impl Default for ReadLimits {
    fn default() -> Self {
        Self {
            max_nodes: MAX_NODES,
            max_depth: MAX_DEPTH,
        }
    }
}

pub fn read_document(bytes: &[u8]) -> Result<ImportedDocument, FormatError> {
    read_document_with_limits(bytes, ReadLimits::default())
}

pub(crate) fn read_document_with_limits(
    bytes: &[u8],
    limits: ReadLimits,
) -> Result<ImportedDocument, FormatError> {
    let docx = docx_rs::read_docx_with_options(
        bytes,
        ReadDocxOptions::default().with_image_previews(false),
    )
    .map_err(|error| FormatError::Parse(format!("не удалось прочитать DOCX: {error:?}")))?;

    let mut context = ReadContext::new(&docx, limits);
    context.warn_headers(&docx);

    let mut blocks = Vec::new();

    for child in &docx.document.children {
        context.convert_document_child(child, &mut blocks, 1)?;
    }

    let content = finalize_blocks(blocks);

    Ok(ImportedDocument {
        document: PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: if content.is_empty() {
                vec![PmNode::paragraph()]
            } else {
                content
            },
        },
        warnings: context.warnings.into_warnings(),
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Ordered,
    Task { checked: bool },
}

struct ListMeta {
    kind: ListKind,
    level: u16,
}

#[derive(Clone, Copy)]
struct NumberLevel {
    ordered: bool,
}

struct ImageData {
    mime: &'static str,
    bytes: Vec<u8>,
}

enum Draft {
    Paragraph {
        node: PmNode,
        list: Option<ListMeta>,
    },
    Atom(PmNode),
    Table(PmNode),
}

struct ReadContext {
    headings: HashMap<String, u8>,
    images: HashMap<String, ImageData>,
    links: HashMap<String, String>,
    numberings: HashMap<usize, Vec<NumberLevel>>,
    warnings: WarningSink,
    nodes: usize,
    limits: ReadLimits,
}

impl ReadContext {
    fn new(docx: &Docx, limits: ReadLimits) -> Self {
        let mut headings = HashMap::new();

        for style in &docx.styles.styles {
            let level = style
                .paragraph_property
                .outline_lvl
                .as_ref()
                .map(|level| (level.v + 1).clamp(1, 6) as u8)
                .or_else(|| {
                    serde_json::to_value(&style.name)
                        .ok()
                        .and_then(|value| value.as_str().map(str::to_owned))
                        .and_then(|name| heading_from_name(&name))
                });

            if let Some(level) = level {
                headings.insert(style.style_id.clone(), level);
            }
        }

        let mut images = HashMap::new();

        for (rid, path, image, _) in &docx.images {
            if let Some(mime) = image_mime(path) {
                images.insert(
                    rid.clone(),
                    ImageData {
                        mime,
                        bytes: image.0.clone(),
                    },
                );
            }
        }

        let links = docx
            .hyperlinks
            .iter()
            .map(|(rid, url, _)| (rid.clone(), url.clone()))
            .collect();

        let mut abstract_levels = HashMap::new();

        for abstract_num in &docx.numberings.abstract_nums {
            let levels = abstract_num
                .levels
                .iter()
                .map(|level| NumberLevel {
                    ordered: level.format.val != "bullet" && level.format.val != "none",
                })
                .collect::<Vec<_>>();

            abstract_levels.insert(abstract_num.id, levels);
        }

        let mut numberings = HashMap::new();

        for numbering in &docx.numberings.numberings {
            if let Some(levels) = abstract_levels.get(&numbering.abstract_num_id) {
                numberings.insert(numbering.id, levels.clone());
            }
        }

        Self {
            headings,
            images,
            links,
            numberings,
            warnings: WarningSink::new(),
            nodes: 0,
            limits,
        }
    }

    fn enter(&mut self, depth: usize) -> Result<(), FormatError> {
        if depth > self.limits.max_depth {
            return Err(FormatError::Parse(format!(
                "превышена максимальная глубина вложенности DOCX {}",
                self.limits.max_depth
            )));
        }

        self.nodes += 1;

        if self.nodes > self.limits.max_nodes {
            return Err(FormatError::Parse(format!(
                "превышено максимальное число узлов DOCX {}",
                self.limits.max_nodes
            )));
        }

        Ok(())
    }

    fn warn_headers(&mut self, docx: &Docx) {
        let section = &docx.document.section_property;

        if section.header.is_some()
            || section.first_header.is_some()
            || section.even_header.is_some()
            || section.footer.is_some()
            || section.first_footer.is_some()
            || section.even_footer.is_some()
        {
            self.warnings
                .warn("Колонтитулы не поддерживаются моделью документа и были пропущены.");
        }
    }

    fn convert_document_child(
        &mut self,
        child: &DocumentChild,
        blocks: &mut Vec<Draft>,
        depth: usize,
    ) -> Result<(), FormatError> {
        match child {
            DocumentChild::Paragraph(paragraph) => {
                blocks.extend(self.convert_paragraph(paragraph, depth)?);
            }
            DocumentChild::Table(table) => {
                if let Some(node) = self.convert_table(table, depth)? {
                    blocks.push(Draft::Table(node));
                }
            }
            DocumentChild::Section(section) => {
                self.warnings
                    .warn("Разделы DOCX упрощаются до обычных абзацев.");
                for child in section.children() {
                    match child {
                        docx_rs::SectionChild::Paragraph(paragraph) => {
                            blocks.extend(self.convert_paragraph(paragraph.as_ref(), depth + 1)?);
                        }
                        docx_rs::SectionChild::Table(table) => {
                            if let Some(node) = self.convert_table(table.as_ref(), depth + 1)? {
                                blocks.push(Draft::Table(node));
                            }
                        }
                        _ => {}
                    }
                }
            }
            DocumentChild::StructuredDataTag(_) => {
                self.warnings.warn(
                    "Элементы управления содержимым DOCX не поддерживаются и были пропущены.",
                );
            }
            DocumentChild::TableOfContents(_) => {
                self.warnings
                    .warn("Оглавление DOCX не поддерживается и было пропущено.");
            }
            DocumentChild::BookmarkStart(_)
            | DocumentChild::BookmarkEnd(_)
            | DocumentChild::CommentStart(_)
            | DocumentChild::CommentEnd(_) => {}
        }

        Ok(())
    }

    fn convert_paragraph(
        &mut self,
        paragraph: &Paragraph,
        depth: usize,
    ) -> Result<Vec<Draft>, FormatError> {
        self.enter(depth)?;
        let property = serde_json::to_value(&paragraph.property).unwrap_or(Value::Null);
        let heading = self.heading_level(&property);
        let list_meta = self.list_meta(&property);
        let mut content = Vec::new();
        let mut page_break = false;

        for child in &paragraph.children {
            self.convert_paragraph_child(child, &[], &mut content, &mut page_break, depth + 1)?;
        }

        trim_nodes(&mut content);
        let mut drafts = Vec::new();

        if property.get("pageBreakBefore").is_some_and(value_bool) {
            drafts.push(Draft::Atom(atom("pageBreak")));
        }

        if content.is_empty() && page_break {
            drafts.push(Draft::Atom(atom("pageBreak")));
            return Ok(drafts);
        }

        if content.is_empty() && has_bottom_border(&property) {
            drafts.push(Draft::Atom(atom("horizontalRule")));
            return Ok(drafts);
        }

        let list = list_meta.map(|mut meta| {
            if matches!(meta.kind, ListKind::Bullet) {
                if let Some(checked) = strip_task_marker(&mut content) {
                    meta.kind = ListKind::Task { checked };
                }
            }
            meta
        });
        let attrs = paragraph_attrs(&property, list.is_some());
        let node = if let Some(level) = heading {
            let mut node = PmNode::with_content("heading", content);
            node.attrs = merge_attrs(Some(json!({ "level": level })), attrs);
            node
        } else {
            let mut node = PmNode::with_content("paragraph", content);
            node.attrs = attrs;
            node
        };

        drafts.push(Draft::Paragraph { node, list });

        if page_break {
            drafts.push(Draft::Atom(atom("pageBreak")));
        }

        Ok(drafts)
    }

    fn convert_paragraph_child(
        &mut self,
        child: &ParagraphChild,
        marks: &[PmMark],
        content: &mut Vec<PmNode>,
        page_break: &mut bool,
        depth: usize,
    ) -> Result<(), FormatError> {
        match child {
            ParagraphChild::Run(run) => {
                self.convert_run(run, marks, content, page_break, depth)?;
            }
            ParagraphChild::Hyperlink(hyperlink) => {
                let link = match &hyperlink.link {
                    HyperlinkData::External { rid, path } => {
                        self.links.get(rid).cloned().or_else(|| {
                            if path.is_empty() {
                                None
                            } else {
                                Some(path.clone())
                            }
                        })
                    }
                    HyperlinkData::Anchor { anchor } => Some(format!("#{anchor}")),
                };
                let mut nested = marks.to_vec();

                if let Some(link) = link {
                    if is_safe_link(&link) {
                        let mut mark = PmMark::new("link");
                        mark.attrs = Some(json!({
                            "href": link,
                            "target": Value::Null,
                            "rel": Value::Null,
                            "class": Value::Null
                        }));
                        nested.push(mark);
                    } else {
                        self.warnings
                            .warn("Ссылка с небезопасным адресом была преобразована в текст.");
                    }
                }

                for child in &hyperlink.children {
                    self.convert_paragraph_child(child, &nested, content, page_break, depth + 1)?;
                }
            }
            ParagraphChild::Insert(_)
            | ParagraphChild::Delete(_)
            | ParagraphChild::MoveFrom(_)
            | ParagraphChild::MoveTo(_) => {
                self.warnings.warn(
                    "Рецензирование DOCX (вставки, удаления, перемещения) не поддерживается.",
                );
            }
            ParagraphChild::StructuredDataTag(_) => {
                self.warnings.warn(
                    "Элементы управления содержимым DOCX не поддерживаются и были пропущены.",
                );
            }
            ParagraphChild::PageNum(_) | ParagraphChild::NumPages(_) => {
                self.warnings.warn("Поля PAGE/NUMPAGES не поддерживаются.");
            }
            ParagraphChild::BookmarkStart(_)
            | ParagraphChild::BookmarkEnd(_)
            | ParagraphChild::CommentStart(_)
            | ParagraphChild::CommentEnd(_) => {}
        }

        Ok(())
    }

    fn convert_run(
        &mut self,
        run: &docx_rs::Run,
        marks: &[PmMark],
        content: &mut Vec<PmNode>,
        page_break: &mut bool,
        depth: usize,
    ) -> Result<(), FormatError> {
        let property = serde_json::to_value(&run.run_property).unwrap_or(Value::Null);
        let mut marks = marks.to_vec();

        for mark in self.run_marks(&property) {
            marks.push(mark);
        }

        for child in &run.children {
            match child {
                RunChild::Text(text) => {
                    self.enter(depth)?;
                    push_text(content, &text.text, &marks);
                }
                RunChild::Tab(_) | RunChild::PTab(_) => {
                    self.enter(depth)?;
                    push_text(content, "\t", &marks);
                }
                RunChild::Break(br) => {
                    self.enter(depth)?;
                    let break_type = serde_json::to_value(br)
                        .ok()
                        .and_then(|value| {
                            value
                                .get("breakType")
                                .and_then(|value| value.as_str())
                                .map(str::to_owned)
                        })
                        .unwrap_or_default();

                    match break_type.as_str() {
                        "page" => *page_break = true,
                        "column" => self.warnings.warn("Разрывы колонок не поддерживаются."),
                        _ => content.push(atom("hardBreak")),
                    }
                }
                RunChild::CarriageReturn(_) => {
                    self.enter(depth)?;
                    content.push(atom("hardBreak"));
                }
                RunChild::Drawing(drawing) => {
                    if let Some(DrawingData::Pic(pic)) = &drawing.data {
                        if let Some(image) = self.image_node(&pic.id) {
                            self.enter(depth)?;
                            content.push(image);
                        }
                    } else {
                        self.warnings
                            .warn("Текстовые поля и фигуры DOCX не поддерживаются.");
                    }
                }
                RunChild::Shape(_) => {
                    self.warnings
                        .warn("Фигуры и текстовые поля DOCX не поддерживаются.");
                }
                RunChild::Sym(sym) => {
                    if let Ok(value) = u32::from_str_radix(&sym.char, 16) {
                        if let Some(character) = char::from_u32(value) {
                            self.enter(depth)?;
                            push_text(content, &character.to_string(), &marks);
                        }
                    }
                }
                RunChild::FieldChar(_)
                | RunChild::InstrText(_)
                | RunChild::InstrTextString(_)
                | RunChild::DeleteInstrText(_) => {
                    self.warnings.warn("Поля DOCX не поддерживаются.");
                }
                RunChild::FootnoteReference(_) => {
                    self.warnings
                        .warn("Сноски DOCX не поддерживаются и были пропущены.");
                }
                RunChild::DeleteText(_) => {
                    self.warnings
                        .warn("Удалённый текст рецензирования пропущен.");
                }
                RunChild::Shading(_) | RunChild::CommentStart(_) | RunChild::CommentEnd(_) => {}
            }
        }

        Ok(())
    }

    fn run_marks(&self, property: &Value) -> Vec<PmMark> {
        let mut marks = Vec::new();

        if property.get("bold").is_some_and(value_bool) {
            marks.push(PmMark::new("bold"));
        }
        if property.get("italic").is_some_and(value_bool) {
            marks.push(PmMark::new("italic"));
        }
        if property
            .get("underline")
            .and_then(value_string)
            .is_some_and(|value| value != "none")
        {
            marks.push(PmMark::new("underline"));
        }
        if property.get("strike").is_some_and(value_bool) {
            marks.push(PmMark::new("strike"));
        }

        let mut text_style = Map::new();

        if let Some(font) = property
            .get("fonts")
            .and_then(|fonts| fonts.get("ascii").or_else(|| fonts.get("hiAnsi")))
            .and_then(value_string)
        {
            if !font.eq_ignore_ascii_case(DEFAULT_FONT_NAME) {
                text_style.insert("fontFamily".to_owned(), json!(canonical_font_family(font)));
            }
        }

        if let Some(size) = property.get("sz").and_then(value_u64) {
            if size != DEFAULT_FONT_SIZE_HALF_POINTS as u64 && size <= u16::MAX as u64 {
                text_style.insert("fontSize".to_owned(), json!(font_size_points(size as u16)));
            }
        }

        if let Some(color) = property
            .get("color")
            .and_then(color_value)
            .and_then(|color| parse_color(&color))
            .filter(|color| *color != (0, 0, 0))
        {
            text_style.insert("color".to_owned(), json!(format_color(color)));
        }

        if !text_style.is_empty() {
            let mut mark = PmMark::new("textStyle");
            mark.attrs = Some(Value::Object(text_style));
            marks.push(mark);
        }

        let highlight = property
            .get("shading")
            .and_then(|shading| shading.get("fill"))
            .and_then(value_string)
            .filter(|fill| !fill.eq_ignore_ascii_case("auto"))
            .and_then(parse_color)
            .or_else(|| {
                property
                    .get("highlight")
                    .and_then(value_string)
                    .and_then(highlight_color)
            });

        if let Some(highlight) = highlight.filter(|color| *color != (0, 0, 0)) {
            let mut mark = PmMark::new("highlight");
            mark.attrs = Some(json!({ "color": format_color(highlight) }));
            marks.push(mark);
        }

        marks
    }

    fn image_node(&mut self, rid: &str) -> Option<PmNode> {
        let image = self.images.get(rid)?;

        if image.bytes.len() > MAX_IMAGE_BYTES {
            self.warnings
                .warn("Изображение DOCX слишком большое и было пропущено (лимит 15 МБ).");
            return None;
        }

        let encoded = STANDARD.encode(&image.bytes);
        let mut node = atom("image");
        node.attrs = Some(json!({
            "src": format!("data:{};base64,{encoded}", image.mime),
            "alt": Value::Null,
            "title": Value::Null
        }));
        Some(node)
    }

    fn heading_level(&self, property: &Value) -> Option<u8> {
        if let Some(level) = property.get("outlineLvl").and_then(value_u64) {
            return Some((level + 1).clamp(1, 6) as u8);
        }

        property
            .get("style")
            .and_then(value_string)
            .and_then(|style| self.headings.get(style).copied())
    }

    fn list_meta(&mut self, property: &Value) -> Option<ListMeta> {
        let numbering = property.get("numberingProperty")?;
        let id = numbering.get("id").and_then(value_u64)? as usize;
        let level = numbering.get("level").and_then(value_u64).unwrap_or(0) as u16;
        let levels = self.numberings.get(&id);
        let number = levels.and_then(|levels| levels.get(level as usize));

        if levels.is_none() {
            self.warnings
                .warn("Не удалось определить тип списка DOCX; используется маркированный.");
        }

        let kind = match number {
            Some(number) if number.ordered => ListKind::Ordered,
            _ => ListKind::Bullet,
        };

        Some(ListMeta { kind, level })
    }

    fn convert_table(
        &mut self,
        table: &docx_rs::Table,
        depth: usize,
    ) -> Result<Option<PmNode>, FormatError> {
        self.enter(depth)?;
        let mut rows: Vec<Vec<PmNode>> = Vec::new();
        let mut merges: HashMap<usize, (usize, usize)> = HashMap::new();

        for row in &table.rows {
            let TableChild::TableRow(row) = row;
            self.enter(depth + 1)?;
            let mut cells = Vec::new();
            let mut column = 0;

            for cell in &row.cells {
                let TableRowChild::TableCell(cell) = cell;
                self.enter(depth + 2)?;
                let property = serde_json::to_value(&cell.property).unwrap_or(Value::Null);
                let colspan = property
                    .get("gridSpan")
                    .and_then(value_u64)
                    .unwrap_or(1)
                    .clamp(1, 100) as usize;
                let merge = property
                    .get("verticalMerge")
                    .and_then(value_string)
                    .unwrap_or_default();

                if colspan > 1 && merge == "restart" {
                    self.warnings.warn(
                        "Объединение ячейки одновременно по строкам и столбцам упрощается при импорте DOCX.",
                    );
                }

                if merge == "continue" {
                    if let Some((row_index, cell_index)) = merges.get(&column) {
                        if let Some(target) = rows
                            .get_mut(*row_index)
                            .and_then(|row| row.get_mut(*cell_index))
                        {
                            increment_rowspan(target);
                        }
                    }
                    column += colspan;
                    continue;
                }

                let mut content = Vec::new();

                for child in &cell.children {
                    match child {
                        TableCellContent::Paragraph(paragraph) => {
                            for draft in self.convert_paragraph(paragraph, depth + 3)? {
                                match draft {
                                    Draft::Paragraph { node, .. } | Draft::Atom(node) => {
                                        content.push(node)
                                    }
                                    Draft::Table(_) => self.warnings.warn(
                                        "Вложенные таблицы DOCX не поддерживаются.",
                                    ),
                                }
                            }
                        }
                        _ => self.warnings.warn(
                            "Вложенные таблицы и элементы управления в ячейках DOCX не поддерживаются.",
                        ),
                    }
                }

                if content.is_empty() {
                    content.push(PmNode::paragraph());
                }

                let mut node = PmNode::with_content(
                    if rows.is_empty() {
                        "tableHeader"
                    } else {
                        "tableCell"
                    },
                    content,
                );
                node.attrs = Some(json!({
                    "colspan": colspan,
                    "rowspan": 1,
                    "colwidth": Value::Null
                }));

                let cell_index = cells.len();
                cells.push(node);

                if merge == "restart" {
                    for offset in 0..colspan {
                        merges.insert(column + offset, (rows.len(), cell_index));
                    }
                }

                column += colspan;
            }

            if !cells.is_empty() {
                rows.push(cells);
            }
        }

        if rows.is_empty() {
            self.warnings.warn("Пустая таблица DOCX была пропущена.");
            return Ok(None);
        }

        Ok(Some(PmNode::with_content(
            "table",
            rows.into_iter()
                .map(|cells| PmNode::with_content("tableRow", cells))
                .collect(),
        )))
    }
}

fn heading_from_name(name: &str) -> Option<u8> {
    let lowered = name.trim().to_ascii_lowercase();
    let level = lowered
        .strip_prefix("heading ")
        .or_else(|| lowered.strip_prefix("заголовок "))?;
    let level: u8 = level.trim().parse().ok()?;

    if (1..=6).contains(&level) {
        Some(level)
    } else {
        None
    }
}

fn image_mime(path: &str) -> Option<&'static str> {
    let extension = path.rsplit('.').next()?.to_ascii_lowercase();

    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn color_value(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.get("val").and_then(value_string).map(str::to_owned))
}

fn highlight_color(value: &str) -> Option<(u8, u8, u8)> {
    let color = match value.to_ascii_lowercase().as_str() {
        "black" => "#000000",
        "blue" => "#0000ff",
        "cyan" => "#00ffff",
        "green" => "#00ff00",
        "magenta" => "#ff00ff",
        "red" => "#ff0000",
        "yellow" => "#ffff00",
        "white" => "#ffffff",
        "darkblue" => "#000080",
        "darkcyan" => "#008080",
        "darkgreen" => "#008000",
        "darkmagenta" => "#800080",
        "darkred" => "#800000",
        "darkyellow" => "#808000",
        "darkgray" | "darkgrey" => "#808080",
        "lightgray" | "lightgrey" => "#c0c0c0",
        _ => return None,
    };

    parse_color(color)
}

fn paragraph_attrs(property: &Value, is_list: bool) -> Option<Value> {
    let mut attrs = Map::new();

    let alignment = property.get("alignment").and_then(value_string);

    match alignment {
        Some("center") => {
            attrs.insert("textAlign".to_owned(), json!("center"));
        }
        Some("right") | Some("end") => {
            attrs.insert("textAlign".to_owned(), json!("right"));
        }
        Some("both") | Some("justify") => {
            attrs.insert("textAlign".to_owned(), json!("justify"));
        }
        _ => {}
    }

    if !is_list {
        if let Some(start) = property
            .get("indent")
            .and_then(|indent| indent.get("start"))
            .and_then(|value| value.as_i64())
            .filter(|start| *start > 0)
        {
            let indent = ((start as f32) / TWIPS_PER_LEVEL as f32)
                .round()
                .clamp(0.0, 8.0) as u64;

            if indent > 0 {
                attrs.insert("indent".to_owned(), json!(indent));
            }
        }
    }

    if let Some(spacing) = property.get("lineSpacing") {
        if let Some(before) = spacing.get("before").and_then(value_u64) {
            if before > 0 {
                attrs.insert(
                    "spaceBefore".to_owned(),
                    json!((before as f32 / TWIPS_PER_POINT as f32).round() as u64),
                );
            }
        }

        if let Some(after) = spacing.get("after").and_then(value_u64) {
            if after > 0 {
                attrs.insert(
                    "spaceAfter".to_owned(),
                    json!((after as f32 / TWIPS_PER_POINT as f32).round() as u64),
                );
            }
        }

        let rule = spacing
            .get("lineRule")
            .and_then(value_string)
            .unwrap_or("auto");

        if rule == "auto" {
            if let Some(line) = spacing.get("line").and_then(|value| value.as_i64()) {
                if let Some(line_height) = line_height_from_twips(line as i32) {
                    attrs.insert("lineHeight".to_owned(), json!(line_height));
                }
            }
        }
    }

    if attrs.is_empty() {
        None
    } else {
        Some(Value::Object(attrs))
    }
}

fn has_bottom_border(property: &Value) -> bool {
    property
        .get("borders")
        .and_then(|borders| borders.get("bottom"))
        .is_some_and(|bottom| !bottom.is_null())
}

fn merge_attrs(base: Option<Value>, extra: Option<Value>) -> Option<Value> {
    match (base, extra) {
        (Some(Value::Object(mut base)), Some(Value::Object(extra))) => {
            for (key, value) in extra {
                base.insert(key, value);
            }
            Some(Value::Object(base))
        }
        (base, None) => base,
        (None, extra) => extra,
        (base, extra) => extra.or(base),
    }
}

fn increment_rowspan(cell: &mut PmNode) {
    let Some(attrs) = cell.attrs.as_mut().and_then(|attrs| attrs.as_object_mut()) else {
        return;
    };
    let rowspan = attrs
        .get("rowspan")
        .and_then(|value| value.as_u64())
        .unwrap_or(1);

    attrs.insert("rowspan".to_owned(), json!(rowspan + 1));
}

fn push_text(content: &mut Vec<PmNode>, text: &str, marks: &[PmMark]) {
    if text.is_empty() {
        return;
    }

    if let Some(PmNode {
        node_type,
        marks: existing,
        text: Some(existing_text),
        ..
    }) = content.last_mut()
    {
        if node_type == "text" && *existing == marks {
            existing_text.push_str(text);
            return;
        }
    }

    content.push(PmNode {
        node_type: "text".to_owned(),
        attrs: None,
        content: Vec::new(),
        marks: marks.to_vec(),
        text: Some(text.to_owned()),
    });
}

fn trim_nodes(content: &mut Vec<PmNode>) {
    if let Some(PmNode {
        node_type,
        text: Some(text),
        ..
    }) = content.first_mut()
    {
        if node_type == "text" {
            *text = text.trim_start().to_owned();
        }
    }

    if let Some(PmNode {
        node_type,
        text: Some(text),
        ..
    }) = content.last_mut()
    {
        if node_type == "text" {
            *text = text.trim_end().to_owned();
        }
    }

    content.retain(|node| {
        node.node_type != "text" || node.text.as_deref().is_some_and(|text| !text.is_empty())
    });
}

fn strip_task_marker(content: &mut [PmNode]) -> Option<bool> {
    let node = content.first_mut()?;

    if node.node_type != "text" {
        return None;
    }

    let text = node.text.as_deref()?;
    let trimmed = text.trim_start();
    let (checked, rest) = trimmed
        .strip_prefix("[x] ")
        .map(|rest| (true, rest))
        .or_else(|| trimmed.strip_prefix("[X] ").map(|rest| (true, rest)))
        .or_else(|| trimmed.strip_prefix("[ ] ").map(|rest| (false, rest)))?;

    node.text = Some(rest.to_owned());

    Some(checked)
}

fn atom(node_type: &str) -> PmNode {
    PmNode::with_content(node_type, Vec::new())
}

fn finalize_blocks(blocks: Vec<Draft>) -> Vec<PmNode> {
    let mut content = Vec::new();
    let mut index = 0;

    while index < blocks.len() {
        match &blocks[index] {
            Draft::Paragraph { list: Some(_), .. } => {
                let mut entries = Vec::new();
                let mut previous_level = 0;

                while index < blocks.len() {
                    let Draft::Paragraph {
                        node,
                        list: Some(meta),
                    } = &blocks[index]
                    else {
                        break;
                    };

                    let level = meta.level.min(previous_level + 1);
                    previous_level = level;
                    entries.push(ListEntry {
                        level,
                        kind: meta.kind,
                        node: node.clone(),
                    });
                    index += 1;
                }

                let mut entry_index = 0;

                while entry_index < entries.len() {
                    let kind = entries[entry_index].kind;
                    content.push(build_list(&entries, &mut entry_index, 0, kind));
                }
            }
            Draft::Paragraph { node, .. } => {
                content.push(node.clone());
                index += 1;
            }
            Draft::Atom(node) => {
                content.push(node.clone());
                index += 1;
            }
            Draft::Table(node) => {
                content.push(node.clone());
                index += 1;
            }
        }
    }

    content
}

#[derive(Clone)]
struct ListEntry {
    level: u16,
    kind: ListKind,
    node: PmNode,
}

fn build_list(entries: &[ListEntry], index: &mut usize, level: u16, kind: ListKind) -> PmNode {
    let list_type = match kind {
        ListKind::Ordered => "orderedList",
        ListKind::Bullet => "bulletList",
        ListKind::Task { .. } => "taskList",
    };
    let mut items = Vec::new();

    while *index < entries.len() {
        let entry = &entries[*index];

        if entry.level != level || !same_family(entry.kind, kind) {
            break;
        }

        *index += 1;

        let mut item_content = vec![entry.node.clone()];

        while *index < entries.len() && entries[*index].level > level {
            let child = &entries[*index];
            let child_level = child.level.min(level + 1);
            let nested = build_list(entries, index, child_level, child.kind);
            item_content.push(nested);
        }

        let item = match kind {
            ListKind::Task { .. } => {
                let checked = matches!(entry.kind, ListKind::Task { checked: true });
                let mut node = PmNode::with_content("taskItem", item_content);
                node.attrs = Some(json!({ "checked": checked }));
                node
            }
            _ => PmNode::with_content("listItem", item_content),
        };

        items.push(item);
    }

    PmNode::with_content(list_type, items)
}

fn same_family(left: ListKind, right: ListKind) -> bool {
    matches!(
        (left, right),
        (ListKind::Ordered, ListKind::Ordered)
            | (ListKind::Bullet, ListKind::Bullet)
            | (ListKind::Task { .. }, ListKind::Task { .. })
    )
}
