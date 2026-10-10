use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
}
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
    fn don_version_and_dependency_reference() {
        let text = "version:'v1.0.0'\npackage:{name:'app'}\ndependencies:{utils:{git:'https://github.com/example/utils.git'\nrev:@version}}";
        let manifest: Manifest = serde_json::from_value(dev_syntax::don::parse(text).unwrap()).unwrap();
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
}
#[derive(Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Lock {
    version: u32,
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
    Ok(project.join(&item.root))
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
    fn files(root: &Path, at: &Path, result: &mut Vec<PathBuf>) -> Result<(), String> {
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
                    files(root, &path, result)?;
                }
            } else if ty.is_file() {
                result.push(path.strip_prefix(root).unwrap().to_owned());
            }
        }
        Ok(())
    }
    let mut paths = Vec::new();
    files(root, root, &mut paths)?;
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
) -> Result<Locked, String> {
    if !identifier(name) {
        return Err(format!("invalid dependency namespace {name}"));
    }
    let (path, commit) = match (&dep.path, &dep.git) {
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
                        if tree_hash(&cached)? != old.sha256
                            || Some(git(Some(&cached), &["rev-parse", "HEAD"])?).as_ref()
                                != old.commit.as_ref()
                        {
                            return Err("cached dependency changed; remove its checkout before reinstalling".into());
                        }
                        return Ok(old.clone());
                    }
                }
            }
            let revision = (if !refresh {
                previous
                    .filter(|p| p.source == *dep)
                    .and_then(|p| p.commit.as_deref())
            } else {
                None
            })
            .unwrap_or(dep.rev.as_deref().unwrap_or("HEAD"));
            if revision.starts_with('-') || revision.contains(['\n', '\r']) {
                return Err("invalid Git revision".into());
            }
            let cache = project.join(".dev/packages");
            std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
            let temp = cache.join(format!("{name}-fetch-{}", std::process::id()));
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
                        "cached dependency changed; remove its checkout before reinstalling".into(),
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
        _ => return Err("dependency needs exactly one of path or git; rev is Git-only".into()),
    };
    let path = dunce::simplified(&path).to_owned();
    let m = manifest(&path)?;
    let modules = m.package.modules;
    if !path.join(&modules).is_dir() {
        return Err("dependency module directory does not exist".into());
    }
    let stored = if dep.git.is_some() {
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
    Ok(Locked {
        source: dep.clone(),
        root: stored,
        commit,
        sha256: tree_hash(&path)?,
        modules,
    })
}

pub fn install(project: &Path, locked: bool, refresh: bool) -> Result<(), String> {
    let previous: Lock = if project.join("dev.lock").exists() {
        read(&project.join("dev.lock"))?
    } else {
        Lock::default()
    };
    if locked && previous.version != 1 {
        return Err("--locked needs an existing version 1 dev.lock".into());
    }
    let mut result = Lock {
        version: 1,
        packages: BTreeMap::new(),
    };
    let mut pending = vec![(project.to_owned(), manifest(project)?.dependencies)];
    while let Some((parent, dependencies)) = pending.pop() {
        for (name, dep) in dependencies {
            if let Some(old) = result.packages.get(&name) {
                if old.source != dep {
                    return Err(format!("conflicting dependency namespace {name}"));
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
                continue;
            }
            if locked && previous.packages.get(&name).is_none_or(|p| p.source != dep) {
                return Err(format!("dependency {name} differs from dev.lock"));
            }
            let resolved = resolve(
                project,
                &parent,
                &name,
                &dep,
                previous.packages.get(&name),
                refresh,
            )?;
            if locked && previous.packages.get(&name) != Some(&resolved) {
                return Err(format!("dependency {name} content changed from dev.lock"));
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
            return Err("dependencies differ from dev.lock".into());
        }
        println!("dependencies verified against dev.lock");
        Ok(())
    } else {
        write(&project.join("dev.lock"), &result)
    }
}
pub fn mappings(project: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let m = manifest(project)?;
    if m.dependencies.is_empty() && !project.join("dev.lock").exists() {
        return Ok(Vec::new());
    }
    let lock: Lock =
        read(&project.join("dev.lock")).map_err(|e| format!("{e}; run d pkg install"))?;
    if lock.version != 1 {
        return Err("unsupported dev.lock version".into());
    }
    let mut pending = vec![(project.to_owned(), m.dependencies)];
    let mut visited = BTreeSet::new();
    let mut dirs = Vec::new();
    while let Some((parent, dependencies)) = pending.pop() {
        for (name, dep) in dependencies {
            let item = lock
                .packages
                .get(&name)
                .ok_or_else(|| format!("dependency {name} missing from lock; run d pkg install"))?;
            if item.source != dep {
                return Err(format!("dependency {name} changed; run d pkg install"));
            }
            let at = locked_root(project, &name, item)?
                .canonicalize()
                .map_err(|e| format!("{e}; run d pkg install"))?;
            if let Some(p) = &dep.path {
                if parent.join(p).canonicalize().map_err(|e| e.to_string())? != at {
                    return Err(format!("local dependency {name} changed"));
                }
            }
            if !visited.insert(name.clone()) {
                continue;
            }
            if tree_hash(&at)? != item.sha256 {
                return Err(format!("dependency {name} content changed; run d pkg install to update local dependencies"));
            }
            if let Some(commit) = &item.commit {
                if git(Some(&at), &["rev-parse", "HEAD"])? != *commit {
                    return Err(format!("dependency {name} checkout commit changed"));
                }
            }
            let child = manifest(&at)?;
            if child.package.modules != item.modules {
                return Err("locked module directory changed".into());
            }
            dirs.push((name, at.join(relative(&item.modules)?)));
            pending.push((at, child.dependencies));
        }
    }
    if visited.len() != lock.packages.len() {
        return Err("dev.lock contains stale dependencies; run d pkg install".into());
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
        "*.dev text eol=lf\n*.don text eol=lf\n*.toml text eol=lf\ndev.lock text eol=lf\n",
    )
    .map_err(|e| e.to_string())?;
    println!("created {}", path.display());
    Ok(0)
}
pub fn command(args: &[String]) -> Result<i32, String> {
    let project = root(&std::env::current_dir().map_err(|e| e.to_string())?)
        .ok_or("no package.don or dev.toml; use d new NAME")?;
    match args.first().map(String::as_str) {
        Some("install") | Some("update") if args.len() <= 2 && args.get(1).is_none_or(|a| a == "--locked") && !(args[0]=="update" && args.len()==2) => {
            install(&project, args.get(1).is_some(), args[0] == "update")?;
        }
        Some("add") => {
            let name = args.get(1).ok_or("pkg add NAME --path DIR | --git URL [--rev REF]")?;
            if !identifier(name) { return Err("invalid dependency name".into()); }
            let mut dep = Dependency { path: None, git: None, rev: None };
            let mut at = 2;
            while at < args.len() {
                let value = args.get(at + 1).ok_or("dependency option needs value")?.clone();
                match args[at].as_str() { "--path" => { let absolute = dunce::canonicalize(&value).map_err(|e| e.to_string())?; dep.path = Some(pathdiff::diff_paths(&absolute,&project).unwrap_or(absolute).to_string_lossy().replace('\\', "/")); }, "--git" => dep.git = Some(value), "--rev" => dep.rev = Some(value), _ => return Err("unknown dependency option".into()) }
                at += 2;
            }
            if dep.path.is_some() == dep.git.is_some() || (dep.path.is_some() && dep.rev.is_some()) { return Err("use exactly one --path or --git; --rev is Git-only".into()); }
            let mut m = manifest(&project)?; m.dependencies.insert(name.clone(), dep); write(&manifest_path(&project), &m)?;
            install(&project, false, false)?;
        }
        Some("remove") if args.len() == 2 => { let mut m = manifest(&project)?; m.dependencies.remove(&args[1]).ok_or("dependency does not exist")?; write(&manifest_path(&project), &m)?; install(&project, false, false)?; }
        Some("list") if args.len() == 1 => { for (name, dir) in mappings(&project)? { println!("{name} {}", dir.display()); } }
        _ => return Err("pkg add NAME --path DIR | --git URL [--rev REF]; pkg install [--locked]; pkg update; pkg remove NAME; pkg list".into()),
    }
    Ok(0)
}
