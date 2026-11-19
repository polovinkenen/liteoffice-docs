const MARKUP_SPECIAL: &[char] = &[
    '\\', '#', '$', '*', '_', '`', '<', '>', '@', '~', '[', ']', '=', '-', '+', '/',
];

pub fn escape_markup(text: &str) -> String {
    let mut out = String::with_capacity(text.len());

    for character in text.chars() {
        match character {
            '\n' => out.push_str("#linebreak()"),
            '\r' => {}
            '\t' => out.push_str("#h(1em)"),
            _ if MARKUP_SPECIAL.contains(&character) => {
                out.push('\\');
                out.push(character);
            }
            _ => out.push(character),
        }
    }

    out
}

pub fn typst_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');

    for character in text.chars() {
        match character {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(character),
        }
    }

    out.push('"');
    out
}

pub fn page_text_markup(text: &str) -> String {
    let mut out = String::new();
    let mut index = 0;

    while index < text.len() {
        let rest = &text[index..];

        if rest.starts_with("{page}") {
            out.push_str("#context counter(page).display()");
            index += "{page}".len();
        } else if rest.starts_with("{pages}") {
            out.push_str("#context counter(page).final().first()");
            index += "{pages}".len();
        } else {
            let character = rest.chars().next().expect("непустой остаток");
            out.push_str(&escape_markup(&character.to_string()));
            index += character.len_utf8();
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_markup_characters() {
        assert_eq!(escape_markup("a*b"), "a\\*b");
        assert_eq!(escape_markup("a\\b"), "a\\\\b");
        assert_eq!(escape_markup("строка\nвторая"), "строка#linebreak()вторая");
    }

    #[test]
    fn builds_string_literals() {
        assert_eq!(typst_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(typst_string("первая\nвторая"), "\"первая\\nвторая\"");
    }

    #[test]
    fn substitutes_page_tokens() {
        assert_eq!(
            page_text_markup("стр. {page} из {pages}"),
            "стр. #context counter(page).display() из #context counter(page).final().first()"
        );
        assert_eq!(page_text_markup("без токенов"), "без токенов");
    }
}
