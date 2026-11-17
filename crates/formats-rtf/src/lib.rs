mod lexer;
mod parser;
mod style;
mod writer;

use wlite_docmodel::PmDoc;
use wlite_formats_core::{DocumentCodec, FormatError, ImportedDocument, WrittenDocument};

pub struct RtfCodec;

static RTF_CODEC: RtfCodec = RtfCodec;

pub fn codec() -> &'static dyn DocumentCodec {
    &RTF_CODEC
}

impl DocumentCodec for RtfCodec {
    fn read(&self, source: &str) -> Result<ImportedDocument, FormatError> {
        parser::parse(source)
    }

    fn write(&self, document: &PmDoc) -> Result<WrittenDocument, FormatError> {
        writer::write(document)
    }

    fn decode(&self, bytes: &[u8]) -> Result<String, FormatError> {
        style::decode_source(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(source: &str) -> ImportedDocument {
        codec().read(source).expect("чтение RTF")
    }

    fn write(document: &PmDoc) -> WrittenDocument {
        codec().write(document).expect("запись RTF")
    }

    #[test]
    fn parses_empty_document() {
        let imported = read("{\\rtf1\\ansi\\pard\\par}");

        assert_eq!(imported.document, PmDoc::empty());
    }

    #[test]
    fn writes_and_reads_formatted_paragraph() {
        let source = r"{\rtf1\ansi\ansicpg1251{\fonttbl{\f0\froman Georgia;}{\f1\fswiss Arial;}}"
            .to_owned()
            + r"{\colortbl;\red255\green0\blue0;}"
            + r"\pard\qc\li480\sa200\sl360\slmul1\f0\fs28\cf1{\b Жирный} "
            + r"{\i курсив} {\ul подчёркнутый} {\strike зачёркнутый}"
            + r" \u1055?\u1088?\u1080?\u1074?\u1077?\u1090?\par}";
        let imported = read(&source);
        let written = write(&imported.document);
        let reopened = read(&written.text);

        assert_eq!(reopened.document, imported.document);
        assert!(written.text.contains("\\par"));
    }

    #[test]
    fn parses_lists_and_tables() {
        let source = r"{\rtf1\ansi
{\*\listtable{\list\listtemplateid1
{\listlevel\levelnfc23{\leveltext\'01\u8226?;}{\levelnumbers;}\f0}
{\listlevel\levelnfc0{\leveltext\'02\'00.;}{\levelnumbers\'01;}\f0}
\listid1}}
{\*\listoverridetable{\listoverride\listid1\listoverridecount0\ls1}}
\pard\ls1\ilvl0 item\par
\pard\ls1\ilvl1 nested\par
\pard\ls1\ilvl0 second\par
\trowd\cellx1800\cellx3600
\pard\intbl A\cell
\pard\intbl B\cell
\row
\pard after\par}";
        let imported = read(source);

        assert_eq!(imported.document.content[0].node_type, "bulletList");
        assert_eq!(
            imported.document.content[0].content[0].content[1].node_type,
            "orderedList"
        );
        assert_eq!(imported.document.content[1].node_type, "table");
        assert_eq!(imported.document.content[2].node_type, "paragraph");
    }

    #[test]
    fn round_trips_supported_blocks() {
        let source = r#"{\rtf1\ansi\ansicpg1251
{\fonttbl{\f0\froman Times New Roman;}{\f1\fswiss Arial;}{\f2\fmodern Courier New;}}
{\colortbl;\red255\green0\blue0;\red255\green255\blue0;}
{\stylesheet{\s0\snext0 Normal;}{\s1\outlinelevel0\snext0 heading 1;}{\s2\outlinelevel1\snext0 heading 2;}}
{\*\listtable{\list\listtemplateid1
{\listlevel\levelnfc23{\leveltext\'01\u8226?;}{\levelnumbers;}\f0}
{\listlevel\levelnfc23{\leveltext\'01\u8226?;}{\levelnumbers;}\f0}
\listid1}}
{\*\listoverridetable{\listoverride\listid1\listoverridecount0\ls1}}
\pard\s1 Heading\par
\pard Text {\b bold} {\i italic} {\ul underline} {\strike strike} {\f1\fs28\cf1 colored} {\highlight2 marked} {\field{\*\fldinst{HYPERLINK "https://example.com"}}{\fldrslt link}}\par
\pard\qc\li480\sa200\sl360\slmul1 centered\par
\pard\ls1\ilvl0 bullet one\par
\pard\ls1\ilvl1 nested\par
\pard\ls1\ilvl0 [x] done\par
\pard\ls1\ilvl0 [ ] not done\par
\trowd\trhdr\cellx1800\cellx3600
\pard\intbl A\cell
\pard\intbl B\cell
\row
\trowd\cellx1800\cellx3600
\pard\intbl C\cell
\pard\intbl D\cell
\row
\pard{\pict\pngblip 89504e47}\par
\page
{\pard\brdrb\brdrs\brdrw10\par}
}"#;
        let imported = read(source);
        let written = write(&imported.document);
        let reopened = read(&written.text);

        assert_eq!(reopened.document, imported.document, "{}", written.text);
        assert!(written.warnings.is_empty(), "{:?}", written.warnings);
    }

    #[test]
    fn parses_hyperlink_field() {
        let imported = read(
            r#"{\rtf1\ansi\pard {\field{\*\fldinst{HYPERLINK "https://example.com"}}{\fldrslt ссылка}}\par}"#,
        );
        let marks = &imported.document.content[0].content[0].marks;

        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].mark_type, "link");
        assert_eq!(
            marks[0].attrs.as_ref().unwrap()["href"],
            "https://example.com"
        );
    }

    #[test]
    fn warns_about_unsupported_destinations() {
        let imported = read(
            r"{\rtf1\ansi\pard text\par{\header header text}{\footnote note}{\object object}}",
        );

        assert!(!imported.warnings.is_empty());
        assert_eq!(imported.document.content.len(), 1);
    }

    #[test]
    fn parses_page_break_and_image() {
        let source = r"{\rtf1\ansi\pard before\par\page
{\pict\pngblip 89504e47}}";
        let imported = read(source);

        assert_eq!(imported.document.content[1].node_type, "pageBreak");
        assert_eq!(imported.document.content[2].content[0].node_type, "image");
    }

    #[test]
    fn decodes_ansi_source() {
        let bytes = b"{\\rtf1\\ansi\\ansicpg1251 \\'cf\\'f0\\'e8\\'e2\\'e5\\'f2\\par}";
        let source = codec().decode(bytes).expect("декодирование");
        let imported = codec().read(&source).expect("чтение");

        assert_eq!(
            imported.document.content[0].content[0].text.as_deref(),
            Some("Привет")
        );
    }
}
