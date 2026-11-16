mod read;
mod style;
mod write;

use wordlite_docmodel::PmDoc;
use wordlite_formats_core::{DocumentCodec, FormatError, ImportedDocument, WrittenDocument};

pub struct HtmlCodec;

static HTML_CODEC: HtmlCodec = HtmlCodec;

pub fn codec() -> &'static dyn DocumentCodec {
    &HTML_CODEC
}

impl DocumentCodec for HtmlCodec {
    fn read(&self, source: &str) -> Result<ImportedDocument, FormatError> {
        read::read_document(source)
    }

    fn write(&self, document: &PmDoc) -> Result<WrittenDocument, FormatError> {
        write::write_document(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_empty_document() {
        let written = codec().write(&PmDoc::empty()).expect("запись HTML");

        assert!(written.text.contains("<p></p>"));

        let imported = codec().read(&written.text).expect("чтение HTML");

        assert_eq!(imported.document, PmDoc::empty());
    }
}
