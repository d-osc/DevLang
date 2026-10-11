#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum PackageScript {
    Shell(String),
    Arguments(Vec<String>),
}
fn offline_mode() -> bool {
    std::env::var_os("DEVLANG_OFFLINE").is_some_and(|v| v == "1")
}
fn download_cache() -> Result<PathBuf, String> {
    let home = global_home()?;
    let cache = home.join("cache/downloads");
    global_cache_directory(&home, &cache)?;
    let marker = cache.join(".devlang-cache");
    if std::fs::symlink_metadata(&marker).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink())
    {
        return Err("invalid download cache marker".into());
    }
    if marker.exists()
        && std::fs::read(&marker).map_err(|e| e.to_string())? != b"DevLang download cache v1\n"
    {
        return Err("invalid download cache marker".into());
    }
    if !marker.exists() {
        std::fs::write(marker, b"DevLang download cache v1\n").map_err(|e| e.to_string())?;
    }
    Ok(cache)
}
fn cached_download(spec: &str, max: u64) -> Result<Vec<u8>, String> {
    let cache = download_cache()?;
    let key = format!("{:x}", Sha256::digest(spec.as_bytes()));
    let entry = cache.join(format!("{key}.don"));
    if std::fs::symlink_metadata(&entry).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err("invalid cache record".into());
    }
    if offline_mode() {
        let record: serde_json::Value =
            read(&entry).map_err(|_| format!("offline: no cached download for {spec}"))?;
        let digest = record["sha256"].as_str().ok_or("invalid cache digest")?;
        if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("invalid cache digest".into());
        }
        let blob = cache.join(format!("{digest}.blob"));
        let metadata = std::fs::symlink_metadata(&blob).map_err(|e| e.to_string())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err("cache symlinks and non-files are unsupported".into());
        }
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(blob)
            .map_err(|e| e.to_string())?
            .take(max + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > max as usize
            || format!("{:x}", Sha256::digest(&bytes)) != digest
            || record["url"] != spec
        {
            return Err("cached download failed integrity verification".into());
        }
        return Ok(bytes);
    }
    let bytes = download_bytes_online(spec, max)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let blob = cache.join(format!("{digest}.blob"));
    if std::fs::symlink_metadata(&blob).is_ok_and(|m| !m.is_file() || m.file_type().is_symlink()) {
        return Err("invalid cache blob".into());
    }
    if blob.exists() && std::fs::read(&blob).map_err(|e| e.to_string())? != bytes {
        return Err(
            "cached download failed integrity verification; clean the download cache".into(),
        );
    }
    if !blob.exists() {
        std::fs::write(&blob, &bytes).map_err(|e| e.to_string())?;
    }
    write(&entry, &serde_json::json!({"url":spec,"sha256":digest}))?;
    Ok(bytes)
}
fn cache_command(args: &[String]) -> Result<i32, String> {
    let cache = download_cache()?;
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "list" if args.len() <= 2 => {
            let mut count = 0;
            let mut bytes = 0;
            for entry in std::fs::read_dir(&cache).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                if entry.file_type().map_err(|e| e.to_string())?.is_file() {
                    count += 1;
                    bytes += entry.metadata().map_err(|e| e.to_string())?.len();
                }
            }
            println!("{}\n{count} files, {bytes} bytes", cache.display());
        }
        "clean" if args.len() == 2 => {
            // Delete only regular cache records and blobs. Package snapshots and runtimes are separate.
            for entry in std::fs::read_dir(&cache).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if let Some((hash, extension)) = name.split_once('.') {
                    if hash.len() == 64
                        && hash.bytes().all(|b| b.is_ascii_hexdigit())
                        && ["don", "blob"].contains(&extension)
                        && entry.file_type().map_err(|e| e.to_string())?.is_file()
                    {
                        std::fs::remove_file(entry.path()).map_err(|e| e.to_string())?;
                    }
                }
            }
            println!("download cache cleaned");
        }
        _ => return Err("pkg cache [list|clean]".into()),
    }
    Ok(0)
}
pub fn task(args: &[String]) -> Result<i32, String> {
    let project = root(&std::env::current_dir().map_err(|e| e.to_string())?)
        .ok_or("task requires a package")?;
    let m = manifest(&project)?;
    let Some(name) = args.first() else {
        for name in m.scripts.keys() {
            println!("{name}");
        }
        return Ok(0);
    };
    let script = m
        .scripts
        .get(name)
        .ok_or_else(|| format!("unknown package script {name}"))?;
    let extra = if args.get(1).is_some_and(|s| s == "--") {
        &args[2..]
    } else {
        &args[1..]
    };
    let mut command = match script {
        PackageScript::Arguments(values) => {
            let program = values.first().ok_or("script argument list is empty")?;
            let mut c = Command::new(if program == "d" {
                std::env::current_exe().map_err(|e| e.to_string())?
            } else {
                PathBuf::from(program)
            });
            c.args(&values[1..]).args(extra);
            c
        }
        PackageScript::Shell(text) => {
            if !extra.is_empty() {
                return Err(
                    "argument forwarding requires an array script; use ['d','run','file.dev']"
                        .into(),
                );
            }
            let mut c = Command::new(if cfg!(windows) { "cmd" } else { "sh" });
            c.args(if cfg!(windows) {
                vec!["/d", "/s", "/c", text]
            } else {
                vec!["-c", text]
            });
            c
        }
    };
    let mut paths = vec![
        project.join(".dev/bin"),
        std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .unwrap()
            .to_owned(),
    ];
    if let Some(path) = std::env::var_os("PATH") {
        paths.extend(std::env::split_paths(&path));
    }
    let status = command
        .current_dir(project)
        .env(
            "PATH",
            std::env::join_paths(paths).map_err(|e| e.to_string())?,
        )
        .env("DEVLANG_SCRIPT", name)
        .status()
        .map_err(|e| e.to_string())?;
    Ok(status.code().unwrap_or(1))
}
fn dependency_report(project: &Path, args: &[String]) -> Result<i32, String> {
    mappings(project)?;
    let lock: Lock = read(&lock_path(project))?;
    let query = if args[0] == "why" {
        Some(args.get(1).ok_or("pkg why NAME")?.as_str())
    } else {
        None
    };
    if args.len() > if query.is_some() { 2 } else { 1 } {
        return Err("pkg tree; pkg why NAME".into());
    }
    let mut edges = lock.edges.clone();
    if lock.version == 1 {
        edges.insert(
            "root".into(),
            root_dependencies(&manifest(project)?, lock.production)?
                .keys()
                .filter(|name| lock.packages.contains_key(*name))
                .map(|name| (name.clone(), name.clone()))
                .collect(),
        );
        for (id, item) in &lock.packages {
            edges.insert(
                id.clone(),
                child_dependencies(&manifest(&locked_root(project, id, item)?)?)?
                    .keys()
                    .filter(|name| lock.packages.contains_key(*name))
                    .map(|name| (name.clone(), name.clone()))
                    .collect(),
            );
        }
    }
    let mut pending = vec![(
        "root".to_owned(),
        vec![manifest(project)?.package.name],
        BTreeSet::new(),
    )];
    let mut found = false;
    let mut count = 0;
    while let Some((context, path, mut seen)) = pending.pop() {
        if !seen.insert(context.clone()) {
            continue;
        }
        count += 1;
        if count > 16384 {
            return Err("dependency report exceeds traversal budget".into());
        }
        for (alias, id) in edges.get(&context).into_iter().flatten().rev() {
            let item = lock.packages.get(id).ok_or("missing graph node")?;
            let mut path = path.clone();
            path.push(alias.clone());
            if query
                .is_none_or(|q| q == alias || q == id || item.source.package.as_deref() == Some(q))
            {
                println!(
                    "{} ({})",
                    path.join(" -> "),
                    item.resolved_version.as_deref().unwrap_or("local/tag")
                );
                found = true;
            }
            pending.push((id.clone(), path, seen.clone()));
        }
    }
    if query.is_some() && !found {
        return Err("dependency not found".into());
    }
    Ok(0)
}
