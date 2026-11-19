#![no_main]

use libfuzzer_sys::fuzz_target;
use formats::{codec_for, DocumentFormat};

const MAX_INPUT_LEN: usize = 1024 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_LEN {
        return;
    }

    if let Ok(source) = std::str::from_utf8(data) {
        let _ = codec_for(DocumentFormat::Html).read(source);
    }
});
