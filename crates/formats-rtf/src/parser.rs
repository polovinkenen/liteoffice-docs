use std::collections::HashMap;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use docmodel::{PmDoc, PmMark, PmNode};
use formats_core::{is_safe_link, FormatError, ImportedDocument, WarningSink};
use serde_json::{json, Map, Value};

use crate::lexer::{tokenize, Token};
use crate::style::{
    canonical_font_family, decode_in_code_page, font_size_points, format_color,
    line_height_from_twips, DEFAULT_FONT_NAME, DEFAULT_FONT_SIZE_HALF_POINTS, LI_TWIPS_PER_LEVEL,
};

const MAX_GROUP_DEPTH: usize = 512;
const MAX_NODES: usize = 1_000_000;
const MAX_IMAGE_BYTES: usize = 15 * 1024 * 1024;

const SKIPPED_DESTINATIONS: &[&str] = &[
    "annotation",
    "atnauthor",
    "atndate",
    "colorschememapping",
    "comment",
    "datastore",
    "deleted",
    "docvar",
    "endnote",
    "filetbl",
    "footer",
    "footerf",
    "footerl",
    "footerr",
    "footnote",
    "generator",
    "header",
    "headerf",
    "headerl",
    "headerr",
    "info",
    "latentstyles",
    "listpicture",
    "listtext",
    "mmathpr",
    "nonshppict",
    "objdata",
    "object",
    "pntext",
    "rsidtbl",
    "shppict",
    "themedata",
    "xmlnstbl",
];

pub fn parse(source: &str) -> Result<ImportedDocument, FormatError> {
    parse_with_limit(source, MAX_NODES)
}

fn parse_with_limit(source: &str, max_nodes: usize) -> Result<ImportedDocument, FormatError> {
    let tokens = tokenize(source).map_err(FormatError::Parse)?;
    let mut parser = Parser::new(tokens, max_nodes);

    parser.run()?;
    parser.finish()
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Align {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Clone, PartialEq, Eq)]
enum ListKind {
    Bullet,
    Ordered,
    Task { checked: bool },
}

#[derive(Clone)]
struct ListMeta {
    kind: ListKind,
    level: u16,
}

#[derive(Clone, Default, PartialEq)]
struct CharStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    font: Option<u16>,
    size: Option<u16>,
    color: Option<u16>,
    highlight: Option<u16>,
    link: Option<String>,
}

#[derive(Clone, Default)]
struct ParaStyle {
    align: Align,
    indent: i32,
    space_before: i32,
    space_after: i32,
    line_height: Option<i32>,
    line_multiple: Option<i32>,
    heading: Option<u8>,
    style_index: Option<u16>,
    list_id: Option<u16>,
    list_level: u16,
    in_table: bool,
    bottom_border: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Destination {
    Body,
    FontTable,
    ColorTable,
    StyleSheet,
    ListTable,
    ListOverride,
    Pict,
    FieldInstruction,
    FieldResult,
    Skipped,
}

#[derive(Clone)]
struct Frame {
    char_style: CharStyle,
    para_style: ParaStyle,
    destination: Destination,
    uc: usize,
    skip: usize,
    star: bool,
    field_link: Option<String>,
    field_text: String,
    pict: Option<PictAccum>,
}

impl Frame {
    fn body() -> Self {
        Self {
            char_style: CharStyle::default(),
            para_style: ParaStyle::default(),
            destination: Destination::Body,
            uc: 1,
            skip: 0,
            star: false,
            field_link: None,
            field_text: String::new(),
            pict: None,
        }
    }
}

#[derive(Clone, Default)]
struct PictAccum {
    kind: Option<ImageKind>,
    unsupported: Option<String>,
    hex: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ImageKind {
    Png,
    Jpeg,
    Gif,
}

impl ImageKind {
    fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
        }
    }
}

enum InlineItem {
    Text(Run),
    Image(PmNode),
    HardBreak,
}

struct Run {
    text: String,
    style: CharStyle,
}

struct ParagraphDraft {
    items: Vec<InlineItem>,
}

enum Block {
    Paragraph {
        node: PmNode,
        list: Option<ListMeta>,
    },
    Atom(PmNode),
    Table(Vec<PmNode>),
}

struct RowDraft {
    header: bool,
    cells: Vec<PmNode>,
}

struct FontEntry {
    index: u16,
    name: String,
}

struct StyleEntry {
    index: u16,
    heading: Option<u8>,
    name: String,
}

struct ListEntry {
    id: Option<u16>,
    level: usize,
    levels: Vec<(usize, bool)>,
    depth: usize,
}

#[derive(Default)]
struct Header {
    code_page: u16,
    fonts: HashMap<u16, String>,
    colors: Vec<Option<(u8, u8, u8)>>,
    styles: HashMap<u16, u8>,
    list_levels: HashMap<(u16, u16), bool>,
    list_overrides: HashMap<u16, u16>,
}

struct Parser<'a> {
    tokens: Vec<Token<'a>>,
    position: usize,
    header: Header,
    warnings: WarningSink,
    frames: Vec<Frame>,
    paragraph: ParagraphDraft,
    pending_bytes: Vec<u8>,
    blocks: Vec<Block>,
    table_rows: Vec<PmNode>,
    row: Option<RowDraft>,
    current_cell: Vec<PmNode>,
    font_entry: Option<FontEntry>,
    style_entry: Option<StyleEntry>,
    list_entry: Option<ListEntry>,
    override_list_id: Option<u16>,
    color_acc: (Option<u8>, Option<u8>, Option<u8>),
    nodes: usize,
    max_nodes: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token<'a>>, max_nodes: usize) -> Self {
        Self {
            tokens,
            position: 0,
            header: Header {
                code_page: 1252,
                ..Header::default()
            },
            warnings: WarningSink::new(),
            frames: vec![Frame::body()],
            paragraph: ParagraphDraft { items: Vec::new() },
            pending_bytes: Vec::new(),
            blocks: Vec::new(),
            table_rows: Vec::new(),
            row: None,
            current_cell: Vec::new(),
            font_entry: None,
            style_entry: None,
            list_entry: None,
            override_list_id: None,
            color_acc: (None, None, None),
            nodes: 0,
            max_nodes,
        }
    }

    fn count_node(&mut self) -> Result<(), FormatError> {
        self.nodes += 1;

        if self.nodes > self.max_nodes {
            return Err(FormatError::Parse(format!(
                "превышено максимальное число узлов RTF {}",
                self.max_nodes
            )));
        }

        Ok(())
    }

    fn run(&mut self) -> Result<(), FormatError> {
        while let Some(token) = self.tokens.get(self.position).copied() {
            self.position += 1;

            match token {
                Token::GroupStart => self.group_start()?,
                Token::GroupEnd => self.group_end()?,
                Token::Control { name, param } => self.control(name, param)?,
                Token::Literal(character) => self.literal(character),
                Token::Symbol(_) => {}
                Token::HexByte(byte) => self.hex_byte(byte),
                Token::Text(text) => self.text(text),
                Token::Star => {
                    if let Some(frame) = self.frames.last_mut() {
                        frame.star = true;
                    }
                }
            }
        }

        Ok(())
    }

    fn group_start(&mut self) -> Result<(), FormatError> {
        if self.frames.len() >= MAX_GROUP_DEPTH {
            return Err(FormatError::Parse(
                "превышена максимальная глубина вложенности RTF".to_owned(),
            ));
        }

        let frame = self.frames.last().cloned().unwrap_or_else(Frame::body);
        if matches!(
            frame.destination,
            Destination::Body | Destination::FieldResult
        ) {
            self.flush_bytes();
        }

        let mut frame = Frame {
            star: false,
            field_text: String::new(),
            pict: None,
            ..frame
        };
        frame.skip = 0;
        self.frames.push(frame);

        Ok(())
    }

    fn group_end(&mut self) -> Result<(), FormatError> {
        if self.frames.len() <= 1 {
            return Ok(());
        }

        let depth = self.frames.len();
        let popped = self.frames.pop().expect("кадр существует");

        if matches!(
            popped.destination,
            Destination::Body | Destination::FieldResult
        ) {
            self.flush_bytes();
        }

        match popped.destination {
            Destination::FontTable => {
                if let Some(entry) = self.font_entry.take() {
                    self.commit_font(entry);
                }
            }
            Destination::StyleSheet => {
                if let Some(entry) = self.style_entry.take() {
                    self.commit_style(entry);
                }
            }
            Destination::ListTable => {
                if self
                    .list_entry
                    .as_ref()
                    .is_some_and(|entry| entry.depth == depth)
                {
                    self.commit_list();
                }
            }
            Destination::FieldInstruction => {
                let parsed = parse_field_link(&popped.field_text);

                if matches!(parsed, FieldLink::Unsafe) {
                    self.warnings
                        .warn("Ссылка с небезопасным адресом была преобразована в текст.");
                }

                let link = match parsed {
                    FieldLink::Safe(link) => Some(link),
                    FieldLink::Unsafe | FieldLink::Missing => None,
                }
                .or(popped.field_link);

                if let Some(link) = link {
                    if let Some(frame) = self.frames.last_mut() {
                        frame.field_link = Some(link);
                    }
                }
            }
            Destination::Pict => {
                if let Some(pict) = popped.pict {
                    self.finish_picture(pict);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn control(&mut self, name: &str, param: Option<i32>) -> Result<(), FormatError> {
        if self.destination() == Destination::Skipped {
            self.warn_unsupported(name);
            return Ok(());
        }

        if let Some(destination) = destination_for(name) {
            if let Some(frame) = self.frames.last_mut() {
                frame.destination = destination;
                frame.star = false;

                if destination == Destination::Pict {
                    frame.pict = Some(PictAccum::default());
                } else if destination == Destination::FieldResult {
                    frame.char_style.link = frame.field_link.clone();
                }
            }

            match destination {
                Destination::FontTable => self.font_entry = None,
                Destination::ColorTable => {
                    self.header.colors.clear();
                    self.color_acc = (None, None, None);
                }
                Destination::StyleSheet => self.style_entry = None,
                Destination::FieldInstruction => {
                    if let Some(frame) = self.frames.last_mut() {
                        frame.field_text.clear();
                    }
                }
                Destination::Skipped => self.warn_unsupported(name),
                _ => {}
            }

            return Ok(());
        }

        if self.frames.last().is_some_and(|frame| frame.star) {
            if let Some(frame) = self.frames.last_mut() {
                frame.destination = Destination::Skipped;
                frame.star = false;
            }
            return Ok(());
        }

        let destination = self.destination();

        if destination == Destination::FontTable {
            self.font_control(name, param);
            return Ok(());
        }

        if destination == Destination::ColorTable {
            self.color_control(name, param);
            return Ok(());
        }

        if destination == Destination::StyleSheet {
            self.style_control(name, param);
            return Ok(());
        }

        if destination == Destination::ListTable {
            self.list_control(name, param);
            return Ok(());
        }

        if destination == Destination::ListOverride {
            self.override_control(name, param);
            return Ok(());
        }

        if destination == Destination::Pict {
            self.pict_control(name, param);
            return Ok(());
        }

        if destination == Destination::FieldInstruction {
            if name == "uc" {
                if let Some(frame) = self.frames.last_mut() {
                    frame.uc = param.unwrap_or(1).max(0) as usize;
                }
            }
            return Ok(());
        }

        match name {
            "ansi" => {}
            "ansicpg" => {
                if let Some(code_page) = param.and_then(|value| u16::try_from(value).ok()) {
                    self.header.code_page = code_page;
                }
            }
            "uc" => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.uc = param.unwrap_or(1).max(0) as usize;
                }
            }
            "u" => self.unicode_character(param),
            "b" => self.set_flag(|style| &mut style.bold, param),
            "i" => self.set_flag(|style| &mut style.italic, param),
            "ul" => self.set_flag(|style| &mut style.underline, param),
            "ulnone" => self.clear_flag(|style| &mut style.underline),
            "strike" => self.set_flag(|style| &mut style.strike, param),
            "plain" => self.reset_character_style(),
            "f" => self.set_font(param),
            "fs" => self.set_font_size(param),
            "cf" => self.set_color(param),
            "highlight" => self.set_highlight(param),
            "ql" => self.set_align(Align::Left),
            "qr" => self.set_align(Align::Right),
            "qc" => self.set_align(Align::Center),
            "qj" => self.set_align(Align::Justify),
            "li" => {
                let value = param.unwrap_or(0).max(0);
                self.with_paragraph_style(|style| style.indent = value);
            }
            "ri" | "fi" => {}
            "sb" => {
                let value = param.unwrap_or(0).max(0);
                self.with_paragraph_style(|style| style.space_before = value);
            }
            "sa" => {
                let value = param.unwrap_or(0).max(0);
                self.with_paragraph_style(|style| style.space_after = value);
            }
            "sl" => self.with_paragraph_style(|style| style.line_height = param),
            "slmul" => self.with_paragraph_style(|style| style.line_multiple = param),
            "pard" => self.reset_paragraph_style(),
            "intbl" => self.with_paragraph_style(|style| style.in_table = true),
            "outlinelevel" => {
                let level = param.unwrap_or(0).clamp(0, 5) as u8 + 1;
                self.with_paragraph_style(|style| style.heading = Some(level));
            }
            "s" => {
                if let Some(index) = param.and_then(|value| u16::try_from(value).ok()) {
                    self.with_paragraph_style(|style| style.style_index = Some(index));
                }
            }
            "ls" => {
                if let Some(id) = param.and_then(|value| u16::try_from(value).ok()) {
                    self.with_paragraph_style(|style| style.list_id = Some(id));
                }
            }
            "ilvl" => {
                let level = param.unwrap_or(0).clamp(0, 8) as u16;
                self.with_paragraph_style(|style| style.list_level = level);
            }
            "par" => {
                self.flush_bytes();
                self.finish_paragraph(true)?;
            }
            "line" => {
                self.flush_bytes();
                self.paragraph.items.push(InlineItem::HardBreak);
            }
            "page" => {
                self.flush_bytes();
                if !self.paragraph.items.is_empty() {
                    self.finish_paragraph(false)?;
                }
                self.flush_table();
                self.blocks.push(Block::Atom(atom("pageBreak")));
            }
            "tab" => self.literal('\t'),
            "bullet" => self.literal('\u{2022}'),
            "emdash" => self.literal('\u{2014}'),
            "endash" => self.literal('\u{2013}'),
            "lquote" => self.literal('\u{2018}'),
            "rquote" => self.literal('\u{2019}'),
            "ldblquote" => self.literal('\u{201c}'),
            "rdblquote" => self.literal('\u{201d}'),
            "trowd" => self.start_row(),
            "trhdr" => {
                if let Some(row) = self.row.as_mut() {
                    row.header = true;
                }
            }
            "cell" => self.finish_cell()?,
            "row" => self.finish_row()?,
            "brdrb" => self.with_paragraph_style(|style| style.bottom_border = true),
            "brdrnone" => self.with_paragraph_style(|style| style.bottom_border = false),
            "header" | "footer" | "footnote" | "endnote" | "annotation" | "object" | "shppict" => {
                self.warn_unsupported(name);
            }
            _ => {}
        }

        Ok(())
    }

    fn text(&mut self, text: &str) {
        match self.destination() {
            Destination::Body | Destination::FieldResult => {
                let (mut remaining, mut skip) = (
                    text,
                    self.frames.last().map(|frame| frame.skip).unwrap_or(0),
                );

                if skip > 0 {
                    let skipped = remaining.chars().take(skip).count();
                    skip -= skipped;

                    if let Some(frame) = self.frames.last_mut() {
                        frame.skip = skip;
                    }

                    remaining = &remaining[remaining
                        .char_indices()
                        .nth(skipped)
                        .map(|(index, _)| index)
                        .unwrap_or(remaining.len())..];
                }

                if remaining.is_empty() {
                    return;
                }

                self.flush_bytes();
                let style = self.current_char_style();
                self.push_text(remaining, &style);
            }
            Destination::FontTable => {
                if let Some(entry) = self.font_entry.as_mut() {
                    entry.name.push_str(text);
                }
            }
            Destination::ColorTable => {
                for character in text.chars() {
                    if character == ';' {
                        self.commit_color();
                    }
                }
            }
            Destination::StyleSheet => {
                if let Some(entry) = self.style_entry.as_mut() {
                    entry.name.push_str(text);
                }
            }
            Destination::FieldInstruction => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.field_text.push_str(text);
                }
            }
            Destination::Pict => {
                if let Some(frame) = self.frames.last_mut() {
                    if let Some(pict) = frame.pict.as_mut() {
                        pict.hex.extend(
                            text.chars()
                                .filter(|character| character.is_ascii_hexdigit()),
                        );
                    }
                }
            }
            Destination::ListTable | Destination::ListOverride | Destination::Skipped => {}
        }
    }

    fn literal(&mut self, character: char) {
        match self.destination() {
            Destination::Body | Destination::FieldResult => {
                if self.take_skip() {
                    return;
                }

                self.flush_bytes();
                let style = self.current_char_style();
                self.push_text(&character.to_string(), &style);
            }
            Destination::FontTable => {
                if let Some(entry) = self.font_entry.as_mut() {
                    entry.name.push(character);
                }
            }
            Destination::StyleSheet => {
                if let Some(entry) = self.style_entry.as_mut() {
                    entry.name.push(character);
                }
            }
            Destination::FieldInstruction => {
                if let Some(frame) = self.frames.last_mut() {
                    frame.field_text.push(character);
                }
            }
            _ => {}
        }
    }

    fn hex_byte(&mut self, byte: u8) {
        match self.destination() {
            Destination::Body | Destination::FieldResult => {
                if self.take_skip() {
                    return;
                }
                self.pending_bytes.push(byte);
            }
            Destination::FontTable => {
                let decoded = decode_in_code_page(&[byte], self.header.code_page);
                if let Some(entry) = self.font_entry.as_mut() {
                    entry.name.push_str(&decoded);
                }
            }
            Destination::StyleSheet => {
                let decoded = decode_in_code_page(&[byte], self.header.code_page);
                if let Some(entry) = self.style_entry.as_mut() {
                    entry.name.push_str(&decoded);
                }
            }
            Destination::Pict => {
                if let Some(frame) = self.frames.last_mut() {
                    if let Some(pict) = frame.pict.as_mut() {
                        pict.hex.push_str(&format!("{byte:02x}"));
                    }
                }
            }
            _ => {}
        }
    }

    fn unicode_character(&mut self, param: Option<i32>) {
        let Some(value) = param else {
            return;
        };
        let character = if value < 0 {
            (value + 65_536) as u32
        } else {
            value as u32
        };
        let Some(character) = char::from_u32(character) else {
            return;
        };

        self.flush_bytes();
        let style = self.current_char_style();
        let mut buffer = [0u8; 4];
        self.push_text(character.encode_utf8(&mut buffer), &style);

        if let Some(frame) = self.frames.last_mut() {
            frame.skip = frame.uc;
        }
    }

    fn flush_bytes(&mut self) {
        if self.pending_bytes.is_empty() {
            return;
        }

        let bytes = std::mem::take(&mut self.pending_bytes);
        let text = decode_in_code_page(&bytes, self.header.code_page);

        if text.is_empty() {
            return;
        }

        let style = self.current_char_style();
        self.push_text(&text, &style);
    }

    fn push_text(&mut self, text: &str, style: &CharStyle) {
        if text.is_empty() {
            return;
        }

        if let Some(InlineItem::Text(run)) = self.paragraph.items.last_mut() {
            if run.style == *style {
                run.text.push_str(text);
                return;
            }
        }

        self.paragraph.items.push(InlineItem::Text(Run {
            text: text.to_owned(),
            style: style.clone(),
        }));
    }

    fn current_char_style(&self) -> CharStyle {
        self.frames
            .last()
            .map(|frame| frame.char_style.clone())
            .unwrap_or_default()
    }

    fn take_skip(&mut self) -> bool {
        if let Some(frame) = self.frames.last_mut() {
            if frame.skip > 0 {
                frame.skip -= 1;
                return true;
            }
        }

        false
    }

    fn destination(&self) -> Destination {
        self.frames
            .last()
            .map(|frame| frame.destination)
            .unwrap_or(Destination::Body)
    }

    fn set_flag(&mut self, flag: impl Fn(&mut CharStyle) -> &mut bool, param: Option<i32>) {
        let value = param.unwrap_or(1) != 0;
        if let Some(frame) = self.frames.last_mut() {
            *flag(&mut frame.char_style) = value;
        }
    }

    fn clear_flag(&mut self, flag: impl Fn(&mut CharStyle) -> &mut bool) {
        if let Some(frame) = self.frames.last_mut() {
            *flag(&mut frame.char_style) = false;
        }
    }

    fn reset_character_style(&mut self) {
        if let Some(frame) = self.frames.last_mut() {
            frame.char_style = CharStyle::default();
        }
    }

    fn reset_paragraph_style(&mut self) {
        self.with_paragraph_style(|style| *style = ParaStyle::default());
    }

    fn with_paragraph_style(&mut self, update: impl FnOnce(&mut ParaStyle)) {
        if let Some(frame) = self.frames.last_mut() {
            update(&mut frame.para_style);
        }
    }

    fn current_paragraph_style(&self) -> ParaStyle {
        self.frames
            .last()
            .map(|frame| frame.para_style.clone())
            .unwrap_or_default()
    }

    fn set_font(&mut self, param: Option<i32>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.char_style.font = param.and_then(|value| u16::try_from(value).ok());
        }
    }

    fn set_font_size(&mut self, param: Option<i32>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.char_style.size = param.and_then(|value| u16::try_from(value).ok());
        }
    }

    fn set_color(&mut self, param: Option<i32>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.char_style.color = param.and_then(|value| u16::try_from(value).ok());
        }
    }

    fn set_highlight(&mut self, param: Option<i32>) {
        if let Some(frame) = self.frames.last_mut() {
            frame.char_style.highlight = param.and_then(|value| u16::try_from(value).ok());
        }
    }

    fn set_align(&mut self, align: Align) {
        self.with_paragraph_style(|style| style.align = align);
    }

    fn font_control(&mut self, name: &str, param: Option<i32>) {
        match name {
            "f" => {
                if let Some(index) = param.and_then(|value| u16::try_from(value).ok()) {
                    self.font_entry = Some(FontEntry {
                        index,
                        name: String::new(),
                    });
                }
            }
            "fcharset" | "fprq" => {}
            _ => {}
        }
    }

    fn commit_font(&mut self, entry: FontEntry) {
        let name = entry.name.trim().trim_end_matches(';').trim().to_owned();

        if !name.is_empty() {
            self.header.fonts.insert(entry.index, name);
        }
    }

    fn color_control(&mut self, name: &str, param: Option<i32>) {
        let value = param
            .and_then(|value| u8::try_from(value.clamp(0, 255)).ok())
            .unwrap_or(0);
        let slot = match name {
            "red" => &mut self.color_acc.0,
            "green" => &mut self.color_acc.1,
            "blue" => &mut self.color_acc.2,
            _ => return,
        };
        *slot = Some(value);
    }

    fn commit_color(&mut self) {
        let color = match (self.color_acc.0, self.color_acc.1, self.color_acc.2) {
            (Some(red), Some(green), Some(blue)) => Some((red, green, blue)),
            _ => None,
        };
        self.header.colors.push(color);
        self.color_acc = (None, None, None);
    }

    fn style_control(&mut self, name: &str, param: Option<i32>) {
        match name {
            "s" => {
                if let Some(index) = param.and_then(|value| u16::try_from(value).ok()) {
                    self.style_entry = Some(StyleEntry {
                        index,
                        heading: None,
                        name: String::new(),
                    });
                }
            }
            "outlinelevel" => {
                if let Some(entry) = self.style_entry.as_mut() {
                    entry.heading = Some(param.unwrap_or(0).clamp(0, 5) as u8 + 1);
                }
            }
            _ => {}
        }
    }

    fn commit_style(&mut self, entry: StyleEntry) {
        let heading = entry.heading.or_else(|| heading_from_name(&entry.name));

        if let Some(heading) = heading {
            self.header.styles.insert(entry.index, heading);
        }
    }

    fn list_control(&mut self, name: &str, param: Option<i32>) {
        match name {
            "list" => {
                self.list_entry = Some(ListEntry {
                    id: None,
                    level: 0,
                    levels: Vec::new(),
                    depth: self.frames.len(),
                });
            }
            "listid" => {
                if let Some(entry) = self.list_entry.as_mut() {
                    entry.id = param.and_then(|value| u16::try_from(value).ok());
                }
            }
            "listlevel" => {
                if let Some(entry) = self.list_entry.as_mut() {
                    entry.level = entry.levels.len();
                }
            }
            "levelnfc" => {
                if let Some(entry) = self.list_entry.as_mut() {
                    let ordered = param.unwrap_or(23) != 23 && param != Some(255);
                    entry.levels.push((entry.level, ordered));
                }
            }
            _ => {}
        }
    }

    fn commit_list(&mut self) {
        if let Some(entry) = self.list_entry.take() {
            if let Some(id) = entry.id {
                for (level, ordered) in entry.levels {
                    self.header.list_levels.insert((id, level as u16), ordered);
                }
            }
        }
    }

    fn override_control(&mut self, name: &str, param: Option<i32>) {
        match name {
            "listid" => {
                self.override_list_id = param.and_then(|value| u16::try_from(value).ok());
            }
            "ls" => {
                if let (Some(list_id), Some(ls)) = (
                    self.override_list_id,
                    param.and_then(|value| u16::try_from(value).ok()),
                ) {
                    self.header.list_overrides.insert(ls, list_id);
                }
            }
            _ => {}
        }
    }

    fn pict_control(&mut self, name: &str, _param: Option<i32>) {
        let Some(frame) = self.frames.last_mut() else {
            return;
        };
        let Some(pict) = frame.pict.as_mut() else {
            return;
        };

        match name {
            "pngblip" => pict.kind = Some(ImageKind::Png),
            "jpegblip" | "jpgblip" => pict.kind = Some(ImageKind::Jpeg),
            "gifblip" => pict.kind = Some(ImageKind::Gif),
            "emfblip" | "wmetafile" | "macpict" | "dibitmap" | "wbitmap" => {
                pict.unsupported = Some(name.to_owned());
            }
            _ => {}
        }
    }

    fn finish_picture(&mut self, pict: PictAccum) {
        if let Some(format) = pict.unsupported {
            self.warnings.warn(format!(
                "Изображения формата {format} не поддерживаются и были пропущены."
            ));
            return;
        }

        let Some(kind) = pict.kind else {
            self.warnings
                .warn("Изображение без формата не поддерживается и было пропущено.");
            return;
        };
        let hex: String = pict
            .hex
            .chars()
            .filter(|character| character.is_ascii_hexdigit())
            .collect();

        if hex.is_empty() {
            return;
        }

        let bytes = decode_hex(&hex);

        if bytes.len() > MAX_IMAGE_BYTES {
            self.warnings
                .warn("Изображение слишком большое и было пропущено (лимит 15 МБ).");
            return;
        }

        let encoded = STANDARD.encode(&bytes);
        let mut image = atom("image");
        image.attrs = Some(json!({
            "src": format!("data:{};base64,{encoded}", kind.mime()),
            "alt": Value::Null,
            "title": Value::Null
        }));
        self.paragraph.items.push(InlineItem::Image(image));
    }

    fn start_row(&mut self) {
        if let Some(row) = self.row.take() {
            if !row.cells.is_empty() {
                self.table_rows
                    .push(PmNode::with_content("tableRow", row.cells));
            }
        }

        self.row = Some(RowDraft {
            header: false,
            cells: Vec::new(),
        });
    }

    fn finish_cell(&mut self) -> Result<(), FormatError> {
        self.flush_bytes();
        self.finish_paragraph(false)?;

        if self.current_cell.is_empty() {
            self.current_cell.push(PmNode::paragraph());
        }

        let Some(header) = self.row.as_ref().map(|row| row.header) else {
            self.current_cell.clear();
            return Ok(());
        };
        let mut cell = PmNode::with_content(
            if header { "tableHeader" } else { "tableCell" },
            std::mem::take(&mut self.current_cell),
        );
        cell.attrs = Some(json!({
            "colspan": 1,
            "rowspan": 1,
            "colwidth": null
        }));
        self.count_node()?;
        if let Some(row) = self.row.as_mut() {
            row.cells.push(cell);
        }
        Ok(())
    }

    fn finish_row(&mut self) -> Result<(), FormatError> {
        self.flush_bytes();

        if !self.current_cell.is_empty() {
            self.finish_cell()?;
        }

        let Some(row) = self.row.take() else {
            return Ok(());
        };

        if !row.cells.is_empty() {
            self.count_node()?;
            self.table_rows
                .push(PmNode::with_content("tableRow", row.cells));
        }

        Ok(())
    }

    fn flush_table(&mut self) {
        if self.table_rows.is_empty() {
            return;
        }

        self.blocks
            .push(Block::Table(std::mem::take(&mut self.table_rows)));
    }

    fn finish_paragraph(&mut self, force: bool) -> Result<(), FormatError> {
        let has_items = !self.paragraph.items.is_empty();
        let style = self.current_paragraph_style();

        if !force && !has_items && !style.bottom_border {
            return Ok(());
        }

        let mut items = std::mem::take(&mut self.paragraph.items);
        trim_items(&mut items);
        let list = self.list_meta(&mut items, &style);
        let content = self.inline_nodes(items)?;
        let attrs = paragraph_attrs(&style, list.is_some());
        let heading = style.heading.or_else(|| {
            style
                .style_index
                .and_then(|index| self.header.styles.get(&index).copied())
        });

        if style.bottom_border && content.is_empty() && !style.in_table {
            self.flush_table();
            self.count_node()?;
            self.blocks.push(Block::Atom(atom("horizontalRule")));
            return Ok(());
        }

        let node = if let Some(level) = heading {
            let mut node = PmNode::with_content("heading", content);
            node.attrs = merge_attrs(Some(json!({ "level": level })), attrs);
            node
        } else {
            let mut node = PmNode::with_content("paragraph", content);
            node.attrs = attrs;
            node
        };

        self.count_node()?;

        if style.in_table {
            self.current_cell.push(node);
            return Ok(());
        }

        self.flush_table();
        self.blocks.push(Block::Paragraph { node, list });

        Ok(())
    }

    fn list_meta(&self, items: &mut [InlineItem], style: &ParaStyle) -> Option<ListMeta> {
        if let Some(ls) = style.list_id {
            let level = style.list_level;
            let ordered = self.resolve_list_kind(ls, level).unwrap_or(false);
            let kind = if ordered {
                ListKind::Ordered
            } else if let Some(checked) = strip_task_marker(items) {
                ListKind::Task { checked }
            } else {
                ListKind::Bullet
            };

            return Some(ListMeta { kind, level });
        }

        if style.indent <= 0 {
            return None;
        }

        let level = ((style.indent / LI_TWIPS_PER_LEVEL) - 1).clamp(0, 8) as u16;

        if strip_bullet_marker(items) {
            let kind = match strip_task_marker(items) {
                Some(checked) => ListKind::Task { checked },
                None => ListKind::Bullet,
            };
            return Some(ListMeta { kind, level });
        }

        if strip_number_marker(items) {
            return Some(ListMeta {
                kind: ListKind::Ordered,
                level,
            });
        }

        None
    }

    fn resolve_list_kind(&self, ls: u16, level: u16) -> Option<bool> {
        let list_id = self.header.list_overrides.get(&ls).copied().unwrap_or(ls);

        self.header
            .list_levels
            .get(&(list_id, level))
            .or_else(|| self.header.list_levels.get(&(list_id, 0)))
            .copied()
    }

    fn inline_nodes(&mut self, items: Vec<InlineItem>) -> Result<Vec<PmNode>, FormatError> {
        let mut content = Vec::new();

        for item in items {
            match item {
                InlineItem::Text(run) => {
                    if run.text.is_empty() {
                        continue;
                    }

                    let marks = self.marks_for(&run.style);

                    if let Some(PmNode {
                        node_type,
                        marks: existing,
                        text: Some(existing_text),
                        ..
                    }) = content.last_mut()
                    {
                        if node_type == "text" && *existing == marks {
                            existing_text.push_str(&run.text);
                            continue;
                        }
                    }

                    self.count_node()?;
                    content.push(PmNode {
                        node_type: "text".to_owned(),
                        attrs: None,
                        content: Vec::new(),
                        marks,
                        text: Some(run.text),
                    });
                }
                InlineItem::Image(image) => {
                    self.count_node()?;
                    content.push(image);
                }
                InlineItem::HardBreak => {
                    self.count_node()?;
                    content.push(atom("hardBreak"));
                }
            }
        }

        Ok(content)
    }

    fn marks_for(&mut self, style: &CharStyle) -> Vec<PmMark> {
        let mut marks = Vec::new();

        if style.bold {
            marks.push(PmMark::new("bold"));
        }
        if style.italic {
            marks.push(PmMark::new("italic"));
        }
        if style.underline {
            marks.push(PmMark::new("underline"));
        }
        if style.strike {
            marks.push(PmMark::new("strike"));
        }

        let mut text_style = Map::new();

        if let Some(font) = style.font {
            if let Some(name) = self.header.fonts.get(&font) {
                if !name.eq_ignore_ascii_case(DEFAULT_FONT_NAME) {
                    text_style.insert("fontFamily".to_owned(), json!(canonical_font_family(name)));
                }
            }
        }

        if let Some(size) = style.size {
            if size != DEFAULT_FONT_SIZE_HALF_POINTS {
                text_style.insert("fontSize".to_owned(), json!(font_size_points(size)));
            }
        }

        if let Some(color) = style
            .color
            .and_then(|index| self.resolve_color(index))
            .filter(|color| *color != (0, 0, 0))
        {
            text_style.insert("color".to_owned(), json!(format_color(color)));
        }

        if !text_style.is_empty() {
            let mut mark = PmMark::new("textStyle");
            mark.attrs = Some(Value::Object(text_style));
            marks.push(mark);
        }

        if let Some(color) = style
            .highlight
            .and_then(|index| self.resolve_color(index))
            .filter(|color| *color != (0, 0, 0))
        {
            let mut mark = PmMark::new("highlight");
            mark.attrs = Some(json!({ "color": format_color(color) }));
            marks.push(mark);
        }

        if let Some(link) = style.link.as_deref() {
            if is_safe_link(link) {
                let mut mark = PmMark::new("link");
                mark.attrs = Some(json!({
                    "href": link,
                    "target": Value::Null,
                    "rel": Value::Null,
                    "class": Value::Null
                }));
                marks.push(mark);
            } else {
                self.warnings
                    .warn("Ссылка с небезопасным адресом была преобразована в текст.");
            }
        }

        marks
    }

    fn resolve_color(&self, index: u16) -> Option<(u8, u8, u8)> {
        self.header.colors.get(index as usize).copied().flatten()
    }

    fn warn_unsupported(&mut self, name: &str) {
        let message = match name {
            "header" | "footer" => "Колонтитулы не поддерживаются и были пропущены.",
            "footnote" | "endnote" => "Сноски не поддерживаются и были пропущены.",
            "annotation" => "Комментарии не поддерживаются и были пропущены.",
            "object" => "Встроенные объекты не поддерживаются и были пропущены.",
            "shppict" => "Фигуры и текстовые поля не поддерживаются и были пропущены.",
            _ => return,
        };
        self.warnings.warn(message);
    }

    fn finish(mut self) -> Result<ImportedDocument, FormatError> {
        self.flush_bytes();
        if !self.paragraph.items.is_empty() {
            self.finish_paragraph(false)?;
        }
        self.flush_table();

        if let Some(row) = self.row.take() {
            if !row.cells.is_empty() {
                self.table_rows
                    .push(PmNode::with_content("tableRow", row.cells));
            }
        }
        self.flush_table();

        let content = finalize_blocks(self.blocks);

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
            warnings: self.warnings.into_warnings(),
        })
    }
}

fn destination_for(name: &str) -> Option<Destination> {
    let destination = match name {
        "fonttbl" => Destination::FontTable,
        "colortbl" => Destination::ColorTable,
        "stylesheet" => Destination::StyleSheet,
        "listtable" => Destination::ListTable,
        "listoverride" | "listoverridetable" => Destination::ListOverride,
        "pict" => Destination::Pict,
        "fldinst" => Destination::FieldInstruction,
        "fldrslt" => Destination::FieldResult,
        name if SKIPPED_DESTINATIONS.contains(&name) => Destination::Skipped,
        _ => return None,
    };

    Some(destination)
}

fn heading_from_name(name: &str) -> Option<u8> {
    let lowered = name
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_ascii_lowercase();
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

fn paragraph_attrs(style: &ParaStyle, is_list: bool) -> Option<Value> {
    let mut attrs = Map::new();

    match style.align {
        Align::Left => {}
        Align::Center => {
            attrs.insert("textAlign".to_owned(), json!("center"));
        }
        Align::Right => {
            attrs.insert("textAlign".to_owned(), json!("right"));
        }
        Align::Justify => {
            attrs.insert("textAlign".to_owned(), json!("justify"));
        }
    }

    if !is_list && style.indent > 0 {
        let indent = ((style.indent as f32) / 480.0).round().clamp(0.0, 8.0) as u64;

        if indent > 0 {
            attrs.insert("indent".to_owned(), json!(indent));
        }
    }

    if style.space_before > 0 {
        attrs.insert(
            "spaceBefore".to_owned(),
            json!((style.space_before as f32 / 20.0).round() as u64),
        );
    }

    if style.space_after > 0 {
        attrs.insert(
            "spaceAfter".to_owned(),
            json!((style.space_after as f32 / 20.0).round() as u64),
        );
    }

    if style.line_multiple.unwrap_or(0) == 1 {
        if let Some(line_height) = style.line_height.and_then(line_height_from_twips) {
            attrs.insert("lineHeight".to_owned(), json!(line_height));
        }
    }

    if attrs.is_empty() {
        None
    } else {
        Some(Value::Object(attrs))
    }
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

#[allow(clippy::ptr_arg)]
fn trim_items(items: &mut Vec<InlineItem>) {
    if let Some(InlineItem::Text(run)) = items.first_mut() {
        let trimmed = run.text.trim_start().to_owned();
        run.text = trimmed;
    }

    if let Some(InlineItem::Text(run)) = items.last_mut() {
        let trimmed = run.text.trim_end().to_owned();
        run.text = trimmed;
    }

    items.retain(|item| match item {
        InlineItem::Text(run) => !run.text.is_empty(),
        _ => true,
    });
}

fn strip_bullet_marker(items: &mut [InlineItem]) -> bool {
    let Some(InlineItem::Text(run)) = items.first_mut() else {
        return false;
    };
    let trimmed = run.text.trim_start();
    let Some(rest) = trimmed
        .strip_prefix('\u{2022}')
        .or_else(|| trimmed.strip_prefix('\u{00b7}'))
        .or_else(|| trimmed.strip_prefix('-'))
    else {
        return false;
    };

    run.text = rest.trim_start_matches([' ', '\t']).to_owned();

    true
}

fn strip_task_marker(items: &mut [InlineItem]) -> Option<bool> {
    let InlineItem::Text(run) = items.first_mut()? else {
        return None;
    };
    let trimmed = run.text.trim_start();
    let (checked, rest) = trimmed
        .strip_prefix("[x] ")
        .map(|rest| (true, rest))
        .or_else(|| trimmed.strip_prefix("[X] ").map(|rest| (true, rest)))
        .or_else(|| trimmed.strip_prefix("[ ] ").map(|rest| (false, rest)))?;

    run.text = rest.to_owned();

    Some(checked)
}

fn strip_number_marker(items: &mut [InlineItem]) -> bool {
    let Some(InlineItem::Text(run)) = items.first_mut() else {
        return false;
    };
    let trimmed = run.text.trim_start();
    let digits: String = trimmed.chars().take_while(char::is_ascii_digit).collect();

    if digits.is_empty() {
        return false;
    }

    let rest = &trimmed[digits.len()..];
    let rest = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'));

    let Some(rest) = rest else {
        return false;
    };

    if !rest.starts_with(' ') && !rest.starts_with('\t') {
        return false;
    }

    run.text = rest.trim_start_matches([' ', '\t']).to_owned();

    true
}

fn finalize_blocks(blocks: Vec<Block>) -> Vec<PmNode> {
    let mut content = Vec::new();
    let mut index = 0;

    while index < blocks.len() {
        match &blocks[index] {
            Block::Paragraph { list: Some(_), .. } => {
                let mut entries = Vec::new();
                let mut previous_level = 0;

                while index < blocks.len() {
                    let Block::Paragraph {
                        node,
                        list: Some(meta),
                    } = &blocks[index]
                    else {
                        break;
                    };

                    let level = meta.level.min(previous_level + 1);
                    previous_level = level;
                    entries.push(ListEntryFinal {
                        level,
                        kind: meta.kind.clone(),
                        node: node.clone(),
                    });
                    index += 1;
                }

                let mut entry_index = 0;

                while entry_index < entries.len() {
                    let kind = entries[entry_index].kind.clone();
                    content.push(build_list(&entries, &mut entry_index, 0, &kind));
                }
            }
            Block::Paragraph { node, .. } => {
                content.push(node.clone());
                index += 1;
            }
            Block::Atom(node) => {
                content.push(node.clone());
                index += 1;
            }
            Block::Table(rows) => {
                content.push(PmNode::with_content("table", rows.clone()));
                index += 1;
            }
        }
    }

    content
}

struct ListEntryFinal {
    level: u16,
    kind: ListKind,
    node: PmNode,
}

fn build_list(
    entries: &[ListEntryFinal],
    index: &mut usize,
    level: u16,
    kind: &ListKind,
) -> PmNode {
    let list_type = match kind {
        ListKind::Ordered => "orderedList",
        ListKind::Bullet => "bulletList",
        ListKind::Task { .. } => "taskList",
    };
    let mut items = Vec::new();

    while *index < entries.len() {
        let entry = &entries[*index];

        if entry.level != level {
            break;
        }

        if !same_family(&entry.kind, kind) {
            break;
        }

        *index += 1;

        let mut item_content = vec![entry.node.clone()];

        while *index < entries.len() && entries[*index].level > level {
            let child = &entries[*index];
            let child_level = child.level.min(level + 1);
            let nested = build_list(entries, index, child_level, &child.kind);
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

fn same_family(left: &ListKind, right: &ListKind) -> bool {
    matches!(
        (left, right),
        (ListKind::Ordered, ListKind::Ordered)
            | (ListKind::Bullet, ListKind::Bullet)
            | (ListKind::Task { .. }, ListKind::Task { .. })
    )
}

enum FieldLink {
    Missing,
    Safe(String),
    Unsafe,
}

fn parse_field_link(text: &str) -> FieldLink {
    let trimmed = text.trim();
    let Some(prefix) = trimmed.get(..9) else {
        return FieldLink::Missing;
    };

    if !prefix.eq_ignore_ascii_case("HYPERLINK") {
        return FieldLink::Missing;
    }

    let rest = trimmed[9..].trim();

    if rest.is_empty() {
        return FieldLink::Missing;
    }

    let link = if let Some(rest) = rest.strip_prefix('"') {
        rest.split('"').next().unwrap_or_default()
    } else {
        rest.split_whitespace().next().unwrap_or_default()
    };

    if is_safe_link(link) {
        FieldLink::Safe(link.to_owned())
    } else {
        FieldLink::Unsafe
    }
}

fn decode_hex(hex: &str) -> Vec<u8> {
    let digits: Vec<u32> = hex
        .chars()
        .filter_map(|character| character.to_digit(16))
        .collect();
    let mut bytes = Vec::with_capacity(digits.len() / 2);

    for pair in digits.as_chunks::<2>().0 {
        bytes.push(((pair[0] << 4) | pair[1]) as u8);
    }

    bytes
}

fn atom(node_type: &str) -> PmNode {
    PmNode::with_content(node_type, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use formats_core::node_text;

    #[test]
    fn rejects_documents_over_node_limit() {
        let error = parse_with_limit(r"{\rtf1\ansi\pard один\par два\par три\par}", 2)
            .expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("узлов"));
    }

    #[test]
    fn warns_when_unsafe_hyperlink_is_dropped() {
        let imported = parse(
            r#"{\rtf1\ansi\pard {\field{\*\fldinst{HYPERLINK "javascript&colon;alert(1)"}}{\fldrslt текст}}\par}"#,
        )
        .expect("чтение RTF");

        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("небезопасным адресом")));
        assert_eq!(node_text(&imported.document.content[0]), "текст");
        assert!(imported.document.content[0]
            .content
            .iter()
            .all(|node| node.marks.iter().all(|mark| mark.mark_type != "link")));
    }
}
