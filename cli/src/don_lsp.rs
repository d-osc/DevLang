use dev_syntax::don_tooling::{tokens, Kind, Token};
use serde_json::{json, Value};

pub fn byte_position(text: &str, byte: usize) -> Value {
    let prefix = text[..byte.min(text.len())]
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let line = prefix.bytes().filter(|b| *b == b'\n').count();
    let col = prefix
        .rsplit('\n')
        .next()
        .unwrap_or("")
        .encode_utf16()
        .count();
    json!({"line":line,"character":col})
}
fn span(text: &str, start: usize, end: usize) -> Value {
    json!({"start":byte_position(text,start),"end":byte_position(text,end)})
}
pub fn diagnostic(text: &str) -> Vec<Value> {
    let Err(error) = dev_syntax::don::parse(text) else {
        return vec![];
    };
    let (location, message) = error.split_once(": ").unwrap_or(("", &error));
    let coordinates = location
        .strip_prefix("DON ")
        .and_then(|s| s.split_once(':'));
    let (line, col) = coordinates
        .and_then(|(l, c)| Some((l.parse::<usize>().ok()?, c.parse::<usize>().ok()?)))
        .unwrap_or((1, 1));
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let start = super::position(&normalized, line, col);
    let mut end = start.clone();
    let chars = normalized
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .chars()
        .collect::<Vec<_>>();
    let width = chars
        .get(col.saturating_sub(1))
        .map(|ch| ch.len_utf16())
        .unwrap_or(0);
    end["character"] = json!(start["character"].as_u64().unwrap_or(0) + width as u64);
    vec![json!({"range":{"start":start,"end":end},"severity":1,"source":"DON","message":message})]
}
struct Entry {
    path: Vec<String>,
    start: usize,
    end: usize,
    symbol: Value,
}
struct Outline<'a> {
    text: &'a str,
    tokens: Vec<Token>,
    at: usize,
    entries: Vec<Entry>,
}
impl Outline<'_> {
    fn is(&self, ch: char) -> bool {
        self.tokens
            .get(self.at)
            .is_some_and(|t| t.kind == Kind::Punctuation(ch))
    }
    fn consume(&mut self, value: &Value, path: Vec<String>, implicit: bool) -> Vec<Value> {
        let mut children = Vec::new();
        if self.is('{') || implicit {
            if !implicit {
                self.at += 1;
            }
            while self.at < self.tokens.len() && !self.is('}') {
                if self.is(',') || self.is(';') {
                    self.at += 1;
                    continue;
                }
                let token = self.tokens[self.at].clone();
                let raw = &self.text[token.start..token.end];
                let key = dev_syntax::don::parse(&format!("{{{raw}: null}}"))
                    .ok()
                    .and_then(|v| v.as_object()?.keys().next().cloned())
                    .unwrap_or(raw.to_owned());
                self.at += 2; // key plus ':' / '='
                let Some(current) = value.get(&key) else {
                    break;
                };
                let mut current_path = path.clone();
                current_path.push(key.clone());
                let nested = self.consume(current, current_path.clone(), false);
                let end = self
                    .tokens
                    .get(self.at.saturating_sub(1))
                    .map(|t| t.end)
                    .unwrap_or(token.end);
                let kind = match current {
                    Value::Object(_) => 19,
                    Value::Array(_) => 18,
                    Value::String(_) => 15,
                    Value::Number(_) => 16,
                    Value::Bool(_) => 17,
                    _ => 13,
                };
                let symbol = json!({"name":key,"kind":kind,"detail":kind_name(current),
                    "range":span(self.text,token.start,end),"selectionRange":span(self.text,token.start,token.end),"children":nested});
                self.entries.push(Entry {
                    path: current_path,
                    start: token.start,
                    end: token.end,
                    symbol: symbol.clone(),
                });
                children.push(symbol);
            }
            if self.is('}') {
                self.at += 1;
            }
        } else if self.is('[') {
            self.at += 1;
            let mut index = 0;
            while self.at < self.tokens.len() && !self.is(']') {
                if self.is(',') || self.is(';') {
                    self.at += 1;
                    continue;
                }
                let Some(current) = value.get(index) else {
                    break;
                };
                let start = self.tokens[self.at].start;
                let mut next = path.clone();
                next.push(index.to_string());
                let nested = self.consume(current, next, false);
                let end = self.tokens[self.at.saturating_sub(1)].end;
                children.push(json!({"name":format!("[{index}]"),"kind":if current.is_object() {19} else {13},
                    "range":span(self.text,start,end),"selectionRange":span(self.text,start,end),"children":nested}));
                index += 1;
            }
            if self.is(']') {
                self.at += 1;
            }
        } else if self.at < self.tokens.len() {
            self.at += 1;
        }
        children
    }
}
fn kind_name(value: &Value) -> &'static str {
    match value {
        Value::Object(_) => "object",
        Value::Array(_) => "array",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "bool",
        _ => "null",
    }
}
fn outline(text: &str) -> Option<(Vec<Value>, Vec<Entry>)> {
    let value = dev_syntax::don::parse(text)
        .or_else(|_| {
            // Reference completion must also work while the target is incomplete.
            let mut replaced = text.to_owned();
            for token in tokens(text)
                .into_iter()
                .rev()
                .filter(|t| t.kind == Kind::Reference)
            {
                replaced.replace_range(token.start..token.end, "null");
            }
            dev_syntax::don::parse(&replaced)
        })
        .ok()?;
    let tokens: Vec<_> = tokens(text)
        .into_iter()
        .filter(|t| !matches!(t.kind, Kind::Comment | Kind::Newline))
        .collect();
    let implicit = value.is_object()
        && tokens
            .first()
            .is_some_and(|t| t.kind != Kind::Punctuation('{') && t.kind != Kind::Reference);
    let mut parser = Outline {
        text,
        tokens,
        at: 0,
        entries: Vec::new(),
    };
    let symbols = parser.consume(&value, Vec::new(), implicit);
    Some((symbols, parser.entries))
}
pub fn symbols(text: &str) -> Vec<Value> {
    outline(text).map(|p| p.0).unwrap_or_default()
}
fn byte_at(text: &str, position: &Value) -> Option<usize> {
    let target_line = position["line"].as_u64()? as usize;
    let target_col = position["character"].as_u64()? as usize;
    let mut line = 0;
    let mut units = 0;
    let mut cr = false;
    for (offset, ch) in text.char_indices() {
        if cr && ch == '\n' {
            cr = false;
            continue;
        }
        if line == target_line && units >= target_col {
            return Some(offset);
        }
        if ch == '\r' || ch == '\n' {
            line += 1;
            units = 0;
        } else {
            units += ch.len_utf16();
        }
        cr = ch == '\r';
    }
    (line == target_line && units == target_col).then_some(text.len())
}
pub fn hover_or_definition(text: &str, position: &Value, uri: &str, definition: bool) -> Value {
    let Some(at) = byte_at(text, position) else {
        return Value::Null;
    };
    let Some((_, entries)) = outline(text) else {
        return Value::Null;
    };
    if let Some(token) = tokens(text)
        .into_iter()
        .find(|t| t.kind == Kind::Reference && t.start <= at && at <= t.end)
    {
        let target: Vec<_> = text[token.start + 1..token.end]
            .split('.')
            .map(str::to_owned)
            .collect();
        if let Some(entry) = entries.iter().find(|e| e.path == target) {
            return if definition {
                json!({"uri":uri,"range":span(text,entry.start,entry.end)})
            } else {
                json!({"contents":{"kind":"plaintext","value":format!("@{}: {}",target.join("."),entry.symbol["detail"].as_str().unwrap_or(""))}})
            };
        }
    }
    if !definition {
        if let Some(entry) = entries.iter().find(|e| e.start <= at && at <= e.end) {
            return json!({"contents":{"kind":"plaintext","value":format!("{}: {}",entry.path.join("."),entry.symbol["detail"].as_str().unwrap_or(""))}});
        }
    }
    Value::Null
}
pub fn completion(text: &str, position: &Value) -> Value {
    let at = byte_at(text, position).unwrap_or(text.len());
    if tokens(text).iter().any(|t| t.start <= at && at < t.end && matches!(t.kind,Kind::String | Kind::Comment)) {
        return json!({"isIncomplete":false,"items":[]});
    }
    let prefix = &text[..at];
    let fragment = prefix
        .rsplit(|c: char| c.is_whitespace() || "{}[]:=,;\"'".contains(c))
        .next()
        .unwrap_or("");
    if fragment.starts_with('@') {
        if let Some((_, entries)) = outline(text) {
            let typed = &fragment[1..];
            let start = at - typed.len();
            let items: Vec<_> = entries.iter().filter(|e| e.path.iter().all(|p| {
                let mut chars = p.chars();
                chars.next().is_some_and(|c| c.is_alphabetic() || "_$".contains(c))
                    && chars.all(|c| c.is_alphanumeric() || "_-$".contains(c))
            }) && e.path.join(".").starts_with(typed)).map(|entry| {
                let label = entry.path.join(".");
                json!({"label":format!("@{label}"),"kind":10,"textEdit":{"range":span(text,start,at),"newText":label}})
            }).collect();
            return json!({"isIncomplete":false,"items":items});
        }
    }
    json!({"isIncomplete":false,"items":[{"label":"true","kind":14},{"label":"false","kind":14},{"label":"null","kind":14}]})
}
