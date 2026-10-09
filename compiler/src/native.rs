//! Automatic C dependency preparation, kept in the compiler executable.
use crate::program::hash;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

struct Work(PathBuf);
impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct CacheLock(PathBuf);
impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn cache_lock(path: PathBuf) -> Result<CacheLock, String> {
    let start = std::time::Instant::now();
    loop {
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => return Ok(CacheLock(path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if start.elapsed() > std::time::Duration::from_secs(30) {
                    return Err(format!("timed out waiting for native dependency cache lock {}; if its builder stopped, remove this stale lock", path.display()));
                }
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(e) => return Err(format!("native dependency cache lock: {e}")),
        }
    }
}

pub fn prepare(directory: &Path, symbols: &[String]) -> Result<PathBuf, String> {
    let directory = directory.canonicalize().map_err(|e| e.to_string())?;
    let mut files = Vec::new();
    fn headers(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for item in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let path = item.path();
            let ty = item.file_type().map_err(|e| e.to_string())?;
            if ty.is_symlink() {
                continue;
            }
            if ty.is_dir() {
                if ![".git", ".dev-cache", "target", "dist"]
                    .contains(&item.file_name().to_string_lossy().as_ref())
                {
                    headers(&path, files)?;
                }
            } else if path.extension().is_some_and(|e| e == "h") {
                files.push(path);
            }
            if files.len() > 512 {
                return Err("native dependency has more than 512 headers/sources".into());
            }
        }
        Ok(())
    }
    headers(&directory, &mut files)?;
    let mut sources = Vec::new();
    for item in std::fs::read_dir(&directory).map_err(|e| e.to_string())? {
        let path = item.map_err(|e| e.to_string())?.path();
        if path.is_file() && path.extension().is_some_and(|e| e == "c") {
            sources.push(path);
        }
    }
    sources.sort();
    if sources.is_empty() {
        return Err("no adjacent C source files".into());
    }
    files.extend(sources.clone());
    if files.len() > 512 {
        return Err("native dependency has more than 512 headers/sources".into());
    }
    files.sort();
    let selection = std::env::var("DEV_CC").unwrap_or_default();
    let mut fingerprint = format!(
        "native-c-v1:{}:{}:{selection}:{}:{symbols:?}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        directory.display()
    )
    .into_bytes();
    let mut c_text = String::new();
    for file in &files {
        let data = std::fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        if data.len() > 8 * 1024 * 1024 {
            return Err(format!("{} exceeds 8 MiB", file.display()));
        }
        fingerprint.extend_from_slice(file.as_os_str().to_string_lossy().as_bytes());
        fingerprint.extend_from_slice(&(data.len() as u64).to_le_bytes());
        fingerprint.extend_from_slice(&data);
        if file.extension().is_some_and(|e| e == "c") {
            c_text.push_str(&String::from_utf8_lossy(&data));
        }
    }
    let cache = directory.join(".dev-cache/native");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    let stem = format!("{:016x}", hash(&fingerprint));
    // Workers can resolve the same foreign dependency concurrently. Only one
    // builder may publish its DLL; Windows cannot replace a DLL already loaded.
    let _lock = cache_lock(cache.join(format!("{stem}.lock")))?;
    let output = cache.join(format!(
        "{stem}.{}",
        if cfg!(windows) {
            "dll"
        } else if cfg!(target_os = "macos") {
            "dylib"
        } else {
            "so"
        }
    ));
    let manifest = cache.join(format!("{stem}.hash"));
    if let (Ok(data), Ok(expected)) = (std::fs::read(&output), std::fs::read_to_string(&manifest)) {
        if expected == format!("{:016x}", hash(&data)) {
            return Ok(output);
        }
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let work = Work(cache.join(format!("work-{}-{nonce}", std::process::id())));
    std::fs::create_dir(&work.0).map_err(|e| e.to_string())?;
    let candidates = if !selection.is_empty() {
        vec![selection.as_str()]
    } else if cfg!(windows) {
        vec!["clang", "gcc"]
    } else {
        vec!["cc", "clang", "gcc"]
    };
    let cc=candidates.into_iter().find(|c|Command::new(c).arg("--version").output().is_ok_and(|r|r.status.success()))
        .ok_or("automatic C dependency preparation needs Clang/GCC on the first run; install a backend or set DEV_CC")?;
    let temporary = work.0.join(output.file_name().unwrap());
    let mut command = Command::new(cc);
    command
        .current_dir(&directory)
        .args(["-shared", "-O2"])
        // Relative paths avoid Windows verbatim paths confusing C include lookup.
        .args(
            sources
                .iter()
                .map(|path| path.strip_prefix(&directory).unwrap()),
        )
        .arg("-o")
        .arg(temporary.strip_prefix(&directory).unwrap());
    if !cfg!(windows) {
        command.arg("-fPIC");
    }
    if cfg!(target_os = "macos") {
        command.arg("-undefined").arg("dynamic_lookup");
    }
    if cfg!(windows) && cc.to_ascii_lowercase().contains("clang") {
        for symbol in symbols {
            if c_text
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .any(|word| word == symbol)
            {
                command.arg(format!("-Wl,/EXPORT:{symbol}"));
            }
        }
    }
    let result = command.output().map_err(|e| e.to_string())?;
    if !result.status.success() {
        return Err(format!(
            "automatic C dependency preparation failed:\n{}",
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    let data = std::fs::read(&temporary).map_err(|e| e.to_string())?;
    // The cache name is input-specific; never replace a successful different build.
    if output.exists() {
        std::fs::remove_file(&output).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&temporary, &output).map_err(|e| e.to_string())?;
    std::fs::write(manifest, format!("{:016x}", hash(&data))).map_err(|e| e.to_string())?;
    Ok(output)
}
