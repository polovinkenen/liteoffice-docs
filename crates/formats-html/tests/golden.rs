use std::fs;
use std::path::{Path, PathBuf};

use wlite_docmodel::PmDoc;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/html")
}

fn documents() -> Vec<PathBuf> {
    let mut documents: Vec<_> = fs::read_dir(fixtures_dir())
        .expect("каталог фикстур")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "html")
        })
        .collect();

    documents.sort();

    documents
}

#[test]
fn html_fixtures_round_trip() {
    let documents = documents();
    assert!(!documents.is_empty(), "нет HTML-фикстур");

    for document in documents {
        let source = fs::read_to_string(&document).expect("чтение фикстуры");
        let imported = wlite_formats_html::codec()
            .read(&source)
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

        let written = wlite_formats_html::codec()
            .write(&imported.document)
            .expect("экспорт фикстуры");
        let reopened = wlite_formats_html::codec()
            .read(&written.text)
            .expect("повторный импорт");

        assert_eq!(
            reopened.document,
            imported.document,
            "round-trip {}",
            document.display()
        );
    }
}
