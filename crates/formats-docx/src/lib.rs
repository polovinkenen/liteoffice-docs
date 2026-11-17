mod read;
mod units;
mod writer;

use std::io::Cursor;

use wordlite_docmodel::PmDoc;
use wordlite_formats_core::{
    DocumentCodec, FormatError, ImportedDocument, WrittenDocument, WrittenFile,
};

const MAX_ENTRIES: usize = 10_000;
const MAX_UNCOMPRESSED_BYTES: u64 = 200 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 100 * 1024 * 1024;
const MAX_COMPRESSION_RATIO: u64 = 200;

pub struct DocxCodec;

static DOCX_CODEC: DocxCodec = DocxCodec;

pub fn codec() -> &'static dyn DocumentCodec {
    &DOCX_CODEC
}

impl DocumentCodec for DocxCodec {
    fn read(&self, _source: &str) -> Result<ImportedDocument, FormatError> {
        Err(FormatError::Parse(
            "DOCX — бинарный формат, требуется чтение байтов".to_owned(),
        ))
    }

    fn write(&self, _document: &PmDoc) -> Result<WrittenDocument, FormatError> {
        Err(FormatError::Write(
            "DOCX — бинарный формат, требуется запись байтов".to_owned(),
        ))
    }

    fn read_bytes(&self, bytes: &[u8]) -> Result<ImportedDocument, FormatError> {
        validate_archive(bytes)?;
        read::read_document(bytes)
    }

    fn write_bytes(&self, document: &PmDoc) -> Result<WrittenFile, FormatError> {
        writer::write_document(document)
    }
}

fn validate_archive(bytes: &[u8]) -> Result<(), FormatError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| FormatError::Parse(format!("некорректный DOCX-архив: {error}")))?;

    if archive.len() > MAX_ENTRIES {
        return Err(FormatError::Parse(
            "DOCX содержит слишком много файлов".to_owned(),
        ));
    }

    let mut total = 0u64;

    for index in 0..archive.len() {
        let entry = archive
            .by_index_raw(index)
            .map_err(|error| FormatError::Parse(format!("некорректный DOCX-архив: {error}")))?;
        let size = entry.size();
        let compressed = entry.compressed_size().max(1);

        if size > MAX_ENTRY_BYTES {
            return Err(FormatError::Parse(
                "файл внутри DOCX слишком большой".to_owned(),
            ));
        }

        total = total.saturating_add(size);

        if total > MAX_UNCOMPRESSED_BYTES {
            return Err(FormatError::Parse(
                "распакованный DOCX слишком большой".to_owned(),
            ));
        }

        if size / compressed > MAX_COMPRESSION_RATIO {
            return Err(FormatError::Parse(
                "DOCX похож на zip-bomb: слишком высокая степень сжатия".to_owned(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordlite_docmodel::PmNode;

    fn document() -> PmDoc {
        PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content(
                "paragraph",
                vec![PmNode::text("Привет")],
            )],
        }
    }

    #[test]
    fn writes_and_reads_docx() {
        let written = writer::write_document(&document()).expect("запись DOCX");
        let imported = codec().read_bytes(&written.bytes).expect("чтение DOCX");

        assert_eq!(imported.document, document());
    }

    #[test]
    fn rejects_invalid_archive() {
        let error = codec()
            .read_bytes(b"not a zip")
            .expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("архив"));
    }

    #[test]
    fn rejects_too_many_entries() {
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buffer);

        for index in 0..(MAX_ENTRIES + 1) {
            writer
                .start_file(
                    format!("file-{index}.txt"),
                    zip::write::SimpleFileOptions::default(),
                )
                .expect("файл");
            std::io::Write::write_all(&mut writer, b"x").expect("запись");
        }

        writer.finish().expect("архив");
        let bytes = buffer.into_inner();

        let error = codec().read_bytes(&bytes).expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("много файлов"));
    }
}

#[cfg(test)]
mod feature_tests {
    use super::*;
    use serde_json::{json, Value};

    fn round_trip(value: Value) -> (PmDoc, PmDoc) {
        let document: PmDoc = serde_json::from_value(value).expect("документ");
        let written = codec().write_bytes(&document).expect("запись DOCX");
        let imported = codec().read_bytes(&written.bytes).expect("чтение DOCX");

        assert!(imported.document.validate().is_ok());

        (document, imported.document)
    }

    #[test]
    fn round_trips_formatting_and_paragraph_attrs() {
        let (original, reopened) = round_trip(json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "attrs": {
                    "textAlign": "center",
                    "indent": 1,
                    "lineHeight": "1.5",
                    "spaceBefore": 6,
                    "spaceAfter": 4
                },
                "content": [
                    { "type": "text", "text": "обычный " },
                    { "type": "text", "text": "жирный", "marks": [{ "type": "bold" }] },
                    { "type": "text", "text": " " },
                    { "type": "text", "text": "курсив", "marks": [{ "type": "italic" }] },
                    { "type": "text", "text": " " },
                    { "type": "text", "text": "подчёркнутый", "marks": [{ "type": "underline" }] },
                    { "type": "text", "text": " " },
                    { "type": "text", "text": "зачёркнутый", "marks": [{ "type": "strike" }] },
                    {
                        "type": "text",
                        "text": "стиль",
                        "marks": [{
                            "type": "textStyle",
                            "attrs": {
                                "fontFamily": "Georgia, serif",
                                "fontSize": "14pt",
                                "color": "#ff0000"
                            }
                        }]
                    },
                    {
                        "type": "text",
                        "text": "выделение",
                        "marks": [{
                            "type": "highlight",
                            "attrs": { "color": "#ffff00" }
                        }]
                    }
                ]
            }]
        }));

        assert_eq!(reopened, original);
    }

    #[test]
    fn round_trips_heading_and_paragraph_without_attrs() {
        let (original, reopened) = round_trip(json!({
            "type": "doc",
            "content": [
                {
                    "type": "heading",
                    "attrs": { "level": 2 },
                    "content": [{ "type": "text", "text": "Заголовок" }]
                },
                {
                    "type": "paragraph",
                    "content": [{ "type": "text", "text": "Текст" }]
                }
            ]
        }));

        assert_eq!(reopened, original);
    }

    #[test]
    fn round_trips_lists() {
        let (original, reopened) = round_trip(json!({
            "type": "doc",
            "content": [
                {
                    "type": "bulletList",
                    "content": [
                        {
                            "type": "listItem",
                            "content": [
                                { "type": "paragraph", "content": [{ "type": "text", "text": "Первый" }] },
                                {
                                    "type": "orderedList",
                                    "content": [{
                                        "type": "listItem",
                                        "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Вложенный" }] }]
                                    }]
                                }
                            ]
                        },
                        {
                            "type": "listItem",
                            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Второй" }] }]
                        }
                    ]
                },
                {
                    "type": "taskList",
                    "content": [
                        {
                            "type": "taskItem",
                            "attrs": { "checked": true },
                            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Сделано" }] }]
                        },
                        {
                            "type": "taskItem",
                            "attrs": { "checked": false },
                            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Не сделано" }] }]
                        }
                    ]
                }
            ]
        }));

        assert_eq!(reopened, original);
    }

    #[test]
    fn round_trips_tables_with_spans() {
        let (original, reopened) = round_trip(json!({
            "type": "doc",
            "content": [{
                "type": "table",
                "content": [
                    {
                        "type": "tableRow",
                        "content": [{
                            "type": "tableHeader",
                            "attrs": { "colspan": 2, "rowspan": 1, "colwidth": null },
                            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Заголовок" }] }]
                        }]
                    },
                    {
                        "type": "tableRow",
                        "content": [
                            {
                                "type": "tableCell",
                                "attrs": { "colspan": 1, "rowspan": 2, "colwidth": null },
                                "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "A" }] }]
                            },
                            {
                                "type": "tableCell",
                                "attrs": { "colspan": 1, "rowspan": 1, "colwidth": null },
                                "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "B" }] }]
                            }
                        ]
                    },
                    {
                        "type": "tableRow",
                        "content": [{
                            "type": "tableCell",
                            "attrs": { "colspan": 1, "rowspan": 1, "colwidth": null },
                            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "C" }] }]
                        }]
                    }
                ]
            }]
        }));

        assert_eq!(reopened, original);
    }

    #[test]
    fn round_trips_images_links_page_break_and_rule() {
        let png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/lK3iAAAAAElFTkSuQmCC";
        let (original, reopened) = round_trip(json!({
            "type": "doc",
            "content": [
                {
                    "type": "paragraph",
                    "content": [
                        {
                            "type": "text",
                            "text": "ссылка",
                            "marks": [{
                                "type": "link",
                                "attrs": {
                                    "href": "https://example.com",
                                    "target": null,
                                    "rel": null,
                                    "class": null
                                }
                            }]
                        },
                        {
                            "type": "image",
                            "attrs": {
                                "src": format!("data:image/png;base64,{png}"),
                                "alt": null,
                                "title": null
                            }
                        }
                    ]
                },
                { "type": "pageBreak" },
                { "type": "horizontalRule" }
            ]
        }));

        assert_eq!(reopened, original);
    }
}

#[cfg(test)]
mod warning_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn warns_about_headers_and_footers() {
        use docx_rs::{Docx, Footer, Header, Paragraph, Run};

        let mut buffer = Cursor::new(Vec::new());
        let docx = Docx::new()
            .header(
                Header::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text("верх"))),
            )
            .footer(
                Footer::new().add_paragraph(Paragraph::new().add_run(Run::new().add_text("низ"))),
            )
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("тело")));
        docx.build().pack(&mut buffer).expect("сборка DOCX");

        let imported = codec()
            .read_bytes(&buffer.into_inner())
            .expect("чтение DOCX");

        assert!(imported
            .warnings
            .iter()
            .any(|warning| warning.contains("Колонтитулы")));
    }
}
