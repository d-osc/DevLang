use dev_syntax::{
    ast::Module,
    lexer::{lex, Kind},
    parser,
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    io::{BufRead, Write},
    path::PathBuf,
};

struct Document {
    text: String,
    version: i64,
}
fn position(text: &str, line: usize, col: usize) -> Value {
    let utf16 = text
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .chars()
        .take(col.saturating_sub(1))
        .map(char::len_utf16)
        .sum::<usize>();
    json!({"line":line.saturating_sub(1),"character":utf16})
}
fn range(text: &str, line: usize, col: usize, width: usize) -> Value {
    json!({"start":position(text,line,col),"end":position(text,line,col+width)})
}
fn path(uri: &str) -> PathBuf {
    url::Url::parse(uri)
        .ok()
        .and_then(|u| u.to_file_path().ok())
        .unwrap_or_else(|| PathBuf::from("<editor>"))
}
fn parsed(uri: &str, doc: &Document) -> Result<Module, String> {
    parser::parse(path(uri), &doc.text)
}
fn symbols(uri: &str, doc: &Document) -> Vec<Value> {
    let Ok(module) = parsed(uri, doc) else {
        return vec![];
    };
    let mut result = Vec::new();
    for f in module.functions {
        let r = range(&doc.text, f.span.line, f.span.col, 1);
        result.push(json!({"name":f.name,"kind":12,"range":r,"selectionRange":r,"detail":format!("fn({}) {}",f.params.iter().map(|(n,t)| format!("{n} {}",t.name())).collect::<Vec<_>>().join(", "),f.ret.name())}));
    }
    for d in module.definitions {
        let r = range(&doc.text, d.span.line, d.span.col, 1);
        result.push(json!({"name":d.name,"kind":if d.enumeration {10} else {23},"range":r,"selectionRange":r,"detail":if d.enumeration {"enum"} else {"struct"}}));
    }
    result
}
fn word(text: &str, p: &Value) -> Option<String> {
    let line = text.lines().nth(p["line"].as_u64()? as usize)?;
    let target = p["character"].as_u64()? as usize;
    let mut units = 0;
    let mut column = 1;
    for ch in line.chars() {
        if units >= target {
            break;
        }
        units += ch.len_utf16();
        column += 1;
    }
    let tokens = lex(text).ok()?;
    tokens.into_iter().find_map(|t| {
        if let Kind::Word(w) = t.kind {
            if t.span.line == p["line"].as_u64()? as usize + 1
                && t.span.col <= column
                && column <= t.span.col + w.len()
            {
                Some(w)
            } else {
                None
            }
        } else {
            None
        }
    })
}
fn diagnostics(uri: &str, doc: &Document) -> Value {
    let errors = match parsed(uri, doc) {
        Ok(module) => {
            let tokens = lex(&doc.text).unwrap_or_default();
            crate::unused::variables(&module).into_iter().filter_map(|(name, span)| {
                // Find the declaration's source token; skip compiler-generated for bindings.
                let index = tokens.iter().position(|t| t.span.line == span.line && t.span.col == span.col)?;
                let keyword = &tokens[index].kind;
                let declaration = if matches!(keyword, Kind::Word(w) if w == "let" || w == "for") {
                    tokens.get(index + 1)?
                } else { return None; };
                if !matches!(&declaration.kind, Kind::Word(w) if w == &name) { return None; }
                Some(json!({"range":range(&doc.text,declaration.span.line,declaration.span.col,name.chars().count()),"severity":4,"tags":[1],"code":"unused-variable","source":"DevLang","message":format!("Variable '{name}' is declared but never read")}))
            }).collect()
        },
        Err(error) => {
            // Parse from the right-hand diagnostic suffix; paths may contain colons.
            let suffix = error
                .rsplit_once(": ")
                .map(|(where_, message)| (where_, message));
            let (line, col, message) = suffix
                .and_then(|(where_, message)| {
                    let (rest, col) = where_.rsplit_once(':')?;
                    let (_, line) = rest.rsplit_once(':')?;
                    Some((
                        line.parse::<usize>().ok()?,
                        col.parse::<usize>().ok()?,
                        message,
                    ))
                })
                .unwrap_or((1, 1, error.as_str()));
            vec![
                json!({"range":range(&doc.text,line,col,1),"severity":1,"source":"DevLang parser","message":message}),
            ]
        }
    };
    json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"version":doc.version,"diagnostics":errors}})
}

#[cfg(test)]
mod unused_tests {
    use super::*;
    #[test]
    fn reports_exact_unused_name_range_and_tag() {
        let doc = Document { text: "fn main() {\nlet unused = 1\nlet used = 2\nprint(used)\n}\nmain()\n".into(), version: 1 };
        let result = diagnostics("file:///test.dev", &doc);
        let items = result["params"]["diagnostics"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["tags"], json!([1]));
        assert_eq!(items[0]["range"], json!({"start":{"line":1,"character":4},"end":{"line":1,"character":10}}));
    }
}
fn send(out: &mut impl Write, message: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    write!(out, "Content-Length: {}\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
    out.write_all(&body).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())
}
fn receive(input: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return if length.is_none() {
                Ok(None)
            } else {
                Err("truncated LSP header".into())
            };
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if line.len() > 8192 {
            return Err("LSP header too large".into());
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.eq_ignore_ascii_case("Content-Length") {
                if length.is_some() {
                    return Err("duplicate Content-Length".into());
                }
                length = Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| "invalid Content-Length")?,
                );
            }
        }
    }
    let length = length.ok_or("missing Content-Length")?;
    if length > 16 * 1024 * 1024 {
        return Err("LSP message exceeds 16 MiB".into());
    }
    let mut body = vec![0; length];
    input.read_exact(&mut body).map_err(|e| e.to_string())?;
    Ok(Some(
        serde_json::from_slice(&body).map_err(|e| e.to_string())?,
    ))
}
pub fn command(args: &[String]) -> Result<i32, String> {
    if !args.is_empty() && args != ["--stdio"] {
        return Err("usage: d lsp [--stdio]".into());
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut out = stdout.lock();
    let mut docs = HashMap::<String, Document>::new();
    let mut initialized = false;
    let mut shutdown = false;
    while let Some(message) = receive(&mut input)? {
        let method = message["method"].as_str().unwrap_or("");
        let id = message.get("id");
        let params = &message["params"];
        if method == "exit" {
            return Ok(if shutdown { 0 } else { 1 });
        }
        if method == "initialize" && !initialized {
            initialized = true;
            if let Some(id) = id {
                send(
                    &mut out,
                    &json!({"jsonrpc":"2.0","id":id,"result":{"capabilities":{"positionEncoding":"utf-16","textDocumentSync":{"openClose":true,"change":1},"completionProvider":{"triggerCharacters":["."]},"hoverProvider":true,"definitionProvider":true,"documentSymbolProvider":true,"documentFormattingProvider":true},"serverInfo":{"name":"DevLang LSP","version":env!("CARGO_PKG_VERSION")}}}),
                )?;
            }
            continue;
        }
        if !initialized || shutdown {
            if let Some(id) = id {
                send(
                    &mut out,
                    &json!({"jsonrpc":"2.0","id":id,"error":{"code":if !initialized {-32002} else {-32600},"message":"server not initialized or already shut down"}}),
                )?;
            }
            continue;
        }
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        match method {
            "textDocument/didOpen" => {
                if let (Some(text), Some(version)) = (
                    params["textDocument"]["text"].as_str(),
                    params["textDocument"]["version"].as_i64(),
                ) {
                    docs.insert(
                        uri.into(),
                        Document {
                            text: text.into(),
                            version,
                        },
                    );
                    send(&mut out, &diagnostics(uri, &docs[uri]))?;
                }
            }
            "textDocument/didChange" => {
                if let Some(doc) = docs.get_mut(uri) {
                    if let (Some(version), Some(changes)) = (
                        params["textDocument"]["version"].as_i64(),
                        params["contentChanges"].as_array(),
                    ) {
                        if version > doc.version
                            && changes
                                .iter()
                                .all(|c| c.get("range").is_none() && c["text"].is_string())
                        {
                            for change in changes {
                                doc.text = change["text"].as_str().unwrap().into();
                            }
                            doc.version = version;
                            send(&mut out, &diagnostics(uri, doc))?;
                        }
                    }
                }
            }
            "textDocument/didClose" => {
                docs.remove(uri);
                send(
                    &mut out,
                    &json!({"jsonrpc":"2.0","method":"textDocument/publishDiagnostics","params":{"uri":uri,"diagnostics":[]}}),
                )?;
            }
            "initialized"
            | "$/cancelRequest"
            | "textDocument/didSave"
            | "workspace/didChangeConfiguration" => {}
            _ if id.is_some() => {
                let mut error = None;
                let result = match method {
                    "shutdown" => { shutdown=true; Value::Null }
                    "textDocument/completion" => {
                        let mut items = "fn let if else while for in return break continue struct enum match unsafe use export as true false null i64 i32 u64 f64 bool str Vec Map Ref Slice".split_whitespace().map(|s|json!({"label":s,"kind":14})).collect::<Vec<_>>();
                        if let Some(doc)=docs.get(uri) { for symbol in symbols(uri,doc) { items.push(json!({"label":symbol["name"],"kind":if symbol["kind"]==12 {3} else {22},"detail":symbol["detail"]})); } }
                        json!({"isIncomplete":false,"items":items})
                    }
                    "textDocument/documentSymbol" => docs.get(uri).map(|d|json!(symbols(uri,d))).unwrap_or(json!([])),
                    "textDocument/hover" | "textDocument/definition" => {
                        docs.get(uri).and_then(|doc| {
                            let name=word(&doc.text,&params["position"])?;
                            let symbol=symbols(uri,doc).into_iter().find(|s|s["name"]==name)?;
                            Some(if method.ends_with("definition") {json!({"uri":uri,"range":symbol["selectionRange"]})} else {json!({"contents":{"kind":"plaintext","value":format!("{} {}",name,symbol["detail"].as_str().unwrap_or(""))}})})
                        }).unwrap_or(Value::Null)
                    }
                    "textDocument/formatting" => {
                        if let Some(doc)=docs.get(uri) {
                            match crate::format::source(&doc.text) {
                                Ok(text) if text!=doc.text => {
                                    let line=doc.text.bytes().filter(|b|*b==b'\n').count();
                                    let column=doc.text.rsplit('\n').next().unwrap_or("").encode_utf16().count();
                                    json!([{"range":{"start":{"line":0,"character":0},"end":{"line":line,"character":column}},"newText":text}])
                                }
                                Ok(_) => json!([]),
                                Err(e) => {error=Some((-32602,e)); Value::Null}
                            }
                        } else {error=Some((-32602,"document is not open".into())); Value::Null}
                    }
                    _ => {error=Some((-32601,format!("unsupported method {method}"))); Value::Null}
                };
                let reply = if let Some((code, message)) = error {
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
                } else {
                    json!({"jsonrpc":"2.0","id":id,"result":result})
                };
                send(&mut out, &reply)?;
            }
            _ => {}
        }
    }
    Ok(if shutdown { 0 } else { 1 })
}
