fn resolve_named(
    project: &Path,
    parent: &Path,
    name: &str,
    cache_name: &str,
    dep: &Dependency,
    previous: Option<&Locked>,
    refresh: bool,
    locked: bool,
) -> Result<Locked, String> {
    RESOLUTION_SEARCH.with(|s| s.borrow_mut().key = cache_name.into());
    let selected = if let (Some(registry), Some(range)) = (&dep.registry, &dep.range) {
        if !refresh && previous.is_some_and(|p| p.source == *dep) {
            previous
                .unwrap()
                .resolved_source
                .clone()
                .unwrap_or_else(|| dep.clone())
        } else {
            let versions = available_versions(name, dep)?;
            let req = VersionRange::parse(range)?;
            let version =
                choose_version(versions.into_iter().filter(|v| req.matches(v)).collect())?;
            let mut selected = registry_dependency(name, registry, Some(&format!("={version}")))?;
            selected.range = dep.range.clone();
            selected.optional = dep.optional;
            selected
        }
    } else {
        dep.clone()
    };
    let old = previous.filter(|p| p.source == *dep).map(|p| {
        let mut p = p.clone();
        p.source = selected.clone();
        p
    });
    let mut result = resolve_selected(
        project,
        parent,
        name,
        cache_name,
        &selected,
        old.as_ref(),
        refresh,
        locked,
    )?;
    result.source = dep.clone();
    result.resolved_source = if selected != *dep {
        Some(selected)
    } else {
        None
    };
    Ok(result)
}
