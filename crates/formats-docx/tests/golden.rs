use std::fs;
use std::path::{Path, PathBuf};

use wordlite_docmodel::PmDoc;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/docx")
}

fn documents() -> Vec<PathBuf> {
    let mut documents: Vec<_> = fs::read_dir(fixtures_dir())
        .expect("каталог фикстур")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "docx")
        })
        .collect();

    documents.sort();

    documents
}

#[test]
fn docx_fixtures_round_trip() {
    let documents = documents();
    assert!(!documents.is_empty(), "нет DOCX-фикстур");

    for document in documents {
        let bytes = fs::read(&document).expect("чтение фикстуры");
        let imported = wordlite_formats_docx::codec()
            .read_bytes(&bytes)
            .expect("импорт фикстуры");
        let actual = serde_json::to_value(&imported.document).expect("json");
        let expected_path = document.with_extension("json");

        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            let pretty = serde_json::to_string_pretty(&actual).expect("json");
            fs::write(&expected_path, format!("{pretty}\n")).expect("запись эталона");
        }

        let expected: PmDoc = serde_json::from_slice(
            &fs::read(&expected_path).expect("эталон отсутствует: запустите UPDATE_GOLDEN=1"),
        )
        .expect("эталон");

        assert_eq!(
            actual,
            serde_json::to_value(&expected).expect("json"),
            "фикстура {}",
            document.display()
        );

        let written = wordlite_formats_docx::codec()
            .write_bytes(&imported.document)
            .expect("экспорт фикстуры");
        let reopened = wordlite_formats_docx::codec()
            .read_bytes(&written.bytes)
            .expect("повторный импорт");

        assert_eq!(
            reopened.document,
            imported.document,
            "round-trip {}",
            document.display()
        );
    }
}
