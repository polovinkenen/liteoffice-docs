use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;

use crate::{AppError, AppResult};

pub const MAX_TEXT_FILE_BYTES: u64 = 50 * 1024 * 1024;

pub fn read_bytes(path: &Path, max_bytes: u64) -> AppResult<Vec<u8>> {
    let metadata = std::fs::metadata(path)?;

    if metadata.len() > max_bytes {
        return Err(AppError::FileTooLarge {
            path: path.display().to_string(),
            size: metadata.len(),
            max_bytes,
        });
    }

    Ok(std::fs::read(path)?)
}

pub fn read_text(path: &Path, max_bytes: u64) -> AppResult<String> {
    let bytes = read_bytes(path, max_bytes)?;
    let text = String::from_utf8(bytes).map_err(|_| {
        AppError::InvalidDocument(format!(
            "файл {} должен быть в кодировке UTF-8",
            path.display()
        ))
    })?;

    Ok(text.strip_prefix('\u{feff}').unwrap_or(&text).to_owned())
}

pub fn write_text_atomic(path: &Path, text: &str) -> AppResult<()> {
    write_bytes_atomic(path, text.as_bytes())
}

pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path.parent().ok_or_else(|| {
        AppError::InvalidDocument(format!(
            "не удалось определить каталог для {}",
            path.display()
        ))
    })?;

    let mut temp = NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.flush()?;
    temp.persist(path)
        .map_err(|error| AppError::Io(error.error))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_utf8_and_strips_bom() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("doc.md");
        std::fs::write(&path, "\u{feff}# Заголовок\n").expect("запись");

        let text = read_text(&path, MAX_TEXT_FILE_BYTES).expect("чтение");

        assert_eq!(text, "# Заголовок\n");
    }

    #[test]
    fn rejects_non_utf8_files() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("legacy.txt");
        std::fs::write(&path, [0xff, 0xfe, 0x00]).expect("запись");

        let error = read_text(&path, MAX_TEXT_FILE_BYTES).expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "invalid_document");
    }

    #[test]
    fn rejects_files_over_limit() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("big.txt");
        std::fs::write(&path, "12345").expect("запись");

        let error = read_text(&path, 4).expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "file_too_large");
    }

    #[test]
    fn writes_file_atomically() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("doc.txt");

        write_text_atomic(&path, "первая версия").expect("первая запись");
        write_text_atomic(&path, "вторая версия").expect("вторая запись");

        assert_eq!(
            std::fs::read_to_string(&path).expect("чтение"),
            "вторая версия"
        );
    }

    #[test]
    fn reports_missing_parent_directory() {
        let dir = tempfile::tempdir().expect("временный каталог");
        let path = dir.path().join("missing").join("doc.txt");

        let error = write_text_atomic(&path, "текст").expect_err("ожидалась ошибка");

        assert_eq!(error.code(), "io_error");
    }
}
