pub use wlite_formats_core::{
    DocumentCodec, DocumentFormat, FormatError, ImportedDocument, WrittenDocument, WrittenFile,
};

pub fn codec_for(format: DocumentFormat) -> &'static dyn DocumentCodec {
    match format {
        DocumentFormat::Txt => wlite_formats_txt::codec(),
        DocumentFormat::Markdown => wlite_formats_markdown::codec(),
        DocumentFormat::Html => wlite_formats_html::codec(),
        DocumentFormat::Rtf => wlite_formats_rtf::codec(),
        DocumentFormat::Docx => wlite_formats_docx::codec(),
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
                .write_bytes(&wlite_docmodel::PmDoc::empty())
                .expect("запись");
            let imported = codec.read_bytes(&written.bytes).expect("чтение");

            assert!(imported.document.validate().is_ok());
        }
    }
}
