use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

pub fn resolve(
    from: &Path,
    import: &str,
    dirs: &HashMap<String, PathBuf>,
) -> Result<PathBuf, String> {
    let name = import.split('/').next().unwrap_or("");
    let from = from.canonicalize().unwrap_or_else(|_| from.to_owned());
    let scoped = dirs.iter().filter_map(|(key, root)| {
        let (alias, scope) = key.split_once('@')?;
        if alias != name || scope.len() % 2 != 0 || !scope.bytes().all(|b|b.is_ascii_hexdigit()) { return None; }
        let bytes = (0..scope.len()).step_by(2).map(|i|u8::from_str_radix(&scope[i..i+2],16)).collect::<Result<Vec<_>,_>>().ok()?;
        let at = PathBuf::from(String::from_utf8(bytes).ok()?);
        let at = at.canonicalize().ok()?;
        from.starts_with(&at).then_some((at.components().count(),root))
    }).max_by_key(|(depth,_)|*depth);
    let mapped = scoped.and_then(|(_,root)|import.split_once('/').map(|(_,suffix)|(root,suffix))).or_else(||import
        .split_once('/')
        .and_then(|(name, suffix)| dirs.get(name).map(|root| (root, suffix))));
    if let Some((root, suffix)) = mapped {
        if suffix.is_empty()
            || !Path::new(suffix)
                .components()
                .all(|p| matches!(p, Component::Normal(_)))
        {
            return Err("module path cannot escape its namespace directory".into());
        }
        let mut target = root.join(suffix);
        if target.extension().is_none() {
            target.set_extension("dev");
        }
        // Canonical containment also rejects symlinks out of a namespace.
        let actual = target
            .canonicalize()
            .map_err(|e| format!("{}: {e}", target.display()))?;
        let root = root.canonicalize().map_err(|e| e.to_string())?;
        if !actual.starts_with(root) {
            return Err("module path cannot escape its namespace directory".into());
        }
        Ok(actual)
    } else {
        let mut target = from.parent().unwrap_or(Path::new(".")).join(import);
        if target.extension().is_none() {
            target.set_extension("dev");
        }
        Ok(target)
    }
}
pub fn mapping(value: &str) -> Result<(String, PathBuf), String> {
    let (name, path) = value.split_once('=').ok_or("--module-dir needs NAME=DIR")?;
    let (alias, scope) = name.split_once('@').map_or((name,None),|(a,s)|(a,Some(s)));
    if scope.is_some_and(|s| s.is_empty() || s.len()%2!=0 || !s.bytes().all(|b|b.is_ascii_hexdigit())) { return Err("invalid module scope".into()); }
    let mut chars = alias.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        || path.is_empty()
    {
        return Err("--module-dir needs an identifier NAME and nonempty directory".into());
    }
    Ok((name.into(), path.into()))
}
