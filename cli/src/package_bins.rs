fn bin_name(name: &str) -> bool {
    let mut chars = name.chars();
    let valid = chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    valid && !["con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8", "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9"].contains(&name.to_ascii_lowercase().as_str())
}
fn package_bins(project: &Path, lock: &Lock) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut roots = vec![project.to_owned()];
    for (name, item) in &lock.packages { roots.push(locked_root(project, name, item)?); }
    let mut bins = BTreeMap::new();
    let mut case_names = BTreeSet::new();
    for root in roots {
        let root = dunce::canonicalize(root).map_err(|e| e.to_string())?;
        for (name, source) in manifest(&root)?.bin {
            let source = dunce::canonicalize(root.join(source)).map_err(|e| format!("bin {name}: {e}"))?;
            if !source.starts_with(&root) || !source.is_file() { return Err("bin source must be a file inside its package".into()); }
            if !case_names.insert(name.to_lowercase()) { return Err(format!("conflicting bin command {name}")); }
            bins.insert(name, source);
        }
    }
    Ok(bins)
}
const BIN_MARKER: &str = "@rem DevLang generated bin\n";
const BIN_SH_MARKER: &str = "#!/bin/sh\n# DevLang generated bin\n";
fn install_bins(project: &Path, bins: &BTreeMap<String, PathBuf>) -> Result<(), String> {
    let directory = project.join(".dev/bin");
    if bins.is_empty() && !directory.exists() { return Ok(()); }
    let root = dunce::canonicalize(project).map_err(|e| e.to_string())?;
    for existing in [project.join(".dev"), directory.clone()] {
        if existing.exists() && !dunce::canonicalize(&existing).map_err(|e| e.to_string())?.starts_with(&root) {
            return Err("bin directory cannot escape project".into());
        }
    }
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    if !dunce::canonicalize(&directory).map_err(|e| e.to_string())?.starts_with(&root) { return Err("bin directory cannot escape project".into()); }
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let executable = executable.to_str().ok_or("non-UTF8 d path")?;
    let project = root.to_str().ok_or("non-UTF8 project path")?;
    if executable.contains(['\n', '\r']) || project.contains(['\n', '\r']) { return Err("unsupported newline in launcher path".into()); }
    let mut contents = BTreeMap::new();
    for name in bins.keys() {
        let (filename, text) = if cfg!(windows) {
            (format!("{name}.cmd"), format!("{BIN_MARKER}@setlocal DisableDelayedExpansion\n@\"{}\" -C \"{}\" exec \"{name}\" -- %*\n@exit /b %errorlevel%\n", executable.replace('%', "%%"), project.replace('%', "%%")))
        } else {
            let quote = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
            (name.clone(), format!("{BIN_SH_MARKER}exec {} -C {} exec {} -- \"$@\"\n", quote(executable), quote(project), quote(name)))
        };
        contents.insert(filename, text);
    }
    // Check every destination before replacing any existing launcher.
    for filename in contents.keys() {
        let path = directory.join(filename);
        if path.exists() && !owned_bin(&path)? { return Err(format!("refusing to overwrite custom launcher {}", path.display())); }
    }
    for item in std::fs::read_dir(&directory).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        if item.file_type().map_err(|e| e.to_string())?.is_file()
            && !contents.contains_key(&item.file_name().to_string_lossy().into_owned()) && owned_bin(&item.path())? {
            std::fs::remove_file(item.path()).map_err(|e| e.to_string())?;
        }
    }
    for (filename, text) in contents {
        let path = directory.join(filename);
        std::fs::write(&path, text).map_err(|e| e.to_string())?;
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn owned_bin(path: &Path) -> Result<bool, String> {
    if std::fs::symlink_metadata(path).map_err(|e| e.to_string())?.file_type().is_symlink() { return Ok(false); }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(bytes.starts_with(BIN_MARKER.as_bytes()) || bytes.starts_with(BIN_SH_MARKER.as_bytes()))
}
pub fn exec_bin(args: &[String]) -> Result<i32, String> {
    let name = args.first().ok_or("usage: d exec NAME [-- arguments]")?;
    let project = root(&std::env::current_dir().map_err(|e| e.to_string())?).ok_or("no package project; use -C DIR")?;
    let dirs = mappings(&project)?;
    let lock: Lock = if lock_path(&project).exists() { read(&lock_path(&project))? } else { Lock::default() };
    let bins = package_bins(&project, &lock)?;
    let entry = bins.get(name).ok_or_else(|| format!("unknown bin '{name}'; run d pkg bin"))?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let runner = executable.parent().unwrap().join(if cfg!(windows) { "devrun.exe" } else { "devrun" });
    let mut command = Command::new(runner);
    command.current_dir(&project).arg(entry);
    for (name, directory) in dirs { command.arg("--module-dir").arg(format!("{name}={}", directory.display())); }
    let arguments = args.get(1..).unwrap_or_default();
    let arguments = if arguments.first().is_some_and(|s| s == "--") { &arguments[1..] } else { arguments };
    let status = command.arg("--").args(arguments).status().map_err(|e| e.to_string())?;
    #[cfg(unix)] { use std::os::unix::process::ExitStatusExt; Ok(status.code().unwrap_or_else(|| 128 + status.signal().unwrap_or(1))) }
    #[cfg(not(unix))] { Ok(status.code().unwrap_or(1)) }
}
