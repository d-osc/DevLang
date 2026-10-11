const DEFAULT_STORE: &str = "https://d-osc.github.io/DevLang/packages.json";
#[derive(Deserialize)]
struct StoreCatalog { schema_version: u32, packages: Vec<StorePackage> }
#[derive(Deserialize)]
struct StorePackage { id: String, name: String, repository: String, #[serde(default)] description: String, #[serde(default)] subdir: Option<String>, versions: Vec<StoreVersion> }
#[derive(Deserialize)]
struct StoreVersion { version: String, tag: String, commit: String }
fn scoped_identity(text: &str) -> Result<&str, String> {
    let identity = text.strip_prefix('@').ok_or("scoped package needs @owner/repository")?;
    let parts:Vec<_>=identity.split('/').collect();
    if !(2..=3).contains(&parts.len()) || parts[..2].iter().any(|s|s.is_empty() || !s.bytes().all(|c|c.is_ascii_alphanumeric() || b"._-".contains(&c)) || *s=="." || *s=="..") || parts.get(2).is_some_and(|s|!identifier(s)) {return Err("invalid scoped package identity".into());}
    Ok(identity)
}
fn store_catalog(spec: &str) -> Result<StoreCatalog, String> {
    let data = download_bytes(spec, 32 * 1024 * 1024)?;
    let catalog: StoreCatalog = serde_json::from_slice(&data).map_err(|e| e.to_string())?;
    if catalog.schema_version != 1 || catalog.packages.len() > 20000 { return Err("unsupported Store catalog".into()); }
    let mut identities = BTreeSet::new();
    for p in &catalog.packages {
        scoped_identity(&format!("@{}",p.id))?;
        if p.id.split('/').count()==3 && (p.id.split('/').next_back()!=Some(p.name.as_str()) || p.subdir.is_none()){return Err("workspace Store identity requires matching namespace and subdir".into());}
        if !identifier(&p.name) || !identities.insert(&p.id) || p.repository != format!("https://github.com/{}",p.id.split('/').take(2).collect::<Vec<_>>().join("/")) { return Err("invalid or duplicate Store package identity".into()); }
        if let Some(path) = &p.subdir { relative(path)?; if path.contains('\\') { return Err("invalid Store subdirectory".into()); } }
        for v in &p.versions {
            if p.id.split('/').count()==3 && !v.tag.starts_with(&format!("{}/",p.name)){return Err("workspace Store releases require namespace tags".into());}
            let version = semver::Version::parse(&v.version).map_err(|e| e.to_string())?;
            if v.tag.strip_prefix(&format!("{}/",p.name)).unwrap_or(&v.tag).strip_prefix('v').unwrap_or(v.tag.strip_prefix(&format!("{}/",p.name)).unwrap_or(&v.tag)) != version.to_string() || ![40,64].contains(&v.commit.len()) || !v.commit.bytes().all(|c| c.is_ascii_hexdigit()) { return Err("invalid Store release".into()); }
        }
    }
    Ok(catalog)
}
fn store_select<'a>(catalog: &'a StoreCatalog, requested: &str) -> Result<&'a StorePackage, String> {
    let identity = requested.strip_prefix('@').unwrap_or(requested);
    if requested.starts_with('@') { scoped_identity(requested)?; }
    let candidates: Vec<_> = catalog.packages.iter().filter(|p| p.id.eq_ignore_ascii_case(identity) || p.name == requested || identity == format!("{}/{}",p.id.split('/').next().unwrap(),p.name)).collect();
    if candidates.len() != 1 { return Err(if candidates.is_empty() { "package not found in Store" } else { "ambiguous package name; use @owner/repository" }.into()); }
    Ok(candidates[0])
}
fn store_dependency(requested: &str, spec: &str, range: Option<&str>) -> Result<(String,Dependency), String> {
    let catalog=store_catalog(spec)?; let p=store_select(&catalog,requested)?;
    let req=peer_requirement(range.unwrap_or("*"))?;
    if !p.versions.iter().any(|v| req.matches(&semver::Version::parse(&v.version).unwrap())) { return Err(format!("no Store version satisfies {req}")); }
    Ok((p.name.clone(),Dependency {git:Some(format!("{}.git",p.repository)),version:Some(range.unwrap_or("*").to_owned()),package:Some(format!("@{}",p.id)),store:Some(spec.to_owned()),subdir:p.subdir.clone(),..Dependency::default()}))
}
fn package_search(args: &[String]) -> Result<i32,String> {
    if args.iter().any(|a| a=="--registry") { return registry_search(args); }
    let mut spec=DEFAULT_STORE;let mut query="";let mut at=1;
    while at<args.len() { if args[at]=="--store" { at+=1;spec=args.get(at).ok_or("--store needs URL")?; } else if query.is_empty() && !args[at].starts_with('-') { query=&args[at]; } else { return Err("pkg search [QUERY] [--store URL]".into()); } at+=1; }
    let query=query.to_lowercase();
    for p in store_catalog(spec)?.packages { if format!("{} {} {}",p.id,p.name,p.description).to_lowercase().contains(&query) { println!("@{} ({}) {}",p.id,p.name,p.description); } }
    Ok(0)
}
fn store_info(args: &[String]) -> Result<i32,String> {
    if args.len()!=2 && !(args.len()==4 && args[2]=="--store") { return Err("pkg info NAME [--store URL]".into()); }
    let c=store_catalog(args.get(3).map(String::as_str).unwrap_or(DEFAULT_STORE))?;let p=store_select(&c,&args[1])?;
    println!("@{}\nnamespace: {}\n{}\n{}",p.id,p.name,p.repository,p.description);
    for v in &p.versions { println!("{} {} {}",v.version,v.tag,v.commit); }
    Ok(0)
}
fn available_versions(name: &str, dep: &Dependency) -> Result<Vec<semver::Version>,String> {
    if let Some(spec)=&dep.store { let c=store_catalog(spec)?;let p=store_select(&c,dep.package.as_deref().ok_or("missing scoped identity")?)?;return Ok(p.versions.iter().map(|v|semver::Version::parse(&v.version).unwrap()).collect()); }
    if let Some(spec)=&dep.registry { let (_,c)=registry_index(spec)?;return Ok(c.packages.get(name).ok_or("package missing from registry")?.keys().map(|v|semver::Version::parse(v).unwrap()).collect()); }
    if let Some(remote)=&dep.git { let tags=git(None,&["ls-remote","--tags","--",remote])?;return Ok(tags.lines().filter_map(|line| line.split_once("refs/tags/")).filter_map(|(_,tag)|semver::Version::parse(tag.strip_prefix('v').unwrap_or(tag)).ok()).collect()); }
    Ok(Vec::new())
}
fn package_outdated(project:&Path,args:&[String])->Result<(),String> {
    if args.len()!=1 {return Err("pkg outdated".into());}
    let m=manifest(project)?;let lock:Lock=read(&lock_path(project))?;
    println!("PACKAGE\tCURRENT\tWANTED\tLATEST");
    for (name,dep) in root_dependencies(&m,false)? {
        let versions=available_versions(&name,&dep)?;
        if versions.is_empty() {continue;}
        let req=peer_requirement(dep.range.as_deref().or(dep.version.as_deref()).unwrap_or("*"))?;
        let current=if lock.version>=2 {lock.edges.get("root").and_then(|e|e.get(&name)).and_then(|id|lock.packages.get(id))}else{lock.packages.get(&name)};
        let current=current.and_then(|p|p.resolved_version.clone()).or_else(||dep.rev.as_ref().map(|v|v.trim_start_matches('v').into())).unwrap_or_else(||"unknown".into());
        let wanted=versions.iter().filter(|v|req.matches(v)).max().map(ToString::to_string).unwrap_or_else(||"none".into());
        let latest=versions.iter().filter(|v|v.pre.is_empty()).max().or_else(||versions.iter().max()).unwrap();
        if current!=wanted || current!=latest.to_string() {println!("{name}\t{current}\t{wanted}\t{latest}");}
    }
    Ok(())
}
fn package_upgrade(project:&Path,args:&[String])->Result<(),String> {
    let mut names=BTreeSet::new();let mut latest=false;let mut dry=false;
    for arg in &args[1..] { match arg.as_str(){"--latest"=>latest=true,"--dry-run"=>dry=true,s if identifier(s)=>{names.insert(s.to_owned());},_=>return Err("pkg upgrade [NAME...] [--latest] [--dry-run]".into())} }
    let mut m=manifest(project)?;
    for name in &names {if !m.dependencies.contains_key(name) && !m.dev_dependencies.contains_key(name){return Err(format!("unknown dependency {name}"));}}
    for (name,dep) in m.dependencies.iter_mut().chain(m.dev_dependencies.iter_mut()) {
        if !names.is_empty() && !names.contains(name){continue;}
        let versions=available_versions(name,dep)?;if versions.is_empty(){continue;}
        let mut range=dep.range.clone().or(dep.version.clone()).unwrap_or_else(||"*".into());
        if latest { if let Some(v)=versions.iter().filter(|v|v.pre.is_empty()).max(){range=format!("^{v}");} }
        if let Some(spec)=&dep.registry { *dep=registry_dependency(name,spec,Some(&range))?; }
        else if dep.git.is_some() { dep.version=Some(range);dep.rev=None;dep.branch=None; }
        println!("{name} {}",dep.range.as_deref().or(dep.version.as_deref()).unwrap_or("*"));
    }
    if !dry {
        let production=if lock_path(project).exists(){read::<Lock>(&lock_path(project))?.production}else{false};
        let path=manifest_path(project);let before=std::fs::read(&path).map_err(|e|e.to_string())?;write(&path,&m)?;
        if let Err(e)=install_mode(project,false,true,production){std::fs::write(path,before).map_err(|e|e.to_string())?;return Err(e);}
    }
    Ok(())
}
fn publish_github(project:&Path,args:&[String])->Result<(),String> {
    if args.len()<3{return Err("pkg publish --github OWNER/REPO [--dry-run] [--receipt FILE]".into());}
    let mut dry=false;let mut receipt=None;let mut at=3;
    while at<args.len(){match args[at].as_str(){"--dry-run" if !dry=>dry=true,"--receipt" if receipt.is_none()=>{at+=1;receipt=Some(PathBuf::from(args.get(at).ok_or("--receipt needs FILE")?));},_=>return Err("pkg publish --github OWNER/REPO [--dry-run] [--receipt FILE]".into())}at+=1;}
    scoped_identity(&format!("@{}",args[2]))?;
    if args[2].split('/').count()!=2{return Err("publication requires OWNER/REPO".into());}
    let m=manifest(project)?;let version=m.version.as_deref().ok_or("publish requires version")?.trim_start_matches('v');semver::Version::parse(version).map_err(|e|e.to_string())?;
    let tag=format!("v{version}");
    let stage=std::env::temp_dir().join(format!("devlang-publish-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    publish_package(project,&["publish".into(),"--registry".into(),stage.to_string_lossy().into()])?;
    let archive=stage.join("packages").join(&m.package.name).join(format!("{version}.tar.gz"));
    let bytes=std::fs::read(&archive).map_err(|e|e.to_string())?;
    let asset=stage.join(format!("{}-{version}.tar.gz",m.package.name));std::fs::rename(&archive,&asset).map_err(|e|e.to_string())?;
    let digest=format!("{:x}",Sha256::digest(bytes));
    std::fs::write(stage.join("sha256.txt"),format!("{digest}  {}\n",asset.file_name().unwrap().to_string_lossy())).map_err(|e|e.to_string())?;
    println!("prepared {} for {}/{}",asset.display(),args[2],tag);
    let save_receipt=||->Result<(),String>{if let Some(path)=&receipt{if let Some(parent)=path.parent().filter(|p|!p.as_os_str().is_empty()){std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}if std::fs::symlink_metadata(path).is_ok_and(|m|!m.is_file() || m.file_type().is_symlink()){return Err("publication receipt must be a regular file".into());}std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"schema_version":1,"repository":args[2],"tag":tag,"asset":asset,"sha256":digest,"checksums":stage.join("sha256.txt"),"uploaded":!dry})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;}Ok(())};
    if dry {return save_receipt();}
    if !git(Some(project), &["status", "--porcelain"])?.is_empty() { return Err("GitHub publication requires a clean committed source tree".into()); }
    let head = git(Some(project), &["rev-parse", "HEAD"])?;
    if git(Some(project), &["rev-parse", "--verify", &format!("{tag}^{{commit}}")])? != head { return Err("release tag must identify the current source commit".into()); }
    let remote = format!("https://github.com/{}.git", args[2]);
    let refs = git(None, &["ls-remote", "--tags", "--", &remote, &format!("refs/tags/{tag}"), &format!("refs/tags/{tag}^{{}}")])?;
    let remote_commit = refs.lines().find(|line|line.ends_with("^{}" )).or_else(||refs.lines().next()).and_then(|line|line.split_once('\t')).map(|(commit,_)|commit);
    if remote_commit != Some(head.as_str()) { return Err("push the release tag to the selected GitHub repository before publishing".into()); }
    // gh authenticates through its own credential store; no token is put in arguments or files.
    let status=Command::new("gh").args(["release","create",&tag,"--repo",&args[2],"--verify-tag","--title",&tag,"--generate-notes"]).arg(&asset).arg(stage.join("sha256.txt")).status().map_err(|e|format!("GitHub publication requires gh CLI: {e}"))?;
    if !status.success(){return Err("GitHub release publication failed; prepared assets are retained".into());}
    save_receipt()?;
    Ok(())
}

fn check_compatibility(m:&Manifest)->Result<(),String>{
    platform_check(m)?;
    if let Some(req)=&m.devlang { if !peer_requirement(req)?.matches(&semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap()) { return Err(format!("package '{}' requires DevLang CLI {req}; current CLI is {}",m.package.name,env!("CARGO_PKG_VERSION"))); } }
    Ok(())
}

fn store_release(dep:&Dependency)->Result<(String,semver::Version,String),String>{
    let catalog=store_catalog(dep.store.as_deref().ok_or("missing Store URL")?)?;
    let p=store_select(&catalog,dep.package.as_deref().ok_or("missing Store identity")?)?;
    if dep.git.as_deref()!=Some(format!("{}.git",p.repository).as_str()) || dep.subdir!=p.subdir {return Err("Store identity/source differs from catalog".into());}
    let req=requirement(dep)?.unwrap_or(VersionRange::parse("*")?);
    let selected=choose_version(p.versions.iter().map(|v|semver::Version::parse(&v.version).unwrap()).filter(|v|req.matches(v)).collect())?;
    let v=p.versions.iter().find(|v|semver::Version::parse(&v.version).unwrap()==selected).ok_or("no Store release satisfies requirement")?;
    Ok((v.tag.clone(),semver::Version::parse(&v.version).unwrap(),v.commit.clone()))
}
