#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct GlobalBin { project: String }
fn global_home() -> Result<PathBuf, String> {
    let home = if let Some(home) = std::env::var_os("DEVLANG_HOME") { PathBuf::from(home) }
    else {
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok_or("cannot locate user home; set DEVLANG_HOME")?;
        PathBuf::from(home).join(".devlang")
    };
    if !home.is_absolute() { return Err("DEVLANG_HOME must be absolute".into()); }
    Ok(home)
}
const GLOBAL_MARKER: &str = "@rem DevLang global bin\n";
const GLOBAL_SH_MARKER: &str = "#!/bin/sh\n# DevLang global bin\n";
fn global_launcher(name: &str, project: &str) -> Result<String, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let executable = executable.to_str().ok_or("non-UTF8 d path")?;
    if executable.contains(['\r', '\n']) || project.contains(['\r', '\n']) { return Err("unsupported newline in launcher path".into()); }
    if cfg!(windows) {
        Ok(format!("{GLOBAL_MARKER}@setlocal DisableDelayedExpansion\n@\"{}\" -C \"{}\" exec \"{name}\" -- %*\n@exit /b %errorlevel%\n", executable.replace('%', "%%"), project.replace('%', "%%")))
    } else {
        let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
        Ok(format!("{GLOBAL_SH_MARKER}exec {} -C {} exec {} -- \"$@\"\n", quote(executable), quote(project), quote(name)))
    }
}
fn global_filename(name: &str) -> String { if cfg!(windows) { format!("{name}.cmd") } else { name.to_owned() } }
fn global_owned(path: &Path) -> Result<bool, String> {
    if std::fs::symlink_metadata(path).map_err(|e| e.to_string())?.file_type().is_symlink() { return Ok(false); }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(bytes.starts_with(GLOBAL_MARKER.as_bytes()) || bytes.starts_with(GLOBAL_SH_MARKER.as_bytes()))
}
fn global_registry(home: &Path) -> Result<BTreeMap<String, GlobalBin>, String> {
    let path = home.join("bins.don");
    let items: BTreeMap<String, GlobalBin> = if path.exists() { read(&path)? } else { BTreeMap::new() };
    if items.keys().any(|name| !bin_name(name)) { return Err("invalid global bin registry".into()); }
    Ok(items)
}
fn register_global_path(directory: &Path) -> Result<(), String> {
    // Isolated tests and callers managing PATH themselves may opt out.
    if std::env::var_os("DEVLANG_NO_PATH_UPDATE").is_some_and(|v| v == "1") { return Ok(()); }
    #[cfg(windows)] {
        let script = r#"$dir = $env:DEVLANG_BIN_DIR; $old = [Environment]::GetEnvironmentVariable('Path', 'User'); $found = $false; foreach ($entry in ($old -split ';')) { if ($entry.TrimEnd('\') -ieq $dir.TrimEnd('\')) { $found = $true } }; if (-not $found) { if ([string]::IsNullOrEmpty($old)) { $next = $dir } else { $next = $old.TrimEnd(';') + ';' + $dir }; [Environment]::SetEnvironmentVariable('Path', $next, 'User') }"#;
        let status = Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", script]).env("DEVLANG_BIN_DIR", directory).status().map_err(|e| e.to_string())?;
        if !status.success() { return Err("global bins installed but updating user PATH failed".into()); }
    }
    #[cfg(unix)] {
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("cannot locate shell home")?);
        let shell = std::env::var("SHELL").unwrap_or_default();
        let file = home.join(if shell.ends_with("zsh") { ".zshrc" } else if shell.ends_with("bash") { ".bashrc" } else { ".profile" });
        let dir = directory.to_str().ok_or("non-UTF8 global bin path")?;
        if dir.contains(['\r', '\n']) { return Err("unsupported newline in global bin path".into()); }
        let line = format!("export PATH='{}':\"$PATH\"", dir.replace('\'', "'\\''"));
        let existing = if file.exists() { std::fs::read_to_string(&file).map_err(|e| e.to_string())? } else { String::new() };
        if !existing.lines().any(|l| l == line) {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(file).map_err(|e| e.to_string())?;
            writeln!(file, "\n# DevLang global commands\n{line}").map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn install_global_bins(project: &Path) -> Result<(), String> {
    mappings(project)?;
    let lock: Lock = read(&lock_path(project))?;
    let bins = package_bins(project, &lock)?;
    let home = global_home()?;
    let directory = home.join("bin");
    if directory.exists() && !dunce::canonicalize(&directory).map_err(|e| e.to_string())?.starts_with(dunce::canonicalize(&home).map_err(|e| e.to_string())?) { return Err("global bin directory cannot escape DEVLANG_HOME".into()); }
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    if !dunce::canonicalize(&directory).map_err(|e| e.to_string())?.starts_with(dunce::canonicalize(&home).map_err(|e| e.to_string())?) { return Err("global bin directory cannot escape DEVLANG_HOME".into()); }
    let project = dunce::canonicalize(project).map_err(|e| e.to_string())?.to_string_lossy().into_owned();
    let mut registry = global_registry(&home)?;
    for name in bins.keys() {
        if registry.iter().any(|(key, owner)| key.eq_ignore_ascii_case(name) && (key != name || owner.project != project)) { return Err(format!("global command '{name}' is owned by another package")); }
        let path = directory.join(global_filename(name));
        if path.exists() && (!registry.contains_key(name) || !global_owned(&path)?) { return Err(format!("refusing to overwrite global command '{name}'")); }
        global_launcher(name, &project)?;
    }
    let stale = registry.iter().filter(|(name, owner)| owner.project == project && !bins.contains_key(*name)).map(|(name, _)| name.clone()).collect::<Vec<_>>();
    for name in stale {
        let path = directory.join(global_filename(&name));
        if path.exists() && global_owned(&path)? { std::fs::remove_file(path).map_err(|e| e.to_string())?; }
        registry.remove(&name);
    }
    for name in bins.keys() {
        let path = directory.join(global_filename(name));
        std::fs::write(&path, global_launcher(name, &project)?).map_err(|e| e.to_string())?;
        #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?; }
        registry.insert(name.clone(), GlobalBin { project: project.clone() });
    }
    write(&home.join("bins.don"), &registry)?;
    if !bins.is_empty() { register_global_path(&directory)?; }
    println!("global commands installed in {}; reopen your terminal application after the first PATH update", directory.display());
    Ok(())
}
fn uninstall_global_bins(project: &Path) -> Result<(), String> {
    let home = global_home()?;
    if !home.exists() { return Ok(()); }
    let project = dunce::canonicalize(project).map_err(|e| e.to_string())?.to_string_lossy().into_owned();
    let mut registry = global_registry(&home)?;
    let names = registry.iter().filter(|(_, owner)| owner.project == project).map(|(name, _)| name.clone()).collect::<Vec<_>>();
    let directory = home.join("bin");
    if directory.exists() && !dunce::canonicalize(&directory).map_err(|e| e.to_string())?.starts_with(dunce::canonicalize(&home).map_err(|e| e.to_string())?) { return Err("global bin directory cannot escape DEVLANG_HOME".into()); }
    for name in names {
        let path = directory.join(global_filename(&name));
        if path.exists() && global_owned(&path)? { std::fs::remove_file(path).map_err(|e| e.to_string())?; }
        registry.remove(&name);
    }
    write(&home.join("bins.don"), &registry)?;
    println!("removed this project's global commands");
    Ok(())
}
