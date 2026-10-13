use super::*;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;
struct Entry {
    hash: u64,
    stamp: Option<(SystemTime, u64)>,
    symbols: Vec<Value>,
    imports: Vec<dev_syntax::ast::Import>,
}
#[derive(Default)]
pub(super) struct WorkspaceIndex {
    roots: Vec<PathBuf>,
    entries: HashMap<String, Entry>,
}
fn uri_for(file: &std::path::Path) -> Option<String> {
    let text = file.to_string_lossy();
    let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
    url::Url::from_file_path(text).ok().map(|u| u.to_string())
}
fn equal_path(a: &std::path::Path, b: &std::path::Path) -> bool {
    let a = a.canonicalize().unwrap_or_else(|_| a.to_owned());
    let b = b.canonicalize().unwrap_or_else(|_| b.to_owned());
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}
impl WorkspaceIndex {
    pub(super) fn initialize(&mut self, params: &Value) {
        if let Some(folders) = params["workspaceFolders"].as_array() {
            self.roots = folders
                .iter()
                .filter_map(|f| f["uri"].as_str().map(path))
                .collect();
        } else if let Some(uri) = params["rootUri"].as_str() {
            self.roots = vec![path(uri)];
        }
        self.roots.retain(|p| p.is_dir());
    }
    fn update(&mut self, uri: &str, doc: &Document, stamp: Option<(SystemTime, u64)>) {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        doc.text.hash(&mut h);
        let hash = h.finish();
        if let Some(e) = self.entries.get_mut(uri) {
            if e.hash == hash {
                e.stamp = stamp;
                return;
            }
        }
        let module = parsed(uri, doc).ok();
        let rows = if is_don(uri, doc) {
            don_lsp::symbols(&doc.text)
        } else {
            module
                .as_ref()
                .map(|m| module_symbols(doc, m))
                .unwrap_or_default()
        };
        let imports = module.map(|m| m.imports).ok_or(()).unwrap_or_else(|_| {
            let source = doc
                .text
                .lines()
                .filter(|line| line.trim_start().starts_with("use "))
                .collect::<Vec<_>>()
                .join("\n");
            parser::parse(path(uri), &source)
                .map(|m| m.imports)
                .unwrap_or_default()
        });
        self.entries.insert(
            uri.into(),
            Entry {
                hash,
                stamp,
                symbols: rows,
                imports,
            },
        );
    }
    pub(super) fn document(&mut self, uri: &str, doc: &Document) -> Vec<Value> {
        self.update(uri, doc, None);
        self.entries
            .get(uri)
            .map(|e| e.symbols.clone())
            .unwrap_or_default()
    }
    fn load(&mut self, uri: &str, docs: &HashMap<String, Document>) {
        if let Some(doc) = docs.get(uri) {
            self.update(uri, doc, None);
            return;
        }
        let file = path(uri);
        let Ok(meta) = std::fs::metadata(&file) else {
            self.entries.remove(uri);
            return;
        };
        if meta.len() > 8 * 1024 * 1024 {
            self.entries.remove(uri);
            return;
        }
        let stamp = meta.modified().ok().map(|t| (t, meta.len()));
        if self
            .entries
            .get(uri)
            .is_some_and(|e| e.stamp.is_some() && e.stamp == stamp)
        {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(file) {
            self.update(
                uri,
                &Document {
                    text,
                    version: 0,
                    language: "devlang".into(),
                },
                stamp,
            );
        }
    }
    fn scan(&mut self, docs: &HashMap<String, Document>) {
        let mut found = HashSet::new();
        let mut stack = self
            .roots
            .iter()
            .cloned()
            .map(|p| (p, 0))
            .collect::<Vec<_>>();
        while let Some((dir, depth)) = stack.pop() {
            if depth > 64 || found.len() >= 10000 {
                break;
            }
            let Ok(items) = std::fs::read_dir(dir) else {
                continue;
            };
            for item in items.flatten() {
                if found.len() >= 10000 {
                    break;
                }
                let file = item.path();
                let Ok(meta) = std::fs::symlink_metadata(&file) else {
                    continue;
                };
                if meta.file_type().is_symlink() {
                    continue;
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        continue;
                    }
                }
                if meta.is_dir() {
                    let name = item.file_name();
                    let name = name.to_string_lossy();
                    if !name.starts_with('.')
                        && !matches!(name.as_ref(), "target" | "out" | "dist" | "node_modules")
                    {
                        stack.push((file, depth + 1));
                    }
                } else if file.extension().is_some_and(|e| e == "dev") {
                    if let Some(uri) = uri_for(&file) {
                        found.insert(uri.clone());
                        self.load(&uri, docs);
                    }
                }
            }
        }
        for (uri, doc) in docs {
            self.update(uri, doc, None);
            found.insert(uri.clone());
        }
        self.entries
            .retain(|uri, _| found.contains(uri) || docs.contains_key(uri));
    }
    pub(super) fn workspace_symbols(
        &mut self,
        query: &str,
        docs: &HashMap<String, Document>,
    ) -> Value {
        self.scan(docs);
        let query = query.to_lowercase();
        let mut rows = Vec::new();
        let mut uris = self.entries.keys().cloned().collect::<Vec<_>>();
        uris.sort();
        for uri in uris {
            for s in &self.entries[&uri].symbols {
                if s["name"]
                    .as_str()
                    .is_some_and(|n| n.to_lowercase().contains(&query))
                {
                    rows.push(json!({"name":s["name"],"kind":s["kind"],"location":{"uri":uri,"range":s["selectionRange"]},"containerName":path(&uri).file_name().unwrap_or_default().to_string_lossy()}));
                    if rows.len() == 1000 {
                        return json!(rows);
                    }
                }
            }
        }
        json!(rows)
    }
    fn qualifier(text: &str, p: &Value) -> Option<String> {
        let line = text.lines().nth(p["line"].as_u64()? as usize)?;
        let target = p["character"].as_u64()? as usize;
        let mut units = 0;
        let prefix = line
            .chars()
            .take_while(|ch| {
                units += ch.len_utf16();
                units <= target
            })
            .collect::<String>();
        let dot = prefix.rfind('.')?;
        if !prefix[dot + 1..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return None;
        }
        let name = prefix[..dot]
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        (!name.is_empty()).then_some(name)
    }
    fn imported_uri(
        &mut self,
        uri: &str,
        alias: &str,
        docs: &HashMap<String, Document>,
    ) -> Option<String> {
        self.load(uri, docs);
        let import = self
            .entries
            .get(uri)?
            .imports
            .iter()
            .find(|i| i.alias == alias)?
            .clone();
        let entry = path(uri);
        let mut dirs = HashMap::new();
        if let Some(root) = crate::packages::root(entry.parent()?) {
            dirs.extend(crate::packages::mappings(&root).ok()?);
        }
        let file = dev_syntax::modules::resolve(&entry, &import.path, &dirs).ok()?;
        let uri = docs
            .keys()
            .find(|uri| equal_path(&path(uri), &file))
            .cloned()
            .or_else(|| uri_for(&file))?;
        self.load(&uri, docs);
        Some(uri)
    }
    pub(super) fn imported_completion(
        &mut self,
        uri: &str,
        doc: &Document,
        p: &Value,
        docs: &HashMap<String, Document>,
    ) -> Option<Vec<Value>> {
        self.update(uri, doc, None);
        let alias = Self::qualifier(&doc.text, p)?;
        if alias == "Math" && !self.entries[uri].imports.iter().any(|i| i.alias == alias) {
            let mut items = dev_syntax::math_intrinsics::METHODS
                .iter()
                .map(|n| json!({"label":n,"kind":3,"detail":"Math numeric function: f64"}))
                .collect::<Vec<_>>();
            items.extend(dev_syntax::math_intrinsics::CONSTANTS.iter().map(|n|json!({"label":n,"kind":21,"detail":format!("readonly f64 = {}",dev_syntax::math_intrinsics::constant(n).unwrap())})));
            return Some(items);
        }
        let target = self.imported_uri(uri, &alias, docs)?;
        Some(self.entries.get(&target)?.symbols.iter().map(|s|json!({"label":s["name"],"kind":if s["kind"]==12{3}else{22},"detail":s["detail"]})).collect())
    }
    pub(super) fn navigation(
        &mut self,
        uri: &str,
        doc: &Document,
        p: &Value,
        docs: &HashMap<String, Document>,
        definition: bool,
    ) -> Value {
        self.update(uri, doc, None);
        let Some(name) = word(&doc.text, p) else {
            return Value::Null;
        };
        let target = if let Some(alias) = Self::qualifier(&doc.text, p) {
            if alias == "Math" && !self.entries[uri].imports.iter().any(|i| i.alias == alias) {
                if definition {
                    return Value::Null;
                }
                let label = dev_syntax::math_intrinsics::constant(&name)
                    .map(|v| format!("readonly Math.{name}: f64 = {v}"))
                    .or_else(|| {
                        dev_syntax::math_intrinsics::arity(&name)
                            .map(|_| format!("Math.{name}(...numeric arguments): f64"))
                    });
                return label
                    .map(|label| json!({"contents":{"kind":"plaintext","value":label}}))
                    .unwrap_or(Value::Null);
            }
            self.imported_uri(uri, &alias, docs)
        } else {
            Some(uri.into())
        };
        let Some(target) = target else {
            return Value::Null;
        };
        let Some(s) = self
            .entries
            .get(&target)
            .and_then(|e| e.symbols.iter().find(|s| s["name"] == name))
        else {
            return Value::Null;
        };
        if definition {
            json!({"uri":target,"range":s["selectionRange"]})
        } else {
            json!({"contents":{"kind":"plaintext","value":format!("{} {}",name,s["detail"].as_str().unwrap_or(""))}})
        }
    }
    pub(super) fn folders_changed(&mut self, params: &Value) {
        if let Some(removed) = params["event"]["removed"].as_array() {
            for folder in removed {
                if let Some(uri) = folder["uri"].as_str() {
                    let remove = path(uri);
                    self.roots.retain(|root| !equal_path(root, &remove));
                }
            }
        }
        if let Some(added) = params["event"]["added"].as_array() {
            for folder in added {
                if let Some(uri) = folder["uri"].as_str() {
                    let add = path(uri);
                    if add.is_dir() && !self.roots.iter().any(|root| equal_path(root, &add)) {
                        self.roots.push(add);
                    }
                }
            }
        }
    }
    pub(super) fn invalidate(&mut self, uri: &str) {
        self.entries.remove(uri);
    }
}
