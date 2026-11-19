mod convert;
mod error;
mod escape;
mod fonts;
mod options;
mod world;

use wlite_docmodel::PmDoc;

pub use error::PdfError;
pub use fonts::{resolve_family, FamilyResolution};
pub use options::{Margins, Orientation, PaperSize, PdfOptions};

#[derive(Debug)]
pub struct RenderedPdf {
    pub bytes: Vec<u8>,
    pub pages: usize,
    pub warnings: Vec<String>,
}

pub fn render(
    document: &PmDoc,
    options: &PdfOptions,
    title: Option<&str>,
) -> Result<RenderedPdf, PdfError> {
    let conversion = convert::convert(document, options, title)?;
    let mut rendered = render_source(&conversion.source, conversion.files)?;

    for warning in conversion.warnings {
        if !rendered.warnings.contains(&warning) {
            rendered.warnings.push(warning);
        }
    }

    Ok(rendered)
}

fn render_source(source: &str, files: Vec<(String, Vec<u8>)>) -> Result<RenderedPdf, PdfError> {
    let world = world::PdfWorld::new(source.to_owned(), files)?;
    let compiled = typst::compile::<typst::layout::PagedDocument>(&world);
    let mut warnings: Vec<String> = compiled
        .warnings
        .iter()
        .map(|warning| warning.message.to_string())
        .collect();
    let document = compiled.output.map_err(|errors| {
        PdfError::Render(
            errors
                .iter()
                .map(|error| error.message.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;
    let pages = document.pages.len();

    let bytes = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        PdfError::Render(
            errors
                .iter()
                .map(|error| error.message.to_string())
                .collect::<Vec<_>>()
                .join("; "),
        )
    })?;

    warnings.sort();
    warnings.dedup();

    Ok(RenderedPdf {
        bytes,
        pages,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_source_with_page_breaks() {
        let output = render_source(
            "Первая\n#pagebreak()\nВторая\n#pagebreak()\nТретья",
            Vec::new(),
        )
        .expect("рендер");

        assert_eq!(output.pages, 3);
        assert!(output.bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn renders_document_with_options() {
        let document = PmDoc {
            node_type: "doc".to_owned(),
            attrs: None,
            content: vec![wlite_docmodel::PmNode::with_content(
                "paragraph",
                vec![wlite_docmodel::PmNode::text("Привет, мир!")],
            )],
        };
        let options = PdfOptions {
            paper: PaperSize::Letter,
            orientation: Orientation::Landscape,
            ..PdfOptions::default()
        };

        let output = render(&document, &options, Some("Тест")).expect("рендер");

        assert_eq!(output.pages, 1);
        assert!(output.bytes.starts_with(b"%PDF-"));
    }
}
