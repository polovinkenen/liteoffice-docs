use std::io::Cursor;
use std::panic::{catch_unwind, AssertUnwindSafe};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use docx_rs::{
    AbstractNumbering, AlignmentType, BorderType, BreakType, Docx, IndentLevel, Level, LevelJc,
    LevelText, LineSpacing, LineSpacingType, NumberFormat, Numbering, NumberingId, Numberings,
    Paragraph, ParagraphBorder, ParagraphBorderPosition, ParagraphBorders, Pic, Run, RunFonts,
    Shading, SpecialIndentType, Start, Style, StyleType, Styles, Table, TableCell, TableRow,
    VMergeType,
};
use wlite_docmodel::{PmDoc, PmNode};
use wlite_formats_core::{is_safe_link, FormatError, WarningSink, WrittenFile};

use crate::units::{
    docx_font_name, font_size_half_points, hex_color, line_height_twips, parse_color,
    TWIPS_PER_LEVEL, TWIPS_PER_POINT,
};

const TABLE_COLUMN_TWIPS: usize = 2000;

pub fn write_document(document: &PmDoc) -> Result<WrittenFile, FormatError> {
    let mut context = WriteContext::new();
    context.collect(document);

    let mut docx = Docx::new().styles(build_styles());

    if !context.lists.is_empty() {
        docx = docx.numberings(build_numberings(&context.lists));
    }

    for block in &document.content {
        docx = context.write_block(docx, block);
    }

    let mut buffer = Cursor::new(Vec::new());
    docx.build()
        .pack(&mut buffer)
        .map_err(|error| FormatError::Write(format!("не удалось собрать DOCX: {error}")))?;

    Ok(WrittenFile {
        bytes: buffer.into_inner(),
        warnings: context.warnings.into_warnings(),
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListLevelKind {
    Bullet,
    Ordered,
    Task,
}

#[derive(Clone)]
struct ListDefinition {
    id: usize,
    levels: Vec<ListLevelKind>,
}

struct WriteContext {
    warnings: WarningSink,
    lists: Vec<ListDefinition>,
    list_cursor: usize,
}

impl WriteContext {
    fn new() -> Self {
        Self {
            warnings: WarningSink::new(),
            lists: Vec::new(),
            list_cursor: 0,
        }
    }

    fn collect(&mut self, document: &PmDoc) {
        self.collect_blocks(&document.content);
    }

    fn collect_blocks(&mut self, blocks: &[PmNode]) {
        for block in blocks {
            match block.node_type.as_str() {
                "bulletList" | "orderedList" | "taskList" => {
                    let mut definition = ListDefinition {
                        id: self.lists.len() + 2,
                        levels: Vec::new(),
                    };
                    self.collect_list_levels(block, 0, &mut definition);
                    self.lists.push(definition);
                }
                "blockquote" => self.collect_blocks(&block.content),
                "table" => {
                    for row in &block.content {
                        for cell in &row.content {
                            self.collect_blocks(&cell.content);
                        }
                    }
                }
                _ => self.collect_blocks(&block.content),
            }
        }
    }

    fn collect_list_levels(
        &mut self,
        list: &PmNode,
        level: usize,
        definition: &mut ListDefinition,
    ) {
        let kind = match list.node_type.as_str() {
            "orderedList" => ListLevelKind::Ordered,
            "taskList" => ListLevelKind::Task,
            _ => ListLevelKind::Bullet,
        };

        if definition.levels.len() <= level {
            definition.levels.resize(level + 1, ListLevelKind::Bullet);
        }
        definition.levels[level] = kind;

        for item in &list.content {
            for child in &item.content {
                if is_list(child) {
                    self.collect_list_levels(child, level + 1, definition);
                } else if child.node_type == "blockquote" {
                    self.collect_blocks(&child.content);
                }
            }
        }
    }

    fn write_block(&mut self, docx: Docx, node: &PmNode) -> Docx {
        match node.node_type.as_str() {
            "paragraph" => docx.add_paragraph(self.build_paragraph(node, None, None)),
            "heading" => docx.add_paragraph(self.build_paragraph(node, None, None)),
            "bulletList" | "orderedList" | "taskList" => self.write_top_list(docx, node),
            "table" => docx.add_table(self.build_table(node)),
            "pageBreak" => {
                docx.add_paragraph(Paragraph::new().add_run(Run::new().add_break(BreakType::Page)))
            }
            "horizontalRule" => docx.add_paragraph(self.build_horizontal_rule()),
            "image" => {
                let mut paragraph = Paragraph::new();
                paragraph = self.add_inline(paragraph, node);
                docx.add_paragraph(paragraph)
            }
            "blockquote" => {
                self.warnings
                    .warn("Цитаты сохраняются в DOCX как обычные абзацы.");
                let mut docx = docx;

                for child in &node.content {
                    docx = self.write_block(docx, child);
                }

                docx
            }
            "codeBlock" => {
                self.warnings
                    .warn("Блок кода сохраняется в DOCX как текст с моноширинным шрифтом.");
                docx.add_paragraph(self.build_code_block(node))
            }
            _ => {
                self.warnings
                    .warn(format!("Узел «{}» не поддерживается DOCX.", node.node_type));
                let mut paragraph = Paragraph::new();
                let text = wlite_formats_core::node_text(node);

                if !text.is_empty() {
                    paragraph = paragraph.add_run(Run::new().add_text(text));
                }

                docx.add_paragraph(paragraph)
            }
        }
    }

    fn write_top_list(&mut self, mut docx: Docx, node: &PmNode) -> Docx {
        let definition = self.lists.get(self.list_cursor).cloned();
        self.list_cursor += 1;

        let Some(definition) = definition else {
            self.warnings
                .warn("Не удалось описать список в DOCX, содержимое сохранено абзацами.");
            for item in &node.content {
                for child in &item.content {
                    docx = self.write_block(docx, child);
                }
            }
            return docx;
        };

        self.write_list(docx, node, &definition, 0)
    }

    fn write_list(
        &mut self,
        mut docx: Docx,
        node: &PmNode,
        definition: &ListDefinition,
        level: usize,
    ) -> Docx {
        let task = matches!(list_kind(node), ListLevelKind::Task);

        for item in &node.content {
            let checked = item
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("checked"))
                .and_then(|checked| checked.as_bool())
                .unwrap_or(false);
            let mut first = true;

            for child in &item.content {
                if is_list(child) {
                    docx = self.write_list(docx, child, definition, level + 1);
                } else if child.node_type == "blockquote" {
                    for nested in &child.content {
                        docx = self.write_block(docx, nested);
                    }
                } else if child.node_type == "paragraph" || child.node_type == "heading" {
                    let prefix = if first && task {
                        Some(if checked { "[x] " } else { "[ ] " })
                    } else {
                        None
                    };
                    let list = if first {
                        Some((definition, level))
                    } else {
                        None
                    };

                    docx = docx.add_paragraph(self.build_paragraph(child, list, prefix));
                    first = false;
                } else {
                    docx = self.write_block(docx, child);
                }
            }

            if first {
                docx = docx.add_paragraph(self.build_paragraph(
                    &PmNode::paragraph(),
                    Some((definition, level)),
                    if task {
                        Some(if checked { "[x]" } else { "[ ]" })
                    } else {
                        None
                    },
                ));
            }
        }

        docx
    }

    fn build_paragraph(
        &mut self,
        node: &PmNode,
        list: Option<(&ListDefinition, usize)>,
        prefix: Option<&str>,
    ) -> Paragraph {
        let mut paragraph = Paragraph::new();
        let attrs = node.attrs.as_ref().and_then(|attrs| attrs.as_object());
        let align = attrs
            .and_then(|attrs| attrs.get("textAlign"))
            .and_then(|value| value.as_str())
            .unwrap_or("left");

        paragraph = match align {
            "center" => paragraph.align(AlignmentType::Center),
            "right" => paragraph.align(AlignmentType::Right),
            "justify" => paragraph.align(AlignmentType::Both),
            _ => paragraph,
        };

        if let Some((definition, level)) = list {
            paragraph =
                paragraph.numbering(NumberingId::new(definition.id), IndentLevel::new(level));
        } else if let Some(indent) = attrs
            .and_then(|attrs| attrs.get("indent"))
            .and_then(|value| value.as_u64())
            .filter(|indent| *indent > 0)
        {
            paragraph = paragraph.indent(Some(TWIPS_PER_LEVEL * indent as i32), None, None, None);
        }

        let space_before = attrs
            .and_then(|attrs| attrs.get("spaceBefore"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let space_after = attrs
            .and_then(|attrs| attrs.get("spaceAfter"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let line_height = attrs
            .and_then(|attrs| attrs.get("lineHeight"))
            .and_then(|value| value.as_str())
            .and_then(line_height_twips);

        if space_before > 0 || space_after > 0 || line_height.is_some() {
            let mut spacing = LineSpacing::new()
                .before(space_before as u32 * TWIPS_PER_POINT as u32)
                .after(space_after as u32 * TWIPS_PER_POINT as u32);

            if let Some(line_height) = line_height {
                spacing = spacing.line(line_height).line_rule(LineSpacingType::Auto);
            }

            paragraph = paragraph.line_spacing(spacing);
        }

        if node.node_type == "heading" {
            let level = attrs
                .and_then(|attrs| attrs.get("level"))
                .and_then(|level| level.as_u64())
                .unwrap_or(1)
                .clamp(1, 6);

            paragraph = paragraph
                .style(format!("Heading{level}").as_str())
                .outline_lvl(level as usize - 1);
        }

        if let Some(prefix) = prefix {
            paragraph = paragraph.add_run(Run::new().add_text(prefix));
        }

        for child in &node.content {
            paragraph = self.add_inline(paragraph, child);
        }

        paragraph
    }

    fn add_inline(&mut self, paragraph: Paragraph, node: &PmNode) -> Paragraph {
        match node.node_type.as_str() {
            "text" => {
                let text = node.text.as_deref().unwrap_or_default();

                if text.is_empty() {
                    return paragraph;
                }

                let run = self.build_run(node, text);

                if let Some(href) = link_href(node) {
                    if is_safe_link(href) {
                        return paragraph.add_hyperlink(
                            docx_rs::Hyperlink::new(href, docx_rs::HyperlinkType::External)
                                .add_run(run),
                        );
                    }

                    self.warnings
                        .warn("Ссылка с небезопасным адресом не сохраняется в DOCX.");
                }

                paragraph.add_run(run)
            }
            "hardBreak" => paragraph.add_run(Run::new().add_break(BreakType::TextWrapping)),
            "image" => {
                if let Some(pic) = self.build_image(node) {
                    paragraph.add_run(Run::new().add_image(pic))
                } else {
                    paragraph
                }
            }
            _ => {
                self.warnings.warn(format!(
                    "Встроенный узел «{}» не поддерживается DOCX и был упрощён.",
                    node.node_type
                ));
                let mut paragraph = paragraph;

                for child in &node.content {
                    paragraph = self.add_inline(paragraph, child);
                }

                paragraph
            }
        }
    }

    fn build_run(&mut self, node: &PmNode, text: &str) -> Run {
        let mut run = Run::new().add_text(text);
        let mut text_style = serde_json::Map::new();

        for mark in &node.marks {
            match mark.mark_type.as_str() {
                "bold" => run = run.bold(),
                "italic" => run = run.italic(),
                "underline" => run = run.underline("single"),
                "strike" => run = run.strike(),
                "code" => {
                    self.warnings
                        .warn("Фрагменты кода сохраняются в DOCX моноширинным шрифтом.");
                    run = run.fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New"));
                }
                "link" => {}
                "textStyle" => {
                    if let Some(attrs) = mark.attrs.as_ref().and_then(|value| value.as_object()) {
                        for (key, value) in attrs {
                            text_style.insert(key.clone(), value.clone());
                        }
                    }
                }
                "highlight" => {
                    if let Some(color) = mark
                        .attrs
                        .as_ref()
                        .and_then(|attrs| attrs.get("color"))
                        .and_then(|value| value.as_str())
                        .and_then(parse_color)
                    {
                        run = run.shading(Shading::new().fill(hex_color(color)));
                    }
                }
                other => self
                    .warnings
                    .warn(format!("Разметка «{other}» не поддерживается DOCX.")),
            }
        }

        if let Some(font) = text_style
            .get("fontFamily")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            let name = docx_font_name(font);
            run = run.fonts(RunFonts::new().ascii(&name).hi_ansi(&name));
        }

        if let Some(size) = text_style
            .get("fontSize")
            .and_then(|value| value.as_str())
            .and_then(font_size_half_points)
        {
            run = run.size(size as usize);
        }

        if let Some(color) = text_style
            .get("color")
            .and_then(|value| value.as_str())
            .and_then(parse_color)
        {
            run = run.color(hex_color(color));
        }

        run
    }

    fn build_image(&mut self, node: &PmNode) -> Option<Pic> {
        let source = node
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("src"))
            .and_then(|src| src.as_str())?;
        let Some((_mime, bytes)) = decode_image(source) else {
            self.warnings
                .warn("Изображение не сохраняется в DOCX: поддерживаются png, jpeg, gif и webp.");
            return None;
        };

        if bytes.len() > 15 * 1024 * 1024 {
            self.warnings
                .warn("Изображение слишком большое и не сохраняется в DOCX (лимит 15 МБ).");
            return None;
        }

        let result = catch_unwind(AssertUnwindSafe(|| Pic::new(&bytes)));

        match result {
            Ok(pic) => {
                if let Some(alt) = node
                    .attrs
                    .as_ref()
                    .and_then(|attrs| attrs.get("alt"))
                    .and_then(|alt| alt.as_str())
                    .filter(|alt| !alt.is_empty())
                {
                    let _ = alt;
                    self.warnings
                        .warn("Подписи изображений не сохраняются в DOCX.");
                }

                Some(pic)
            }
            Err(_) => {
                self.warnings
                    .warn("Не удалось прочитать изображение для DOCX, оно было пропущено.");
                None
            }
        }
    }

    fn build_table(&mut self, node: &PmNode) -> Table {
        let row_count = node.content.len();
        let columns = node
            .content
            .iter()
            .map(|row| row.content.iter().map(colspan).sum::<usize>())
            .max()
            .unwrap_or(1)
            .max(1);
        let mut occupancy = vec![vec![Coverage::Empty; columns]; row_count];

        for (row_index, row) in node.content.iter().enumerate() {
            let mut column = 0;

            for (cell_index, cell) in row.content.iter().enumerate() {
                if colspan(cell) > 1 && rowspan(cell) > 1 {
                    self.warnings.warn(
                        "Объединение ячейки одновременно по строкам и столбцам упрощается в DOCX.",
                    );
                }

                while column < columns && matches!(occupancy[row_index][column], Coverage::Vertical)
                {
                    column += 1;
                }

                if column >= columns {
                    break;
                }

                let colspan = colspan(cell).min(columns.saturating_sub(column).max(1));
                let rowspan = if colspan > 1 {
                    1
                } else {
                    rowspan(cell).min(row_count.saturating_sub(row_index).max(1))
                };

                for row_offset in 0..rowspan {
                    for column_offset in 0..colspan {
                        let target_row = row_index + row_offset;
                        let target_column = column + column_offset;

                        if target_row >= row_count || target_column >= columns {
                            continue;
                        }

                        occupancy[target_row][target_column] =
                            if row_offset == 0 && column_offset == 0 {
                                Coverage::Origin {
                                    cell_index,
                                    colspan,
                                    rowspan,
                                }
                            } else if row_offset > 0 {
                                Coverage::Vertical
                            } else {
                                Coverage::Horizontal
                            };
                    }
                }

                column += colspan;
            }
        }

        let mut rows = Vec::new();

        for (row_index, row) in node.content.iter().enumerate() {
            let mut cells = Vec::new();

            for coverage in occupancy[row_index].iter().copied() {
                match coverage {
                    Coverage::Origin {
                        cell_index,
                        colspan,
                        rowspan,
                    } => {
                        let cell = row.content.get(cell_index);
                        let mut table_cell = TableCell::new();
                        let paragraphs = cell
                            .map(|cell| self.cell_paragraphs(cell, row_index == 0))
                            .unwrap_or_default();

                        if paragraphs.is_empty() {
                            table_cell = table_cell.add_paragraph(Paragraph::new());
                        } else {
                            for paragraph in paragraphs {
                                table_cell = table_cell.add_paragraph(paragraph);
                            }
                        }

                        if colspan > 1 {
                            table_cell = table_cell.grid_span(colspan);
                        }
                        if rowspan > 1 {
                            table_cell = table_cell.vertical_merge(VMergeType::Restart);
                        }

                        cells.push(table_cell);
                    }
                    Coverage::Vertical => {
                        cells.push(
                            TableCell::new()
                                .add_paragraph(Paragraph::new())
                                .vertical_merge(VMergeType::Continue),
                        );
                    }
                    Coverage::Horizontal | Coverage::Empty => {
                        if matches!(coverage, Coverage::Empty) {
                            cells.push(TableCell::new().add_paragraph(Paragraph::new()));
                        }
                    }
                }
            }

            if cells.is_empty() {
                cells.push(TableCell::new().add_paragraph(Paragraph::new()));
            }

            rows.push(TableRow::new(cells));
        }

        if rows.is_empty() {
            rows.push(TableRow::new(vec![
                TableCell::new().add_paragraph(Paragraph::new())
            ]));
        }

        Table::new(rows).set_grid(vec![TABLE_COLUMN_TWIPS; columns.max(1)])
    }

    fn cell_paragraphs(&mut self, cell: &PmNode, header: bool) -> Vec<Paragraph> {
        let mut paragraphs = Vec::new();

        for child in &cell.content {
            match child.node_type.as_str() {
                "paragraph" | "heading" => {
                    let _ = header;
                    paragraphs.push(self.build_paragraph(child, None, None));
                }
                _ => {
                    self.warnings
                        .warn("Вложенные таблицы и списки внутри ячеек DOCX упрощаются до текста.");
                    let text = wlite_formats_core::node_text(child);
                    let mut paragraph = Paragraph::new();

                    if !text.is_empty() {
                        paragraph = paragraph.add_run(Run::new().add_text(text));
                    }

                    paragraphs.push(paragraph);
                }
            }
        }

        paragraphs
    }

    fn build_code_block(&mut self, node: &PmNode) -> Paragraph {
        let text = wlite_formats_core::node_text(node);
        let mut paragraph = Paragraph::new();
        let lines: Vec<&str> = text.split('\n').collect();

        for (index, line) in lines.iter().enumerate() {
            if index > 0 {
                paragraph = paragraph.add_run(Run::new().add_break(BreakType::TextWrapping));
            }

            paragraph = paragraph.add_run(
                Run::new()
                    .add_text(*line)
                    .fonts(RunFonts::new().ascii("Courier New").hi_ansi("Courier New")),
            );
        }

        paragraph
    }

    fn build_horizontal_rule(&mut self) -> Paragraph {
        let borders = ParagraphBorders::with_empty().set(
            ParagraphBorder::new(ParagraphBorderPosition::Bottom)
                .val(BorderType::Single)
                .size(8)
                .color("auto"),
        );
        let mut paragraph = Paragraph::new();
        paragraph.property = paragraph.property.set_borders(borders);
        paragraph
    }
}

fn link_href(node: &PmNode) -> Option<&str> {
    node.marks
        .iter()
        .find(|mark| mark.mark_type == "link")
        .and_then(|mark| mark.attrs.as_ref())
        .and_then(|attrs| attrs.get("href"))
        .and_then(|href| href.as_str())
        .filter(|href| !href.is_empty())
}

fn is_list(node: &PmNode) -> bool {
    matches!(
        node.node_type.as_str(),
        "bulletList" | "orderedList" | "taskList"
    )
}

fn list_kind(node: &PmNode) -> ListLevelKind {
    match node.node_type.as_str() {
        "orderedList" => ListLevelKind::Ordered,
        "taskList" => ListLevelKind::Task,
        _ => ListLevelKind::Bullet,
    }
}

fn colspan(cell: &PmNode) -> usize {
    cell.attrs
        .as_ref()
        .and_then(|attrs| attrs.get("colspan"))
        .and_then(|value| value.as_u64())
        .unwrap_or(1)
        .clamp(1, 100) as usize
}

fn rowspan(cell: &PmNode) -> usize {
    cell.attrs
        .as_ref()
        .and_then(|attrs| attrs.get("rowspan"))
        .and_then(|value| value.as_u64())
        .unwrap_or(1)
        .clamp(1, 100) as usize
}

fn decode_image(source: &str) -> Option<(&'static str, Vec<u8>)> {
    let (mime, encoded) = source
        .strip_prefix("data:image/png;base64,")
        .map(|encoded| ("image/png", encoded))
        .or_else(|| {
            source
                .strip_prefix("data:image/jpeg;base64,")
                .map(|encoded| ("image/jpeg", encoded))
        })
        .or_else(|| {
            source
                .strip_prefix("data:image/jpg;base64,")
                .map(|encoded| ("image/jpeg", encoded))
        })
        .or_else(|| {
            source
                .strip_prefix("data:image/gif;base64,")
                .map(|encoded| ("image/gif", encoded))
        })
        .or_else(|| {
            source
                .strip_prefix("data:image/webp;base64,")
                .map(|encoded| ("image/webp", encoded))
        })?;

    let bytes = STANDARD.decode(encoded).ok()?;

    Some((mime, bytes))
}

#[derive(Clone, Copy)]
enum Coverage {
    Empty,
    Horizontal,
    Vertical,
    Origin {
        cell_index: usize,
        colspan: usize,
        rowspan: usize,
    },
}

fn build_styles() -> Styles {
    let mut styles = Styles::new();

    for level in 1..=6u8 {
        let size = match level {
            1 => 32,
            2 => 28,
            3 => 26,
            4 => 24,
            5 => 22,
            _ => 20,
        };
        let style = Style::new(format!("Heading{level}"), StyleType::Paragraph)
            .name(format!("Heading {level}"))
            .based_on("Normal")
            .next("Normal")
            .q_format(true)
            .outline_lvl(level as usize - 1)
            .size(size)
            .bold();

        styles = styles.add_style(style);
    }

    styles
}

fn build_numberings(lists: &[ListDefinition]) -> Numberings {
    let mut numberings = Numberings::new();

    for definition in lists {
        let mut abstract_num = AbstractNumbering::new(definition.id);

        for (level, kind) in definition.levels.iter().enumerate() {
            let ordered = matches!(kind, ListLevelKind::Ordered);
            let format = if ordered { "decimal" } else { "bullet" };
            let text = if ordered {
                format!("%{}.", level + 1)
            } else {
                "•".to_owned()
            };
            let indentation = TWIPS_PER_LEVEL * (level as i32 + 1);

            let level = Level::new(
                level,
                Start::new(1),
                NumberFormat::new(format),
                LevelText::new(text),
                LevelJc::new("left"),
            )
            .indent(
                Some(indentation),
                Some(SpecialIndentType::Hanging(TWIPS_PER_LEVEL / 2)),
                None,
                None,
            );

            abstract_num = abstract_num.add_level(level);
        }

        numberings = numberings
            .add_abstract_numbering(abstract_num)
            .add_numbering(Numbering::new(definition.id, definition.id));
    }

    numberings
}
