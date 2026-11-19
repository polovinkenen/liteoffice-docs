use std::collections::HashMap;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use crate::fonts::{bundled_font_book, bundled_fonts};
use crate::PdfError;

pub const MAIN_PATH: &str = "/main.typ";

pub struct PdfWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    source: Source,
    files: HashMap<FileId, Bytes>,
}

impl PdfWorld {
    pub fn new(source: String, files: Vec<(String, Vec<u8>)>) -> Result<Self, PdfError> {
        let main = FileId::new(None, VirtualPath::new(MAIN_PATH));
        let source = Source::new(main, source);
        let mut resolved = HashMap::new();

        for (path, bytes) in files {
            let id = FileId::new(None, VirtualPath::new(&path));
            resolved.insert(id, Bytes::new(bytes));
        }

        Ok(Self {
            library: LazyHash::new(Library::builder().build()),
            book: LazyHash::new(bundled_font_book().clone()),
            fonts: bundled_fonts().to_vec(),
            main,
            source,
            files: resolved,
        })
    }
}

impl World for PdfWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main {
            Ok(self.source.clone())
        } else {
            Err(FileError::NotFound(
                id.vpath().as_rootless_path().to_path_buf(),
            ))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or_else(|| FileError::NotFound(id.vpath().as_rootless_path().to_path_buf()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
