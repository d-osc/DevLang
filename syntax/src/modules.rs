use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
};

pub fn resolve(
    from: &Path,
    import: &str,
    dirs: &HashMap<String, PathBuf>,
) -> Result<PathBuf, String> {
    let mapped = import
        .split_once('/')
        .and_then(|(name, suffix)| dirs.get(name).map(|root| (root, suffix)));
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
    let mut chars = name.chars();
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
