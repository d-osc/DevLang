use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    #[serde(rename = "tag", alias = "rev", skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub workspace: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
}
include!("workspace.rs");
include!("package_archives.rs");
include!("package_bins.rs");
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub name: String,
    #[serde(default = "entry")]
    pub entry: String,
    #[serde(default = "modules")]
    pub modules: String,
}
fn entry() -> String {
    "src/main.dev".into()
}

#[cfg(test)]
mod manifest_tests {
    use super::*;
    #[test]
    fn legacy_lock_migrates_to_don_after_locked_verification() {
        let root = std::env::temp_dir().join(format!("dev-lock-migration-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("package.don"), "package: { name: 'migration' }").unwrap();
        let old = "version = 1\n[packages]\n";
        std::fs::write(root.join("dev.lock"), old).unwrap();
        install(&root, true, false).unwrap();
        let lock: Lock = read(&root.join("package-lock.don")).unwrap();
        assert_eq!(lock.version, 1);
        assert!(lock.packages.is_empty());
        assert_eq!(std::fs::read_to_string(root.join("dev.lock")).unwrap(), old);
        // A malformed new lock must not silently fall back to the old one.
        std::fs::write(root.join("package-lock.don"), "invalid:").unwrap();
        assert!(install(&root, true, false).is_err());
        for file in ["package.don", "dev.lock", "package-lock.don"] { std::fs::remove_file(root.join(file)).unwrap(); }
        std::fs::remove_dir(root).unwrap();
    }
    #[test]
    fn tag_is_canonical_and_rev_remains_a_legacy_alias() {
        let current = serde_json::json!({"git":"https://github.com/example/math.git","tag":"v1.2.0"});
        let legacy = serde_json::json!({"git":"https://github.com/example/math.git","rev":"v1.2.0"});
        let dep: Dependency = serde_json::from_value(current.clone()).unwrap();
        let old: Dependency = serde_json::from_value(legacy).unwrap();
        assert!(dep == old);
        assert_eq!(serde_json::to_value(&dep).unwrap(), current);
        assert!(serde_json::from_value::<Dependency>(serde_json::json!({"git":"https://github.com/example/math.git","tag":"v1","rev":"v2"})).is_err());
        assert!(validate_dependency(&Dependency { version: Some("^1".into()), ..dep }).is_err());
    }
    #[test]
    fn don_version_and_dependency_reference() {
        let text = "version:'v1.0.0'\npackage:{name:'app'}\ndependencies:{utils:{git:'https://github.com/example/utils.git'\nrev:@version}}";
        let manifest: Manifest =
            serde_json::from_value(dev_syntax::don::parse(text).unwrap()).unwrap();
        assert_eq!(manifest.version.as_deref(), Some("v1.0.0"));
        assert_eq!(manifest.dependencies["utils"].rev, manifest.version);
        assert_eq!(manifest.package.entry, "src/main.dev");
        let legacy: Manifest = toml::from_str("[package]\nname='app'").unwrap();
        assert!(legacy.version.is_none());
    }
}
fn modules() -> String {
    "src".into()
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub package: Package,
    #[serde(default)]
    pub dependencies: BTreeMap<String, Dependency>,
    #[serde(default, rename = "devDependencies", alias = "dev-dependencies", skip_serializing_if = "BTreeMap::is_empty")]
    pub dev_dependencies: BTreeMap<String, Dependency>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bin: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<Workspace>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub members: Vec<String>,
}
#[derive(Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Lock {
    version: u32,
    #[serde(default, skip_serializing_if = "is_false")]
    production: bool,
    packages: BTreeMap<String, Locked>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Locked {
    source: Dependency,
    root: String,
    commit: Option<String>,
    sha256: String,
    modules: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolved_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    additional_sources: Vec<Dependency>,
}
fn locked_root(project: &Path, name: &str, item: &Locked) -> Result<PathBuf, String> {
    if !identifier(name) {
        return Err("invalid locked namespace".into());
    }
    if item.source.git.is_some() {
        let commit = item
            .commit
            .as_deref()
            .ok_or("Git lock entry needs commit")?;
        if ![40, 64].contains(&commit.len()) || !commit.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("invalid locked Git commit".into());
        }
        if item.root != format!(".dev/packages/{name}-{commit}") {
            return Err("Git lock path must match its package and commit".into());
        }
    }
    if item.source.url.is_some() {
        let digest = item.source.sha256.as_deref().ok_or("archive lock needs sha256")?;
        if digest.len() != 64 || !digest.bytes().all(|c| c.is_ascii_hexdigit())
            || item.root != format!(".dev/packages/{name}-archive-{}", digest.to_lowercase()) {
            return Err("invalid archive lock path".into());
        }
    }
    Ok(project.join(&item.root))
}
fn lock_path(project: &Path) -> PathBuf {
    let current = project.join("package-lock.don");
    if current.exists() || !project.join("dev.lock").exists() { current }
    else { project.join("dev.lock") }
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    if path.extension().is_some_and(|ext| ext == "don") {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        return serde_json::from_value(
            dev_syntax::don::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?,
        )
        .map_err(|e| format!("{}: {e}", path.display()));
    }
    toml::from_str(&std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| format!("{}: {e}", path.display()))
}
fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(
        &temporary,
        if path.extension().is_some_and(|ext| ext == "don") {
            dev_syntax::don::stringify(&serde_json::to_value(value).map_err(|e| e.to_string())?)?
        } else {
            toml::to_string_pretty(value).map_err(|e| e.to_string())?
        },
    )
    .map_err(|e| e.to_string())?;
    // rename replaces an ordinary file on supported hosts.
    std::fs::rename(temporary, path).map_err(|e| e.to_string())
}
pub fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name != "std"
}
pub fn root(from: &Path) -> Option<PathBuf> {
    from.ancestors()
        .find(|p| p.join("package.don").is_file() || p.join("dev.toml").is_file())
        .and_then(|p| dunce::canonicalize(p).ok())
}
fn relative(path: &str) -> Result<&Path, String> {
    let p = Path::new(path);
    if path.is_empty()
        || !p
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
    {
        return Err("package entry/modules must be relative paths without '..'".into());
    }
    Ok(p)
}
pub fn manifest(root: &Path) -> Result<Manifest, String> {
    let m: Manifest = read(&manifest_path(root))?;
    if !identifier(&m.package.name) {
        return Err("invalid package name".into());
    }
    relative(&m.package.entry)?;
    relative(&m.package.modules)?;
    for (name, entry) in &m.bin {
        if !bin_name(name) { return Err("invalid bin command name".into()); }
        relative(entry)?;
        if !entry.ends_with(".dev") { return Err("bin entry must be a .dev source file".into()); }
    }
    Ok(m)
}
fn manifest_path(root: &Path) -> PathBuf {
    root.join(if root.join("package.don").is_file() {
        "package.don"
    } else {
        "dev.toml"
    })
}
fn git(dir: Option<&Path>, args: &[&str]) -> Result<String, String> {
    let mut command = Command::new("git");
    command.env("GIT_TERMINAL_PROMPT", "0");
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    let output = command
        .args(args)
        .output()
        .map_err(|e| format!("git: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().into())
}
fn tree_hash(root: &Path) -> Result<String, String> {
    tree_hash_mode(root, false)
}
fn tree_matches(root: &Path, expected: &str) -> Result<bool, String> {
    Ok(tree_hash(root)? == expected || tree_hash_mode(root, true)? == expected)
}
fn tree_hash_mode(root: &Path, include_lock: bool) -> Result<String, String> {
    fn files(
        root: &Path,
        at: &Path,
        result: &mut Vec<PathBuf>,
        include_lock: bool,
    ) -> Result<(), String> {
        for item in std::fs::read_dir(at).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let ty = item.file_type().map_err(|e| e.to_string())?;
            if ty.is_symlink() {
                return Err("package symlinks are unsupported".into());
            }
            let path = item.path();
            if ty.is_dir() {
                if ![".git", ".dev", "target", "dist", "out", "node_modules"]
                    .contains(&item.file_name().to_string_lossy().as_ref())
                {
                    files(root, &path, result, include_lock)?;
                }
            } else if ty.is_file()
                && (include_lock || path.file_name().is_none_or(|name| name != "package-lock.don" && name != "dev.lock"))
            {
                result.push(path.strip_prefix(root).unwrap().to_owned());
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    files(root, root, &mut paths, include_lock)?;
    paths.sort();
    let mut hash = Sha256::new();
    for path in paths {
        let name = path.to_string_lossy().replace('\\', "/");
        let bytes = std::fs::read(root.join(path)).map_err(|e| e.to_string())?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn resolve(
    project: &Path,
    parent: &Path,
    name: &str,
    dep: &Dependency,
    previous: Option<&Locked>,
    refresh: bool,
    locked: bool,
) -> Result<Locked, String> {
    if !identifier(name) {
        return Err(format!("invalid dependency namespace {name}"));
    }
    validate_dependency(dep)?;
    let req = requirement(dep)?;
    let mut selected_version = None;
    let (path, commit) = if dep.workspace {
        (workspace_dependency(parent, name)?, None)
    } else if dep.url.is_some() {
        (fetch_archive(project, name, dep, previous)?, None)
    } else {
        match (&dep.path, &dep.git) {
            (Some(p), None) if dep.rev.is_none() => (
                parent.join(p).canonicalize().map_err(|e| e.to_string())?,
                None,
            ),
            (None, Some(remote)) => {
                if !(remote.starts_with("https://") || remote.starts_with("file://")) {
                    return Err("Git sources require https:// or file:// URLs".into());
                }
                if remote.contains('@') && remote.starts_with("https://") {
                    return Err("Git URL must not contain embedded credentials".into());
                }
                if !refresh {
                    if let Some(old) = previous.filter(|p| p.source == *dep) {
                        let cached = locked_root(project, name, old)?;
                        if cached.is_dir() {
                            if !tree_matches(&cached, &old.sha256)?
                                || Some(git(Some(&cached), &["rev-parse", "HEAD"])?).as_ref()
                                    != old.commit.as_ref()
                            {
                                return Err("cached dependency changed; remove its checkout before reinstalling".into());
                            }
                            let mut item = old.clone();
                            item.additional_sources.clear();
                            if let Some(req) = &req {
                                let version =
                                    package_version(&manifest(&cached)?, req)?.to_string();
                                if old.resolved_version.as_deref() != Some(&version) {
                                    return Err("locked package version changed".into());
                                }
                            }
                            if !locked {
                                item.sha256 = tree_hash(&cached)?;
                            }
                            return Ok(item);
                        }
                    }
                }
                let pinned = if !refresh {
                    previous
                        .filter(|p| p.source == *dep)
                        .and_then(|p| p.commit.as_deref())
                } else {
                    None
                };
                let branch_ref = dep.branch.as_ref().map(|name| format!("refs/remotes/origin/{name}"));
                let revision = pinned.unwrap_or(branch_ref.as_deref().or(dep.rev.as_deref()).unwrap_or("HEAD"));
                if revision.starts_with('-') || revision.contains(['\n', '\r']) {
                    return Err("invalid Git revision".into());
                }
                let cache = project.join(".dev/packages");
                std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
                let temp = cache.join(format!(
                    "{name}-fetch-{}-{}",
                    std::process::id(),
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| e.to_string())?
                        .as_nanos()
                ));
                if temp.exists() {
                    return Err(format!(
                        "stale fetch directory {}; remove it before retrying",
                        temp.display()
                    ));
                }
                git(
                    None,
                    &[
                        "-c",
                        "core.autocrlf=false",
                        "clone",
                        "--no-checkout",
                        "--",
                        remote,
                        temp.to_str().ok_or("non-UTF8 cache path")?,
                    ],
                )?;
                let chosen;
                let revision = if pinned.is_none() && req.is_some() {
                    let (tag, version) = semver_tag(&temp, req.as_ref().unwrap())?;
                    chosen = tag;
                    selected_version = Some(version);
                    chosen.as_str()
                } else {
                    revision
                };
                let commit = git(
                    Some(&temp),
                    &["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
                )?;
                git(
                    Some(&temp),
                    &[
                        "-c",
                        "core.autocrlf=false",
                        "-c",
                        "core.hooksPath=",
                        "checkout",
                        "--detach",
                        &commit,
                    ],
                )?;
                let destination = cache.join(format!("{name}-{commit}"));
                if destination.exists() {
                    // Keep the existing checkout; verify its content against this fresh copy.
                    if tree_hash(&temp)? != tree_hash(&destination)? {
                        return Err(
                            "cached dependency changed; remove its checkout before reinstalling"
                                .into(),
                        );
                    }
                    // No recursive deletion: retain the fetch checkout for recovery.
                    std::fs::rename(
                        &temp,
                        cache.join(format!(
                            "{name}-verified-{}",
                            std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_nanos()
                        )),
                    )
                    .map_err(|e| e.to_string())?;
                } else {
                    std::fs::rename(temp, &destination).map_err(|e| e.to_string())?;
                }
                (
                    destination.canonicalize().map_err(|e| e.to_string())?,
                    Some(commit),
                )
            }
            _ => return Err("dependency needs exactly one of path or git; tag is Git-only".into()),
        }
    };
    let path = dunce::simplified(&path).to_owned();
    let m = manifest(&path)?;
    let resolved_version = req
        .as_ref()
        .map(|req| package_version(&m, req))
        .transpose()?;
    if selected_version
        .as_ref()
        .zip(resolved_version.as_ref())
        .is_some_and(|(tag, actual)| tag != actual)
    {
        return Err("Git tag version differs from package manifest version".into());
    }
    let modules = m.package.modules;
    if !path.join(&modules).is_dir() {
        return Err("dependency module directory does not exist".into());
    }
    let stored = if dep.git.is_some() || dep.url.is_some() {
        path.strip_prefix(project)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/")
    } else {
        pathdiff::diff_paths(&path, project)
            .unwrap_or(path.clone())
            .to_string_lossy()
            .replace('\\', "/")
    };
    let sha256 = if locked {
        if let Some(old) = previous.filter(|old| old.source == *dep) {
            if !tree_matches(&path, &old.sha256)? {
                return Err(format!("dependency {name} content changed from package-lock.don"));
            }
            old.sha256.clone()
        } else {
            tree_hash(&path)?
        }
    } else {
        tree_hash(&path)?
    };
    Ok(Locked {
        source: dep.clone(),
        root: stored,
        commit,
        sha256,
        modules,
        resolved_version: resolved_version.map(|v| v.to_string()),
        additional_sources: Vec::new(),
    })
}

pub fn install(project: &Path, locked: bool, refresh: bool) -> Result<(), String> {
    install_mode(project, locked, refresh, false)
}
fn root_dependencies(m: &Manifest, production: bool) -> Result<BTreeMap<String, Dependency>, String> {
    if m.dependencies.keys().any(|name| m.dev_dependencies.contains_key(name)) {
        return Err("dependency namespace cannot appear in both dependencies and devDependencies".into());
    }
    let mut dependencies = m.dependencies.clone();
    if !production { dependencies.extend(m.dev_dependencies.clone()); }
    Ok(dependencies)
}
fn install_mode(project: &Path, locked: bool, refresh: bool, production: bool) -> Result<(), String> {
    let old_path = lock_path(project);
    let previous: Lock = if old_path.exists() {
        read(&old_path)?
    } else {
        Lock::default()
    };
    if locked && previous.version != 1 {
        return Err("--locked needs an existing version 1 package-lock.don".into());
    }
    if locked && previous.production != production {
        return Err("lock installation mode differs; run pkg install with the requested --production mode first".into());
    }
    let mut result = Lock {
        version: 1,
        production,
        packages: BTreeMap::new(),
    };
    let mut pending = vec![(project.to_owned(), root_dependencies(&manifest(project)?, production)?)];
    while let Some((parent, dependencies)) = pending.pop() {
        for (name, dep) in dependencies {
            if let Some(old) = result.packages.get_mut(&name) {
                if old.source != dep {
                    if !compatible_source(&old.source, &dep, old.resolved_version.as_deref())? {
                        return Err(format!("conflicting dependency namespace {name}"));
                    }
                    if !old.additional_sources.contains(&dep) {
                        old.additional_sources.push(dep.clone());
                        old.additional_sources.sort();
                    }
                }
                if let Some(p) = &dep.path {
                    if parent.join(p).canonicalize().map_err(|e| e.to_string())?
                        != project
                            .join(&old.root)
                            .canonicalize()
                            .map_err(|e| e.to_string())?
                    {
                        return Err(format!("conflicting local dependency {name}"));
                    }
                }
                if dep.workspace
                    && workspace_dependency(&parent, &name)?
                        != dunce::canonicalize(project.join(&old.root))
                            .map_err(|e| e.to_string())?
                {
                    return Err(format!("conflicting workspace dependency {name}"));
                }
                continue;
            }
            if locked && previous.packages.get(&name).is_none_or(|p| p.source != dep) {
                return Err(format!("dependency {name} differs from package-lock.don"));
            }
            let resolved = resolve(
                project,
                &parent,
                &name,
                &dep,
                previous.packages.get(&name),
                refresh,
                locked,
            )?;
            if locked
                && previous.packages.get(&name).map(|p| {
                    let mut p = p.clone();
                    p.additional_sources.clear();
                    p
                }) != Some(resolved.clone())
            {
                return Err(format!("dependency {name} content changed from package-lock.don"));
            }
            let dep_root = project.join(&resolved.root);
            pending.push((dep_root.clone(), manifest(&dep_root)?.dependencies));
            println!(
                "installed {name} {}",
                resolved.commit.as_deref().unwrap_or("local")
            );
            result.packages.insert(name, resolved);
        }
    }
    if locked {
        if result != previous {
            return Err("dependencies differ from package-lock.don".into());
        }
        let bins = package_bins(project, &result)?;
        install_bins(project, &bins)?;
        if old_path.file_name().is_some_and(|n| n == "dev.lock") {
            write(&project.join("package-lock.don"), &previous)?;
        }
        println!("dependencies verified against package-lock.don");
        Ok(())
    } else {
        let bins = package_bins(project, &result)?;
        install_bins(project, &bins)?;
        write(&project.join("package-lock.don"), &result)
    }
}
pub fn mappings(project: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let m = manifest(project)?;
    if m.dependencies.is_empty() && m.dev_dependencies.is_empty() && !lock_path(project).exists() {
        return Ok(Vec::new());
    }
    let lock: Lock =
        read(&lock_path(project)).map_err(|e| format!("{e}; run d pkg install"))?;
    if lock.version != 1 {
        return Err("unsupported package-lock.don version".into());
    }
    let mut pending = vec![(project.to_owned(), root_dependencies(&m, lock.production)?)];
    let mut visited = BTreeSet::new();
    let mut dirs = Vec::new();
    let mut declarations = BTreeMap::<String, BTreeSet<Dependency>>::new();
    while let Some((parent, dependencies)) = pending.pop() {
        for (name, dep) in dependencies {
            let item = lock
                .packages
                .get(&name)
                .ok_or_else(|| format!("dependency {name} missing from lock; run d pkg install"))?;
            if item.source != dep && !item.additional_sources.contains(&dep) {
                return Err(format!("dependency {name} changed; run d pkg install"));
            }
            declarations
                .entry(name.clone())
                .or_default()
                .insert(dep.clone());
            let at = locked_root(project, &name, item)?
                .canonicalize()
                .map_err(|e| format!("{e}; run d pkg install"))?;
            if let Some(p) = &dep.path {
                if parent.join(p).canonicalize().map_err(|e| e.to_string())? != at {
                    return Err(format!("local dependency {name} changed"));
                }
            }
            validate_dependency(&dep)?;
            if dep.workspace
                && workspace_dependency(&parent, &name)?
                    != dunce::canonicalize(&at).map_err(|e| e.to_string())?
            {
                return Err(format!("workspace dependency {name} changed"));
            }
            if let Some(req) = requirement(&dep)? {
                let version = semver::Version::parse(
                    item.resolved_version
                        .as_deref()
                        .ok_or("missing locked package version")?,
                )
                .map_err(|e| e.to_string())?;
                if !req.matches(&version) {
                    return Err(format!("locked version {version} does not satisfy {req}"));
                }
            }
            if !visited.insert(name.clone()) {
                continue;
            }
            if !tree_matches(&at, &item.sha256)? {
                return Err(format!("dependency {name} content changed; run d pkg install to update local dependencies"));
            }
            if let Some(commit) = &item.commit {
                if git(Some(&at), &["rev-parse", "HEAD"])? != *commit {
                    return Err(format!("dependency {name} checkout commit changed"));
                }
            }
            let child = manifest(&at)?;
            if let Some(req) = requirement(&dep)? {
                let version = package_version(&child, &req)?.to_string();
                if item.resolved_version.as_deref() != Some(&version) {
                    return Err("locked package version changed".into());
                }
            }
            if child.package.modules != item.modules {
                return Err("locked module directory changed".into());
            }
            dirs.push((name, at.join(relative(&item.modules)?)));
            pending.push((at, child.dependencies));
        }
    }
    if visited.len() != lock.packages.len() {
        return Err("package-lock.don contains stale dependencies; run d pkg install".into());
    }
    for (name, item) in &lock.packages {
        let expected: BTreeSet<_> = std::iter::once(item.source.clone())
            .chain(item.additional_sources.iter().cloned())
            .collect();
        if declarations.get(name) != Some(&expected) {
            return Err(
                "package-lock.don contains stale dependency requirements; run d pkg install".into(),
            );
        }
    }
    dirs.sort();
    Ok(dirs)
}
pub fn new(path: &Path) -> Result<i32, String> {
    if path.exists() {
        return Err("new project destination already exists".into());
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("invalid project path")?;
    if !identifier(name) {
        return Err("project name must be a Dev identifier".into());
    }
    std::fs::create_dir_all(path.join("src")).map_err(|e| e.to_string())?;
    write(
        &path.join("package.don"),
        &Manifest {
            version: Some("0.1.0".into()),
            package: Package {
                name: name.into(),
                entry: entry(),
                modules: modules(),
            },
            dependencies: BTreeMap::new(),
            dev_dependencies: BTreeMap::new(),
            bin: BTreeMap::new(),
            workspace: None,
        },
    )?;
    std::fs::write(
        path.join("src/main.dev"),
        "fn main() {\n    print(\"Hello, DevLang!\")\n}\n\nmain()\n",
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(path.join(".gitignore"), ".dev/\nout/\n").map_err(|e| e.to_string())?;
    std::fs::write(
        path.join(".gitattributes"),
        "*.dev text eol=lf\n*.don text eol=lf\n*.toml text eol=lf\npackage-lock.don text eol=lf\n",
    )
    .map_err(|e| e.to_string())?;
    println!("created {}", path.display());
    Ok(0)
}
pub fn command(args: &[String]) -> Result<i32, String> {
    let project = root(&std::env::current_dir().map_err(|e| e.to_string())?)
        .ok_or("no package.don or dev.toml; use d new NAME")?;
    match args.first().map(String::as_str) {
        Some("install") | Some("update") => {
            let mut locked=false; let mut all=false; let mut production=false;
            for option in &args[1..] {
                match option.as_str() {
                    "--locked" if !locked && args[0] == "install" => locked=true,
                    "--workspace" if !all => all=true,
                    "--production" if !production => production=true,
                    _ => return Err("pkg install [--locked] [--workspace] [--production]; pkg update [--workspace] [--production]".into()),
                }
            }
            if all {
                let root=workspace_root(&project)?.ok_or("--workspace needs workspace.members")?;
                let members=workspace_members(&root)?;
                install_mode(&root,locked,args[0]=="update",production)?;
                for (name,at) in members { println!("package {name}"); install_mode(&at,locked,args[0]=="update",production)?; }
            } else { install_mode(&project, locked, args[0] == "update", production)?; }
        }
        Some("workspace") if args.len()==1 => {
            let root=workspace_root(&project)?.ok_or("no workspace.members")?;
            for (name,at) in workspace_members(&root)? { println!("{name} {}",at.display()); }
        }
        Some("add") => {
            let name = args.get(1).ok_or("pkg add NAME --path DIR | --url URL --sha256 HEX | --git URL [--tag TAG | --branch NAME]")?;
            if !identifier(name) { return Err("invalid dependency name".into()); }
            let mut dep = Dependency { path: None, git: None, rev: None, branch: None, url: None, sha256: None, version: None, workspace: false };
            let mut at = 2;
            let mut development = false;
            let mut seen=BTreeSet::new();
            while at < args.len() {
                let option = if args[at] == "--rev" { "--tag" } else { args[at].as_str() };
                if !seen.insert(option.to_owned()) { return Err("duplicate dependency option".into()); }
                if args[at] == "--workspace" { dep.workspace=true; at+=1; continue; }
                if args[at] == "--dev" { development=true; at+=1; continue; }
                let value = args.get(at + 1).ok_or("dependency option needs value")?.clone();
                match args[at].as_str() { "--path" => { let absolute = dunce::canonicalize(&value).map_err(|e| e.to_string())?; dep.path = Some(pathdiff::diff_paths(&absolute,&project).unwrap_or(absolute).to_string_lossy().replace('\\', "/")); }, "--git" => dep.git = Some(value), "--tag" | "--rev" => dep.rev = Some(value), "--url" => dep.url = Some(value), "--sha256" => dep.sha256 = Some(value), "--branch" => dep.branch = Some(value), "--version" => dep.version=Some(value), _ => return Err("unknown dependency option".into()) }
                at += 2;
            }
            validate_dependency(&dep)?;
            let mut m = manifest(&project)?;
            if development { m.dev_dependencies.insert(name.clone(), dep); }
            else { m.dependencies.insert(name.clone(), dep); }
            root_dependencies(&m, false)?;
            change_manifest(&project,&m)?;
        }
        Some("remove") if args.len() == 2 || (args.len() == 3 && args[2] == "--dev") => {
            let mut m = manifest(&project)?;
            let dependencies = if args.len() == 3 { &mut m.dev_dependencies } else { &mut m.dependencies };
            dependencies.remove(&args[1]).ok_or("dependency does not exist")?;
            change_manifest(&project,&m)?;
        }
        Some("list") if args.len() == 1 => { for (name, dir) in mappings(&project)? { println!("{name} {}", dir.display()); } }
        Some("bin") if args.len() == 1 => {
            mappings(&project)?;
            let lock: Lock = if lock_path(&project).exists() { read(&lock_path(&project))? } else { Lock::default() };
            for (name, entry) in package_bins(&project, &lock)? { println!("{name} {}", entry.display()); }
        }
        _ => return Err("pkg add NAME --path DIR | --url URL --sha256 HEX | --git URL | --workspace [--tag TAG | --branch NAME | --version REQUIREMENT]; pkg install [--locked] [--workspace] [--production]; pkg update [--workspace] [--production]; pkg workspace; pkg remove NAME [--dev]; pkg list".into()),
    }
    Ok(0)
}
