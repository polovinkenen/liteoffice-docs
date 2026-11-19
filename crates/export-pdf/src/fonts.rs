use std::sync::OnceLock;

use typst::foundations::Bytes;
use typst::text::{Font, FontBook};

struct FontSet {
    fonts: Vec<Font>,
    book: FontBook,
}

static FONTS: OnceLock<FontSet> = OnceLock::new();

const FONT_FILES: &[&[u8]] = &[
    include_bytes!("../assets/fonts/LiberationSerif-Regular.ttf"),
    include_bytes!("../assets/fonts/LiberationSerif-Bold.ttf"),
    include_bytes!("../assets/fonts/LiberationSerif-Italic.ttf"),
    include_bytes!("../assets/fonts/LiberationSerif-BoldItalic.ttf"),
    include_bytes!("../assets/fonts/LiberationSans-Regular.ttf"),
    include_bytes!("../assets/fonts/LiberationSans-Bold.ttf"),
    include_bytes!("../assets/fonts/LiberationSans-Italic.ttf"),
    include_bytes!("../assets/fonts/LiberationSans-BoldItalic.ttf"),
    include_bytes!("../assets/fonts/LiberationMono-Regular.ttf"),
    include_bytes!("../assets/fonts/LiberationMono-Bold.ttf"),
    include_bytes!("../assets/fonts/LiberationMono-Italic.ttf"),
    include_bytes!("../assets/fonts/LiberationMono-BoldItalic.ttf"),
];

fn font_set() -> &'static FontSet {
    FONTS.get_or_init(|| {
        let fonts: Vec<Font> = FONT_FILES
            .iter()
            .filter_map(|bytes| Font::new(Bytes::new(*bytes), 0))
            .collect();

        assert_eq!(
            fonts.len(),
            FONT_FILES.len(),
            "встроенный шрифт PDF повреждён"
        );

        let book = FontBook::from_fonts(&fonts);

        FontSet { fonts, book }
    })
}

pub fn bundled_fonts() -> &'static [Font] {
    &font_set().fonts
}

pub fn bundled_font_book() -> &'static FontBook {
    &font_set().book
}

pub const SERIF_FAMILY: &str = "Liberation Serif";
pub const SANS_FAMILY: &str = "Liberation Sans";
pub const MONO_FAMILY: &str = "Liberation Mono";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyResolution {
    Known(&'static str),
    Fallback(&'static str),
}

pub fn resolve_family(family: &str) -> FamilyResolution {
    let first = family
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(['"', '\''])
        .to_ascii_lowercase();

    let known = match first.as_str() {
        "" | "system" | "system-ui" => return FamilyResolution::Known(SERIF_FAMILY),
        "liberation serif" | "times new roman" | "times" | "georgia" | "serif" => SERIF_FAMILY,
        "liberation sans" | "arial" | "helvetica" | "verdana" | "sans-serif" => SANS_FAMILY,
        "liberation mono" | "courier new" | "courier" | "monospace" => MONO_FAMILY,
        _ => return FamilyResolution::Fallback(SERIF_FAMILY),
    };

    FamilyResolution::Known(known)
}
