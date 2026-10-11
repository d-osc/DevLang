fn graph_scopes(
    project: &Path,
    lock: &Lock,
) -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
    let mut scopes: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut pending = vec![("root".to_owned(), BTreeMap::new())];
    while let Some((context, mut inherited)) = pending.pop() {
        let own = lock
            .edges
            .get(&context)
            .ok_or("dependency graph changed; missing package edges")?;
        inherited.extend(own.clone());
        if let Some(previous) = scopes.get(&context) {
            if previous != &inherited {
                if let Some(item) = lock.packages.get(&context) {
                    let m = manifest(&locked_root(project, &context, item)?)?;
                    if m.peer_dependencies
                        .keys()
                        .any(|name| previous.get(name) != inherited.get(name))
                    {
                        return Err(format!("package {} has incompatible ancestor peer scopes; use distinct dependency declarations",m.package.name));
                    }
                }
                continue;
            }
            continue;
        }
        scopes.insert(context.clone(), inherited.clone());
        for child in own.values() {
            pending.push((child.clone(), inherited.clone()));
        }
        if scopes.len() > 4097 {
            return Err("peer scope exceeds graph budget".into());
        }
    }
    Ok(scopes)
}
