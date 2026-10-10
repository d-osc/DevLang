fn global_cache_directory(home: &Path, directory: &Path) -> Result<(), String> {
    let relative = directory.strip_prefix(home).map_err(|_| "global cache directory cannot escape DEVLANG_HOME")?;
    std::fs::create_dir_all(home).map_err(|e| e.to_string())?;
    let mut current = home.to_owned();
    for component in relative.components() {
        if !matches!(component, std::path::Component::Normal(_)) { return Err("invalid global cache directory".into()); }
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => return Err("global cache directories must be real directories".into()),
            Ok(_) => {},
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => std::fs::create_dir(&current).map_err(|e| e.to_string())?,
            Err(e) => return Err(e.to_string()),
        }
        if !dunce::canonicalize(&current).map_err(|e| e.to_string())?.starts_with(dunce::canonicalize(home).map_err(|e| e.to_string())?) { return Err("global cache directory cannot escape DEVLANG_HOME".into()); }
    }
    Ok(())
}
fn copy_package_source(source: &Path, target: &Path, count: &mut usize, total: &mut u64) -> Result<(), String> {
    std::fs::create_dir_all(target).map_err(|e| e.to_string())?;
    for item in std::fs::read_dir(source).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let name = item.file_name();
        let kind = item.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() { return Err("global source snapshot does not support symlinks".into()); }
        if kind.is_dir() {
            if ![".git", ".dev", ".dev-cache", "target", "dist", "out", "node_modules", "__pycache__"].contains(&name.to_string_lossy().as_ref()) {
                copy_package_source(&item.path(), &target.join(name), count, total)?;
            }
        } else if kind.is_file() && name != "dev.lock" && name != "package-lock.don" {
            *count += 1;
            *total += item.metadata().map_err(|e| e.to_string())?.len();
            if *count > 16384 || *total > 256 * 1024 * 1024 { return Err("global source snapshot exceeds 16384 files or 256 MiB".into()); }
            std::fs::copy(item.path(), target.join(name)).map_err(|e| e.to_string())?;
        } else if !kind.is_file() { return Err("global source snapshot does not support special files".into()); }
    }
    Ok(())
}
fn snapshot_dependencies(dependencies: &BTreeMap<String, Dependency>, package: &Path, stage: &Path, lock: &Lock) -> Result<BTreeMap<String, Dependency>, String> {
    dependencies.iter().map(|(name, dep)| {
        if !lock.packages.contains_key(name) { return Err(format!("snapshot dependency {name} missing from lock")); }
        let path = pathdiff::diff_paths(stage.join("deps").join(name), package).ok_or("cannot make snapshot relative path")?;
        Ok((name.clone(), Dependency { path: Some(path.to_string_lossy().replace('\\', "/")), git: None, rev: None, branch: None, url: None, sha256: None, version: dep.version.clone(), workspace: false }))
    }).collect()
}
fn cached_global_runtime(home: &Path) -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = executable.parent().unwrap();
    let mut files = vec![(if cfg!(windows) { "d.exe" } else { "d" }.to_owned(), executable.clone())];
    for tool in ["devrun", "devc"] {
        let name = if cfg!(windows) { format!("{tool}.exe") } else { tool.to_owned() };
        let file = directory.join(&name);
        if file.is_file() { files.push((name, file)); }
        else if tool == "devrun" { return Err("global install needs devrun next to d".into()); }
    }
    let mut hash = Sha256::new();
    let mut data = Vec::new();
    for (name, file) in files {
        let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
        hash.update(name.as_bytes()); hash.update((bytes.len() as u64).to_le_bytes()); hash.update(&bytes);
        data.push((name, bytes));
    }
    let directory = home.join("runtime").join(format!("{:x}", hash.finalize()));
    global_cache_directory(home, &directory)?;
    for (name, bytes) in data {
        let path = directory.join(name);
        if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink() || !m.is_file()) { return Err("cached global runtime must be a regular file".into()); }
        if path.exists() {
            if std::fs::read(&path).map_err(|e| e.to_string())? != bytes { return Err("cached global runtime changed".into()); }
        } else { std::fs::write(&path, bytes).map_err(|e| e.to_string())?; }
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?; }
    }
    Ok(directory.join(if cfg!(windows) { "d.exe" } else { "d" }))
}
fn make_global_snapshot(home: &Path, project: &Path, lock: &Lock) -> Result<(PathBuf, PathBuf), String> {
    let identity = format!("{:x}", Sha256::digest(project.to_string_lossy().as_bytes()));
    let stage = home.join("packages").join(identity).join(format!("{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    global_cache_directory(home, &stage)?;
    let mut count = 0; let mut total = 0;
    let mut roots = vec![(project.to_owned(), stage.join("root"), true)];
    for (name, item) in &lock.packages { roots.push((locked_root(project, name, item)?, stage.join("deps").join(name), false)); }
    for (source, target, is_root) in roots {
        copy_package_source(&source, &target, &mut count, &mut total)?;
        let mut m = manifest(&source)?;
        m.dependencies = snapshot_dependencies(&m.dependencies, &target, &stage, lock)?;
        m.dev_dependencies = if is_root && !lock.production { snapshot_dependencies(&m.dev_dependencies, &target, &stage, lock)? } else { BTreeMap::new() };
        m.workspace = None;
        write(&target.join("package.don"), &m)?;
    }
    let root = stage.join("root");
    write(&stage.join("source-lock.don"), lock)?;
    install_mode(&root, false, false, lock.production)?;
    mappings(&root)?;
    let runtime = cached_global_runtime(home)?;
    Ok((dunce::canonicalize(root).map_err(|e| e.to_string())?, runtime))
}
