// Version-2 locks isolate dependency namespaces by the importing package directory.
fn graph_identity(project:&Path,parent:&Path,name:&str,dep:&Dependency)->Result<String,String>{
    let mut normalized=dep.clone();
    if let Some(path)=&dep.path {let absolute=dunce::canonicalize(parent.join(path)).map_err(|e|e.to_string())?;normalized.path=Some(pathdiff::diff_paths(absolute,dunce::canonicalize(project).map_err(|e|e.to_string())?).ok_or("cannot normalize package path")?.to_string_lossy().replace('\\',"/"));}
    if dep.workspace {normalized.path=Some(pathdiff::diff_paths(workspace_dependency(parent,name)?,dunce::canonicalize(project).map_err(|e|e.to_string())?).ok_or("cannot normalize workspace path")?.to_string_lossy().replace('\\',"/"));normalized.workspace=false;}
    let bytes=serde_json::to_vec(&normalized).map_err(|e|e.to_string())?;
    Ok(format!("{name}__{:x}",Sha256::digest(bytes)))
}
fn install_mode(project:&Path,locked:bool,refresh:bool,production:bool)->Result<(),String>{
    if lock_path(project).exists() && read::<Lock>(&lock_path(project))?.version==2 {return graph_install(project,locked,refresh,production);}
    match legacy_install_mode(project,locked,refresh,production){
        Err(e) if !locked && (e.starts_with("conflicting dependency namespace") || e.starts_with("conflicting local dependency") || e.starts_with("conflicting workspace dependency")) => graph_install(project,false,refresh,production),
        result=>result,
    }
}
fn graph_install(project:&Path,locked:bool,refresh:bool,production:bool)->Result<(),String>{
    let previous:Lock=if lock_path(project).exists(){read(&lock_path(project))?}else{Lock::default()};
    if locked && (previous.version!=2 || previous.production!=production){return Err("--locked needs a matching version-2 lock".into());}
    let root=manifest(project)?;check_compatibility(&root)?;
    let mut result=Lock{version:2,production,packages:BTreeMap::new(),edges:BTreeMap::new()};
    let mut pending=vec![("root".to_owned(),project.to_owned(),root_dependencies(&root,production)?)];
    while let Some((context,parent,deps))=pending.pop(){
        let mut edges=BTreeMap::new();
        for (name,dep) in deps {
            if !identifier(&name){return Err("invalid dependency namespace".into());}
            let id=graph_identity(project,&parent,&name,&dep)?;
            edges.insert(name.clone(),id.clone());
            if let Some(item)=result.packages.get_mut(&id) {
                if item.source!=dep && !item.additional_sources.contains(&dep) { item.additional_sources.push(dep); item.additional_sources.sort(); }
                continue;
            }
            if result.packages.len()>=4096{return Err("dependency graph exceeds 4096 packages".into());}
            let old=previous.packages.get(&id);
            if locked && old.is_none_or(|p|p.source!=dep){return Err("dependency graph differs from package-lock.don".into());}
            let item=resolve_named(project,&parent,&name,&id,&dep,old,refresh,locked)?;
            if locked && old.map(|old| {let mut old=old.clone();old.additional_sources.clear();old})!=Some(item.clone()){return Err("dependency content changed from package-lock.don".into());}
            let at=locked_root(project,&id,&item)?;
            let child=manifest(&at)?;check_compatibility(&child)?;
            pending.push((id.clone(),at,child.dependencies));
            result.packages.insert(id,item);
        }
        result.edges.insert(context,edges);
    }
    graph_verify_peers(project,&result)?;
    if locked && previous!=result {return Err("dependency graph differs from package-lock.don".into());}
    let bins=package_bins(project,&result)?;install_bins(project,&bins)?;
    if !locked{write(&project.join("package-lock.don"),&result)?;}
    println!("resolved {} packages with isolated namespaces",result.packages.len());
    Ok(())
}
fn graph_verify_peers(project:&Path,lock:&Lock)->Result<(),String>{
    let root=manifest(project)?;
    let root_edges=lock.edges.get("root").ok_or("lock is missing root dependency edges")?;
    let mut root_versions=BTreeMap::from([(root.package.name.clone(),root.version.clone())]);
    for (name,id) in root_edges {let item=lock.packages.get(id).ok_or("missing graph node")?;root_versions.insert(name.clone(),manifest(&locked_root(project,id,item)?)?.version);}
    for (context,m) in std::iter::once(("root".to_owned(),root)).chain(lock.packages.iter().map(|(id,item)|Ok((id.clone(),manifest(&locked_root(project,id,item)?)?))).collect::<Result<Vec<_>,String>>()?) {
        let mut versions=root_versions.clone();
        for (name,id) in lock.edges.get(&context).ok_or("missing dependency edges")? {let item=lock.packages.get(id).ok_or("missing graph node")?;versions.insert(name.clone(),manifest(&locked_root(project,id,item)?)?.version);}
        for (name,req) in &m.peer_dependencies {let version=versions.get(name).and_then(|v|v.as_deref()).ok_or_else(||format!("package '{}' requires peer '{name}' {req}; declare it in the consumer dependencies",m.package.name))?;let v=semver::Version::parse(version.trim_start_matches('v')).map_err(|e|e.to_string())?;if !peer_requirement(req)?.matches(&v){return Err(format!("peer '{name}' version {v} does not satisfy {req}"));}}
    }
    Ok(())
}
fn scoped_mapping(alias:&str,at:&Path)->String {
    let bytes=dunce::canonicalize(at).unwrap_or_else(|_|at.to_owned()).to_string_lossy().as_bytes().iter().map(|byte|format!("{byte:02x}")).collect::<String>();
    format!("{alias}@{bytes}")
}
pub fn mappings(project:&Path)->Result<Vec<(String,PathBuf)>,String>{
    if !lock_path(project).exists() || read::<Lock>(&lock_path(project))?.version!=2{return legacy_mappings(project);}
    let lock:Lock=read(&lock_path(project))?;
    let m=manifest(project)?;check_compatibility(&m)?;
    let mut pending=vec![("root".to_owned(),project.to_owned(),root_dependencies(&m,lock.production)?)];
    let mut visited=BTreeSet::new();let mut contexts=BTreeSet::new();let mut dirs=BTreeMap::new();
    let mut declarations=BTreeMap::<String,BTreeSet<Dependency>>::new();
    while let Some((context,parent,deps))=pending.pop(){
        if !contexts.insert(context.clone()){continue;}
        let edges=lock.edges.get(&context).ok_or("lock is missing package dependency edges")?;
        let expected: BTreeMap<_,_>=deps.iter().map(|(name,dep)|Ok((name.clone(),graph_identity(project,&parent,name,dep)?))).collect::<Result<_,String>>()?;
        if *edges!=expected{return Err("dependency graph changed; run d pkg install".into());}
        // Root providers can satisfy peers, while explicit child dependencies take precedence.
        for (name,id) in lock.edges.get("root").ok_or("missing root edges")? {let item=lock.packages.get(id).ok_or("missing graph node")?;dirs.insert(scoped_mapping(name,&parent),locked_root(project,id,item)?.join(relative(&item.modules)?));}
        for (name,id) in edges {
            let item=lock.packages.get(id).ok_or("lock is missing graph node")?;
            let dep=deps.get(name).unwrap();validate_dependency(dep)?;
            if item.source!=*dep && !item.additional_sources.contains(dep){return Err("dependency source changed; run d pkg install".into());}
            declarations.entry(id.clone()).or_default().insert(dep.clone());
            let at=locked_root(project,id,item)?;
            dirs.insert(scoped_mapping(name,&parent),at.join(relative(&item.modules)?));
            if !visited.insert(id.clone()){continue;}
            if !tree_matches(&at,&item.sha256)?{return Err("dependency content changed; run d pkg install".into());}
            if let Some(commit)=&item.commit {if git(Some(&at),&["rev-parse","HEAD"])?!=*commit{return Err("dependency checkout changed".into());}}
            let child=manifest(&at)?;check_compatibility(&child)?;
            if child.package.modules!=item.modules{return Err("locked module directory changed".into());}
            if let Some(req)=requirement(dep)?{if item.resolved_version.as_deref()!=Some(package_version(&child,&req)?.to_string().as_str()){return Err("locked package version changed".into());}}
            pending.push((id.clone(),at,child.dependencies));
        }
        if !edges.contains_key(&m.package.name){dirs.insert(scoped_mapping(&m.package.name,&parent),project.join(relative(&m.package.modules)?));}
    }
    if visited.len()!=lock.packages.len() || contexts.len()!=lock.edges.len(){return Err("lock contains stale graph nodes or edges".into());}
    for (id,item) in &lock.packages {
        let expected:BTreeSet<_>=std::iter::once(item.source.clone()).chain(item.additional_sources.iter().cloned()).collect();
        if declarations.get(id)!=Some(&expected){return Err("lock contains stale source requirements".into());}
    }
    graph_verify_peers(project,&lock)?;
    Ok(dirs.into_iter().collect())
}
