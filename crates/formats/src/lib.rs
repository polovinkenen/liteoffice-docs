pub use wordlite_formats_core::{
    DocumentCodec, DocumentFormat, FormatError, ImportedDocument, WrittenDocument,
};

pub fn codec_for(format: DocumentFormat) -> &'static dyn DocumentCodec {
    match format {
        DocumentFormat::Txt => wordlite_formats_txt::codec(),
        DocumentFormat::Markdown => wordlite_formats_markdown::codec(),
        DocumentFormat::Html => wordlite_formats_html::codec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_codec_for_every_format() {
        for format in DocumentFormat::ALL {
            let codec = codec_for(format);
            let written = codec
                .write(&wordlite_docmodel::PmDoc::empty())
                .expect("запись");
            let imported = codec.read(&written.text).expect("чтение");

            assert!(imported.document.validate().is_ok());
        }
    }
}
