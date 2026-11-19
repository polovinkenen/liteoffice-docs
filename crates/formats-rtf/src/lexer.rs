#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Token<'a> {
    GroupStart,
    GroupEnd,
    Control { name: &'a str, param: Option<i32> },
    Literal(char),
    Symbol(char),
    HexByte(u8),
    Text(&'a str),
    Star,
}

const MAX_TOKENS: usize = 10_000_000;

pub fn tokenize(source: &str) -> Result<Vec<Token<'_>>, String> {
    tokenize_with_limit(source, MAX_TOKENS)
}

fn tokenize_with_limit(source: &str, max_tokens: usize) -> Result<Vec<Token<'_>>, String> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'{' => {
                tokens.push(Token::GroupStart);
                index += 1;
            }
            b'}' => {
                tokens.push(Token::GroupEnd);
                index += 1;
            }
            b'\r' | b'\n' => index += 1,
            b'\\' => {
                let (token, next) = control(bytes, source, index)?;

                if let Some(token) = token {
                    tokens.push(token);
                }

                index = next;
            }
            _ => {
                let start = index;

                while index < bytes.len()
                    && !matches!(bytes[index], b'{' | b'}' | b'\\' | b'\r' | b'\n')
                {
                    index += 1;
                }

                tokens.push(Token::Text(&source[start..index]));
            }
        }

        if tokens.len() > max_tokens {
            return Err(format!(
                "превышено максимальное число токенов RTF {max_tokens}"
            ));
        }
    }

    Ok(tokens)
}

fn control<'a>(
    bytes: &'a [u8],
    source: &'a str,
    start: usize,
) -> Result<(Option<Token<'a>>, usize), String> {
    let Some(&next) = bytes.get(start + 1) else {
        return Err("незавершённая escape-последовательность RTF".to_owned());
    };

    match next {
        b'{' => Ok((Some(Token::Literal('{')), start + 2)),
        b'}' => Ok((Some(Token::Literal('}')), start + 2)),
        b'\\' => Ok((Some(Token::Literal('\\')), start + 2)),
        b'*' => Ok((Some(Token::Star), start + 2)),
        b'~' => Ok((Some(Token::Literal('\u{00a0}')), start + 2)),
        b'_' => Ok((Some(Token::Literal('\u{2011}')), start + 2)),
        b'-' => Ok((None, start + 2)),
        b'\'' => {
            let hex = source
                .get(start + 2..start + 4)
                .ok_or_else(|| "незавершённый hex-код RTF".to_owned())?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| format!("некорректный hex-код RTF: \\'{hex}"))?;

            Ok((Some(Token::HexByte(byte)), start + 4))
        }
        b'\r' | b'\n' => Ok((None, start + 2)),
        byte if byte.is_ascii_alphabetic() => {
            let mut end = start + 1;

            while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
                end += 1;
            }

            let name = &source[start + 1..end];
            let (param, cursor) = parameter(bytes, end);

            Ok((Some(Token::Control { name, param }), cursor))
        }
        _ => {
            let symbol = source[start + 1..].chars().next().unwrap_or('?');

            Ok((Some(Token::Symbol(symbol)), start + 1 + symbol.len_utf8()))
        }
    }
}

/// Разбирает числовой параметр control word без промежуточной `String`.
///
/// Возвращает параметр и позицию после него (вместе с одним пробелом-разделителем).
/// Переполнение `i32` и отсутствие цифр дают `None` — так же, как раньше вёл себя
/// `String::parse::<i32>()`.
fn parameter(bytes: &[u8], start: usize) -> (Option<i32>, usize) {
    let mut cursor = start;
    let negative = bytes.get(cursor) == Some(&b'-');

    if negative {
        cursor += 1;
    }

    let mut value: i64 = 0;
    let mut digits = 0;
    let mut overflow = false;

    while let Some(byte) = bytes.get(cursor) {
        if !byte.is_ascii_digit() {
            break;
        }

        digits += 1;

        if !overflow {
            value = value * 10 + i64::from(byte - b'0');

            if value > i64::from(i32::MAX) + 1 {
                overflow = true;
            }
        }

        cursor += 1;
    }

    let param = if digits == 0 || overflow {
        None
    } else if negative {
        i32::try_from(-value).ok()
    } else {
        i32::try_from(value).ok()
    };

    if bytes.get(cursor) == Some(&b' ') {
        cursor += 1;
    }

    (param, cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(tokens: &[Token<'_>]) -> Vec<String> {
        tokens
            .iter()
            .filter_map(|token| match token {
                Token::Control { name, .. } => Some((*name).to_owned()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reads_control_words_and_text() {
        let tokens = tokenize(r"{\rtf1\ansi\b\fs24 Привет}").expect("токены");

        assert_eq!(names(&tokens), vec!["rtf", "ansi", "b", "fs"]);
        assert!(tokens.contains(&Token::Text("Привет")));
        assert!(tokens.contains(&Token::Control {
            name: "fs",
            param: Some(24)
        }));
    }

    #[test]
    fn decodes_escapes_and_hex_bytes() {
        let tokens = tokenize(r"a\{b\}c\\d\~e\'e0").expect("токены");

        assert!(tokens.contains(&Token::Literal('{')));
        assert!(tokens.contains(&Token::Literal('}')));
        assert!(tokens.contains(&Token::Literal('\\')));
        assert!(tokens.contains(&Token::Literal('\u{00a0}')));
        assert!(tokens.contains(&Token::HexByte(0xe0)));
    }

    #[test]
    fn keeps_negative_and_missing_parameters() {
        let tokens = tokenize(r"\fi-360\ql").expect("токены");

        assert_eq!(
            tokens[0],
            Token::Control {
                name: "fi",
                param: Some(-360)
            }
        );
        assert_eq!(
            tokens[1],
            Token::Control {
                name: "ql",
                param: None
            }
        );
    }

    #[test]
    fn rejects_broken_hex_escape() {
        assert!(tokenize(r"\rtf1\'zz").is_err());
        assert!(tokenize(r"\rtf1\'f").is_err());
    }

    #[test]
    fn rejects_sources_over_token_limit() {
        let error = tokenize_with_limit(r"{\rtf1\ansi\b текст}", 3).expect_err("ожидалась ошибка");

        assert!(error.contains("токенов"));
    }

    #[test]
    fn token_limit_fits_memory_budget() {
        const BUDGET_BYTES: usize = 300 * 1024 * 1024;
        let per_token = std::mem::size_of::<Token<'_>>();
        let worst_case = per_token * MAX_TOKENS;

        assert!(
            worst_case <= BUDGET_BYTES,
            "Vec<Token> на пределе лимита занимает {worst_case} байт \
             ({per_token} байт × {MAX_TOKENS}), это больше бюджета 300 МБ"
        );
    }

    #[test]
    fn keeps_none_for_overflowing_parameters() {
        let tokens = tokenize(r"\fs999999999999999999999999\fi-360").expect("токены");

        assert_eq!(
            tokens[0],
            Token::Control {
                name: "fs",
                param: None
            }
        );
        assert_eq!(
            tokens[1],
            Token::Control {
                name: "fi",
                param: Some(-360)
            }
        );
    }
}
