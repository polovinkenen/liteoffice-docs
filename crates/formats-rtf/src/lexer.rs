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

pub fn tokenize(source: &str) -> Result<Vec<Token<'_>>, String> {
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
                tokens.extend(token);
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
    }

    Ok(tokens)
}

fn control<'a>(
    bytes: &'a [u8],
    source: &'a str,
    start: usize,
) -> Result<(Vec<Token<'a>>, usize), String> {
    let Some(&next) = bytes.get(start + 1) else {
        return Err("незавершённая escape-последовательность RTF".to_owned());
    };

    match next {
        b'{' => Ok((vec![Token::Literal('{')], start + 2)),
        b'}' => Ok((vec![Token::Literal('}')], start + 2)),
        b'\\' => Ok((vec![Token::Literal('\\')], start + 2)),
        b'*' => Ok((vec![Token::Star], start + 2)),
        b'~' => Ok((vec![Token::Literal('\u{00a0}')], start + 2)),
        b'_' => Ok((vec![Token::Literal('\u{2011}')], start + 2)),
        b'-' => Ok((Vec::new(), start + 2)),
        b'\'' => {
            let hex = source
                .get(start + 2..start + 4)
                .ok_or_else(|| "незавершённый hex-код RTF".to_owned())?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| format!("некорректный hex-код RTF: \\'{hex}"))?;

            Ok((vec![Token::HexByte(byte)], start + 4))
        }
        b'\r' | b'\n' => Ok((Vec::new(), start + 2)),
        byte if byte.is_ascii_alphabetic() => {
            let mut end = start + 1;

            while end < bytes.len() && bytes[end].is_ascii_alphabetic() {
                end += 1;
            }

            let name = &source[start + 1..end];
            let mut cursor = end;
            let mut number = String::new();

            if bytes.get(cursor) == Some(&b'-') {
                number.push('-');
                cursor += 1;
            }

            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                number.push(bytes[cursor] as char);
                cursor += 1;
            }

            let param = number.parse::<i32>().ok();

            if bytes.get(cursor) == Some(&b' ') {
                cursor += 1;
            }

            Ok((vec![Token::Control { name, param }], cursor))
        }
        _ => {
            let symbol = source[start + 1..].chars().next().unwrap_or('?');

            Ok((vec![Token::Symbol(symbol)], start + 1 + symbol.len_utf8()))
        }
    }
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
}
