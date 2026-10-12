//! Dev Object Notation: a deterministic data format, without code execution.
use serde_json::{Map, Value};
use std::collections::HashMap;

pub fn parse(text: &str) -> Result<Value, String> {
    if text.len() > 8 * 1024 * 1024 {
        return Err("DON input exceeds 8 MiB".into());
    }
    let mut p = Parser {
        text,
        at: 0,
        path: Vec::new(),
        refs: HashMap::new(),
        escaped_dollars: Vec::new(),
    };
    p.space()?;
    if p.peek().is_none() {
        return Ok(Value::Object(Map::new()));
    }
    let value = if p.peek() == Some('{') || p.peek() == Some('[') || !p.root_object()? {
        p.value(0)?
    } else {
        p.object(false, 0)?
    };
    p.space()?;
    if p.at != text.len() {
        return p.fail("unexpected trailing input");
    }
    if p.refs.is_empty() {
        Ok(value)
    } else {
        resolve(&value, &p.refs)
    }
}

#[derive(Clone)]
enum Part {
    Literal(Value),
    Reference(Vec<String>),
    Interpolation(Vec<String>),
}
type References = HashMap<Vec<String>, (Vec<Part>, String)>;
fn resolve(root: &Value, refs: &References) -> Result<Value, String> {
    fn visit(
        root: &Value,
        refs: &References,
        path: &mut Vec<String>,
        active: &mut Vec<Vec<String>>,
        depth: usize,
        budget: &mut usize,
    ) -> Result<Value, String> {
        if depth > 128 {
            return Err("DON reference expansion exceeds 128 levels".into());
        }
        if active.contains(path) {
            return Err(format!("DON cyclic reference at {}", path.join(".")));
        }
        active.push(path.clone());
        if let Some((parts, location)) = refs.get(path) {
            let result = (|| {
                let mut values = Vec::new();
                for part in parts {
                    values.push(match part {
                        Part::Literal(value) => value.clone(),
                        Part::Reference(target) => {
                            visit(root, refs, &mut target.clone(), active, depth + 1, budget)?
                        }
                        Part::Interpolation(target) => {
                            let value =
                                visit(root, refs, &mut target.clone(), active, depth + 1, budget)?;
                            Value::String(match value {
                                Value::String(s) => s,
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                Value::Null => "null".into(),
                                _ => return Err("DON interpolation requires a scalar value".into()),
                            })
                        }
                    });
                }
                if values.len() == 1 {
                    return Ok(values.remove(0));
                }
                let mut text = String::new();
                for value in values {
                    let Value::String(piece) = value else {
                        return Err("DON string concatenation requires string operands".into());
                    };
                    *budget = budget
                        .checked_sub(piece.len())
                        .ok_or("DON resolved data exceeds 8 MiB expansion budget")?;
                    text.push_str(&piece);
                }
                Ok(Value::String(text))
            })()
            .map_err(|e: String| format!("{location}: {e}"));
            active.pop();
            return result;
        }
        let mut value = root;
        for part in path.iter() {
            value = match value {
                Value::Object(map) => map.get(part),
                Value::Array(array) => part.parse::<usize>().ok().and_then(|i| array.get(i)),
                _ => None,
            }
            .ok_or_else(|| format!("DON missing reference target @{}", path.join(".")))?;
        }
        let cost = match value {
            Value::String(s) => s.len(),
            Value::Number(n) => n.to_string().len(),
            _ => 1,
        };
        *budget = budget
            .checked_sub(cost + 1)
            .ok_or("DON resolved data exceeds 8 MiB expansion budget")?;
        let result = match value {
            Value::Object(map) => {
                let mut result = Map::new();
                for key in map.keys() {
                    *budget = budget
                        .checked_sub(key.len())
                        .ok_or("DON resolved data exceeds 8 MiB expansion budget")?;
                    path.push(key.clone());
                    let child = visit(root, refs, path, active, depth + 1, budget)?;
                    path.pop();
                    result.insert(key.clone(), child);
                }
                Value::Object(result)
            }
            Value::Array(array) => {
                let mut result = Vec::new();
                for i in 0..array.len() {
                    path.push(i.to_string());
                    result.push(visit(root, refs, path, active, depth + 1, budget)?);
                    path.pop();
                }
                Value::Array(result)
            }
            _ => value.clone(),
        };
        active.pop();
        Ok(result)
    }
    if refs.is_empty() {
        return Ok(root.clone());
    }
    visit(
        root,
        refs,
        &mut Vec::new(),
        &mut Vec::new(),
        0,
        &mut (8 * 1024 * 1024),
    )
}

pub fn stringify(value: &Value) -> Result<String, String> {
    fn render(value: &Value, level: usize) -> Result<String, String> {
        if level > 128 {
            return Err("DON nesting exceeds 128 levels".into());
        }
        let indent = "  ".repeat(level + 1);
        let closing = "  ".repeat(level);
        Ok(match value {
            Value::Object(map) if !map.is_empty() => {
                let mut rows = Vec::new();
                for (key, value) in map {
                    let mut chars = key.chars();
                    let bare = chars
                        .next()
                        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
                        && chars.all(|c| c.is_alphanumeric() || "_-$".contains(c));
                    let key = if bare {
                        key.clone()
                    } else {
                        serde_json::to_string(key).map_err(|e| e.to_string())?
                    };
                    rows.push(format!("{indent}{key}: {}", render(value, level + 1)?));
                }
                format!("{{\n{}\n{closing}}}", rows.join("\n"))
            }
            Value::Array(values) if !values.is_empty() => {
                let rows = values
                    .iter()
                    .map(|v| Ok(format!("{indent}{}", render(v, level + 1)?)))
                    .collect::<Result<Vec<_>, String>>()?;
                format!("[\n{}\n{closing}]", rows.join("\n"))
            }
            Value::String(text) => serde_json::to_string(text)
                .map_err(|e| e.to_string())?
                .replace("${", "\\${"),
            _ => serde_json::to_string(value).map_err(|e| e.to_string())?,
        })
    }
    render(value, 0)
}

struct Parser<'a> {
    text: &'a str,
    at: usize,
    path: Vec<String>,
    refs: References,
    escaped_dollars: Vec<usize>,
}
impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.text[self.at..].chars().next()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += c.len_utf8();
        Some(c)
    }
    fn fail<T>(&self, message: &str) -> Result<T, String> {
        let prefix = &self.text[..self.at];
        let mut line = 1;
        let mut col = 1;
        let mut cr = false;
        for c in prefix.chars() {
            if c == '\r' {
                line += 1;
                col = 1;
            } else if c == '\n' {
                if !cr {
                    line += 1;
                }
                col = 1;
            } else {
                col += 1;
            }
            cr = c == '\r';
        }
        Err(format!("DON {line}:{col}: {message}"))
    }
    fn space(&mut self) -> Result<bool, String> {
        let mut newline = false;
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                newline |= matches!(self.bump(), Some('\n' | '\r'));
            }
            let tail = &self.text[self.at..];
            if tail.starts_with('#') || tail.starts_with("//") {
                while self.peek().is_some_and(|c| c != '\n' && c != '\r') {
                    self.bump();
                }
            } else if tail.starts_with("/*") {
                self.at += 2;
                if let Some(end) = self.text[self.at..].find("*/") {
                    newline |= self.text[self.at..self.at + end].contains('\n');
                    self.at += end + 2;
                } else {
                    return self.fail("unterminated block comment");
                }
            } else {
                break;
            }
        }
        Ok(newline)
    }
    fn key(&mut self) -> Result<String, String> {
        match self.peek() {
            Some('"' | '\'') => self.string(),
            Some(c) if c.is_alphabetic() || c == '_' || c == '$' => {
                let start = self.at;
                while self
                    .peek()
                    .is_some_and(|c| c.is_alphanumeric() || "_-$".contains(c))
                {
                    self.bump();
                }
                Ok(self.text[start..self.at].into())
            }
            _ => self.fail("expected quoted key or identifier"),
        }
    }
    fn root_object(&mut self) -> Result<bool, String> {
        let save = self.at;
        let result = self.key().is_ok() && {
            self.space()?;
            matches!(self.peek(), Some(':' | '='))
        };
        self.at = save;
        Ok(result)
    }
    fn string(&mut self) -> Result<String, String> {
        self.escaped_dollars.clear();
        let quote = self.bump().unwrap();
        let start = self.at;
        if quote == '"' && self.text[start..].starts_with("\"\"") {
            self.at += 2;
            let start = self.at;
            let Some(end) = self.text[start..].find("\"\"\"") else {
                return self.fail("unterminated multiline string");
            };
            self.at += end + 3;
            return Ok(self.text[start..start + end].into());
        }
        let mut result = String::new();
        loop {
            match self.bump() {
                Some(c) if c == quote => return Ok(result),
                Some('\\') => match self.bump() {
                    Some('$') => {
                        self.escaped_dollars.push(result.len());
                        result.push('$');
                    }
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('b') => result.push('\u{8}'),
                    Some('f') => result.push('\u{c}'),
                    Some(c @ ('"' | '\'' | '\\' | '/')) => result.push(c),
                    Some('u') => {
                        // Delegate surrogate pairs and Unicode escape validation to JSON.
                        let begin = self.at - 2;
                        for _ in 0..4 {
                            if !self.bump().is_some_and(|c| c.is_ascii_hexdigit()) {
                                return self.fail("invalid Unicode escape");
                            }
                        }
                        let first =
                            u16::from_str_radix(&self.text[begin + 2..self.at], 16).unwrap();
                        if (0xd800..=0xdbff).contains(&first)
                            && self.text[self.at..].starts_with("\\u")
                        {
                            self.at += 2;
                            for _ in 0..4 {
                                if !self.bump().is_some_and(|c| c.is_ascii_hexdigit()) {
                                    return self.fail("invalid Unicode surrogate");
                                }
                            }
                        }
                        let encoded = format!("\"{}\"", &self.text[begin..self.at]);
                        let decoded: String = serde_json::from_str(&encoded).map_err(|_| {
                            self.fail::<String>("invalid Unicode escape").unwrap_err()
                        })?;
                        result.push_str(&decoded);
                    }
                    _ => return self.fail("invalid string escape"),
                },
                Some(c) if c < ' ' => {
                    return self
                        .fail("control character in string; use triple quotes for multiline text")
                }
                Some(c) => result.push(c),
                None => return self.fail("unterminated string"),
            }
        }
    }
    fn separator(&mut self, end: Option<char>) -> Result<bool, String> {
        let newline = self.space()?;
        if self.peek() == end {
            return Ok(false);
        }
        if matches!(self.peek(), Some(',' | ';')) {
            self.bump();
            self.space()?;
            return Ok(self.peek() != end);
        }
        if newline {
            return Ok(true);
        }
        self.fail("expected comma, semicolon or newline between entries")
    }
    fn object(&mut self, braces: bool, depth: usize) -> Result<Value, String> {
        if braces {
            self.bump();
        }
        self.space()?;
        let end = if braces { Some('}') } else { None };
        let mut map = Map::new();
        while self.peek() != end {
            if self.peek().is_none() {
                return self.fail("unterminated object");
            }
            let key = self.key()?;
            self.space()?;
            if !matches!(self.bump(), Some(':' | '=')) {
                return self.fail("expected ':' or '=' after key");
            }
            self.path.push(key.clone());
            let value = self.value(depth + 1)?;
            self.path.pop();
            if map.insert(key, value).is_some() {
                return self.fail("duplicate object key");
            }
            if !self.separator(end)? {
                break;
            }
        }
        if braces && self.bump() != Some('}') {
            return self.fail("expected '}'");
        }
        Ok(Value::Object(map))
    }
    fn value(&mut self, depth: usize) -> Result<Value, String> {
        let first = self.atom(depth)?;
        let mut parts = self
            .refs
            .remove(&self.path)
            .map(|(p, _)| p)
            .unwrap_or_else(|| vec![Part::Literal(first.clone())]);
        let mut joined = false;
        loop {
            let saved = self.at;
            self.space()?;
            if self.peek() != Some('+') {
                self.at = saved;
                break;
            }
            joined = true;
            self.bump();
            let next = self.atom(depth)?;
            parts.extend(
                self.refs
                    .remove(&self.path)
                    .map(|(p, _)| p)
                    .unwrap_or_else(|| vec![Part::Literal(next)]),
            );
        }
        if joined || parts.iter().any(|p| !matches!(p, Part::Literal(_))) {
            if joined {
                parts.insert(0, Part::Literal(Value::String(String::new())));
            }
            let location = self.fail::<Value>("string expression").unwrap_err();
            self.refs.insert(self.path.clone(), (parts, location));
            Ok(Value::Null)
        } else {
            Ok(first)
        }
    }
    fn atom(&mut self, depth: usize) -> Result<Value, String> {
        if depth > 128 {
            return self.fail("nesting exceeds 128 levels");
        }
        self.space()?;
        match self.peek() {
            Some('@') => {
                let location = self.fail::<Value>("reference").unwrap_err();
                self.bump();
                let mut target = Vec::new();
                loop {
                    if !self
                        .peek()
                        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
                    {
                        return self.fail("expected reference identifier after '@' or '.'");
                    }
                    target.push(self.key()?);
                    if self.peek() != Some('.') {
                        break;
                    }
                    self.bump();
                }
                self.refs
                    .insert(self.path.clone(), (vec![Part::Reference(target)], location));
                Ok(Value::Null)
            }
            Some('{') => self.object(true, depth),
            Some('[') => {
                self.bump();
                self.space()?;
                let mut values = Vec::new();
                while self.peek() != Some(']') {
                    self.path.push(values.len().to_string());
                    values.push(self.value(depth + 1)?);
                    self.path.pop();
                    if !self.separator(Some(']'))? {
                        break;
                    }
                }
                if self.bump() != Some(']') {
                    return self.fail("expected ']'");
                }
                Ok(Value::Array(values))
            }
            Some('"' | '\'') => {
                let interpolate =
                    self.peek() == Some('"') && !self.text[self.at..].starts_with("\"\"\"");
                let text = self.string()?;
                if !interpolate {
                    return Ok(Value::String(text));
                }
                let mut parts = Vec::new();
                let mut at = 0;
                let mut tail = 0;
                while let Some(offset) = text[at..].find("${") {
                    let begin = at + offset;
                    if self.escaped_dollars.contains(&begin) {
                        at = begin + 2;
                        continue;
                    }
                    parts.push(Part::Literal(Value::String(text[tail..begin].into())));
                    let Some(end) = text[begin + 2..].find('}') else {
                        return self.fail("unterminated string interpolation");
                    };
                    let end = begin + 2 + end;
                    let name = text[begin + 2..end].trim().trim_start_matches('@');
                    let path = name.split('.').map(str::to_owned).collect::<Vec<_>>();
                    if path.iter().any(|p| {
                        p.is_empty() || !p.chars().all(|c| c.is_alphanumeric() || "_-$".contains(c))
                    }) {
                        return self.fail("interpolation expects a reference path");
                    }
                    parts.push(Part::Interpolation(path));
                    at = end + 1;
                    tail = at;
                }
                if parts.is_empty() {
                    return Ok(Value::String(text));
                }
                parts.push(Part::Literal(Value::String(text[tail..].into())));
                let location = self.fail::<Value>("string interpolation").unwrap_err();
                self.refs.insert(self.path.clone(), (parts, location));
                Ok(Value::Null)
            }
            _ => {
                let start = self.at;
                while self
                    .peek()
                    .is_some_and(|c| !c.is_whitespace() && !",;]}#".contains(c))
                {
                    if self.text[self.at..].starts_with("//")
                        || self.text[self.at..].starts_with("/*")
                    {
                        break;
                    }
                    if self.peek() == Some('+') {
                        let numeric = self.text[start..]
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_digit() || c == '-');
                        if !numeric || !self.text[..self.at].ends_with(['e', 'E']) {
                            break;
                        }
                    }
                    self.bump();
                }
                let token = &self.text[start..self.at];
                if token.is_empty() {
                    return self.fail("expected value");
                }
                match token {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    "null" => Ok(Value::Null),
                    _ => {
                        if !token
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_ascii_digit() || c == '-')
                        {
                            let path = token.split('.').map(str::to_owned).collect::<Vec<_>>();
                            if path.iter().any(|p| {
                                p.is_empty()
                                    || !p.chars().all(|c| c.is_alphanumeric() || "_-$".contains(c))
                            }) {
                                return self.fail("strings must be quoted");
                            }
                            let location = self.fail::<Value>("reference").unwrap_err();
                            self.refs
                                .insert(self.path.clone(), (vec![Part::Reference(path)], location));
                            return Ok(Value::Null);
                        }
                        // Digit separators may only appear between decimal digits.
                        let bytes = token.as_bytes();
                        for (i, b) in bytes.iter().enumerate() {
                            if *b == b'_'
                                && (i == 0
                                    || i + 1 == bytes.len()
                                    || !bytes[i - 1].is_ascii_digit()
                                    || !bytes[i + 1].is_ascii_digit())
                            {
                                return self.fail("invalid numeric separator");
                            }
                        }
                        let value: Value = serde_json::from_str(&token.replace('_', ""))
                            .map_err(|_| self.fail::<Value>("invalid number").unwrap_err())?;
                        if !value.is_number() {
                            return self.fail("expected number");
                        }
                        Ok(value)
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn references_are_root_relative_typed_and_checked() {
        let value = parse("copy:@config\nversion:'v1.0.0'\nconfig:{rev:@version\nport:3000}\nports:[@config.port, @config.port]\nliteral:'@version'").unwrap();
        assert_eq!(value["copy"]["rev"], "v1.0.0");
        assert_eq!(value["ports"], serde_json::json!([3000, 3000]));
        assert_eq!(value["literal"], "@version");
        assert_eq!(parse(&stringify(&value).unwrap()).unwrap(), value);
        for source in [
            "a:@a",
            "a:@b\nb:@a",
            "a:{child:@a}",
            "a:@missing",
            "a:@",
            "a:@config.missing\nconfig:{}",
        ] {
            assert!(parse(source).is_err(), "accepted {source}");
        }
        let mut bomb = "a0:'abcdefghijklmnopqrstuvwxyz'\n".to_string();
        for i in 1..23 {
            bomb.push_str(&format!("a{i}:[@a{},@a{}]\n", i - 1, i - 1));
        }
        assert!(parse(&bomb).unwrap_err().contains("budget"));
    }
    #[test]
    fn flexible_and_roundtrip() {
        let value = parse("# config\nname = 'Dev'\nnested: { enabled: true; count: 1_000, }\nitems: [1\n2, null,]\ntext: \"\"\"hello\nworld\"\"\"\nemoji: '\\uD83D\\uDE80'").unwrap();
        assert_eq!(value["nested"]["count"], 1000);
        assert_eq!(value["emoji"], "🚀");
        assert_eq!(value["text"], "hello\nworld");
        assert_eq!(parse(&stringify(&value).unwrap()).unwrap(), value);
        assert_eq!(parse("# empty").unwrap(), serde_json::json!({}));
        assert_eq!(parse("a:1\rb:2").unwrap()["b"], 2);
        assert_eq!(parse("# comment\ra:1\r// comment\rb:2").unwrap()["b"], 2);
        for text in [
            r#"{"@app": "C:\\work", "ภาษา": [null, {}, []], "number": 184467440737095516161}"#,
            r#"{"quote": "a\"b", "slash": "a\\b", "newline": "a\nb"}"#,
        ] {
            let value: Value = serde_json::from_str(text).unwrap();
            assert_eq!(parse(&stringify(&value).unwrap()).unwrap(), value);
        }
        assert_eq!(
            parse("{\"x\": [true, false], \"n\": 1e3}").unwrap()["n"],
            1000.0
        );
    }
    #[test]
    fn rejects_ambiguity_and_bad_input() {
        for text in [
            "x:1 x:2",
            "x:1\nx:2",
            "x: bare",
            "[1 2]",
            "{a:1",
            "/* unfinished",
            "x:'\\uD800'",
            "x:1__0",
            "x:NaN",
            "true false",
            "x:'bad\ntext'",
        ] {
            assert!(parse(text).is_err(), "accepted {text}");
        }
        assert!(parse(&format!("{}0{}", "[".repeat(130), "]".repeat(130))).is_err());
        assert!(parse(&" ".repeat(8 * 1024 * 1024 + 1)).is_err());
        assert!(parse("a:1\nb:").unwrap_err().contains("2:"));
    }
}

#[cfg(test)]
mod string_expression_tests {
    use super::*;
    #[test]
    fn concatenation_interpolation_and_forward_references() {
        let value = parse(
            r#"greeting: "Hello " + name + "!"
message: "${greeting} ${age} ${ready}"
name: 'Dev'
age: 18
ready: true
literal: "\${name}"
raw: '${name}'
"#,
        )
        .unwrap();
        assert_eq!(value["greeting"], "Hello Dev!");
        assert_eq!(value["message"], "Hello Dev! 18 true");
        assert_eq!(value["literal"], "${name}");
        assert_eq!(value["raw"], "${name}");
        assert_eq!(parse(&stringify(&value).unwrap()).unwrap(), value);
    }
    #[test]
    fn expressions_preserve_error_boundaries() {
        for text in [
            r#"a: "${missing}""#,
            r#"a: "${a}""#,
            r#"a: 'x'+1"#,
            r#"a: "${}""#,
            r#"a: "${name""#,
            r#"a: "${x}"; x: {}"#,
        ] {
            assert!(parse(text).is_err(), "accepted {text}");
        }
        assert_eq!(
            parse(r#"x: {name:'Dev'}; a: "Hi ${x.name}""#).unwrap()["a"],
            "Hi Dev"
        );
    }
}
