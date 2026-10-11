// Keep each logical resolution in the lock, then share physical instances only
// when their source bytes, dependencies and complete visible scopes agree.
fn context_origin(item: &Locked) -> Result<String, String> {
    let mut origin = item
        .resolved_source
        .as_ref()
        .unwrap_or(&item.source)
        .clone();
    origin.version = None;
    origin.range = None;
    origin.optional = false;
    origin.rev = None;
    origin.branch = None;
    origin.workspace = false;
    if origin.git.is_none() && origin.url.is_none() {
        origin.path = Some(item.root.clone());
    }
    serde_json::to_string(&(origin, &item.commit, &item.sha256, &item.modules))
        .map_err(|e| e.to_string())
}

fn context_colors(signatures: &BTreeMap<String, String>) -> BTreeMap<String, usize> {
    let ranks: BTreeMap<_, _> = signatures
        .values()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(rank, value)| (value, rank))
        .collect();
    signatures
        .iter()
        .map(|(key, value)| (key.clone(), ranks[value]))
        .collect()
}

#[cfg(test)]
mod context_version_tests {
    use super::*;
    use std::fs;

    #[test]
    fn legacy_context_lock_stays_frozen_and_migrates_online() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "dev-context-migration-{}-{unique}",
            std::process::id()
        ));
        let app = root.join("app");
        let library = root.join("library");
        for directory in [&app, &library] {
            fs::create_dir_all(directory.join("src")).unwrap();
            fs::write(
                directory.join("src/lib.dev"),
                "fn value() i64 { return 42 }\n",
            )
            .unwrap();
        }
        fs::write(app.join("package.don"), r#"{ "version": "1.0.0", "package": { "name": "migration_app" }, "dependencies": { "library": { "path": "../library" } } }"#).unwrap();
        fs::write(
            library.join("package.don"),
            r#"{ "version": "1.0.0", "package": { "name": "library" } }"#,
        )
        .unwrap();
        graph_install_mode(&app, false, false, false, false).unwrap();
        let base: Lock = read(&lock_path(&app)).unwrap();
        let legacy = context_lock(&app, &base, true).unwrap();
        assert_eq!(legacy.version, 3);
        write(&lock_path(&app), &legacy).unwrap();
        mappings(&app).unwrap();
        install_mode(&app, true, false, false).unwrap();
        assert!(read::<Lock>(&lock_path(&app)).unwrap() == legacy);
        install_mode(&app, false, false, false).unwrap();
        assert_eq!(read::<Lock>(&lock_path(&app)).unwrap().version, 4);
        mappings(&app).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}

fn shared_context_lock(project: &Path, base: &Lock, materialize: bool) -> Result<Lock, String> {
    if base.packages.len() > 4096 {
        return Err("dependency context graph exceeds 4096 instances".into());
    }
    let scopes = graph_scopes(project, base)?;
    let origins = base
        .packages
        .iter()
        .map(|(key, item)| Ok((key.clone(), context_origin(item)?)))
        .collect::<Result<BTreeMap<_, _>, String>>()?;
    let mut colors = context_colors(&origins);
    // Monotone partition refinement handles cycles without recursive hashing.
    // Each split increases the class count; there are at most N classes.
    for _ in 0..=base.packages.len() {
        let previous_count = colors.values().collect::<BTreeSet<_>>().len();
        let mut signatures = BTreeMap::new();
        for key in base.packages.keys() {
            let edges = base
                .edges
                .get(key)
                .ok_or("missing resolution edges")?
                .iter()
                .map(|(alias, id)| {
                    Ok((
                        alias.clone(),
                        *colors.get(id).ok_or("missing resolution package")?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            let scope = scopes
                .get(key)
                .ok_or("missing resolution scope")?
                .iter()
                .map(|(alias, id)| {
                    Ok((
                        alias.clone(),
                        *colors.get(id).ok_or("missing resolution provider")?,
                    ))
                })
                .collect::<Result<BTreeMap<_, _>, String>>()?;
            signatures.insert(
                key.clone(),
                serde_json::to_string(&(colors[key], edges, scope)).map_err(|e| e.to_string())?,
            );
        }
        let next = context_colors(&signatures);
        let next_count = next.values().collect::<BTreeSet<_>>().len();
        colors = next;
        if next_count == previous_count {
            break;
        }
    }
    let mut representatives = BTreeMap::new();
    for (key, color) in &colors {
        representatives.entry(*color).or_insert(key.clone());
    }
    let mut ids = BTreeMap::new();
    for (color, key) in &representatives {
        let item = &base.packages[key];
        let identity =
            serde_json::to_vec(&("shared", key, &item.sha256)).map_err(|e| e.to_string())?;
        ids.insert(*color, format!("context__{:x}", Sha256::digest(identity)));
    }
    let translate = |edges: &BTreeMap<String, String>| -> Result<BTreeMap<String, String>, String> {
        edges
            .iter()
            .map(|(alias, key)| {
                Ok((
                    alias.clone(),
                    ids.get(colors.get(key).ok_or("missing resolution target")?)
                        .ok_or("missing instance target")?
                        .clone(),
                ))
            })
            .collect()
    };
    let mut result = Lock {
        version: 4,
        production: base.production,
        packages: BTreeMap::new(),
        edges: BTreeMap::new(),
        omitted: base.omitted.clone(),
        resolutions: Some(Resolutions {
            packages: base.packages.clone(),
            edges: base.edges.clone(),
            omitted: base.omitted.clone(),
        }),
    };
    result.edges.insert(
        "root".into(),
        translate(
            base.edges
                .get("root")
                .ok_or("missing root resolution edges")?,
        )?,
    );
    for (color, key) in representatives {
        let id = &ids[&color];
        let mut item = base.packages[&key].clone();
        if item.source_key.is_some() || item.source_root.is_some() {
            return Err("nested source context metadata".into());
        }
        let original = locked_root(project, &key, &item)?;
        item.source_key = Some(key.clone());
        item.source_root = Some(item.root.clone());
        item.root = format!(".dev/contexts/{id}");
        if materialize {
            if !tree_matches(&original, &item.sha256)? {
                return Err("dependency source content changed".into());
            }
            let target = project.join(&item.root);
            global_cache_directory(project, &target)?;
            if !target.join("package.don").exists() && !target.join("dev.toml").exists() {
                copy_package_source(&original, &target, &mut 0, &mut 0)?;
            }
            if !tree_matches(&target, &item.sha256)? {
                return Err(
                    "dependency context content changed; remove its cache and reinstall".into(),
                );
            }
        }
        result.edges.insert(
            id.clone(),
            translate(base.edges.get(&key).ok_or("missing instance edges")?)?,
        );
        result.packages.insert(id.clone(), item);
    }
    Ok(result)
}
