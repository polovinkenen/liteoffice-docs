use docmodel::PmDoc;
use export_pdf::{render, Margins, Orientation, PaperSize, PdfOptions};
use serde_json::json;

const TINY_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

fn rich_document() -> PmDoc {
    serde_json::from_value(json!({
        "type": "doc",
        "content": [
            {
                "type": "heading",
                "attrs": { "level": 1 },
                "content": [{ "type": "text", "text": "Отчёт" }]
            },
            {
                "type": "paragraph",
                "attrs": { "textAlign": "center", "lineHeight": "1.5", "spaceAfter": 6 },
                "content": [
                    { "type": "text", "text": "Обычный " },
                    { "type": "text", "text": "жирный", "marks": [{ "type": "bold" }] },
                    { "type": "text", "text": " и " },
                    { "type": "text", "text": "курсив", "marks": [{ "type": "italic" }] },
                    { "type": "text", "text": " " },
                    {
                        "type": "text",
                        "text": "ссылка",
                        "marks": [{ "type": "link", "attrs": { "href": "https://example.com" } }]
                    }
                ]
            },
            {
                "type": "bulletList",
                "content": [
                    {
                        "type": "listItem",
                        "content": [
                            {
                                "type": "paragraph",
                                "content": [{ "type": "text", "text": "пункт" }]
                            },
                            {
                                "type": "bulletList",
                                "content": [
                                    {
                                        "type": "listItem",
                                        "content": [{
                                            "type": "paragraph",
                                            "content": [{ "type": "text", "text": "вложенный" }]
                                        }]
                                    }
                                ]
                            }
                        ]
                    }
                ]
            },
            {
                "type": "orderedList",
                "content": [
                    {
                        "type": "listItem",
                        "content": [{
                            "type": "paragraph",
                            "content": [{ "type": "text", "text": "первый" }]
                        }]
                    }
                ]
            },
            {
                "type": "taskList",
                "content": [
                    {
                        "type": "taskItem",
                        "attrs": { "checked": true },
                        "content": [{
                            "type": "paragraph",
                            "content": [{ "type": "text", "text": "готово" }]
                        }]
                    },
                    {
                        "type": "taskItem",
                        "attrs": { "checked": false },
                        "content": [{
                            "type": "paragraph",
                            "content": [{ "type": "text", "text": "сделать" }]
                        }]
                    }
                ]
            },
            {
                "type": "blockquote",
                "content": [{
                    "type": "paragraph",
                    "content": [{ "type": "text", "text": "цитата" }]
                }]
            },
            {
                "type": "codeBlock",
                "content": [{ "type": "text", "text": "let x = 1;\nprintln!(\"{x}\");" }]
            },
            {
                "type": "table",
                "content": [
                    {
                        "type": "tableRow",
                        "content": [
                            {
                                "type": "tableHeader",
                                "attrs": { "colspan": 2, "rowspan": 1 },
                                "content": [{
                                    "type": "paragraph",
                                    "content": [{ "type": "text", "text": "Шапка" }]
                                }]
                            }
                        ]
                    },
                    {
                        "type": "tableRow",
                        "content": [
                            {
                                "type": "tableCell",
                                "attrs": { "colspan": 1, "rowspan": 2 },
                                "content": [{
                                    "type": "paragraph",
                                    "content": [{ "type": "text", "text": "объединено" }]
                                }]
                            },
                            {
                                "type": "tableCell",
                                "attrs": { "colspan": 1, "rowspan": 1 },
                                "content": [{
                                    "type": "paragraph",
                                    "content": [{ "type": "text", "text": "раз" }]
                                }]
                            }
                        ]
                    },
                    {
                        "type": "tableRow",
                        "content": [
                            {
                                "type": "tableCell",
                                "attrs": { "colspan": 1, "rowspan": 1 },
                                "content": [{
                                    "type": "paragraph",
                                    "content": [{ "type": "text", "text": "два" }]
                                }]
                            }
                        ]
                    }
                ]
            },
            {
                "type": "paragraph",
                "content": [{
                    "type": "image",
                    "attrs": {
                        "src": format!("data:image/png;base64,{TINY_PNG}"),
                        "alt": "схема"
                    }
                }]
            },
            { "type": "horizontalRule" },
            { "type": "pageBreak" },
            {
                "type": "paragraph",
                "content": [{ "type": "text", "text": "После разрыва" }]
            }
        ]
    }))
    .expect("документ")
}

fn media_box(pdf: &lopdf::Document, id: lopdf::ObjectId) -> Vec<f64> {
    let mut current = Some(id);
    let mut seen = 0;

    while let Some(id) = current {
        seen += 1;

        if seen > 10 {
            break;
        }

        let Ok(dict) = pdf.get_object(id).and_then(|object| object.as_dict()) else {
            break;
        };

        if let Ok(array) = dict.get(b"MediaBox").and_then(|value| value.as_array()) {
            return array
                .iter()
                .filter_map(|value| {
                    value
                        .as_float()
                        .ok()
                        .map(|v| v as f64)
                        .or_else(|| value.as_i64().ok().map(|v| v as f64))
                })
                .collect();
        }

        current = dict
            .get(b"Parent")
            .ok()
            .and_then(|value| value.as_reference().ok());
    }

    Vec::new()
}

#[test]
fn renders_rich_document_without_typst_errors() {
    let document = rich_document();
    let output = render(&document, &PdfOptions::default(), Some("Отчёт")).expect("рендер");

    assert!(output.bytes.starts_with(b"%PDF-"));
    assert!(output.pages >= 2);
    assert!(
        output.warnings.is_empty(),
        "неожиданные предупреждения: {:#?}",
        output.warnings
    );

    let pdf = lopdf::Document::load_mem(&output.bytes).expect("PDF читается lopdf");
    assert_eq!(pdf.get_pages().len(), output.pages);

    let text = pdf_extract::extract_text_from_mem(&output.bytes).expect("текст извлекается");
    assert!(text.contains("Отчёт"), "{text}");
    assert!(text.contains("После разрыва"), "{text}");
}

#[test]
fn renders_fifty_pages_with_tables_and_images() {
    let mut content = Vec::new();

    for section in 0..60 {
        content.push(json!({
            "type": "heading",
            "attrs": { "level": 2 },
            "content": [{ "type": "text", "text": format!("Раздел {section}") }]
        }));

        for line in 0..6 {
            content.push(json!({
                "type": "paragraph",
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Строка {line} раздела {section}. Повторяющийся текст для проверки постраничной вёрстки PDF-документа."
                    )
                }]
            }));
        }

        content.push(json!({
            "type": "table",
            "content": [
                {
                    "type": "tableRow",
                    "content": [
                        {
                            "type": "tableHeader",
                            "attrs": { "colspan": 1, "rowspan": 1 },
                            "content": [{
                                "type": "paragraph",
                                "content": [{ "type": "text", "text": "Параметр" }]
                            }]
                        },
                        {
                            "type": "tableHeader",
                            "attrs": { "colspan": 1, "rowspan": 1 },
                            "content": [{
                                "type": "paragraph",
                                "content": [{ "type": "text", "text": "Значение" }]
                            }]
                        }
                    ]
                },
                {
                    "type": "tableRow",
                    "content": [
                        {
                            "type": "tableCell",
                            "attrs": { "colspan": 1, "rowspan": 1 },
                            "content": [{
                                "type": "paragraph",
                                "content": [{ "type": "text", "text": "Строк" }]
                            }]
                        },
                        {
                            "type": "tableCell",
                            "attrs": { "colspan": 1, "rowspan": 1 },
                            "content": [{
                                "type": "paragraph",
                                "content": [{ "type": "text", "text": format!("{section}") }]
                            }]
                        }
                    ]
                }
            ]
        }));

        content.push(json!({
            "type": "paragraph",
            "content": [{
                "type": "image",
                "attrs": {
                    "src": format!("data:image/png;base64,{TINY_PNG}"),
                    "alt": "точка"
                }
            }]
        }));
    }

    let document: PmDoc =
        serde_json::from_value(json!({ "type": "doc", "content": content })).expect("документ");
    let output = render(&document, &PdfOptions::default(), Some("Большой отчёт")).expect("рендер");

    assert!(output.pages >= 50, "страниц: {}", output.pages);
    assert!(
        output.bytes.len() > 100_000,
        "размер: {}",
        output.bytes.len()
    );
}

#[test]
fn uses_letter_landscape_media_box() {
    let document: PmDoc = serde_json::from_value(json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": "Страница" }]
        }]
    }))
    .expect("документ");
    let options = PdfOptions {
        paper: PaperSize::Letter,
        orientation: Orientation::Landscape,
        margins: Margins::uniform(15.0),
        ..PdfOptions::default()
    };
    let output = render(&document, &options, None).expect("рендер");
    let pdf = lopdf::Document::load_mem(&output.bytes).expect("PDF читается lopdf");
    let page = *pdf.get_pages().values().next().expect("страница");
    let box_values = media_box(&pdf, page);

    assert_eq!(box_values.len(), 4);
    assert!((box_values[2] - 792.0).abs() < 1.5, "{box_values:?}");
    assert!((box_values[3] - 612.0).abs() < 1.5, "{box_values:?}");
}

#[test]
fn renders_header_footer_tokens() {
    let document: PmDoc = serde_json::from_value(json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{ "type": "text", "text": "Текст с колонтитулами" }]
        }]
    }))
    .expect("документ");
    let options = PdfOptions {
        header: Some("Отчёт · стр. {page}".to_owned()),
        footer: Some("{page} из {pages}".to_owned()),
        page_numbers: false,
        ..PdfOptions::default()
    };
    let output = render(&document, &options, Some("Отчёт")).expect("рендер");

    assert_eq!(output.pages, 1);
    assert!(output.warnings.is_empty(), "{:#?}", output.warnings);

    let text = pdf_extract::extract_text_from_mem(&output.bytes).expect("текст");
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(normalized.contains("Отчёт"), "{normalized}");
    assert!(normalized.contains("стр. 1"), "{normalized}");
    assert!(normalized.contains("1 из 1"), "{normalized}");
}

fn link_uris(pdf: &lopdf::Document) -> Vec<String> {
    let mut uris = Vec::new();

    for object in pdf.objects.values() {
        let Ok(dict) = object.as_dict() else {
            continue;
        };
        let Ok(action) = dict.get(b"A") else {
            continue;
        };
        let action = match action {
            lopdf::Object::Reference(id) => match pdf.get_object(*id) {
                Ok(object) => object,
                Err(_) => continue,
            },
            other => other,
        };
        let Ok(action) = action.as_dict() else {
            continue;
        };
        let Ok(uri) = action.get(b"URI") else {
            continue;
        };

        if let Ok(uri) = uri.as_str() {
            uris.push(String::from_utf8_lossy(uri).into_owned());
        }
    }

    uris
}

#[test]
fn skips_unsafe_links_in_pdf() {
    let safe_links = [
        "https://example.com",
        "http://example.com/path",
        "mailto:user@example.com",
        "tel:+123456",
        "asset:image-1",
        "./relative.html",
        "data:image/png;base64,AAAA",
    ];
    let unsafe_links = [
        "javascript&colon;alert(1)",
        "javascript:alert(1)",
        "file:///etc/passwd",
        "data:text/html;base64,AAAA",
    ];
    let mut content = Vec::new();

    for (index, href) in safe_links.iter().chain(unsafe_links.iter()).enumerate() {
        content.push(json!({
            "type": "paragraph",
            "content": [{
                "type": "text",
                "text": format!("Ссылка {index}"),
                "marks": [{ "type": "link", "attrs": { "href": href } }]
            }]
        }));
    }

    let document: PmDoc =
        serde_json::from_value(json!({ "type": "doc", "content": content })).expect("документ");
    let output = render(&document, &PdfOptions::default(), Some("Ссылки")).expect("рендер");
    let pdf = lopdf::Document::load_mem(&output.bytes).expect("PDF читается lopdf");
    let uris = link_uris(&pdf);

    for href in safe_links {
        assert!(
            uris.iter().any(|uri| uri == href),
            "ссылка «{href}» не попала в PDF: {uris:?}"
        );
    }

    for href in unsafe_links {
        assert!(
            !uris.iter().any(|uri| uri == href),
            "небезопасная ссылка «{href}» попала в PDF: {uris:?}"
        );
    }

    assert!(
        !uris.iter().any(|uri| uri.contains("javascript")),
        "в PDF есть javascript-ссылка: {uris:?}"
    );
    assert!(
        !uris.iter().any(|uri| uri.starts_with("file:")),
        "в PDF есть file-ссылка: {uris:?}"
    );
    assert!(
        !uris.iter().any(|uri| uri.starts_with("data:text/html")),
        "в PDF есть небезопасный data-URL: {uris:?}"
    );

    let warnings = output.warnings.join("\n");
    assert!(
        warnings.contains("javascript&colon;alert(1)"),
        "нет предупреждения о небезопасной ссылке: {warnings}"
    );
}
