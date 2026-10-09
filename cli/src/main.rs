use std::{env, ffi::OsString, process::Command};

const HELP: &str = "Dev Lang

  d FILE.dev [-- arguments]                 Run source with the runtime
  d run|--run|-r FILE.dev [-- arguments]     Run source with the runtime
  d build|--build|-b FILE.dev [options]      Compile a native program
  d compiler|--compiler|-c FILE.dev [options]  Alias for build
  d -e|--eval 'print(40 + 2)'               Run inline source
  d check|--check FILE.dev [options]        Check a native program
  d emit|--emit FILE.dev [options]          Emit C source

Common options (before --):
  -h, --help       Show help, including after a command or filename
  -v, -V, --version  Show d version
  -C, --cwd DIR    Run from a working directory
  --timings        Print runtime or compiler timings to stderr
  --ffi-lib PATH   Load a native shared library in runtime mode (repeatable)
  --engine MODE    Runtime: auto (default) or ast
  -- ARGUMENTS     Forward program arguments in runtime mode

Compiler options (after FILE.dev):
  -o, --output PATH  Set executable/archive/generated-source output
  --debug          Use -O0 (default; last debug/release option wins)
  --release        Use -O3
  --native         Tune for this CPU
  --fast           Use TinyCC for fast builds
  --jobs N         Set compilation concurrency
  --cc PATH        Select a C compiler
  --module-dir NAME=DIR  Map an imported module namespace
  --link PATH      Link a C source/object/library (repeatable)
  --lib            Build a static library
  --freestanding   Disable hosted features
  --cache-dir DIR  Set the object cache directory

devrun and devc are separate executables next to d.
Build mode requires a C backend; runtime mode does not.";

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("d: {error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<i32, String> {
    let raw: Vec<OsString> = env::args_os().skip(1).collect();
    let mut args = Vec::new();
    let mut cwd = None;
    let mut timings = false;
    let mut at = 0;
    while at < raw.len() {
        match raw[at].to_str().unwrap_or("") {
            "--" => {
                args.extend_from_slice(&raw[at..]);
                break;
            }
            "--help" | "-h" => {
                println!("{HELP}");
                return Ok(0);
            }
            "--version" | "-V" | "-v" => {
                println!("d {}", env!("CARGO_PKG_VERSION"));
                return Ok(0);
            }
            "--cwd" | "-C" => {
                at += 1;
                cwd = Some(std::path::PathBuf::from(
                    raw.get(at).ok_or("--cwd/-C needs a directory")?,
                ));
            }
            "--timings" => timings = true,
            "-e" | "--eval" | "-o" | "--output" | "--cc" | "--ar" | "--link" | "--cflag"
            | "--ldflag" | "--module-dir" | "--jobs" | "--cache-dir" | "--ffi-lib" | "--engine" => {
                args.push(raw[at].clone());
                at += 1;
                args.push(raw.get(at).ok_or("option needs a value")?.clone());
            }
            _ => args.push(raw[at].clone()),
        }
        at += 1;
    }
    let Some(first) = args.first() else {
        println!("{HELP}");
        return Ok(0);
    };
    let selector = first.to_str().unwrap_or("");
    if matches!(selector, "--help" | "-h" | "help") {
        println!("{HELP}");
        return Ok(0);
    }
    if matches!(selector, "--version" | "-V") {
        println!("d {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    let (tool, command) = match selector {
        "run" | "--run" | "-r" => ("devrun", Some("run")),
        "build" | "--build" | "-b" | "compiler" | "--compiler" | "-c" => ("devc", Some("build")),
        "check" | "--check" => ("devc", Some("check")),
        "emit" | "--emit" => ("devc", Some("emit")),
        "-e" | "--eval" => ("devrun", None),
        s if s.starts_with('-') => return Err(format!("unknown option '{s}'\n{HELP}")),
        _ => ("devrun", None),
    };
    if let Some(command) = command {
        args.remove(0);
        if args.is_empty() {
            return Err("expected a .dev entry file".into());
        }
        args.insert(0, command.into());
    }
    if timings {
        let boundary = args.iter().position(|a| a == "--").unwrap_or(args.len());
        args.insert(boundary, "--timings".into());
    }
    let executable = env::current_exe().map_err(|e| e.to_string())?;
    let directory = executable
        .parent()
        .ok_or("cannot find executable directory")?;
    let binary = directory.join(if cfg!(windows) {
        format!("{tool}.exe")
    } else {
        tool.into()
    });
    let mut child = Command::new(&binary);
    child.args(args);
    if let Some(cwd) = cwd {
        if !cwd.is_dir() {
            return Err(format!(
                "working directory does not exist: {}",
                cwd.display()
            ));
        }
        child.current_dir(cwd);
    }
    let status = child.status().map_err(|e| {
        format!(
            "cannot start {}: {e}; place {tool} next to d",
            binary.display()
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        Ok(status
            .code()
            .unwrap_or_else(|| 128 + status.signal().unwrap_or(1)))
    }
    #[cfg(not(unix))]
    Ok(status.code().unwrap_or(1))
}
