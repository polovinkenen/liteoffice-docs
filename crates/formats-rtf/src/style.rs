use encoding_rs::{
    Encoding, IBM866, UTF_8, WINDOWS_1250, WINDOWS_1251, WINDOWS_1252, WINDOWS_1253, WINDOWS_1254,
    WINDOWS_1255, WINDOWS_1256, WINDOWS_1257, WINDOWS_1258, WINDOWS_874,
};
use wlite_formats_core::FormatError;

pub const DEFAULT_FONT_SIZE_HALF_POINTS: u16 = 24;
pub const DEFAULT_FONT_NAME: &str = "Times New Roman";
pub const LI_TWIPS_PER_LEVEL: i32 = 720;

pub fn decode_source(bytes: &[u8]) -> Result<String, FormatError> {
    if let Ok(source) = std::str::from_utf8(bytes) {
        return Ok(strip_bom(source).to_owned());
    }

    let code_page = sniff_code_page(bytes).unwrap_or(1251);
    let decoded = decode_in_code_page(bytes, code_page);

    if decoded.trim_start().is_empty() {
        return Err(FormatError::Parse(
            "не удалось декодировать RTF: файл пустой или повреждён".to_owned(),
        ));
    }

    Ok(strip_bom(&decoded).to_owned())
}

pub fn decode_in_code_page(bytes: &[u8], code_page: u16) -> String {
    if bytes.is_empty() {
        return String::new();
    }

    if code_page == 65001 {
        return String::from_utf8_lossy(bytes).into_owned();
    }

    let (decoded, _) = encoding_for(code_page).decode_without_bom_handling(bytes);

    decoded.into_owned()
}

pub fn encoding_for(code_page: u16) -> &'static Encoding {
    match code_page {
        65001 => UTF_8,
        866 => IBM866,
        874 => WINDOWS_874,
        1250 => WINDOWS_1250,
        1251 => WINDOWS_1251,
        1252 => WINDOWS_1252,
        1253 => WINDOWS_1253,
        1254 => WINDOWS_1254,
        1255 => WINDOWS_1255,
        1256 => WINDOWS_1256,
        1257 => WINDOWS_1257,
        1258 => WINDOWS_1258,
        _ => WINDOWS_1251,
    }
}

fn sniff_code_page(bytes: &[u8]) -> Option<u16> {
    let prefix = &bytes[..bytes.len().min(4096)];
    let text = String::from_utf8_lossy(prefix);
    let index = text.find("\\ansicpg")?;
    let digits: String = text[index + "\\ansicpg".len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();

    digits.parse().ok()
}

fn strip_bom(value: &str) -> &str {
    value.strip_prefix('\u{feff}').unwrap_or(value)
}

pub fn parse_color(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.trim();

    if let Some(hex) = value.strip_prefix('#') {
        return match hex.len() {
            3 => {
                let red = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let green = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let blue = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                Some((red, green, blue))
            }
            6 => {
                let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some((red, green, blue))
            }
            _ => None,
        };
    }

    let lowered = value.to_ascii_lowercase();
    let inner = lowered
        .strip_prefix("rgb(")
        .or_else(|| lowered.strip_prefix("rgba("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();

    if parts.len() < 3 {
        return None;
    }

    let red = parse_color_channel(parts[0])?;
    let green = parse_color_channel(parts[1])?;
    let blue = parse_color_channel(parts[2])?;

    Some((red, green, blue))
}

fn parse_color_channel(value: &str) -> Option<u8> {
    if let Some(percent) = value.strip_suffix('%') {
        let percent: f32 = percent.parse().ok()?;
        return Some(((percent / 100.0) * 255.0).round().clamp(0.0, 255.0) as u8);
    }

    value.parse::<u8>().ok()
}

pub fn format_color((red, green, blue): (u8, u8, u8)) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}")
}

pub fn font_size_half_points(value: &str) -> Option<u16> {
    let value = value.trim();

    if value.is_empty() {
        return None;
    }

    let unit_start = value
        .char_indices()
        .find(|(_, character)| !character.is_ascii_digit() && *character != '.')
        .map(|(index, _)| index)
        .unwrap_or(value.len());
    let number: f32 = value[..unit_start].parse().ok()?;
    let unit = &value[unit_start..];

    let points = match unit.to_ascii_lowercase().as_str() {
        "" => number,
        "pt" => number,
        "px" => number * 0.75,
        "em" | "rem" => number * 12.0,
        "%" => number * 0.12,
        _ => return None,
    };

    let half_points = (points * 2.0).round();

    if (1.0..=u16::MAX as f32).contains(&half_points) {
        Some(half_points as u16)
    } else {
        None
    }
}

pub fn font_size_points(half_points: u16) -> String {
    if half_points.is_multiple_of(2) {
        format!("{}pt", half_points / 2)
    } else {
        format!("{:.1}pt", half_points as f32 / 2.0)
    }
}

pub fn line_height_twips(value: &str) -> Option<i32> {
    let value = value.trim();
    let number: f32 = value
        .trim_end_matches(|character: char| character.is_ascii_alphabetic())
        .parse()
        .ok()?;

    if !number.is_finite() || number <= 0.0 {
        return None;
    }

    Some((number * 240.0).round() as i32)
}

pub fn line_height_from_twips(twips: i32) -> Option<String> {
    if twips <= 0 {
        return None;
    }

    let value = twips as f32 / 240.0;
    let mut formatted = format!("{value:.2}");

    while formatted.ends_with('0') {
        formatted.pop();
    }

    if formatted.ends_with('.') {
        formatted.pop();
    }

    Some(formatted)
}

pub fn canonical_font_family(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "arial" => "Arial, Helvetica, sans-serif".to_owned(),
        "georgia" => "Georgia, serif".to_owned(),
        "times new roman" => "\"Times New Roman\", Times, serif".to_owned(),
        "courier new" => "\"Courier New\", Courier, monospace".to_owned(),
        "verdana" => "Verdana, Geneva, sans-serif".to_owned(),
        _ => name.trim().to_owned(),
    }
}

pub fn rtf_font_name(font_family: &str) -> String {
    let first = font_family.split(',').next().unwrap_or(font_family).trim();
    first.trim_matches('"').trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_code_page_1251() {
        let bytes = [0xcf, 0xf0, 0xe8, 0xe2, 0xe5, 0xf2];

        assert_eq!(decode_in_code_page(&bytes, 1251), "Привет");
    }

    #[test]
    fn sniffs_code_page_from_header() {
        let source = br"{\rtf1\ansi\ansicpg1251 \'cf\'f0}".to_vec();
        let bytes = [0xff, 0xfe];

        assert!(decode_source(&bytes).is_ok());
        assert_eq!(sniff_code_page(&source), Some(1251));
    }

    #[test]
    fn converts_font_sizes() {
        assert_eq!(font_size_half_points("14pt"), Some(28));
        assert_eq!(font_size_half_points("16px"), Some(24));
        assert_eq!(font_size_half_points("1.5em"), Some(36));
        assert_eq!(font_size_points(28), "14pt");
        assert_eq!(font_size_points(25), "12.5pt");
    }

    #[test]
    fn converts_line_heights() {
        assert_eq!(line_height_twips("1.5"), Some(360));
        assert_eq!(line_height_from_twips(360).as_deref(), Some("1.5"));
        assert_eq!(line_height_from_twips(276).as_deref(), Some("1.15"));
    }

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("#ff0000"), Some((255, 0, 0)));
        assert_eq!(parse_color("#f00"), Some((255, 0, 0)));
        assert_eq!(parse_color("rgb(0, 128, 255)"), Some((0, 128, 255)));
        assert_eq!(format_color((0, 128, 255)), "#0080ff");
    }

    #[test]
    fn normalizes_font_families() {
        assert_eq!(
            rtf_font_name("\"Times New Roman\", Times, serif"),
            "Times New Roman"
        );
        assert_eq!(
            canonical_font_family("Arial"),
            "Arial, Helvetica, sans-serif"
        );
        assert_eq!(canonical_font_family("Unknown"), "Unknown");
    }
}
