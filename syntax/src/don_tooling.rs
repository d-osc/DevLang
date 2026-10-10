//! Lossless tokens and editor formatting for DON; parsing still owns validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Atom,
    String,
    Reference,
    Comment,
    Newline,
    Punctuation(char),
}
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}
pub fn tokens(text: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        let start = at;
        let ch = text[at..].chars().next().unwrap();
        let kind;
        if ch == '\n' || ch == '\r' {
            at += 1;
            if ch == '\r' && text[at..].starts_with('\n') {
                at += 1;
            }
            kind = Kind::Newline;
        } else if ch.is_whitespace() {
            at += ch.len_utf8();
            continue;
        } else if text[at..].starts_with("//") || ch == '#' {
            at = text[at..]
                .find(['\n', '\r'])
                .map(|n| at + n)
                .unwrap_or(text.len());
            kind = Kind::Comment;
        } else if text[at..].starts_with("/*") {
            at = text[at + 2..]
                .find("*/")
                .map(|n| at + n + 4)
                .unwrap_or(text.len());
            kind = Kind::Comment;
        } else if ch == '\'' || ch == '"' {
            let triple = ch.to_string().repeat(3);
            if ch == '"' && text[at..].starts_with(&triple) {
                at = text[at + 3..]
                    .find(&triple)
                    .map(|n| at + n + 6)
                    .unwrap_or(text.len());
            } else {
                at += 1;
                while at < text.len() {
                    let next = text[at..].chars().next().unwrap();
                    at += next.len_utf8();
                    if next == '\\' && at < text.len() {
                        at += text[at..].chars().next().unwrap().len_utf8();
                    } else if next == ch {
                        break;
                    }
                }
            }
            kind = Kind::String;
        } else if "{}[]:=,;".contains(ch) {
            at += 1;
            kind = Kind::Punctuation(ch);
        } else {
            at += ch.len_utf8();
            while at < text.len() {
                let next = text[at..].chars().next().unwrap();
                if next.is_whitespace()
                    || "{}[]:=,;#\"'".contains(next)
                    || text[at..].starts_with("//")
                    || text[at..].starts_with("/*")
                {
                    break;
                }
                at += next.len_utf8();
            }
            kind = if ch == '@' {
                Kind::Reference
            } else {
                Kind::Atom
            };
        }
        out.push(Token {
            kind,
            start,
            end: at,
        });
    }
    out
}
pub fn format(text: &str) -> Result<String, String> {
    let before = crate::don::parse(text)?;
    let tokens = tokens(text);
    let mut out = String::new();
    let mut depth = 0usize;
    fn newline(out: &mut String) {
        while out.ends_with(' ') || out.ends_with('\t') || out.ends_with('\r') {
            out.pop();
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
    }
    fn emit(out: &mut String, depth: usize, text: &str) {
        if out.is_empty() || out.ends_with('\n') {
            out.push_str(&"    ".repeat(depth));
        }
        out.push_str(text);
    }
    for (index, token) in tokens.iter().enumerate() {
        let raw = &text[token.start..token.end];
        let previous = index.checked_sub(1).map(|i| tokens[i].kind);
        match token.kind {
            Kind::Newline => newline(&mut out),
            Kind::Punctuation('{' | '[') => {
                emit(&mut out, depth, raw);
                let close = if raw == "{" { '}' } else { ']' };
                if tokens
                    .get(index + 1)
                    .is_some_and(|t| t.kind == Kind::Punctuation(close))
                {
                    continue;
                }
                depth += 1;
                newline(&mut out);
            }
            Kind::Punctuation('}' | ']') => {
                let open = if raw == "}" { '{' } else { '[' };
                if previous != Some(Kind::Punctuation(open)) {
                    depth = depth.saturating_sub(1);
                    newline(&mut out);
                }
                emit(&mut out, depth, raw);
            }
            Kind::Punctuation(',' | ';') => {
                out.push_str(raw);
                newline(&mut out);
            }
            Kind::Punctuation(':') => {
                out.push_str(": ");
            }
            Kind::Punctuation('=') => {
                out.push_str(" = ");
            }
            Kind::Comment => {
                if !out.is_empty() && !out.ends_with(['\n', ' ', '\t']) {
                    out.push(' ');
                }
                emit(&mut out, depth, raw);
                if raw.starts_with('#') || raw.starts_with("//") {
                    newline(&mut out);
                } else {
                    out.push(' ');
                }
            }
            _ => {
                if previous == Some(Kind::Comment) && !out.ends_with(['\n', ' ']) {
                    out.push(' ');
                }
                emit(&mut out, depth, raw);
            }
        }
    }
    newline(&mut out);
    if crate::don::parse(&out)? != before {
        return Err("DON formatting changed data".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_comments_references_and_multiline_bytes() {
        let text = "// config\nversion='v1'\nobj:{rev:@version; list:[1,2]} # tail\nraw:\"\"\"a\n  b\n\"\"\"\nquoted: \"@version\"\n";
        let result = format(text).unwrap();
        assert!(result.contains("rev: @version"));
        assert!(result.contains("\"\"\"a\n  b\n\"\"\""));
        let significant = |s: &str| {
            tokens(s)
                .into_iter()
                .filter(|t| t.kind != Kind::Newline)
                .map(|t| (t.kind, s[t.start..t.end].to_owned()))
                .collect::<Vec<_>>()
        };
        assert_eq!(significant(text), significant(&result));
        assert_eq!(format(&result).unwrap(), result);
    }
    #[test]
    fn validates_before_formatting_and_handles_empty_containers() {
        assert!(format("a: @missing").is_err());
        for text in [
            "{a:{},b:[]}",
            "a: {\n}\nb: [\n]\n",
            "[1\n2\n3]",
            "a /* key */ : 1\nb: 2",
            "a:1\rb:2",
            "# comment\ra:1\r// other\rb:2",
        ] {
            let result = format(text).unwrap();
            assert_eq!(
                crate::don::parse(text).unwrap(),
                crate::don::parse(&result).unwrap()
            );
            assert_eq!(format(&result).unwrap(), result);
        }
    }
}
