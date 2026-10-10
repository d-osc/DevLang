fn archive_bytes(spec: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let url = url::Url::parse(spec).map_err(|e| e.to_string())?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err("archive URL must not contain credentials or fragment".into());
    }
    let max = 64 * 1024 * 1024;
    let mut bytes = Vec::new();
    match url.scheme() {
        "file" => {
            let path = url.to_file_path().map_err(|_| "invalid file URL")?;
            std::fs::File::open(path).map_err(|e| e.to_string())?.take(max + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        }
        "https" | "http" => {
            let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(30))).build().into();
            let mut response = agent.get(spec).call().map_err(|e| e.to_string())?;
            response.body_mut().as_reader().take(max + 1).read_to_end(&mut bytes).map_err(|e| e.to_string())?;
        }
        _ => return Err("archive URL requires https://, http:// or file://".into()),
    }
    if bytes.len() > max as usize { return Err("archive download exceeds 64 MiB".into()); }
    Ok(bytes)
}

fn unpack_package(bytes: Vec<u8>, spec: &str, target: &Path) -> Result<PathBuf, String> {
    use std::io::{Cursor, Read};
    let url = url::Url::parse(spec).map_err(|e| e.to_string())?;
    let mut seen = BTreeSet::new();
    let mut total = 0u64;
    let mut write_entry = |name: &str, directory: bool, reader: &mut dyn Read| -> Result<(), String> {
        if name.contains(['\\', ':', '\0']) || name.starts_with('/') { return Err("unsafe archive path".into()); }
        let name = name.trim_end_matches('/');
        if name.is_empty() || name.split('/').any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' '])) { return Err("unsafe archive path".into()); }
        if !seen.insert(name.to_lowercase()) || seen.len() > 8192 { return Err("duplicate archive path or too many entries".into()); }
        let path = target.join(name);
        if directory { std::fs::create_dir_all(path).map_err(|e| e.to_string())?; return Ok(()); }
        let mut data = Vec::new();
        reader.take(16 * 1024 * 1024 + 1).read_to_end(&mut data).map_err(|e| e.to_string())?;
        total += data.len() as u64;
        if data.len() > 16 * 1024 * 1024 || total > 128 * 1024 * 1024 { return Err("archive expansion exceeds limit".into()); }
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::OpenOptions::new().write(true).create_new(true).open(path).and_then(|mut file| std::io::Write::write_all(&mut file, &data)).map_err(|e| e.to_string())?;
        Ok(())
    };
    if url.path().ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
            if entry.unix_mode().is_some_and(|m| m & 0o170000 != 0 && m & 0o170000 != 0o100000 && m & 0o170000 != 0o040000) { return Err("archive links and special files are unsupported".into()); }
            let name = entry.name().map_err(|e| e.to_string())?.into_owned();
            write_entry(&name, entry.is_dir(), &mut entry)?;
        }
    } else {
        let reader: Box<dyn Read> = if url.path().ends_with(".tar.gz") || url.path().ends_with(".tgz") {
            Box::new(flate2::read::GzDecoder::new(Cursor::new(bytes)))
        } else if url.path().ends_with(".tar") { Box::new(Cursor::new(bytes)) }
        else { return Err("archive URL must end in .zip, .tar, .tar.gz or .tgz".into()); };
        let mut archive = tar::Archive::new(reader);
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            let kind = entry.header().entry_type();
            if !kind.is_file() && !kind.is_dir() { return Err("archive links and special files are unsupported".into()); }
            let name = entry.path().map_err(|e| e.to_string())?.to_str().ok_or("non-UTF8 archive path")?.to_owned();
            write_entry(&name, kind.is_dir(), &mut entry)?;
        }
    }
    if target.join("package.don").is_file() || target.join("dev.toml").is_file() { return Ok(target.to_owned()); }
    let entries = std::fs::read_dir(target).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    if entries.len() == 1 && entries[0].file_type().map_err(|e| e.to_string())?.is_dir() {
        let root = entries[0].path();
        if root.join("package.don").is_file() || root.join("dev.toml").is_file() { return Ok(root); }
    }
    Err("archive must contain a package manifest at root or in one wrapper directory".into())
}

fn fetch_archive(project: &Path, name: &str, dep: &Dependency, previous: Option<&Locked>) -> Result<PathBuf, String> {
    let digest = dep.sha256.as_deref().unwrap().to_lowercase();
    let destination = project.join(format!(".dev/packages/{name}-archive-{digest}"));
    if destination.exists() {
        let old = previous.filter(|p| p.source == *dep).ok_or("archive cache has no matching lock; remove it before reinstalling")?;
        if !tree_matches(&destination, &old.sha256)? { return Err("cached archive dependency changed".into()); }
        return dunce::canonicalize(destination).map_err(|e| e.to_string());
    }
    let bytes = archive_bytes(dep.url.as_deref().unwrap())?;
    if format!("{:x}", Sha256::digest(&bytes)) != digest { return Err("archive sha256 mismatch".into()); }
    let temporary = project.join(format!(".dev/packages/{name}-extract-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&temporary).map_err(|e| e.to_string())?;
    let root = unpack_package(bytes, dep.url.as_deref().unwrap(), &temporary)?;
    manifest(&root)?;
    std::fs::rename(root, &destination).map_err(|e| e.to_string())?;
    dunce::canonicalize(destination).map_err(|e| e.to_string())
}
