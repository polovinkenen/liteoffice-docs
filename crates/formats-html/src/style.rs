use serde_json::{json, Map, Value};
use wordlite_docmodel::PmMark;

pub const DEFAULT_FONT_SIZE_UNITS: &[&str] = &["px", "pt", "em", "rem", "%"];

pub fn parse_declarations(style: &str) -> Vec<(String, String)> {
    let mut declarations = Vec::new();

    for declaration in style.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };

        let property = property.trim().to_ascii_lowercase();
        let value = value.trim().trim_end_matches("!important").trim();

        if property.is_empty() || value.is_empty() {
            continue;
        }

        declarations.push((property, value.to_owned()));
    }

    declarations
}

fn declaration<'a>(declarations: &'a [(String, String)], name: &str) -> Option<&'a str> {
    declarations
        .iter()
        .find(|(property, _)| property == name)
        .map(|(_, value)| value.as_str())
}

pub fn paragraph_attrs(declarations: &[(String, String)]) -> Option<Value> {
    let mut attrs = Map::new();

    if let Some(value) = declaration(declarations, "text-align") {
        if matches!(value, "left" | "right" | "center" | "justify") && value != "left" {
            attrs.insert("textAlign".to_owned(), json!(value));
        }
    }

    if let Some(value) = declaration(declarations, "margin-left") {
        if let Some(indent) = parse_indent(value) {
            if indent > 0 {
                attrs.insert("indent".to_owned(), json!(indent));
            }
        }
    }

    if let Some(value) = declaration(declarations, "line-height") {
        if is_safe_css_value(value) {
            attrs.insert("lineHeight".to_owned(), json!(value));
        }
    }

    if let Some(value) = declaration(declarations, "margin-top") {
        if let Some(space) = parse_points(value) {
            if space > 0 {
                attrs.insert("spaceBefore".to_owned(), json!(space));
            }
        }
    }

    if let Some(value) = declaration(declarations, "margin-bottom") {
        if let Some(space) = parse_points(value) {
            if space > 0 {
                attrs.insert("spaceAfter".to_owned(), json!(space));
            }
        }
    }

    if attrs.is_empty() {
        None
    } else {
        Some(Value::Object(attrs))
    }
}

pub fn text_marks(declarations: &[(String, String)]) -> Vec<PmMark> {
    let mut marks = Vec::new();
    let mut text_style = Map::new();

    if let Some(value) = declaration(declarations, "font-family") {
        if is_safe_css_value(value) {
            text_style.insert("fontFamily".to_owned(), json!(value));
        }
    }

    if let Some(value) = declaration(declarations, "font-size") {
        if is_safe_font_size(value) {
            text_style.insert("fontSize".to_owned(), json!(value));
        }
    }

    if let Some(value) = declaration(declarations, "color") {
        if is_safe_css_value(value) {
            text_style.insert("color".to_owned(), json!(value));
        }
    }

    if !text_style.is_empty() {
        let mut mark = PmMark::new("textStyle");
        mark.attrs = Some(Value::Object(text_style));
        marks.push(mark);
    }

    if let Some(value) = declaration(declarations, "background-color") {
        if is_safe_css_value(value) {
            let mut mark = PmMark::new("highlight");
            mark.attrs = Some(json!({ "color": value }));
            marks.push(mark);
        }
    }

    marks
}

pub fn merge_marks(marks: &[PmMark], extra: PmMark) -> Vec<PmMark> {
    let mut merged = marks.to_vec();

    if let Some(existing) = merged
        .iter_mut()
        .find(|mark| mark.mark_type == extra.mark_type)
    {
        existing.attrs = merge_attrs(existing.attrs.take(), extra.attrs);
        return merged;
    }

    merged.push(extra);
    merged
}

fn merge_attrs(base: Option<Value>, extra: Option<Value>) -> Option<Value> {
    match (base, extra) {
        (Some(Value::Object(mut base)), Some(Value::Object(extra))) => {
            for (key, value) in extra {
                if value.is_null() {
                    continue;
                }
                base.insert(key, value);
            }
            Some(Value::Object(base))
        }
        (None, extra) => extra,
        (base, None) => base,
        (base, extra) => extra.or(base),
    }
}

pub fn is_safe_css_value(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }

    let lowered = value.to_ascii_lowercase();
    if lowered.contains("url(") || lowered.contains("expression") {
        return false;
    }

    !value.chars().any(|character| {
        character.is_control()
            || matches!(
                character,
                ';' | '{' | '}' | '<' | '>' | '"' | '\'' | '\\' | '\n' | '\r'
            )
    })
}

pub fn is_safe_font_size(value: &str) -> bool {
    let value = value.trim();

    if value.is_empty() {
        return false;
    }

    let digits = value.trim_end_matches(|character: char| character.is_ascii_alphabetic());
    let unit = &value[digits.len()..];

    if digits.is_empty()
        || !digits
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return false;
    }

    unit.is_empty() || DEFAULT_FONT_SIZE_UNITS.contains(&unit)
}

pub fn is_safe_link(url: &str) -> bool {
    let trimmed = url.trim();

    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return false;
    }

    let lowered = trimmed.to_ascii_lowercase();
    !lowered.starts_with("javascript:")
        && !lowered.starts_with("vbscript:")
        && !lowered.starts_with("data:text/html")
}

pub fn paragraph_style(attrs: Option<&Value>) -> Vec<(String, String)> {
    let mut style = Vec::new();
    let Some(attrs) = attrs.and_then(|attrs| attrs.as_object()) else {
        return style;
    };

    let align = attrs
        .get("textAlign")
        .and_then(|value| value.as_str())
        .unwrap_or("left");

    if matches!(align, "right" | "center" | "justify") {
        style.push(("text-align".to_owned(), align.to_owned()));
    }

    let indent = attrs
        .get("indent")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    if indent > 0 {
        style.push(("margin-left".to_owned(), format!("{}em", indent * 2)));
    }

    if let Some(line_height) = attrs
        .get("lineHeight")
        .and_then(|value| value.as_str())
        .filter(|value| is_safe_css_value(value))
    {
        style.push(("line-height".to_owned(), line_height.to_owned()));
    }

    let space_before = attrs
        .get("spaceBefore")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    if space_before > 0 {
        style.push(("margin-top".to_owned(), format!("{space_before}pt")));
    }

    let space_after = attrs
        .get("spaceAfter")
        .and_then(|value| value.as_u64())
        .unwrap_or(0);

    if space_after > 0 {
        style.push(("margin-bottom".to_owned(), format!("{space_after}pt")));
    }

    style
}

pub fn text_style(mark_attrs: Option<&Value>) -> Vec<(String, String)> {
    let mut style = Vec::new();
    let Some(attrs) = mark_attrs.and_then(|attrs| attrs.as_object()) else {
        return style;
    };

    if let Some(font_family) = attrs
        .get("fontFamily")
        .and_then(|value| value.as_str())
        .filter(|value| is_safe_css_value(value))
    {
        style.push(("font-family".to_owned(), font_family.to_owned()));
    }

    if let Some(font_size) = attrs
        .get("fontSize")
        .and_then(|value| value.as_str())
        .filter(|value| is_safe_font_size(value))
    {
        style.push(("font-size".to_owned(), font_size.to_owned()));
    }

    if let Some(color) = attrs
        .get("color")
        .and_then(|value| value.as_str())
        .filter(|value| is_safe_css_value(value))
    {
        style.push(("color".to_owned(), color.to_owned()));
    }

    style
}

fn parse_indent(value: &str) -> Option<u64> {
    let value = value.trim();
    let number = value.strip_suffix("em")?;

    if !number
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
    {
        return None;
    }

    let em: f64 = number.parse().ok()?;
    let indent = (em / 2.0).round();

    if indent.is_finite() && indent >= 0.0 {
        Some(indent.min(8.0) as u64)
    } else {
        None
    }
}

fn parse_points(value: &str) -> Option<u64> {
    let value = value.trim();
    let number = value.strip_suffix("pt")?;

    if !number
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
    {
        return None;
    }

    let points: f64 = number.parse().ok()?;

    if points.is_finite() && points >= 0.0 {
        Some(points.round() as u64)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_paragraph_style() {
        let declarations = parse_declarations(
            "text-align: center; margin-left: 4em; line-height: 1.5; margin-top: 6pt",
        );

        let attrs = paragraph_attrs(&declarations).expect("атрибуты");

        assert_eq!(attrs["textAlign"], "center");
        assert_eq!(attrs["indent"], 2);
        assert_eq!(attrs["lineHeight"], "1.5");
        assert_eq!(attrs["spaceBefore"], 6);
    }

    #[test]
    fn parses_text_style_and_highlight() {
        let declarations = parse_declarations(
            "font-family: Georgia; font-size: 14pt; color: #ff0000; background-color: #ffff00",
        );

        let marks = text_marks(&declarations);

        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0].mark_type, "textStyle");
        assert_eq!(marks[0].attrs.as_ref().unwrap()["fontFamily"], "Georgia");
        assert_eq!(marks[1].mark_type, "highlight");
        assert_eq!(marks[1].attrs.as_ref().unwrap()["color"], "#ffff00");
    }

    #[test]
    fn rejects_unsafe_values() {
        assert!(!is_safe_css_value("red; background-image: url(x)"));
        assert!(!is_safe_css_value("expression(alert(1))"));
        assert!(!is_safe_link("javascript:alert(1)"));
        assert!(!is_safe_font_size("14pt; color: red"));
        assert!(is_safe_font_size("1.5rem"));
        assert!(is_safe_link("https://example.com"));
    }

    #[test]
    fn builds_paragraph_style_back() {
        let attrs = json!({
            "textAlign": "center",
            "indent": 2,
            "lineHeight": "1.5",
            "spaceBefore": 6,
            "spaceAfter": 3
        });

        let style = paragraph_style(Some(&attrs));
        let declarations = parse_declarations(
            &style
                .iter()
                .map(|(property, value)| format!("{property}: {value}"))
                .collect::<Vec<_>>()
                .join("; "),
        );

        assert_eq!(paragraph_attrs(&declarations), Some(attrs));
    }
}
