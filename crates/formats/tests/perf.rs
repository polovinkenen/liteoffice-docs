//! Бенчмарк импорта документа объёмом ~300 страниц.
//!
//! Единственный тест помечен `#[ignore]`, поэтому обычные `cargo test
//! --workspace` и CI его не запускают. Кроме замеров он проверяет, что
//! 300-страничный RTF не упирается в лимит токенов. Точная команда запуска:
//!
//! ```text
//! cargo test -p formats --test perf -- --ignored --nocapture
//! ```
//!
//! Методика: документ генерируется детерминированно (без `rand`), для каждого
//! формата выполняется один прогревочный импорт, затем [`RUNS`] замеров; в отчёт
//! идут медиана и минимум. Память считается глобальным аллокатором-обёрткой над
//! `System`: пиковый прирост за один импорт. Пороги по умолчанию — 2000 мс и
//! 500 МБ, переопределяются переменными `LITEOFFICE_DOCS_PERF_MAX_MS` и
//! `LITEOFFICE_DOCS_PERF_MAX_MB`.

use std::alloc::{GlobalAlloc, Layout, System};
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use docmodel::{PmDoc, PmNode};
use formats::{codec_for, DocumentFormat, ImportedDocument};

const PAGES: usize = 300;
const PARAGRAPHS_PER_PAGE: usize = 25;
const RUNS: usize = 3;
const DEFAULT_MAX_MS: u64 = 2000;
const DEFAULT_MAX_MB: u64 = 500;

/// Тестовый аллокатор считает текущий объём и пик, не добавляя зависимостей.
///
/// Кроме `alloc`/`dealloc` переопределены `alloc_zeroed` и `realloc`, иначе
/// рост через `Vec::reserve`/`String` не попадал бы в счётчик.
struct CountingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);

        if !pointer.is_null() {
            record_growth(layout.size());
        }

        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc_zeroed(layout);

        if !pointer.is_null() {
            record_growth(layout.size());
        }

        pointer
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let resized = System.realloc(pointer, layout, new_size);

        if !resized.is_null() {
            if new_size >= layout.size() {
                record_growth(new_size - layout.size());
            } else {
                ALLOCATED.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }

        resized
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
    }
}

#[global_allocator]
static GLOBAL_ALLOCATOR: CountingAllocator = CountingAllocator;

fn record_growth(size: usize) {
    let current = ALLOCATED.fetch_add(size, Ordering::Relaxed) + size;
    PEAK.fetch_max(current, Ordering::Relaxed);
}

fn memory_baseline() -> usize {
    let baseline = ALLOCATED.load(Ordering::SeqCst);
    PEAK.store(baseline, Ordering::SeqCst);
    baseline
}

fn memory_peak_delta(baseline: usize) -> usize {
    PEAK.load(Ordering::SeqCst).saturating_sub(baseline)
}

const WORDS: [&str; 40] = [
    "документ",
    "страница",
    "абзац",
    "редактор",
    "формат",
    "импорт",
    "текст",
    "строка",
    "модель",
    "вёрстка",
    "заголовок",
    "таблица",
    "изображение",
    "ссылка",
    "раздел",
    "проверка",
    "значение",
    "поле",
    "стиль",
    "размер",
    "отступ",
    "интервал",
    "выравнивание",
    "список",
    "цитата",
    "код",
    "символ",
    "слово",
    "предложение",
    "изменение",
    "сохранение",
    "открытие",
    "производительность",
    "память",
    "время",
    "замер",
    "прогон",
    "медиана",
    "порог",
    "проверка",
];

/// Детерминированный текст абзаца: 40–60 слов.
fn paragraph_text(index: usize) -> String {
    let word_count = 40 + index % 21;
    let mut text = String::with_capacity(word_count * 12);

    for position in 0..word_count {
        if position > 0 {
            text.push(' ');
        }

        text.push_str(WORDS[(index * 7 + position * 13) % WORDS.len()]);
    }

    text.push('.');
    text
}

fn three_hundred_page_document() -> PmDoc {
    let mut content = Vec::with_capacity(PAGES * (PARAGRAPHS_PER_PAGE + 1));
    let mut index = 0;

    for page in 0..PAGES {
        for _ in 0..PARAGRAPHS_PER_PAGE {
            content.push(PmNode::with_content(
                "paragraph",
                vec![PmNode::text(paragraph_text(index))],
            ));
            index += 1;
        }

        if page + 1 < PAGES {
            content.push(PmNode::with_content("pageBreak", Vec::new()));
        }
    }

    let document = PmDoc {
        node_type: "doc".to_owned(),
        attrs: None,
        content,
    };

    document
        .validate()
        .expect("сгенерированный документ валиден");
    document
}

enum Input {
    Text(String),
    Bytes(Vec<u8>),
}

impl Input {
    fn len(&self) -> usize {
        match self {
            Self::Text(text) => text.len(),
            Self::Bytes(bytes) => bytes.len(),
        }
    }
}

struct PreparedInput {
    format: DocumentFormat,
    input: Input,
}

fn prepare_inputs(document: &PmDoc) -> Vec<PreparedInput> {
    DocumentFormat::ALL
        .into_iter()
        .map(|format| {
            let codec = codec_for(format);
            let input = match format {
                DocumentFormat::Docx => Input::Bytes(
                    codec
                        .write_bytes(document)
                        .expect("DOCX: подготовка байтов")
                        .bytes,
                ),
                _ => Input::Text(codec.write(document).expect("подготовка текста").text),
            };

            PreparedInput { format, input }
        })
        .collect()
}

fn read_prepared(prepared: &PreparedInput) -> ImportedDocument {
    let codec = codec_for(prepared.format);

    match &prepared.input {
        Input::Text(text) => codec.read(text).expect("импорт текстового формата"),
        Input::Bytes(bytes) => codec.read_bytes(bytes).expect("импорт DOCX"),
    }
}

fn measure_time<T>(mut operation: impl FnMut() -> T) -> (Duration, Duration) {
    let warmup = operation();
    std::hint::black_box(&warmup);
    drop(warmup);

    let mut samples = Vec::with_capacity(RUNS);

    for _ in 0..RUNS {
        let start = Instant::now();
        let value = operation();
        let elapsed = start.elapsed();

        std::hint::black_box(&value);
        drop(value);

        samples.push(elapsed);
    }

    samples.sort_unstable();

    (samples[samples.len() / 2], samples[0])
}

fn measure_memory<T>(operation: impl FnOnce() -> T) -> usize {
    let baseline = memory_baseline();
    let value = operation();
    let peak = memory_peak_delta(baseline);

    std::hint::black_box(&value);
    drop(value);

    peak
}

struct Measurement {
    name: &'static str,
    input_bytes: Option<usize>,
    median: Duration,
    min: Duration,
    peak_bytes: usize,
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn megabytes(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn env_limit(name: &str, default: u64) -> u64 {
    match env::var(name) {
        Ok(raw) if !raw.trim().is_empty() => raw
            .trim()
            .parse()
            .unwrap_or_else(|error| panic!("переменная {name} должна быть целым числом: {error}")),
        _ => default,
    }
}

/// Проверка лимита токенов RTF на ~300 страницах. Выполняется внутри
/// единственного `#[ignore]`-теста, чтобы замеры памяти шли последовательно и
/// глобальный пик аллокатора не смешивался с параллельным тестом.
fn check_rtf_token_limit(document: &PmDoc) {
    let codec = codec_for(DocumentFormat::Rtf);
    let source = codec
        .write(document)
        .expect("RTF: подготовка русского текста")
        .text;
    let imported = codec.read(&source).unwrap_or_else(|error| {
        panic!("300-страничный русский RTF не должен упираться в лимит токенов: {error}")
    });

    assert!(!imported.document.content.is_empty());
    imported
        .document
        .validate()
        .expect("импортированный RTF валиден");
}

#[test]
#[ignore = "бенчмарк импорта ~300 страниц и проверка лимита RTF, запускается вручную с --ignored"]
fn imports_three_hundred_pages_within_budget() {
    let document = three_hundred_page_document();

    // Идёт первым и без замеров: глобальный пик памяти должен отражать только
    // измеряемые импорты ниже.
    check_rtf_token_limit(&document);

    let prepared = prepare_inputs(&document);

    let mut measurements = Vec::with_capacity(prepared.len() + 2);

    for input in &prepared {
        let imported = read_prepared(input);
        assert!(
            !imported.document.content.is_empty(),
            "{}: импорт вернул пустой документ",
            input.format.id()
        );
        imported
            .document
            .validate()
            .expect("импортированный документ валиден");

        let (median, min) = measure_time(|| read_prepared(input));
        let peak_bytes = measure_memory(|| read_prepared(input));

        measurements.push(Measurement {
            name: input.format.id(),
            input_bytes: Some(input.input.len()),
            median,
            min,
            peak_bytes,
        });
    }

    let json = serde_json::to_string(&document).expect("документ сериализуется в JSON");

    let (json_median, json_min) = measure_time(|| {
        let parsed: PmDoc = serde_json::from_str(&json).expect("JSON разбирается");
        parsed.validate().expect("документ из JSON валиден");
        parsed
    });
    let json_peak = measure_memory(|| {
        let parsed: PmDoc = serde_json::from_str(&json).expect("JSON разбирается");
        parsed.validate().expect("документ из JSON валиден");
        parsed
    });

    measurements.push(Measurement {
        name: "json",
        input_bytes: Some(json.len()),
        median: json_median,
        min: json_min,
        peak_bytes: json_peak,
    });

    let (validate_median, validate_min) = measure_time(|| document.validate());
    let validate_peak = measure_memory(|| document.validate());

    measurements.push(Measurement {
        name: "validate",
        input_bytes: None,
        median: validate_median,
        min: validate_min,
        peak_bytes: validate_peak,
    });

    let max_ms = env_limit("LITEOFFICE_DOCS_PERF_MAX_MS", DEFAULT_MAX_MS);
    let max_mb = env_limit("LITEOFFICE_DOCS_PERF_MAX_MB", DEFAULT_MAX_MB);
    let max_bytes = max_mb.saturating_mul(1024 * 1024);

    println!();
    println!(
        "Импорт документа ~{PAGES} страниц ({PAGES} × {PARAGRAPHS_PER_PAGE} абзацев), \
         медиана по {RUNS} прогонам после прогрева"
    );
    println!(
        "{:<10} {:>12} {:>14} {:>14} {:>12}",
        "Формат", "Вход, байт", "Медиана, мс", "Минимум, мс", "Пик, МБ"
    );

    for row in &measurements {
        let input = row
            .input_bytes
            .map(|bytes| bytes.to_string())
            .unwrap_or_else(|| "—".to_owned());

        println!(
            "{:<10} {:>12} {:>14.2} {:>14.2} {:>12.2}",
            row.name,
            input,
            millis(row.median),
            millis(row.min),
            megabytes(row.peak_bytes)
        );
    }

    println!(
        "Пороги: время ≤ {max_ms} мс, пик памяти ≤ {max_mb} МБ \
         (LITEOFFICE_DOCS_PERF_MAX_MS, LITEOFFICE_DOCS_PERF_MAX_MB)"
    );

    for row in &measurements {
        assert!(
            row.median <= Duration::from_millis(max_ms),
            "{}: медиана импорта {:.2} мс превышает порог {max_ms} мс",
            row.name,
            millis(row.median)
        );
        assert!(
            row.peak_bytes as u64 <= max_bytes,
            "{}: пиковый прирост памяти {:.2} МБ превышает порог {max_mb} МБ",
            row.name,
            megabytes(row.peak_bytes)
        );
    }
}
