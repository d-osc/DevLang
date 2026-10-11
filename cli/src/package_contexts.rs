// Version 3 gives each dependency path its own physical importer directory.
pub fn mapping_file(project: &Path) -> Result<Option<PathBuf>, String> {
    let mappings = mappings(project)?;
    if mappings.is_empty() {
        return Ok(None);
    }
    if mappings.len() > 65536 {
        return Err("module mappings exceed 65536 entries".into());
    }
    let values: Vec<_> = mappings
        .iter()
        .map(|(name, path)| format!("{name}={}", path.display()))
        .collect();
    let bytes = serde_json::to_vec(&values).map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("module mappings exceed 64 MiB".into());
    }
    let directory = project.join(".dev/maps");
    global_cache_directory(project, &directory)?;
    let path = directory.join(format!("{:x}.json", Sha256::digest(&bytes)));
    if path.exists() {
        if std::fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
            || std::fs::read(&path).map_err(|e| e.to_string())? != bytes
        {
            return Err("cached module mappings changed".into());
        }
    } else {
        let temporary = directory.join(format!(
            "{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        ));
        std::fs::write(&temporary, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
    }
    Ok(Some(path))
}

// An ancestor edge reuses that ancestor instance, so dependency cycles terminate.
fn context_lock(project: &Path, base: &Lock, materialize: bool) -> Result<Lock, String> {
    let mut result = Lock {
        resolutions: None,
        version: 3,
        production: base.production,
        packages: BTreeMap::new(),
        edges: BTreeMap::new(),
        omitted: base.omitted.clone(),
    };
    let mut pending = vec![(
        "root".to_owned(),
        "root".to_owned(),
        BTreeMap::<String, String>::new(),
    )];
    while let Some((context, source, ancestors)) = pending.pop() {
        let mut edges = BTreeMap::new();
        for (alias, key) in base
            .edges
            .get(&source)
            .ok_or("missing source context edges")?
        {
            let id = if let Some(ancestor) = ancestors.get(key) {
                ancestor.clone()
            } else {
                let original_item = base
                    .packages
                    .get(key)
                    .ok_or("missing source context package")?;
                let identity = serde_json::to_vec(&(&context, alias, key, &original_item.sha256))
                    .map_err(|e| e.to_string())?;
                let id = format!("context__{:x}", Sha256::digest(identity));
                if result.packages.len() >= 4096 {
                    return Err("dependency context graph exceeds 4096 instances".into());
                }
                let mut item = original_item.clone();
                if item.source_key.is_some() || item.source_root.is_some() {
                    return Err("nested source context metadata".into());
                }
                let original = locked_root(project, key, &item)?;
                item.source_key = Some(key.clone());
                item.source_root = Some(item.root.clone());
                item.root = format!(".dev/contexts/{id}");
                if materialize {
                    if !tree_matches(&original, &item.sha256)? {
                        return Err("dependency source content changed".into());
                    }
                    let target = project.join(&item.root);
                    global_cache_directory(project, &target)?;
                    if !target.join("package.don").exists()
                        && !target.join("dev.toml").exists()
                        && !target.join("package.json").exists()
                    {
                        copy_package_source(&original, &target, &mut 0, &mut 0)?;
                    }
                    if !tree_matches(&target, &item.sha256)? {
                        return Err(
                            "dependency context content changed; remove its cache and reinstall"
                                .into(),
                        );
                    }
                }
                result.packages.insert(id.clone(), item);
                let mut ancestry = ancestors.clone();
                ancestry.insert(key.clone(), id.clone());
                pending.push((id.clone(), key.clone(), ancestry));
                id
            };
            edges.insert(alias.clone(), id);
        }
        result.edges.insert(context, edges);
    }
    Ok(result)
}

fn source_lock(project: &Path, lock: &Lock) -> Result<Lock, String> {
    if lock.version == 4 {
        let resolutions = lock
            .resolutions
            .as_ref()
            .ok_or("lock is missing contextual resolutions")?;
        if resolutions
            .packages
            .values()
            .any(|p| p.source_key.is_some() || p.source_root.is_some())
        {
            return Err("nested source context metadata".into());
        }
        for (id, item) in &lock.packages {
            locked_root(project, id, item)?;
            let key = item.source_key.as_ref().ok_or("context needs source key")?;
            let mut original = item.clone();
            original.source_key = None;
            original.root = original
                .source_root
                .take()
                .ok_or("context needs source root")?;
            if resolutions.packages.get(key) != Some(&original) {
                return Err("inconsistent source context packages".into());
            }
        }
        return Ok(Lock {
            version: 4,
            production: lock.production,
            packages: resolutions.packages.clone(),
            edges: resolutions.edges.clone(),
            omitted: resolutions.omitted.clone(),
            resolutions: None,
        });
    }
    if lock.resolutions.is_some() {
        return Err("unexpected contextual resolutions".into());
    }
    let mut base = Lock {
        resolutions: None,
        version: 2,
        production: lock.production,
        packages: BTreeMap::new(),
        edges: BTreeMap::new(),
        omitted: lock.omitted.clone(),
    };
    for (id, item) in &lock.packages {
        locked_root(project, id, item)?;
        let key = item
            .source_key
            .as_ref()
            .ok_or("context needs source key")?
            .clone();
        let mut original = item.clone();
        original.source_key = None;
        original.root = original
            .source_root
            .take()
            .ok_or("context needs source root")?;
        if let Some(previous) = base.packages.insert(key, original.clone()) {
            if previous != original {
                return Err("inconsistent source context packages".into());
            }
        }
    }
    for (context, edges) in &lock.edges {
        let source = if context == "root" {
            context.clone()
        } else {
            lock.packages
                .get(context)
                .and_then(|p| p.source_key.clone())
                .ok_or("missing context package")?
        };
        let edges = edges
            .iter()
            .map(|(alias, id)| {
                Ok((
                    alias.clone(),
                    lock.packages
                        .get(id)
                        .and_then(|p| p.source_key.clone())
                        .ok_or("missing context edge")?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        if let Some(previous) = base.edges.insert(source, edges.clone()) {
            if previous != edges {
                return Err("inconsistent source context edges".into());
            }
        }
    }
    Ok(base)
}

fn context_mappings(project: &Path, lock: &Lock) -> Result<Vec<(String, PathBuf)>, String> {
    let base = source_lock(project, lock)?;
    graph_mappings(project, &base, false)?;
    if (if lock.version == 4 {
        shared_context_lock(project, &base, false)?
    } else {
        context_lock(project, &base, false)?
    }) != *lock
    {
        return Err("dependency context graph changed; run d pkg install".into());
    }
    let scopes = graph_scopes(project, lock)?;
    let root = manifest(project)?;
    let mut dirs = BTreeMap::new();
    for (context, scope) in scopes {
        let parent = if context == "root" {
            project.to_owned()
        } else {
            let item = &lock.packages[&context];
            let at = locked_root(project, &context, item)?;
            if !tree_matches(&at, &item.sha256)? {
                return Err("dependency context content changed; run d pkg install".into());
            }
            at
        };
        let has_root = scope.contains_key(&root.package.name);
        for (alias, id) in scope {
            let item = lock.packages.get(&id).ok_or("missing context provider")?;
            dirs.insert(
                scoped_mapping(&alias, &parent),
                locked_root(project, &id, item)?.join(relative(&item.modules)?),
            );
        }
        if !has_root {
            dirs.insert(
                scoped_mapping(&root.package.name, &parent),
                project.join(relative(&root.package.modules)?),
            );
        }
    }
    graph_verify_peers(project, lock)?;
    Ok(dirs.into_iter().collect())
}
