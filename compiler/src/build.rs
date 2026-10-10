use crate::codegen::{Generated, TYPES_HEADER};
use crate::program::{hash, module_name, Program};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub struct Options {
    pub output: PathBuf,
    pub cache: PathBuf,
    pub cc: Option<String>,
    pub ar: String,
    pub release: bool,
    pub native: bool,
    pub fast: bool,
    pub library: bool,
    pub freestanding: bool,
    pub jobs: usize,
    pub links: Vec<PathBuf>,
    pub cflags: Vec<String>,
    pub ldflags: Vec<String>,
    pub timings: bool,
}
pub fn entry_wrapper(program: &Program) -> Result<String, String> {
    Ok(format!(
        "\nint main(void) {{ return dev_script_{}(); }}\n",
        module_name(&program.modules[0].path)
    ))
}
pub fn write_generated(dir: &Path, modules: &[Generated]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("dev_types.h"), TYPES_HEADER).map_err(|e| e.to_string())?;
    for m in modules {
        std::fs::write(dir.join(format!("{}.h", m.name)), &m.header).map_err(|e| e.to_string())?;
        std::fs::write(dir.join(format!("{}.c", m.name)), &m.source).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub struct BuildResult {
    pub compiled: usize,
    pub cached: usize,
    pub linked: bool,
}
struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn build(options: &Options, modules: &[Generated]) -> Result<BuildResult, String> {
    if options.freestanding && !options.library {
        return Err(
            "freestanding builds need --lib; link the library with your platform startup code"
                .into(),
        );
    }
    let start = Instant::now();
    let mut flags: Vec<String> = if options.fast {
        vec!["-std=c11".into(), "-Wall".into()]
    } else {
        vec![
            "-std=c11".into(),
            "-fwrapv".into(),
            "-Wall".into(),
            "-Wextra".into(),
            "-Werror=implicit-function-declaration".into(),
        ]
    };
    if !options.fast {
        flags.push(if options.release { "-O3" } else { "-O0" }.into());
        if !options.release {
            flags.push("-gdwarf-4".into());
            flags.push("-fno-omit-frame-pointer".into());
        }
    }
    if options.native {
        flags.push("-march=native".into());
    }
    if options.freestanding {
        flags.push("-ffreestanding".into());
        flags.push("-fno-builtin".into());
    }
    flags.extend(options.cflags.clone());
    std::fs::create_dir_all(&options.cache).map_err(|e| format!("cache: {e}"))?;
    let cache = std::fs::canonicalize(&options.cache).map_err(|e| e.to_string())?;
    let work = cache.join(format!(
        "work-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&work).map_err(|e| e.to_string())?;
    let _scratch = Scratch(work.clone());
    let fast_cc = if options.fast {
        Some(fast_compiler(options.cc.as_deref())?)
    } else {
        None
    };
    let (cc, version) = compiler(fast_cc.as_deref().or(options.cc.as_deref()), &cache, &work)?;
    if cfg!(windows) && !options.fast && !options.release && version.contains("windows-msvc") {
        // link.exe truncates DWARF section names in PE images. LLD preserves them.
        flags.push("-fuse-ld=lld".into());
        flags.push("-Wl,/debug:dwarf".into());
    }
    if options.fast {
        if !version.contains("tcc version") {
            return Err("--fast requires a TinyCC backend; use --cc /path/to/tcc".into());
        }
        if let Some(base) = Path::new(&cc).parent().and_then(|p| p.parent()) {
            let support = base.join("lib/tcc");
            if support.is_dir() {
                flags.push(format!("-B{}", support.display()));
            }
        }
    }
    let identity = format!(
        "devc:{}:{}:{}:{cc}:{version}:{flags:?}:{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        if options.native {
            native_identity()
        } else {
            String::new()
        }
    );
    if modules.len() == 1 && !options.library {
        return build_single(
            options,
            modules,
            (&cc, &flags, &identity),
            &cache,
            &work,
            start,
        );
    }
    let mut objects = Vec::new();
    let mut pending = Vec::new();
    for (i, m) in modules.iter().enumerate() {
        let mut data = identity.clone();
        data.push_str(TYPES_HEADER);
        data.push_str(&m.source);
        for dep in &m.dependencies {
            data.push_str(&modules[*dep].header);
        }
        let key = hash(data.as_bytes());
        let object = cache.join(format!("{key:016x}.o"));
        let valid = std::fs::read(&object).ok().is_some_and(|bytes| {
            std::fs::read_to_string(object.with_extension("hash"))
                .ok()
                .is_some_and(|saved| saved == format!("{:016x}", hash(&bytes)))
        });
        if !valid || !options.cflags.is_empty() {
            pending.push((i, object.clone()));
        }
        objects.push(object);
    }
    let cached = objects.len() - pending.len();
    let compiled = pending.len();
    if compiled > 0 {
        write_generated(&work, modules)?;
    }
    let workers = options.jobs.max(1).min(pending.len());
    // Each worker has private temporary output; the final object is published only after success.
    let errors = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..workers {
            let pending = &pending;
            let cc = &cc;
            let flags = &flags;
            let work = &work;
            handles.push(scope.spawn(move || -> Result<(), String> {
                for (i, object) in pending.iter().skip(worker).step_by(workers) {
                    let source = work.join(format!("{}.c", modules[*i].name));
                    let temp = work.join(format!("output-{i}.o"));
                    run(Command::new(cc)
                        .args(flags)
                        .arg("-c")
                        .arg(&source)
                        .arg("-o")
                        .arg(&temp))?;
                    let bytes = std::fs::read(&temp).map_err(|e| e.to_string())?;
                    publish(&temp, object)?;
                    let manifest = work.join(format!("object-{i}.hash"));
                    std::fs::write(&manifest, format!("{:016x}", hash(&bytes)))
                        .map_err(|e| e.to_string())?;
                    publish(&manifest, &object.with_extension("hash"))?;
                }
                Ok(())
            }));
        }
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err("compiler worker panicked".into()))
            })
            .collect::<Vec<_>>()
    });
    for result in errors {
        result?;
    }
    let compile_time = start.elapsed();
    if let Some(parent) = options
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let output = absolute(&options.output)?;
    let mut link_data = format!(
        "{identity}:{:?}:{}:{}:{}",
        options.ldflags,
        options.library,
        options.ar,
        output.display()
    );
    for object in &objects {
        link_data.push_str(&format!(
            "{}:{}",
            object.display(),
            std::fs::read_to_string(object.with_extension("hash")).map_err(|e| e.to_string())?
        ));
    }
    let mut seen = HashSet::new();
    let mut has_c_source = false;
    for link in &options.links {
        let path = std::fs::canonicalize(link).map_err(|e| format!("{}: {e}", link.display()))?;
        if !seen.insert(path.clone()) {
            return Err(format!("duplicate --link input: {}", path.display()));
        }
        has_c_source |= matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("c" | "C" | "s" | "S")
        );
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        link_data.push_str(&format!("{}:{:016x}", path.display(), hash(&bytes)));
    }
    // Arbitrary link flags may refer to external libraries and linker scripts. Re-link them so
    // changes outside the Dev graph cannot silently produce a stale executable.
    let link_key = hash(link_data.as_bytes());
    let manifest = cache.join(format!(
        "link-{:016x}.txt",
        hash(output.to_string_lossy().as_bytes())
    ));
    let linked = has_c_source
        || !options.ldflags.is_empty()
        || !options.cflags.is_empty()
        || !valid_output(&output, &manifest, link_key);
    if linked {
        if options.library && !options.links.is_empty() {
            return Err(
                "--lib archives Dev modules; link external C objects in the consuming application"
                    .into(),
            );
        }
        let temporary = output.with_file_name(format!(
            ".devc-{}-{}",
            std::process::id(),
            output.file_name().unwrap().to_string_lossy()
        ));
        let status = if options.library {
            run(Command::new(&options.ar)
                .arg("rcs")
                .arg(&temporary)
                .args(&objects))
        } else {
            run(Command::new(&cc)
                .args(&flags)
                .args(&objects)
                .args(&options.links)
                .args(&options.ldflags)
                .arg("-o")
                .arg(&temporary))
        };
        if let Err(e) = status {
            let _ = std::fs::remove_file(&temporary);
            return Err(e);
        }
        let bytes = std::fs::read(&temporary).map_err(|e| e.to_string())?;
        publish(&temporary, &output)?;
        let saved = work.join("link.txt");
        std::fs::write(&saved, format!("{link_key:016x}:{:016x}", hash(&bytes)))
            .map_err(|e| e.to_string())?;
        publish(&saved, &manifest)?;
    }
    if options.timings {
        eprintln!(
            "native {:.3} ms; total build {:.3} ms; {compiled} compiled, {cached} cached; link {}",
            compile_time.as_secs_f64() * 1000.0,
            start.elapsed().as_secs_f64() * 1000.0,
            if linked { "yes" } else { "cached" }
        );
    }
    Ok(BuildResult {
        compiled,
        cached,
        linked,
    })
}

// A single source can be compiled and linked in one driver invocation. Cache the
// executable itself instead of launching the driver twice to retain an object.
fn build_single(
    options: &Options,
    modules: &[Generated],
    backend: (&str, &[String], &str),
    cache: &Path,
    work: &Path,
    start: Instant,
) -> Result<BuildResult, String> {
    let (cc, flags, identity) = backend;
    let module = &modules[0];
    let mut data = format!(
        "single:{identity}:{}:{}:{}:{:?}",
        TYPES_HEADER, module.header, module.source, options.ldflags
    );
    let mut external = !options.cflags.is_empty() || !options.ldflags.is_empty();
    let mut seen = HashSet::new();
    for input in &options.links {
        let path = std::fs::canonicalize(input).map_err(|e| format!("{}: {e}", input.display()))?;
        if !seen.insert(path.clone()) {
            return Err(format!("duplicate --link input: {}", input.display()));
        }
        external |= matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("c" | "C" | "s" | "S")
        );
        data.push_str(&format!(
            "{}:{:016x}",
            path.display(),
            hash(&std::fs::read(&path).map_err(|e| e.to_string())?)
        ));
    }
    let artifact = cache.join(format!("exe-{:016x}.bin", hash(data.as_bytes())));
    let saved_hash = artifact.with_extension("hash");
    let valid = !external
        && std::fs::read_to_string(&saved_hash)
            .ok()
            .is_some_and(|saved| {
                std::fs::read(&artifact)
                    .ok()
                    .is_some_and(|bytes| saved == format!("{:016x}", hash(&bytes)))
            });
    if !valid {
        write_generated(work, modules)?;
        let temporary = work.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        run(Command::new(cc)
            .args(flags)
            .arg(work.join(format!("{}.c", module.name)))
            .args(&options.links)
            .args(&options.ldflags)
            .arg("-o")
            .arg(&temporary))?;
        let bytes = std::fs::read(&temporary).map_err(|e| e.to_string())?;
        publish(&temporary, &artifact)?;
        let manifest = work.join("executable.hash");
        std::fs::write(&manifest, format!("{:016x}", hash(&bytes))).map_err(|e| e.to_string())?;
        publish(&manifest, &saved_hash)?;
    }
    let output = absolute(&options.output)?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = std::fs::read(&artifact).map_err(|e| e.to_string())?;
    if std::fs::read(&output)
        .ok()
        .is_none_or(|current| current != bytes)
    {
        let temporary = output.with_file_name(format!(
            ".devc-{}-{}",
            std::process::id(),
            output.file_name().unwrap().to_string_lossy()
        ));
        std::fs::copy(&artifact, &temporary).map_err(|e| e.to_string())?;
        publish(&temporary, &output)?;
    }
    let compiled = usize::from(!valid);
    if options.timings {
        eprintln!(
            "native {:.3} ms; {compiled} compiled, {} cached; single compiler invocation",
            start.elapsed().as_secs_f64() * 1000.0,
            usize::from(valid)
        );
    }
    Ok(BuildResult {
        compiled,
        cached: usize::from(valid),
        linked: !valid,
    })
}

fn fast_compiler(explicit: Option<&str>) -> Result<String, String> {
    if let Some(path) = explicit {
        return Ok(path.into());
    }
    if let Ok(path) = std::env::var("DEV_TCC") {
        return Ok(path);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let bundled = parent.join(if cfg!(windows) {
                "backend/bin/tcc.exe"
            } else {
                "backend/bin/tcc"
            });
            if bundled.is_file() {
                return Ok(bundled.to_string_lossy().into_owned());
            }
        }
    }
    Ok("tcc".into())
}

fn native_identity() -> String {
    #[cfg(target_os = "linux")]
    if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
        return info
            .lines()
            .filter(|line| {
                line.starts_with("model name")
                    || line.starts_with("flags")
                    || line.starts_with("Features")
            })
            .take(2)
            .collect::<Vec<_>>()
            .join(";");
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        return format!(
            "sse3={};ssse3={};sse4={};avx={};avx2={};avx512f={};avx512bw={};fma={};bmi2={}",
            std::is_x86_feature_detected!("sse3"),
            std::is_x86_feature_detected!("ssse3"),
            std::is_x86_feature_detected!("sse4.2"),
            std::is_x86_feature_detected!("avx"),
            std::is_x86_feature_detected!("avx2"),
            std::is_x86_feature_detected!("avx512f"),
            std::is_x86_feature_detected!("avx512bw"),
            std::is_x86_feature_detected!("fma"),
            std::is_x86_feature_detected!("bmi2")
        );
    }
    #[allow(unreachable_code)]
    std::env::consts::ARCH.into()
}
fn valid_output(output: &Path, manifest: &Path, key: u64) -> bool {
    let Ok(saved) = std::fs::read_to_string(manifest) else {
        return false;
    };
    let Ok(bytes) = std::fs::read(output) else {
        return false;
    };
    saved == format!("{key:016x}:{:016x}", hash(&bytes))
}
fn compiler(explicit: Option<&str>, cache: &Path, work: &Path) -> Result<(String, String), String> {
    let from_env = std::env::var("DEV_CC").ok();
    let selected = explicit.or(from_env.as_deref());
    let candidates: Vec<&str> = if let Some(s) = selected {
        vec![s]
    } else if cfg!(windows) {
        vec!["clang", "gcc"]
    } else {
        vec!["cc", "clang", "gcc"]
    };
    for candidate in candidates {
        let fingerprint = tool_fingerprint(candidate);
        let saved = fingerprint
            .as_ref()
            .map(|key| cache.join(format!("tool-{:016x}.txt", hash(key.as_bytes()))));
        if let Some(saved) = &saved {
            if let Ok(version) = std::fs::read_to_string(saved) {
                if !version.trim().is_empty() {
                    return Ok((
                        candidate.into(),
                        format!("{}\n{version}", fingerprint.as_ref().unwrap()),
                    ));
                }
            }
        }
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).into_owned();
                if let Some(saved) = saved {
                    let temp = work.join("tool.txt");
                    std::fs::write(&temp, &version).map_err(|e| e.to_string())?;
                    publish(&temp, &saved)?;
                }
                return Ok((
                    candidate.into(),
                    format!("{}\n{version}", fingerprint.unwrap_or_default()),
                ));
            }
        }
    }
    Err(
        "no C compiler found; install Clang/GCC or pass --cc <compiler> (DEV_CC is also supported)"
            .into(),
    )
}
// Stat the resolved compiler on every invocation, but spawn --version only after its
// path, size or modification time changes. Keep the invocation name (e.g. clang vs
// clang++) in the identity because one binary may implement several driver modes.
fn tool_fingerprint(candidate: &str) -> Option<String> {
    let input = Path::new(candidate);
    let mut paths = Vec::new();
    if input.is_absolute() || input.components().count() > 1 {
        paths.push(input.to_path_buf());
    } else {
        if cfg!(windows) {
            paths.push(std::env::current_dir().ok()?.join(input));
        }
        paths.extend(std::env::split_paths(&std::env::var_os("PATH")?).map(|dir| dir.join(input)));
    }
    for mut path in paths {
        if cfg!(windows) && path.extension().is_none() {
            path.set_extension("exe");
        }
        if let Ok(actual) = std::fs::canonicalize(path) {
            let metadata = std::fs::metadata(&actual).ok()?;
            if !metadata.is_file() {
                continue;
            }
            let modified = metadata
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_nanos();
            return Some(format!(
                "{candidate}:{}:{}:{modified}",
                actual.display(),
                metadata.len()
            ));
        }
    }
    None
}
fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|e| {
        format!(
            "cannot execute {}: {e}",
            command.get_program().to_string_lossy()
        )
    })?;
    if !output.status.success() {
        return Err(format!(
            "{} failed ({})\n{}{}",
            command.get_program().to_string_lossy(),
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
fn publish(from: &Path, to: &Path) -> Result<(), String> {
    // rename replaces atomically on Unix. Windows requires removing the existing destination.
    #[cfg(windows)]
    if to.exists() {
        std::fs::remove_file(to).map_err(|e| format!("{}: {e}", to.display()))?;
    }
    std::fs::rename(from, to).map_err(|e| format!("{}: {e}", to.display()))
}
pub fn absolute(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.into())
    } else {
        Ok(std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path))
    }
}
