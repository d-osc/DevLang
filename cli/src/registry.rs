#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct RegistryIndex {
    version: u32,
    packages: BTreeMap<String, BTreeMap<String, RegistryRelease>>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RegistryRelease {
    url: String,
    sha256: String,
}
fn validate_registry(index: &RegistryIndex) -> Result<(), String> {
    if index.version != 1 {
        return Err("unsupported registry index version".into());
    }
    for (name, releases) in &index.packages {
        if !identifier(name) {
            return Err("invalid registry package name".into());
        }
        for (version, release) in releases {
            semver::Version::parse(version)
                .map_err(|e| format!("invalid registry version: {e}"))?;
            if release.sha256.len() != 64 || !release.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("invalid registry archive checksum".into());
            }
        }
    }
    Ok(())
}
fn registry_index(spec: &str) -> Result<(url::Url, RegistryIndex), String> {
    let url = url::Url::parse(spec).map_err(|e| e.to_string())?;
    let bytes = download_bytes(spec, 1024 * 1024)?;
    if bytes.len() > 1024 * 1024 {
        return Err("registry index exceeds 1 MiB".into());
    }
    let text = std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
    let index: RegistryIndex =
        serde_json::from_value(dev_syntax::don::parse(text)?).map_err(|e| e.to_string())?;
    validate_registry(&index)?;
    Ok((url, index))
}
fn registry_dependency(
    name: &str,
    spec: &str,
    requested: Option<&str>,
) -> Result<Dependency, String> {
    let (index_url, index) = registry_index(spec)?;
    let requirement = peer_requirement(requested.unwrap_or("*"))?;
    let releases = index
        .packages
        .get(name)
        .ok_or_else(|| format!("registry package '{name}' not found"))?;
    let (version, release) = releases
        .iter()
        .filter_map(|(version, release)| {
            let version = semver::Version::parse(version).ok()?;
            requirement.matches(&version).then_some((version, release))
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .ok_or_else(|| format!("no registry version of '{name}' satisfies {requirement}"))?;
    let url = index_url.join(&release.url).map_err(|e| e.to_string())?;
    if !matches!(url.scheme(), "http" | "https" | "file")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("invalid registry archive URL".into());
    }
    let dep = Dependency {
        path: None,
        git: None,
        rev: None,
        branch: None,
        url: Some(url.to_string()),
        sha256: Some(release.sha256.to_lowercase()),
        version: Some(format!("={version}")),
        workspace: false,
    };
    validate_dependency(&dep)?;
    Ok(dep)
}
fn registry_search(args: &[String]) -> Result<i32, String> {
    if !(args.len() == 3 || args.len() == 4) || args[1] != "--registry" {
        return Err("pkg search --registry INDEX_URL [QUERY]".into());
    }
    let (_, index) = registry_index(&args[2])?;
    let query = args.get(3).map(String::as_str).unwrap_or("");
    for (name, releases) in index.packages {
        if name.contains(query) {
            let versions = releases
                .keys()
                .map(|v| semver::Version::parse(v).unwrap())
                .collect::<BTreeSet<_>>();
            for version in versions {
                println!("{name} {version}");
            }
        }
    }
    Ok(0)
}
struct PublishLock(PathBuf);
impl Drop for PublishLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn publish_package(project: &Path, args: &[String]) -> Result<(), String> {
    if args.len() != 3 || args[1] != "--registry" {
        return Err("pkg publish --registry DIRECTORY (local static registry)".into());
    }
    let mut m = manifest(project)?;
    let version = semver::Version::parse(
        m.version
            .as_deref()
            .ok_or("publish requires a package version")?
            .strip_prefix('v')
            .unwrap_or(m.version.as_deref().unwrap()),
    )
    .map_err(|e| e.to_string())?;
    for dep in m.dependencies.values() {
        validate_dependency(dep)?;
        if let Some(remote) = &dep.git {
            if !remote.starts_with("https://") || remote.contains('@') {
                return Err(
                    "published Git dependencies require credential-free HTTPS URLs and a tag"
                        .into(),
                );
            }
        }
        if let Some(spec) = &dep.url {
            let url = url::Url::parse(spec).map_err(|e| e.to_string())?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.fragment().is_some()
            {
                return Err("published archive dependencies require portable HTTP(S) URLs without credentials or fragments".into());
            }
        }
        if dep.path.is_some()
            || dep.workspace
            || dep.branch.is_some()
            || dep.git.is_some() && dep.rev.is_none()
        {
            return Err("publish requires portable URL/checksum or Git tag dependencies; path/workspace/branch dependencies are unsupported".into());
        }
    }
    std::fs::create_dir_all(&args[2]).map_err(|e| e.to_string())?;
    let registry = dunce::canonicalize(&args[2]).map_err(|e| e.to_string())?;
    if registry.starts_with(dunce::canonicalize(project).map_err(|e| e.to_string())?) {
        return Err("registry directory must be outside the source package".into());
    }
    let lock_path = registry.join(".publish.lock");
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .map_err(|e| format!("cannot lock registry for publication: {e}"))?;
    let _lock = PublishLock(lock_path);
    let index_path = registry.join("index.don");
    if std::fs::symlink_metadata(&index_path)
        .is_ok_and(|m| m.file_type().is_symlink() || !m.is_file())
    {
        return Err("registry index must be a regular file".into());
    }
    if index_path.exists()
        && std::fs::metadata(&index_path)
            .map_err(|e| e.to_string())?
            .len()
            > 1024 * 1024
    {
        return Err("registry index exceeds 1 MiB".into());
    }
    let mut index: RegistryIndex = if index_path.exists() {
        read(&index_path)?
    } else {
        RegistryIndex {
            version: 1,
            ..RegistryIndex::default()
        }
    };
    validate_registry(&index)?;
    if index
        .packages
        .get(&m.package.name)
        .is_some_and(|versions| versions.contains_key(&version.to_string()))
    {
        return Err("package version is already published; increment version".into());
    }
    let staging = registry.join(format!(".stage-{}", std::process::id()));
    if staging.exists() {
        return Err("publication staging directory already exists".into());
    }
    std::fs::create_dir(&staging).map_err(|e| e.to_string())?;
    let result = (|| {
        copy_package_source(project, &staging.join("source"), &mut 0, &mut 0)?;
        m.version = Some(version.to_string());
        m.dev_dependencies.clear();
        m.workspace = None;
        write(&staging.join("source/package.don"), &m)?;
        if !staging
            .join("source")
            .join(relative(&m.package.modules)?)
            .is_dir()
        {
            return Err("published package module directory is missing".into());
        }
        for entry in m.bin.values() {
            if !staging.join("source").join(relative(entry)?).is_file() {
                return Err("published bin entry is missing".into());
            }
        }
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut archive = tar::Builder::new(encoder);
        fn append(
            archive: &mut tar::Builder<flate2::write::GzEncoder<Vec<u8>>>,
            root: &Path,
            directory: &Path,
        ) -> Result<(), String> {
            let mut entries = std::fs::read_dir(directory)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    archive
                        .append_dir(path.strip_prefix(root).unwrap(), &path)
                        .map_err(|e| e.to_string())?;
                    append(archive, root, &path)?;
                } else {
                    archive
                        .append_path_with_name(&path, path.strip_prefix(root).unwrap())
                        .map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        }
        append(
            &mut archive,
            &staging.join("source"),
            &staging.join("source"),
        )?;
        let bytes = archive
            .into_inner()
            .map_err(|e| e.to_string())?
            .finish()
            .map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("published archive exceeds 64 MiB".into());
        }
        // Exercise the same extraction limits and path validation used by consumers.
        unpack_package(
            bytes.clone(),
            "file:///package.tar.gz",
            &staging.join("verify"),
        )?;
        let directory = registry.join("packages").join(&m.package.name);
        global_cache_directory(&registry, &directory)?;
        let target = directory.join(format!("{version}.tar.gz"));
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| e.to_string())?;
        if let Err(e) = file.write_all(&bytes).and_then(|_| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(&target);
            return Err(e.to_string());
        }
        drop(file);
        let url = format!("packages/{}/{version}.tar.gz", m.package.name);
        index
            .packages
            .entry(m.package.name.clone())
            .or_default()
            .insert(
                version.to_string(),
                RegistryRelease {
                    url,
                    sha256: format!("{:x}", Sha256::digest(&bytes)),
                },
            );
        if dev_syntax::don::stringify(&serde_json::to_value(&index).map_err(|e| e.to_string())?)?
            .len()
            > 1024 * 1024
        {
            let _ = std::fs::remove_file(&target);
            return Err("registry index exceeds 1 MiB".into());
        }
        if let Err(e) = write(&index_path, &index) {
            let _ = std::fs::remove_file(&target);
            return Err(e);
        }
        println!(
            "published {} {version} to {}",
            m.package.name,
            index_path.display()
        );
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}
