fn platform_matches(list: &[String], current: &str) -> bool {
    !list.iter().any(|s| s == &format!("!{current}"))
        && (list.iter().all(|s| s.starts_with('!'))
            || list.iter().any(|s| s == current || s == "any"))
}
fn platform_check(m: &Manifest) -> Result<(), String> {
    let os = match std::env::consts::OS {
        "windows" => "win32",
        "macos" => "darwin",
        s => s,
    };
    let cpu = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        s => s,
    };
    if !platform_matches(&m.os, os) || !platform_matches(&m.cpu, cpu) {
        return Err(format!(
            "package {} is not supported on {os}/{cpu}",
            m.package.name
        ));
    }
    Ok(())
}
fn effective_dependencies(
    project: &Path,
    parent: &Path,
    deps: BTreeMap<String, Dependency>,
) -> Result<BTreeMap<String, Dependency>, String> {
    let overrides = manifest(project)?.overrides;
    deps.into_iter()
        .map(|(name, dep)| {
            let mut selected = overrides.get(&name).cloned().unwrap_or_else(|| dep.clone());
            selected.optional = dep.optional;
            if overrides.contains_key(&name) {
                if let Some(path) = &selected.path {
                    let target = project.join(path);
                    let target = dunce::canonicalize(&target).unwrap_or(target);
                    let parent = dunce::canonicalize(parent).map_err(|e| e.to_string())?;
                    selected.path = Some(
                        pathdiff::diff_paths(target, parent)
                            .ok_or("cannot rebase override path")?
                            .to_string_lossy()
                            .replace('\\', "/"),
                    );
                }
            }
            validate_dependency(&selected)?;
            Ok((name, selected))
        })
        .collect()
}
fn child_dependencies(m: &Manifest) -> Result<BTreeMap<String, Dependency>, String> {
    root_dependencies(m, true)
}
fn try_auto_peer(project: &Path, error: &str) -> Result<bool, String> {
    if std::env::var_os("DEVLANG_AUTO_PEERS").is_none_or(|v| v != "1") {
        return Ok(false);
    }
    let Some((_, tail)) = error.split_once("requires peer '") else {
        return Ok(false);
    };
    let Some((name, range)) = tail.split_once("' ") else {
        return Ok(false);
    };
    let range = range.split(';').next().unwrap().trim();
    let mut m = manifest(project)?;
    if m.dependencies.contains_key(name)
        || m.dev_dependencies.contains_key(name)
        || m.optional_dependencies.contains_key(name)
    {
        return Ok(false);
    }
    let (_, dep) = store_dependency(
        name,
        &std::env::var("DEVLANG_PEER_STORE").unwrap_or_else(|_| DEFAULT_STORE.into()),
        Some(range),
    )?;
    m.dependencies.insert(name.into(), dep);
    write(&manifest_path(project), &m)?;
    println!("added missing peer {name} {range}");
    Ok(true)
}
