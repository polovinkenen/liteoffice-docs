use serde_json::Value;

pub const DEFAULT_FONT_SIZE_HALF_POINTS: u16 = 24;
pub const DEFAULT_FONT_NAME: &str = "Times New Roman";
pub const TWIPS_PER_LEVEL: i32 = 720;
pub const TWIPS_PER_POINT: i32 = 20;

pub fn parse_color(value: &str) -> Option<(u8, u8, u8)> {
    let value = value.trim().trim_start_matches('#');

    if value.len() == 6 {
        let red = u8::from_str_radix(&value[0..2], 16).ok()?;
        let green = u8::from_str_radix(&value[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&value[4..6], 16).ok()?;
        return Some((red, green, blue));
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

    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

pub fn format_color((red, green, blue): (u8, u8, u8)) -> String {
    format!("#{red:02x}{green:02x}{blue:02x}")
}

pub fn hex_color((red, green, blue): (u8, u8, u8)) -> String {
    format!("{red:02x}{green:02x}{blue:02x}")
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
    let unit = value[unit_start..].to_ascii_lowercase();

    let points = match unit.as_str() {
        "" | "pt" => number,
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

pub fn docx_font_name(font_family: &str) -> String {
    let first = font_family.split(',').next().unwrap_or(font_family).trim();
    first.trim_matches('"').trim().to_owned()
}

pub fn value_string(value: &Value) -> Option<&str> {
    value.as_str().filter(|value| !value.is_empty())
}

pub fn value_bool(value: &Value) -> bool {
    value.as_bool().unwrap_or(false)
}

pub fn value_u64(value: &Value) -> Option<u64> {
    value.as_u64()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_font_sizes() {
        assert_eq!(font_size_half_points("14pt"), Some(28));
        assert_eq!(font_size_half_points("16px"), Some(24));
        assert_eq!(font_size_points(28), "14pt");
    }

    #[test]
    fn converts_line_heights() {
        assert_eq!(line_height_twips("1.5"), Some(360));
        assert_eq!(line_height_from_twips(360).as_deref(), Some("1.5"));
    }

    #[test]
    fn parses_colors() {
        assert_eq!(parse_color("#ff0000"), Some((255, 0, 0)));
        assert_eq!(parse_color("ff0000"), Some((255, 0, 0)));
        assert_eq!(format_color((0, 128, 255)), "#0080ff");
    }

    #[test]
    fn normalizes_fonts() {
        assert_eq!(
            docx_font_name("\"Times New Roman\", Times"),
            "Times New Roman"
        );
        assert_eq!(
            canonical_font_family("Arial"),
            "Arial, Helvetica, sans-serif"
        );
    }
}
