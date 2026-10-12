use crate::ast::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Word(String),
    Number(String),
    String(String),
    Template(Vec<(bool, String, usize)>),
    Symbol(String),
    Newline,
    Eof,
}
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
    pub end_line: usize,
}

pub fn lex(source: &str) -> Result<Vec<Token>, (Span, String)> {
    let chars: Vec<char> = source.chars().collect();
    let (mut i, mut line, mut col, mut nesting) = (0, 1, 1, 0usize);
    let mut brace_nesting = Vec::new();
    let mut tokens = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        let span = Span { line, col };
        if c == '\n' {
            if nesting == 0 {
                tokens.push(Token {
                    kind: Kind::Newline,
                    span,
                    end_line: line,
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
        if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2; col += 2;
            while i < chars.len() && !(chars[i] == '*' && chars.get(i+1) == Some(&'/')) {
                if chars[i] == '\n' { line += 1; col = 1; } else { col += 1; }
                i += 1;
            }
            if i == chars.len() { return Err((span, "unterminated block comment".into())); }
            i += 2; col += 2;
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
                end_line: line,
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
                end_line: line,
            });
            continue;
        }
        if c == '"' || c == '\'' {
            if c == '"' && chars[i..].starts_with(&['"','"','"']) {
                i += 3; col += 3;
                let start = i;
                while i < chars.len() && !chars[i..].starts_with(&['"','"','"']) {
                    if chars[i] == '\n' { line += 1; col = 1; } else { col += 1; }
                    i += 1;
                }
                if i == chars.len() { return Err((span,"unterminated multiline string".into())); }
                let value = chars[start..i].iter().collect();
                i += 3; col += 3;
                tokens.push(Token { kind: Kind::String(value), span, end_line: line });
                continue;
            }
            let quote = c;
            let string_start = i;
            let mut parts = Vec::new();
            i += 1;
            col += 1;
            let mut value = String::new();
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\n' {
                    return Err((span, "newline in string; use \\n".into()));
                }
                if quote == '"' && chars[i] == '$' && chars.get(i + 1) == Some(&'{') {
                    parts.push((false, std::mem::take(&mut value), 0));
                    let start = i + 2;
                    let mut end = start;
                    let mut depth = 1usize;
                    let mut quoted = false;
                    let mut expression_quote = '"';
                    let mut escaped = false;
                    while end < chars.len() {
                        let ch = chars[end];
                        if ch == '\n' {
                            return Err((span, "newline in string interpolation".into()));
                        }
                        if quoted {
                            if escaped {
                                escaped = false;
                            } else if ch == '\\' {
                                escaped = true;
                            } else if ch == expression_quote {
                                quoted = false;
                            }
                        } else if ch == '"' || ch == '\'' {
                            expression_quote = ch;
                            quoted = true;
                        } else if ch == '{' {
                            depth += 1;
                        } else if ch == '}' {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        end += 1;
                    }
                    if end == chars.len() {
                        return Err((span, "unterminated string interpolation".into()));
                    }
                    parts.push((
                        true,
                        chars[start..end].iter().collect(),
                        start - string_start,
                    ));
                    col += end + 1 - i;
                    i = end + 1;
                    continue;
                }
                if chars[i] == '\\' {
                    i += 1;
                    col += 1;
                    value.push(match chars.get(i) {
                        Some('n') => '\n',
                        Some('r') => '\r',
                        Some('t') => '\t',
                        Some('0') => '\0',
                        Some('$') => '$',
                        Some('"') => '"',
                        Some('\'') => '\'',
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
                kind: if parts.is_empty() {
                    Kind::String(value)
                } else {
                    parts.push((false, value, 0));
                    Kind::Template(parts)
                },
                span,
                end_line: line,
            });
            continue;
        }
        if chars[i..].starts_with(&['.', '.', '.']) {
            i += 3;
            col += 3;
            tokens.push(Token {
                kind: Kind::Symbol("...".into()),
                span,
                end_line: line,
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
                end_line: line,
            });
            continue;
        }
        if "(){}[],;:.+-*/%=!<>&|^~".contains(c) {
            if c == '{' {
                brace_nesting.push(nesting);
                nesting = 0;
            } else if c == '}' {
                nesting = brace_nesting.pop().unwrap_or(0);
            }
            if c == '(' || c == '[' {
                nesting += 1;
            }
            if c == ')' || c == ']' {
                nesting = nesting.saturating_sub(1);
            }
            tokens.push(Token {
                kind: Kind::Symbol(c.to_string()),
                span,
                end_line: line,
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
        end_line: line,
    });
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    #[test]
    fn callback_blocks_keep_newline_statement_boundaries() {
        let source = "fn call(handler fn() void) {}\ncall(fn() {\nlet x = 1\nprint(x)\n})";
        assert!(crate::parser::parse("callback.dev".into(), source).is_ok());
        assert!(crate::parser::parse("call.dev".into(), "print(\n1 +\n2\n)").is_ok());
    }
}
