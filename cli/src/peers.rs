fn peer_requirement(requirement: &str) -> Result<semver::VersionReq, String> {
    if requirement.len() > 256 { return Err("peer requirement exceeds 256 bytes".into()); }
    semver::VersionReq::parse(requirement).map_err(|e| format!("invalid peer requirement: {e}"))
}
fn verify_peers(project: &Path, lock: &Lock) -> Result<(), String> {
    let root = manifest(project)?;
    let mut providers = BTreeMap::new();
    providers.insert(root.package.name.clone(), root.version.clone());
    let mut manifests = vec![root];
    for (name, item) in &lock.packages {
        let m = manifest(&locked_root(project, name, item)?)?;
        providers.insert(name.clone(), m.version.clone());
        manifests.push(m);
    }
    for m in manifests {
        for (name, requirement) in m.peer_dependencies {
            let version = providers.get(&name).ok_or_else(|| format!("package '{}' requires peer '{name}' {requirement}; declare it in the consumer dependencies", m.package.name))?;
            let version = version.as_deref().ok_or_else(|| format!("peer '{name}' must declare a package version"))?;
            let version = semver::Version::parse(version.strip_prefix('v').unwrap_or(version)).map_err(|e| format!("invalid peer version: {e}"))?;
            if !peer_requirement(&requirement)?.matches(&version) { return Err(format!("peer '{name}' version {version} does not satisfy {requirement} required by '{}'", m.package.name)); }
        }
    }
    Ok(())
}
