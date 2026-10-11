// Version-2 locks isolate dependency namespaces by the importing package directory.
fn graph_identity(project:&Path,parent:&Path,name:&str,dep:&Dependency)->Result<String,String>{
    let mut normalized=dep.clone();
    if let Some(path)=&dep.path {let absolute=dunce::canonicalize(parent.join(path)).or_else(|e|if dep.optional{Ok(parent.join(path))}else{Err(e)}).map_err(|e|e.to_string())?;normalized.path=Some(pathdiff::diff_paths(absolute,dunce::canonicalize(project).map_err(|e|e.to_string())?).ok_or("cannot normalize package path")?.to_string_lossy().replace('\\',"/"));}
    if dep.workspace {normalized.path=Some(pathdiff::diff_paths(workspace_dependency(parent,name)?,dunce::canonicalize(project).map_err(|e|e.to_string())?).ok_or("cannot normalize workspace path")?.to_string_lossy().replace('\\',"/"));normalized.workspace=false;}
    let bytes=serde_json::to_vec(&normalized).map_err(|e|e.to_string())?;
    Ok(format!("{name}__{:x}",Sha256::digest(bytes)))
}
fn install_attempt(project:&Path,locked:bool,refresh:bool,production:bool)->Result<(),String>{
    if std::env::var_os("DEVLANG_DEDUPE").is_none() && lock_path(project).exists() && read::<Lock>(&lock_path(project))?.version>=2 {return graph_install(project,locked,refresh,production);}
    match legacy_install_mode(project,locked,refresh,production){
        Err(e) if !locked && (e.starts_with("conflicting dependency namespace") || e.starts_with("conflicting local dependency") || e.starts_with("conflicting workspace dependency")) => graph_install(project,false,refresh,production),
        result=>result,
    }
}
fn graph_install(project:&Path,locked:bool,refresh:bool,production:bool)->Result<(),String>{
    let previous_lock:Lock=if lock_path(project).exists(){read(&lock_path(project))?}else{Lock::default()};
    if locked && (!matches!(previous_lock.version,2|3) || previous_lock.production!=production){return Err("--locked needs a matching graph lock".into());}
    let previous=if previous_lock.version==3 { source_lock(project,&previous_lock)? } else { previous_lock.clone() };
    let root=manifest(project)?;check_compatibility(&root)?;
    let mut result=Lock{version:2,production,packages:BTreeMap::new(),edges:BTreeMap::new(),omitted:BTreeMap::new()};
    let mut pending=vec![("root".to_owned(),project.to_owned(),root_dependencies(&root,production)?)];
    while let Some((context,parent,deps))=pending.pop(){
        let mut edges=BTreeMap::new();
        for (name,dep) in effective_dependencies(project,&parent,deps)? {
            if !identifier(&name){return Err("invalid dependency namespace".into());}
            let id=graph_identity(project,&parent,&name,&dep)?;
            if locked && previous.omitted.get(&id)==Some(&dep) && dep.optional {result.omitted.insert(id,dep);continue;}
            if let Some(item)=result.packages.get_mut(&id) {
                edges.insert(name.clone(),id.clone());
                if item.source!=dep && !item.additional_sources.contains(&dep) { item.additional_sources.push(dep); item.additional_sources.sort(); }
                continue;
            }
            if result.packages.len()>=4096{return Err("dependency graph exceeds 4096 packages".into());}
            let old=previous.packages.get(&id);
            if locked && old.is_none_or(|p|p.source!=dep){return Err("dependency graph differs from package-lock.don".into());}
            let item=match resolve_named(project,&parent,&name,&id,&dep,old,refresh,locked){Ok(item)=>item,Err(error) if dep.optional && !locked=>{println!("optional {name} omitted: {error}");result.omitted.insert(id,dep);continue;},Err(error)=>return Err(error)};
            edges.insert(name.clone(),id.clone());
            if locked && old.map(|old| {let mut old=old.clone();old.additional_sources.clear();old})!=Some(item.clone()){return Err("dependency content changed from package-lock.don".into());}
            let at=locked_root(project,&id,&item)?;
            let child=manifest(&at)?;check_compatibility(&child)?;
            pending.push((id.clone(),at,child_dependencies(&child)?));
            result.packages.insert(id,item);
        }
        result.edges.insert(context,edges);
    }
    if previous_lock.version==3 || graph_scopes(project,&result).is_err_and(|e|e.contains("incompatible ancestor peer scopes")) { result=context_lock(project,&result,true)?; }
    graph_verify_peers(project,&result)?;
    if locked && previous_lock!=result {return Err("dependency graph differs from package-lock.don".into());}
    let bins=package_bins(project,&result)?;install_bins(project,&bins)?;
    if !locked{write(&project.join("package-lock.don"),&result)?;}
    println!("resolved {} packages with isolated namespaces",result.packages.len());
    Ok(())
}
fn graph_verify_peers(project:&Path,lock:&Lock)->Result<(),String>{
    let root=manifest(project)?;
    let scopes=graph_scopes(project,lock)?;
    let root_edges=lock.edges.get("root").ok_or("lock is missing root dependency edges")?;
    let mut root_versions=BTreeMap::from([(root.package.name.clone(),root.version.clone())]);
    for (name,id) in root_edges {let item=lock.packages.get(id).ok_or("missing graph node")?;root_versions.insert(name.clone(),manifest(&locked_root(project,id,item)?)?.version);}
    for (context,m) in std::iter::once(("root".to_owned(),root)).chain(lock.packages.iter().map(|(id,item)|Ok((id.clone(),manifest(&locked_root(project,id,item)?)?))).collect::<Result<Vec<_>,String>>()?) {
        let mut versions=root_versions.clone();
        for (name,id) in scopes.get(&context).ok_or("missing dependency scope")? {let item=lock.packages.get(id).ok_or("missing graph node")?;versions.insert(name.clone(),manifest(&locked_root(project,id,item)?)?.version);}
        for (name,req) in &m.peer_dependencies {let version=versions.get(name).and_then(|v|v.as_deref()).ok_or_else(||format!("package '{}' requires peer '{name}' {req}; declare it in the consumer dependencies",m.package.name))?;let v=semver::Version::parse(version.trim_start_matches('v')).map_err(|e|e.to_string())?;if !peer_requirement(req)?.matches(&v){return Err(format!("peer '{name}' version {v} does not satisfy {req}"));}}
    }
    Ok(())
}
fn scoped_mapping(alias:&str,at:&Path)->String {
    let bytes=dunce::canonicalize(at).unwrap_or_else(|_|at.to_owned()).to_string_lossy().as_bytes().iter().map(|byte|format!("{byte:02x}")).collect::<String>();
    format!("{alias}@{bytes}")
}
pub fn mappings(project:&Path)->Result<Vec<(String,PathBuf)>,String>{
    if !lock_path(project).exists(){return legacy_mappings(project);}
    let lock:Lock=read(&lock_path(project))?;
    match lock.version { 1=>legacy_mappings(project), 2=>graph_mappings(project,&lock,true), 3=>context_mappings(project,&lock), _=>Err("unsupported package lock version".into()) }
}
fn graph_mappings(project:&Path,lock:&Lock,verify_peers:bool)->Result<Vec<(String,PathBuf)>,String>{
    let m=manifest(project)?;check_compatibility(&m)?;
    let scopes=if verify_peers {graph_scopes(project,lock)?} else {lock.edges.clone()};
    let mut pending=vec![("root".to_owned(),project.to_owned(),root_dependencies(&m,lock.production)?)];
    let mut visited=BTreeSet::new();let mut contexts=BTreeSet::new();let mut dirs=BTreeMap::new();let mut omitted=BTreeMap::new();
    let mut declarations=BTreeMap::<String,BTreeSet<Dependency>>::new();
    while let Some((context,parent,deps))=pending.pop(){
        if !contexts.insert(context.clone()){continue;}
        let deps=effective_dependencies(project,&parent,deps)?;
        for (name,dep) in &deps {let id=graph_identity(project,&parent,name,dep)?;if dep.optional && lock.omitted.get(&id)==Some(dep){omitted.insert(id,dep.clone());}}
        let edges=lock.edges.get(&context).ok_or("lock is missing package dependency edges")?;
        let expected: BTreeMap<_,_>=deps.iter().filter_map(|(name,dep)|{let id=match graph_identity(project,&parent,name,dep){Ok(id)=>id,Err(e)=>return Some(Err(e))}; if dep.optional && lock.omitted.get(&id)==Some(dep){None}else{Some(Ok((name.clone(),id)))}}).collect::<Result<_,String>>()?;
        if *edges!=expected{return Err("dependency graph changed; run d pkg install".into());}
        // Root providers can satisfy peers, while explicit child dependencies take precedence.
        for (name,id) in scopes.get(&context).ok_or("missing package scope")? {let item=lock.packages.get(id).ok_or("missing graph node")?;dirs.insert(scoped_mapping(name,&parent),locked_root(project,id,item)?.join(relative(&item.modules)?));}
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
            pending.push((id.clone(),at,child_dependencies(&child)?));
        }
        if !edges.contains_key(&m.package.name){dirs.insert(scoped_mapping(&m.package.name,&parent),project.join(relative(&m.package.modules)?));}
    }
    if visited.len()!=lock.packages.len() || contexts.len()!=lock.edges.len() || omitted!=lock.omitted || omitted.keys().any(|id|lock.packages.contains_key(id)){return Err("lock contains stale graph nodes, edges or omissions".into());}
    for (id,item) in &lock.packages {
        let expected:BTreeSet<_>=std::iter::once(item.source.clone()).chain(item.additional_sources.iter().cloned()).collect();
        if declarations.get(id)!=Some(&expected){return Err("lock contains stale source requirements".into());}
    }
    if verify_peers {graph_verify_peers(project,lock)?;}
    Ok(dirs.into_iter().collect())
}
