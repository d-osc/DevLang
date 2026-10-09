use crate::ast::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Word(String),
    Number(String),
    String(String),
    Symbol(String),
    Newline,
    Eof,
}
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<Token>, (Span, String)> {
    let chars: Vec<char> = source.chars().collect();
    let (mut i, mut line, mut col, mut nesting) = (0, 1, 1, 0usize);
    let mut tokens = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        let span = Span { line, col };
        if c == '\n' {
            if nesting == 0 {
                tokens.push(Token {
                    kind: Kind::Newline,
                    span,
                });
            }
            i += 1;
            line += 1;
            col = 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            col += 1;
            continue;
        }
        if c == '#' || (c == '/' && chars.get(i + 1) == Some(&'/')) {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
                col += 1;
            }
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
                col += 1;
            }
            tokens.push(Token {
                kind: Kind::Word(chars[start..i].iter().collect()),
                span,
            });
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            if c == '0' && matches!(chars.get(i + 1), Some('x' | 'X')) {
                i += 2;
                col += 2;
                let digits = i;
                while i < chars.len() && (chars[i].is_ascii_hexdigit() || chars[i] == '_') {
                    i += 1;
                    col += 1;
                }
                if i == digits {
                    return Err((span, "expected hexadecimal digits".into()));
                }
            } else {
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                    i += 1;
                    col += 1;
                }
                if chars.get(i) == Some(&'.')
                    && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit())
                {
                    i += 1;
                    col += 1;
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '_') {
                        i += 1;
                        col += 1;
                    }
                }
                if matches!(chars.get(i), Some('e' | 'E')) {
                    i += 1;
                    col += 1;
                    if matches!(chars.get(i), Some('+' | '-')) {
                        i += 1;
                        col += 1;
                    }
                    let digits = i;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                        col += 1;
                    }
                    if digits == i {
                        return Err((span, "expected exponent digits".into()));
                    }
                }
            }
            tokens.push(Token {
                kind: Kind::Number(chars[start..i].iter().filter(|c| **c != '_').collect()),
                span,
            });
            continue;
        }
        if c == '"' {
            i += 1;
            col += 1;
            let mut value = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\n' {
                    return Err((span, "newline in string; use \\n".into()));
                }
                if chars[i] == '\\' {
                    i += 1;
                    col += 1;
                    value.push(match chars.get(i) {
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('0') => '\0',
                        Some('"') => '"',
                        Some('\\') => '\\',
                        _ => return Err((Span { line, col }, "invalid string escape".into())),
                    });
                } else {
                    value.push(chars[i]);
                }
                i += 1;
                col += 1;
            }
            if i == chars.len() {
                return Err((span, "unterminated string".into()));
            }
            i += 1;
            col += 1;
            tokens.push(Token {
                kind: Kind::String(value),
                span,
            });
            continue;
        }
        if chars[i..].starts_with(&['.', '.', '.']) {
            i += 3;
            col += 3;
            tokens.push(Token {
                kind: Kind::Symbol("...".into()),
                span,
            });
            continue;
        }
        let pair: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        if [
            "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "+=", "-=", "*=", "/=", "%=", "&=",
            "|=", "^=", "->", "..", "=>",
        ]
        .contains(&pair.as_str())
        {
            i += 2;
            col += 2;
            tokens.push(Token {
                kind: Kind::Symbol(pair),
                span,
            });
            continue;
        }
        if "(){}[],;:.+-*/%=!<>&|^~".contains(c) {
            if c == '(' || c == '[' {
                nesting += 1;
            }
            if c == ')' || c == ']' {
                nesting = nesting.saturating_sub(1);
            }
            tokens.push(Token {
                kind: Kind::Symbol(c.to_string()),
                span,
            });
            i += 1;
            col += 1;
            continue;
        }
        return Err((span, format!("unexpected character '{c}'")));
    }
    tokens.push(Token {
        kind: Kind::Eof,
        span: Span { line, col },
    });
    Ok(tokens)
}
