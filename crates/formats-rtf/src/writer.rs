use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use docmodel::{PmDoc, PmNode};
use formats_core::{is_safe_link, FormatError, WarningSink, WrittenDocument};
use serde_json::Value;

use crate::style::{
    font_size_half_points, line_height_twips, parse_color, rtf_font_name, DEFAULT_FONT_NAME,
};

const LI_TWIPS_PER_LEVEL: i32 = 720;
const PARAGRAPH_INDENT_TWIPS: i32 = 480;

pub fn write(document: &PmDoc) -> Result<WrittenDocument, FormatError> {
    let mut context = WriteContext::new();
    context.collect(document);

    let mut output = String::new();
    output.push_str("{\\rtf1\\ansi\\ansicpg1251\\deff0\\uc1\n");
    context.write_header(&mut output);

    for block in &document.content {
        context.write_block(block, &mut output);
    }

    output.push_str("}\n");

    Ok(WrittenDocument {
        text: output,
        warnings: context.warnings.into_warnings(),
    })
}

#[derive(Default)]
struct Fonts {
    names: Vec<String>,
    indices: HashMap<String, u16>,
}

impl Fonts {
    fn new() -> Self {
        let mut fonts = Self::default();
        fonts.index(DEFAULT_FONT_NAME);
        fonts
    }

    fn index(&mut self, name: &str) -> u16 {
        let name = if name.trim().is_empty() {
            DEFAULT_FONT_NAME
        } else {
            name.trim()
        };

        if let Some(index) = self.indices.get(name) {
            return *index;
        }

        let index = self.names.len() as u16;
        self.names.push(name.to_owned());
        self.indices.insert(name.to_owned(), index);
        index
    }

    fn get(&self, name: &str) -> Option<u16> {
        self.indices.get(name.trim()).copied()
    }
}

#[derive(Default)]
struct Colors {
    values: Vec<(u8, u8, u8)>,
    indices: HashMap<(u8, u8, u8), u16>,
}

impl Colors {
    fn index(&mut self, color: (u8, u8, u8)) -> u16 {
        if let Some(index) = self.indices.get(&color) {
            return *index;
        }

        let index = self.values.len() as u16 + 1;
        self.values.push(color);
        self.indices.insert(color, index);
        index
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListLevelKind {
    Bullet,
    Ordered,
    Task,
}

#[derive(Clone)]
struct ListDefinition {
    id: u16,
    levels: Vec<ListLevelKind>,
}

struct WriteContext {
    warnings: WarningSink,
    fonts: Fonts,
    colors: Colors,
    lists: Vec<ListDefinition>,
    list_cursor: usize,
}

impl WriteContext {
    fn new() -> Self {
        Self {
            warnings: WarningSink::new(),
            fonts: Fonts::new(),
            colors: Colors::default(),
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
                "paragraph" | "heading" => self.collect_inline(block),
                "codeBlock" => {
                    self.fonts.index("Courier New");
                }
                "bulletList" | "orderedList" | "taskList" => {
                    let mut definition = ListDefinition {
                        id: self.lists.len() as u16 + 1,
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
                } else {
                    self.collect_inline(child);
                }
            }
        }
    }

    fn collect_inline(&mut self, node: &PmNode) {
        for mark in &node.marks {
            match mark.mark_type.as_str() {
                "textStyle" => {
                    let Some(attrs) = mark.attrs.as_ref().and_then(|value| value.as_object())
                    else {
                        continue;
                    };

                    if let Some(font) = attrs
                        .get("fontFamily")
                        .and_then(|value| value.as_str())
                        .map(rtf_font_name)
                        .filter(|font| !font.is_empty())
                    {
                        self.fonts.index(&font);
                    }

                    if let Some(color) = attrs
                        .get("color")
                        .and_then(|value| value.as_str())
                        .and_then(parse_color)
                    {
                        self.colors.index(color);
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
                        self.colors.index(color);
                    }
                }
                "code" => {
                    self.fonts.index("Courier New");
                }
                _ => {}
            }
        }

        for child in &node.content {
            self.collect_inline(child);
        }
    }

    fn write_header(&self, output: &mut String) {
        output.push_str("{\\fonttbl");
        for (index, name) in self.fonts.names.iter().enumerate() {
            output.push_str(&format!("{{\\f{index}{} ", font_family_flag(name)));
            write_escaped(output, name);
            output.push_str(";}");
        }
        output.push_str("}\n");

        output.push_str("{\\colortbl;");
        for (red, green, blue) in &self.colors.values {
            output.push_str(&format!("\\red{red}\\green{green}\\blue{blue};"));
        }
        output.push_str("}\n");

        output.push_str("{\\stylesheet{\\s0\\snext0 Normal;}");
        for level in 1..=6 {
            output.push_str(&format!(
                "{{\\s{level}\\outlinelevel{}\\snext0 heading {level};}}",
                level - 1
            ));
        }
        output.push_str("}\n");

        if !self.lists.is_empty() {
            self.write_list_table(output);
        }
    }

    fn write_list_table(&self, output: &mut String) {
        output.push_str("{\\*\\listtable");

        for definition in &self.lists {
            output.push_str(&format!("{{\\list\\listtemplateid{}", definition.id));

            for kind in &definition.levels {
                let ordered = matches!(kind, ListLevelKind::Ordered);
                let nfc = if ordered { 0 } else { 23 };

                output.push_str(&format!(
                    "{{\\listlevel\\levelnfc{nfc}\\levelnfcn{nfc}\\leveljc0\\levelfollow0\\levelstartat1\\levelspace0\\levelindent0"
                ));

                if ordered {
                    output.push_str("{\\leveltext\\'02\\'00.;}{\\levelnumbers\\'01;}");
                } else {
                    output.push_str("{\\leveltext\\'01");
                    write_unicode_escape(output, '\u{2022}');
                    output.push_str(";}{\\levelnumbers;}");
                }

                output.push_str("\\f0}");
            }

            output.push_str(&format!("\\listid{}}}", definition.id));
        }

        output.push_str("}\n{\\*\\listoverridetable");

        for definition in &self.lists {
            output.push_str(&format!(
                "{{\\listoverride\\listid{}\\listoverridecount0\\ls{}}}",
                definition.id, definition.id
            ));
        }

        output.push_str("}\n");
    }

    fn write_block(&mut self, node: &PmNode, output: &mut String) {
        match node.node_type.as_str() {
            "paragraph" => self.write_paragraph(node, None, None, output),
            "heading" => self.write_heading(node, output),
            "bulletList" | "orderedList" | "taskList" => {
                self.write_top_list(node, output);
            }
            "blockquote" => {
                self.warnings
                    .warn("Цитаты сохраняются в RTF как обычные абзацы.");
                for child in &node.content {
                    self.write_block(child, output);
                }
            }
            "codeBlock" => {
                self.warnings
                    .warn("Блок кода сохраняется в RTF как текст с моноширинным шрифтом.");
                self.write_code_block(node, output);
            }
            "table" => self.write_table(node, output),
            "pageBreak" => output.push_str("\\page\n"),
            "horizontalRule" => output.push_str("{\\pard\\brdrb\\brdrs\\brdrw10\\par}\n"),
            "image" => {
                let paragraph = PmNode::with_content("paragraph", vec![node.clone()]);
                self.write_paragraph(&paragraph, None, None, output);
            }
            _ => {
                self.warnings
                    .warn(format!("Узел «{}» не поддерживается RTF.", node.node_type));
                let text = formats_core::node_text(node);
                let mut paragraph = PmNode::with_content("paragraph", Vec::new());
                if !text.is_empty() {
                    paragraph.content.push(PmNode::text(text));
                }
                self.write_paragraph(&paragraph, None, None, output);
            }
        }
    }

    fn write_heading(&mut self, node: &PmNode, output: &mut String) {
        let level = node
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("level"))
            .and_then(|level| level.as_u64())
            .unwrap_or(1)
            .clamp(1, 6);

        output.push_str("\\pard");
        output.push_str(&format!("\\s{level}"));
        self.write_paragraph_controls(Some(node), None, output);
        self.write_inline_children(node, output);
        output.push_str("\\par\n");
    }

    fn write_paragraph(
        &mut self,
        node: &PmNode,
        list: Option<(&ListDefinition, usize)>,
        prefix: Option<&str>,
        output: &mut String,
    ) {
        output.push_str("\\pard");
        self.write_paragraph_controls(Some(node), list, output);

        if let Some(prefix) = prefix {
            write_escaped(output, prefix);
        }

        self.write_inline_children(node, output);
        output.push_str("\\par\n");
    }

    fn write_paragraph_controls(
        &mut self,
        node: Option<&PmNode>,
        list: Option<(&ListDefinition, usize)>,
        output: &mut String,
    ) {
        let attrs = node
            .and_then(|node| node.attrs.as_ref())
            .and_then(|attrs| attrs.as_object());
        let align = attrs
            .and_then(|attrs| attrs.get("textAlign"))
            .and_then(|value| value.as_str())
            .unwrap_or("left");

        match align {
            "center" => output.push_str("\\qc"),
            "right" => output.push_str("\\qr"),
            "justify" => output.push_str("\\qj"),
            _ => {}
        }

        let indent = attrs
            .and_then(|attrs| attrs.get("indent"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0);

        if let Some((definition, level)) = list {
            output.push_str(&format!(
                "\\ls{}\\ilvl{}\\li{}\\fi-360",
                definition.id,
                level,
                LI_TWIPS_PER_LEVEL * (level as i32 + 1)
            ));
        } else if indent > 0 {
            output.push_str(&format!("\\li{}", PARAGRAPH_INDENT_TWIPS * indent as i32));
        }

        if let Some(space_before) = attrs
            .and_then(|attrs| attrs.get("spaceBefore"))
            .and_then(|value| value.as_u64())
            .filter(|value| *value > 0)
        {
            output.push_str(&format!("\\sb{}", space_before * 20));
        }

        if let Some(space_after) = attrs
            .and_then(|attrs| attrs.get("spaceAfter"))
            .and_then(|value| value.as_u64())
            .filter(|value| *value > 0)
        {
            output.push_str(&format!("\\sa{}", space_after * 20));
        }

        if let Some(line_height) = attrs
            .and_then(|attrs| attrs.get("lineHeight"))
            .and_then(|value| value.as_str())
            .and_then(line_height_twips)
        {
            output.push_str(&format!("\\sl{line_height}\\slmul1"));
        }
    }

    fn write_code_block(&mut self, node: &PmNode, output: &mut String) {
        let font = self.fonts.get("Courier New").unwrap_or(0);
        let text = formats_core::node_text(node);
        output.push_str("\\pard");
        output.push_str(&format!("\\f{font} "));

        for (index, line) in text.split('\n').enumerate() {
            if index > 0 {
                output.push_str("\\line ");
            }
            write_escaped(output, line);
        }

        output.push_str("\\par\n");
    }

    fn write_top_list(&mut self, node: &PmNode, output: &mut String) {
        let definition = self.lists.get(self.list_cursor).cloned();
        self.list_cursor += 1;

        let Some(definition) = definition else {
            self.warnings
                .warn("Не удалось описать список в RTF, содержимое сохранено текстом.");
            for item in &node.content {
                for child in &item.content {
                    self.write_block(child, output);
                }
            }
            return;
        };

        self.write_list(node, &definition, 0, output);
    }

    fn write_list(
        &mut self,
        node: &PmNode,
        definition: &ListDefinition,
        level: usize,
        output: &mut String,
    ) {
        for item in &node.content {
            let task = matches!(list_kind(node), ListLevelKind::Task);
            let checked = item
                .attrs
                .as_ref()
                .and_then(|attrs| attrs.get("checked"))
                .and_then(|checked| checked.as_bool())
                .unwrap_or(false);
            let mut first = true;

            for child in &item.content {
                if is_list(child) {
                    self.write_list(child, definition, level + 1, output);
                } else if child.node_type == "blockquote" {
                    for nested in &child.content {
                        self.write_block(nested, output);
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
                    self.write_paragraph(child, list, prefix, output);
                    first = false;
                } else {
                    self.write_block(child, output);
                }
            }

            if first {
                let prefix = if task {
                    Some(if checked { "[x]" } else { "[ ]" })
                } else {
                    None
                };
                let mut empty = PmNode::paragraph();
                empty.attrs = None;
                self.write_paragraph(&empty, Some((definition, level)), prefix, output);
            }
        }
    }

    fn write_table(&mut self, node: &PmNode, output: &mut String) {
        if node
            .content
            .iter()
            .flat_map(|row| row.content.iter())
            .any(has_span)
        {
            self.warnings
                .warn("Объединённые ячейки не сохраняются в RTF.");
        }

        for (row_index, row) in node.content.iter().enumerate() {
            output.push_str("\\trowd\\trgaph108\\trleft0");

            if row_index == 0
                && row
                    .content
                    .iter()
                    .all(|cell| cell.node_type == "tableHeader")
            {
                output.push_str("\\trhdr");
            }

            for column in 0..row.content.len().max(1) {
                output.push_str(&format!("\\cellx{}", 1800 * (column as i32 + 1)));
            }

            output.push('\n');

            for cell in &row.content {
                output.push_str("\\pard\\intbl");
                self.write_cell_content(cell, output);
                output.push_str("\\cell\n");
            }

            output.push_str("\\row\n");
        }
    }

    fn write_cell_content(&mut self, cell: &PmNode, output: &mut String) {
        let mut first = true;

        for child in &cell.content {
            match child.node_type.as_str() {
                "paragraph" | "heading" => {
                    if !first {
                        output.push_str("\\par");
                    }
                    self.write_inline_children(child, output);
                }
                _ => {
                    let text = formats_core::node_text(child);
                    write_escaped(output, &text);
                }
            }
            first = false;
        }

        if first {
            output.push_str("\\par");
        }
    }

    fn write_inline_children(&mut self, node: &PmNode, output: &mut String) {
        for child in &node.content {
            self.write_inline(child, output);
        }
    }

    fn write_inline(&mut self, node: &PmNode, output: &mut String) {
        match node.node_type.as_str() {
            "text" => self.write_run(node, output),
            "hardBreak" => output.push_str("\\line "),
            "image" => self.write_image(node, output),
            _ => {
                self.warnings.warn(format!(
                    "Встроенный узел «{}» не поддерживается RTF и был упрощён.",
                    node.node_type
                ));
                self.write_inline_children(node, output);
            }
        }
    }

    fn write_run(&mut self, node: &PmNode, output: &mut String) {
        let text = node.text.as_deref().unwrap_or_default();

        if text.is_empty() {
            return;
        }

        let link = node
            .marks
            .iter()
            .find(|mark| mark.mark_type == "link")
            .and_then(|mark| mark.attrs.as_ref())
            .and_then(|attrs| attrs.get("href"))
            .and_then(|href| href.as_str())
            .filter(|href| !href.is_empty());

        if let Some(link) = link {
            if is_safe_link(link) {
                output.push_str("{\\field{\\*\\fldinst{HYPERLINK \"");
                write_field_url(output, link);
                output.push_str("\"}}{\\fldrslt{");
                self.write_formatted_text(node, text, output);
                output.push_str("}}}");
                return;
            }

            self.warnings
                .warn("Ссылка с небезопасным адресом не сохраняется в RTF.");
        }

        self.write_formatted_text(node, text, output);
    }

    fn write_formatted_text(&mut self, node: &PmNode, text: &str, output: &mut String) {
        output.push('{');
        let mut controls = false;

        for mark in &node.marks {
            match mark.mark_type.as_str() {
                "bold" => {
                    output.push_str("\\b");
                    controls = true;
                }
                "italic" => {
                    output.push_str("\\i");
                    controls = true;
                }
                "underline" => {
                    output.push_str("\\ul");
                    controls = true;
                }
                "strike" => {
                    output.push_str("\\strike");
                    controls = true;
                }
                "code" => {
                    let font = self.fonts.get("Courier New").unwrap_or(0);
                    output.push_str(&format!("\\f{font}"));
                    controls = true;
                }
                "textStyle" => {
                    controls |= self.write_text_style(mark.attrs.as_ref(), output);
                }
                "highlight" => {
                    controls |= self.write_highlight(mark.attrs.as_ref(), output);
                }
                "link" => {}
                other => self
                    .warnings
                    .warn(format!("Разметка «{other}» не поддерживается RTF.")),
            }
        }

        if controls {
            output.push(' ');
        }

        write_escaped(output, text);
        output.push('}');
    }

    fn write_text_style(&mut self, attrs: Option<&Value>, output: &mut String) -> bool {
        let mut written = false;
        let Some(attrs) = attrs.and_then(|value| value.as_object()) else {
            return written;
        };

        if let Some(font) = attrs
            .get("fontFamily")
            .and_then(|value| value.as_str())
            .filter(|value| !value.trim().is_empty())
        {
            let name = rtf_font_name(font);
            let index = self.fonts.index(&name);
            output.push_str(&format!("\\f{index}"));
            written = true;
        }

        if let Some(size) = attrs
            .get("fontSize")
            .and_then(|value| value.as_str())
            .and_then(font_size_half_points)
        {
            output.push_str(&format!("\\fs{size}"));
            written = true;
        }

        if let Some(color) = attrs
            .get("color")
            .and_then(|value| value.as_str())
            .and_then(parse_color)
        {
            let index = self.colors.index(color);
            output.push_str(&format!("\\cf{index}"));
            written = true;
        }

        written
    }

    fn write_highlight(&mut self, attrs: Option<&Value>, output: &mut String) -> bool {
        let color = attrs
            .and_then(|attrs| attrs.get("color"))
            .and_then(|value| value.as_str())
            .and_then(parse_color);

        if let Some(color) = color {
            let index = self.colors.index(color);
            output.push_str(&format!("\\highlight{index}"));
            true
        } else {
            false
        }
    }

    fn write_image(&mut self, node: &PmNode, output: &mut String) {
        let source = node
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("src"))
            .and_then(|src| src.as_str())
            .unwrap_or_default();

        let Some((mime, bytes)) = decode_image(source) else {
            self.warnings
                .warn("Изображение не сохраняется в RTF: поддерживаются png, jpeg и gif.");
            return;
        };

        let control = match mime {
            "image/png" => "pngblip",
            "image/jpeg" => "jpegblip",
            "image/gif" => "gifblip",
            _ => return,
        };

        if let Some(alt) = node
            .attrs
            .as_ref()
            .and_then(|attrs| attrs.get("alt"))
            .and_then(|alt| alt.as_str())
            .filter(|alt| !alt.is_empty())
        {
            let _ = alt;
            self.warnings
                .warn("Подписи изображений не сохраняются в RTF.");
        }

        output.push_str("{\\pict\\");
        output.push_str(control);
        output.push(' ');

        for byte in bytes {
            output.push_str(&format!("{byte:02x}"));
        }

        output.push('}');
    }
}

fn font_family_flag(name: &str) -> &'static str {
    match name.to_ascii_lowercase().as_str() {
        "arial" | "verdana" | "helvetica" => "\\fswiss",
        "courier new" | "courier" => "\\fmodern",
        "times new roman" | "times" | "georgia" => "\\froman",
        _ => "\\fnil",
    }
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

fn has_span(cell: &PmNode) -> bool {
    let Some(attrs) = cell.attrs.as_ref().and_then(|attrs| attrs.as_object()) else {
        return false;
    };

    attrs
        .get("colspan")
        .and_then(|value| value.as_u64())
        .unwrap_or(1)
        > 1
        || attrs
            .get("rowspan")
            .and_then(|value| value.as_u64())
            .unwrap_or(1)
            > 1
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
        })?;

    let bytes = STANDARD.decode(encoded).ok()?;

    Some((mime, bytes))
}

fn write_field_url(output: &mut String, url: &str) {
    for character in url.chars() {
        match character {
            '"' => output.push_str("%22"),
            '\\' => output.push_str("%5C"),
            '{' => output.push_str("%7B"),
            '}' => output.push_str("%7D"),
            '\r' | '\n' => {}
            _ => output.push(character),
        }
    }
}

fn write_escaped(output: &mut String, text: &str) {
    for character in text.chars() {
        match character {
            '\\' => output.push_str("\\\\"),
            '{' => output.push_str("\\{"),
            '}' => output.push_str("\\}"),
            '\n' => output.push_str("\\line "),
            '\t' => output.push_str("\\tab "),
            character if character.is_ascii() => output.push(character),
            character => write_unicode_escape(output, character),
        }
    }
}

fn write_unicode_escape(output: &mut String, character: char) {
    let mut buffer = [0u16; 2];
    let units = character.encode_utf16(&mut buffer);

    for unit in units {
        let value = if *unit > 32_767 {
            *unit as i32 - 65_536
        } else {
            *unit as i32
        };
        output.push_str(&format!("\\u{value}?"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docmodel::PmMark;
    use serde_json::json;

    fn write(document: &PmDoc) -> WrittenDocument {
        crate::codec().write(document).expect("запись RTF")
    }

    #[test]
    fn writes_header_and_paragraphs() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode::text("Привет, мир")],
            )],
        };
        let written = write(&document);

        assert!(written.text.starts_with("{\\rtf1"));
        assert!(written.text.contains("\\fonttbl"));
        assert!(written.text.contains("\\u1055?"));
        assert!(written.text.contains("\\par"));
    }

    #[test]
    fn writes_heading_and_table() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![
                PmNode {
                    attrs: Some(json!({ "level": 2 })),
                    ..PmNode::with_content("heading", vec![PmNode::text("Заголовок")])
                },
                PmNode::with_content(
                    "table",
                    vec![PmNode::with_content(
                        "tableRow",
                        vec![PmNode::with_content(
                            "tableHeader",
                            vec![PmNode::with_content(
                                "paragraph",
                                vec![PmNode::text("Ячейка")],
                            )],
                        )],
                    )],
                ),
            ],
        };
        let written = write(&document);

        assert!(written.text.contains("\\s2"));
        assert!(written.text.contains("\\trowd"));
        assert!(written.text.contains("\\trhdr"));
        assert!(written.text.contains("\\cell"));
        assert!(written.text.contains("\\row"));
    }

    fn link_mark(href: &str) -> PmMark {
        let mut mark = PmMark::new("link");
        mark.attrs = Some(json!({ "href": href }));
        mark
    }

    #[test]
    fn drops_unsafe_links_on_write() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
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
            )],
        };

        let written = write(&document);

        assert!(!written.text.contains("javascript:"));
        let mut expected_text = String::new();
        write_escaped(&mut expected_text, "опасная");
        assert!(written.text.contains(&expected_text));
        assert!(written
            .warnings
            .iter()
            .any(|warning| warning.contains("небезопасн")));
        assert!(written.text.contains("HYPERLINK \"https://example.com\""));
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
            content: vec![PmNode::with_content(
                "paragraph",
                safe_links
                    .iter()
                    .map(|href| PmNode {
                        marks: vec![link_mark(href)],
                        ..PmNode::text("ссылка")
                    })
                    .collect(),
            )],
        };

        let written = write(&document);

        for href in safe_links {
            assert!(
                written.text.contains(&format!("HYPERLINK \"{href}\"")),
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
