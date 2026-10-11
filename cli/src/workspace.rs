fn workspace_root(from: &Path) -> Result<Option<PathBuf>, String> {
    for at in from.ancestors() {
        if at.join("package.don").is_file() || at.join("dev.toml").is_file() {
            if manifest(at)?.workspace.is_some() {
                return Ok(Some(dunce::canonicalize(at).map_err(|e| e.to_string())?));
            }
        }
    }
    Ok(None)
}
fn workspace_members(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let m = manifest(root)?;
    let workspace = m.workspace.ok_or("project has no workspace.members")?;
    if workspace.members.is_empty() || workspace.members.len() > 128 {
        return Err("workspace needs 1..128 explicit member directories".into());
    }
    let mut result = BTreeMap::new();
    for member in workspace.members {
        let relative = relative(&member)?;
        let mut at = root.to_owned();
        for component in relative.components() {
            at.push(component);
            if std::fs::symlink_metadata(&at)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
            {
                return Err("workspace member symlinks are unsupported".into());
            }
        }
        let at = dunce::canonicalize(at).map_err(|e| e.to_string())?;
        if !at.starts_with(root) || at == root {
            return Err("workspace member must stay inside the workspace".into());
        }
        if result
            .values()
            .any(|other: &PathBuf| at.starts_with(other) || other.starts_with(&at))
        {
            return Err("workspace members must not overlap or repeat".into());
        }
        let child = manifest(&at)?;
        if child.workspace.is_some() {
            return Err("nested member workspaces are unsupported".into());
        }
        if child.package.name == m.package.name || result.insert(child.package.name, at).is_some() {
            return Err("workspace package names must be unique".into());
        }
    }
    Ok(result)
}
fn workspace_dependency(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let parent = dunce::canonicalize(parent).map_err(|e| e.to_string())?;
    let root = workspace_root(&parent)?.ok_or("workspace dependency has no workspace root")?;
    let members = workspace_members(&root)?;
    if parent != root && !members.values().any(|p| p == &parent) {
        return Err("workspace dependencies require a declared member package".into());
    }
    members
        .get(name)
        .cloned()
        .ok_or_else(|| format!("workspace member {name} not found"))
}
pub fn select_project(from: &Path, name: &str) -> Result<PathBuf, String> {
    if !identifier(name) {
        return Err("invalid --package name".into());
    }
    let root = workspace_root(from)?.ok_or("--package needs a workspace root")?;
    if manifest(&root)?.package.name == name {
        return Ok(root);
    }
    workspace_members(&root)?
        .remove(name)
        .ok_or_else(|| format!("workspace member {name} not found"))
}
fn requirement(dep: &Dependency) -> Result<Option<VersionRange>, String> {
    dep.range.as_ref().or(dep.version.as_ref())
        .map(String::as_str)
        .map(|text| {
            if text.len() > 256 {
                return Err("dependency version requirement exceeds 256 bytes".into());
            }
            VersionRange::parse(text).map_err(|e| format!("invalid SemVer requirement: {e}"))
        })
        .transpose()
}
fn package_version(m: &Manifest, req: &VersionRange) -> Result<semver::Version, String> {
    let text = m
        .version
        .as_deref()
        .ok_or("version dependency needs root manifest version")?;
    let version = semver::Version::parse(text.strip_prefix('v').unwrap_or(text))
        .map_err(|e| format!("invalid package SemVer: {e}"))?;
    if !req.matches(&version) {
        return Err(format!("package version {version} does not satisfy {req}"));
    }
    Ok(version)
}
fn semver_tag(repo: &Path, req: &VersionRange) -> Result<(String, semver::Version), String> {
    let tags = git(Some(repo), &["tag", "--list"])?;
    if tags.len() > 1024 * 1024 {
        return Err("Git tag listing exceeds 1 MiB".into());
    }
    let mut matches = Vec::new();
    for tag in tags.lines() {
        if let Ok(version) = semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag)) {
            if req.matches(&version) {
                matches.push((tag.to_owned(), version));
            }
        }
    }
    matches.sort_by(|(a, av), (b, bv)| av.cmp_precedence(bv).then_with(|| a.cmp(b)));
    if matches.is_empty(){return Err(format!("no Git SemVer tag satisfies {req}"));}
    let selected_version=choose_version(matches.iter().map(|(_,v)|v.clone()).collect())?;
    let selected=matches.iter().find(|(_,v)|*v==selected_version).cloned().ok_or("no matching Git tag")?;
    if matches.iter().filter(|(_,v)|v.cmp_precedence(&selected.1).is_eq()).count()>1 {
        return Err("ambiguous Git tags with equal SemVer precedence".into());
    }
    Ok(selected)
}
fn validate_dependency(dep: &Dependency) -> Result<(), String> {
    if usize::from(dep.path.is_some()) + usize::from(dep.git.is_some()) + usize::from(dep.workspace) + usize::from(dep.url.is_some())
        != 1
    {
        return Err("dependency needs exactly one of path, git, url or workspace".into());
    }
    if dep.url.is_some() {
        let digest = dep.sha256.as_deref().ok_or("url dependency requires sha256")?;
        if digest.len() != 64 || !digest.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("archive sha256 must contain 64 hexadecimal characters".into());
        }
    } else if dep.sha256.is_some() {
        return Err("sha256 is only supported for url dependencies".into());
    }
    if dep.rev.is_some() && (dep.git.is_none() || dep.version.is_some()) {
        return Err("tag is Git-only and cannot be combined with version".into());
    }
    if let Some(branch) = &dep.branch {
        if dep.git.is_none() || dep.rev.is_some() || dep.version.is_some() {
            return Err("branch is Git-only and cannot be combined with tag or version".into());
        }
        if branch.is_empty() || branch.starts_with('-') || branch == "HEAD"
            || git(None, &["check-ref-format", &format!("refs/heads/{branch}")]).is_err() {
            return Err("invalid Git branch name".into());
        }
    }
    if let Some(subdir) = &dep.subdir { if dep.git.is_none() || subdir.contains('\\') { return Err("subdir requires a Git source and forward slashes".into()); } relative(subdir)?; }
    if let Some(package) = &dep.package { scoped_identity(package)?; if dep.git.is_none() { return Err("scoped package identity requires Git".into()); } }
    if dep.store.is_some() && dep.package.is_none() { return Err("store source needs scoped package identity".into()); }
    if dep.registry.is_some() && dep.url.is_none() { return Err("registry metadata requires an archive source".into()); }
    if let Some(range) = &dep.range { peer_requirement(range)?; if dep.registry.is_none() { return Err("range metadata requires registry".into()); } }
    requirement(dep)?;
    Ok(())
}
fn change_manifest(project: &Path, value: &Manifest) -> Result<(), String> {
    let path = manifest_path(project);
    let previous = std::fs::read(&path).map_err(|e| e.to_string())?;
    write(&path, value)?;
    if let Err(error) = install(project, false, false) {
        std::fs::write(&path, previous)
            .map_err(|e| format!("{error}; manifest rollback failed: {e}"))?;
        return Err(error);
    }
    Ok(())
}
fn compatible_source(
    a: &Dependency,
    b: &Dependency,
    version: Option<&str>,
) -> Result<bool, String> {
    if a.subdir != b.subdir || a.package != b.package || a.store != b.store || a.registry != b.registry || a.range != b.range
        || a.path != b.path
        || a.git != b.git
        || a.rev != b.rev
        || a.branch != b.branch
        || a.url != b.url
        || a.sha256 != b.sha256
        || a.workspace != b.workspace
        || a.version.is_none()
        || b.version.is_none()
    {
        return Ok(false);
    }
    let Some(version) = version else {
        return Ok(false);
    };
    let version = semver::Version::parse(version).map_err(|e| e.to_string())?;
    Ok(requirement(a)?.unwrap().matches(&version) && requirement(b)?.unwrap().matches(&version))
}
