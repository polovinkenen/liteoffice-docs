mod read;
mod units;
mod writer;

use std::io::{Cursor, Read};

use wlite_docmodel::PmDoc;
use wlite_formats_core::{
    DocumentCodec, FormatError, ImportedDocument, WrittenDocument, WrittenFile,
};

const MAX_ENTRIES: usize = 10_000;
const MAX_UNCOMPRESSED_BYTES: u64 = 200 * 1024 * 1024;
const MAX_ENTRY_BYTES: u64 = 100 * 1024 * 1024;
const MAX_COMPRESSION_RATIO: u64 = 1000;
const MIN_RATIO_CHECK_BYTES: u64 = 1024 * 1024;

#[derive(Clone, Copy)]
struct ArchiveLimits {
    max_entries: usize,
    max_uncompressed_bytes: u64,
    max_entry_bytes: u64,
    max_compression_ratio: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_entries: MAX_ENTRIES,
            max_uncompressed_bytes: MAX_UNCOMPRESSED_BYTES,
            max_entry_bytes: MAX_ENTRY_BYTES,
            max_compression_ratio: MAX_COMPRESSION_RATIO,
        }
    }
}

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
    validate_archive_with_limits(bytes, ArchiveLimits::default())
}

fn validate_archive_with_limits(bytes: &[u8], limits: ArchiveLimits) -> Result<(), FormatError> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| FormatError::Parse(format!("некорректный DOCX-архив: {error}")))?;

    if archive.len() > limits.max_entries {
        return Err(FormatError::Parse(
            "DOCX содержит слишком много файлов".to_owned(),
        ));
    }

    let mut declared_total = 0u64;
    let mut actual_total = 0u64;

    for index in 0..archive.len() {
        // Заголовки архива недоверенные: сначала быстрые проверки по заявленным
        // размерам, чтобы отсечь заведомо огромные записи без распаковки.
        let (declared, compressed) = {
            let entry = archive
                .by_index_raw(index)
                .map_err(|error| FormatError::Parse(format!("некорректный DOCX-архив: {error}")))?;
            let declared = entry.size();
            let compressed = entry.compressed_size().max(1);

            if declared > limits.max_entry_bytes {
                return Err(FormatError::Parse(
                    "файл внутри DOCX слишком большой".to_owned(),
                ));
            }

            declared_total = declared_total.saturating_add(declared);

            if declared_total > limits.max_uncompressed_bytes {
                return Err(FormatError::Parse(
                    "распакованный DOCX слишком большой".to_owned(),
                ));
            }

            (declared, compressed)
        };

        // Затем реально распаковываем запись с жёстким лимитом, не аллоцируя её
        // целиком: читаем не больше `max_entry_bytes + 1` байт в `sink`.
        let mut entry = archive.by_index(index).map_err(|error| {
            FormatError::Parse(format!("не удалось распаковать файл внутри DOCX: {error}"))
        })?;
        let limit = limits.max_entry_bytes.saturating_add(1);
        let actual = std::io::copy(&mut (&mut entry).take(limit), &mut std::io::sink()).map_err(
            |error| FormatError::Parse(format!("не удалось распаковать файл внутри DOCX: {error}")),
        )?;

        if actual > limits.max_entry_bytes {
            return Err(FormatError::Parse(
                "файл внутри DOCX слишком большой".to_owned(),
            ));
        }

        // Расхождение фактического размера с заголовком означает подделку или
        // повреждение: второй проход docx-rs аллоцирует буфер по заявленному
        // размеру, поэтому такому архиву доверять нельзя.
        if actual != declared {
            return Err(FormatError::Parse(
                "размер файла в DOCX не совпадает с заголовком архива".to_owned(),
            ));
        }

        actual_total = actual_total.saturating_add(actual);

        if actual_total > limits.max_uncompressed_bytes {
            return Err(FormatError::Parse(
                "распакованный DOCX слишком большой".to_owned(),
            ));
        }

        if actual >= MIN_RATIO_CHECK_BYTES && actual / compressed > limits.max_compression_ratio {
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
    use wlite_docmodel::PmNode;

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

    const ZIP_LOCAL_HEADER: u32 = 0x0403_4b50;
    const ZIP_CENTRAL_HEADER: u32 = 0x0201_4b50;
    const LOCAL_UNCOMPRESSED_SIZE_OFFSET: usize = 22;
    const CENTRAL_UNCOMPRESSED_SIZE_OFFSET: usize = 24;

    /// Подменяет заявленный несжатый размер записи в local и central header.
    fn forge_declared_uncompressed_size(bytes: &mut [u8], declared: u32) {
        let mut offset = 0;

        while offset + 4 <= bytes.len() {
            let signature =
                u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 байта"));
            let size_offset = match signature {
                ZIP_LOCAL_HEADER => Some(offset + LOCAL_UNCOMPRESSED_SIZE_OFFSET),
                ZIP_CENTRAL_HEADER => Some(offset + CENTRAL_UNCOMPRESSED_SIZE_OFFSET),
                _ => None,
            };

            if let Some(size_offset) = size_offset {
                if size_offset + 4 <= bytes.len() {
                    bytes[size_offset..size_offset + 4].copy_from_slice(&declared.to_le_bytes());
                }
            }

            offset += 1;
        }
    }

    fn archive_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buffer);

        for (name, data) in entries {
            writer
                .start_file(
                    *name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .expect("файл");
            std::io::Write::write_all(&mut writer, data).expect("запись");
        }

        writer.finish().expect("архив");
        buffer.into_inner()
    }

    #[test]
    fn rejects_too_large_archive_entry() {
        let limits = ArchiveLimits {
            max_entry_bytes: 4,
            ..ArchiveLimits::default()
        };
        let bytes = archive_with_entries(&[("file.txt", b"12345")]);

        let error = validate_archive_with_limits(&bytes, limits).expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("слишком большой"));
    }

    #[test]
    fn rejects_archive_with_too_large_total_size() {
        let limits = ArchiveLimits {
            max_uncompressed_bytes: 10,
            ..ArchiveLimits::default()
        };
        let bytes = archive_with_entries(&[("a.txt", b"123456"), ("b.txt", b"123456")]);

        let error = validate_archive_with_limits(&bytes, limits).expect_err("ожидалась ошибка");

        assert!(error
            .to_string()
            .contains("распакованный DOCX слишком большой"));
    }

    #[test]
    fn rejects_archive_with_high_compression_ratio() {
        let limits = ArchiveLimits {
            max_compression_ratio: 2,
            ..ArchiveLimits::default()
        };
        let data = vec![0u8; 2 * 1024 * 1024];
        let bytes = archive_with_entries(&[("zeros.txt", &data)]);

        let error = validate_archive_with_limits(&bytes, limits).expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("zip-bomb"));
    }

    fn repack_with_deflate(bytes: &[u8]) -> Vec<u8> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("архив");
        let mut buffer = Cursor::new(Vec::new());
        let mut writer = zip::ZipWriter::new(&mut buffer);

        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).expect("запись");
            let name = entry.name().to_owned();
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            if entry.is_dir() {
                writer.add_directory(name, options).expect("каталог");
                continue;
            }

            writer.start_file(name, options).expect("файл");
            std::io::copy(&mut entry, &mut writer).expect("копия");
        }

        writer.finish().expect("архив");
        buffer.into_inner()
    }

    #[test]
    fn accepts_deflated_docx_with_high_compression_ratio() {
        // Текст длиннее 1 MiB, чтобы ветка проверки ratio действительно исполнялась.
        // Повторяющийся, но неоднородный шаблон держит степень сжатия заметно ниже 1000.
        let text = "abcdefghijklmnopqrstuvwxyz0123456789".repeat(50_000);
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![PmNode::with_content("paragraph", vec![PmNode::text(text)])],
        };
        let written = writer::write_document(&document).expect("запись DOCX");
        let repacked = repack_with_deflate(&written.bytes);
        let imported = codec().read_bytes(&repacked).expect("чтение DOCX");

        assert_eq!(imported.document, document);
    }

    #[test]
    fn rejects_forged_uncompressed_size_header() {
        // 2 МиБ нулей сжимаются в несколько килобайт, но заголовок заявляет 1000
        // байт — именно такая подделка раньше обходила лимиты и позволяла
        // docx-rs аллоцировать буфер по фактическому размеру.
        let data = vec![0u8; 2 * 1024 * 1024];
        let mut bytes = archive_with_entries(&[("zeros.txt", &data)]);
        forge_declared_uncompressed_size(&mut bytes, 1000);

        let error = codec()
            .read_bytes(&bytes)
            .expect_err("подделанный архив должен отклоняться");

        assert!(matches!(&error, FormatError::Parse(_)));
        assert!(
            error.to_string().contains("не совпадает"),
            "неожиданная ошибка: {error}"
        );
    }

    #[test]
    fn rejects_documents_over_node_limit() {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/docx/basic.docx"),
        )
        .expect("фикстура");
        let limits = read::ReadLimits {
            max_nodes: 1,
            max_depth: wlite_docmodel::MAX_DEPTH,
        };

        let error = read::read_document_with_limits(&bytes, limits).expect_err("ожидалась ошибка");

        assert!(error.to_string().contains("узлов"));
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
