use std::{env, ffi::OsString, process::Command};
mod debug;
mod don;
mod format;
mod lsp;
mod packages;
mod unused;

const HELP: &str = "Dev Lang

  d FILE.dev [-- arguments]                 Run source with the runtime
  d run|--run|-r FILE.dev [-- arguments]     Run source with the runtime
  d build|--build|-b FILE.dev [options]      Compile a native program
  d compiler|--compiler|-c FILE.dev [options]  Alias for build
  d -e|--eval 'print(40 + 2)'               Run inline source
  d check|--check FILE.dev [options]        Check a native program
  d emit|--emit FILE.dev [options]          Emit C source
  d new NAME                              Create a project with package.don
  d exec NAME [-- arguments]              Run a local or installed package bin
  d pkg bin                              List available bin commands
  d pkg install -g|--global               Install bins on the user PATH
  d pkg uninstall -g|--global [NAME]      Remove global bins by project/name
  d pkg list --global                    List installed global packages
  d pkg add NAME --peer --version REQ     Require a consumer-provided version
  d pkg add|install|update|remove|list|workspace  Manage path/Git/workspace dependencies
  d don check|fmt|fmt-source|to-json|from-json FILE   Validate or convert data (output to stdout)
  d fmt [--check|--stdout] [FILES/DIRS]     Format source (defaults to src/)
  d lsp [--stdio]                         Start the language server
  d debug FILE.dev [--no-launch]           Build with symbols and launch LLDB
  d debug --vscode                        Generate VS Code debug configuration
  d run|build|check                       Use package.don/dev.toml entry when FILE is omitted

Common options (before --):
  -h, --help       Show help, including after a command or filename
  -v, -V, --version  Show d version
  -C, --cwd DIR    Run from a working directory
  --package NAME   Select a named workspace package (changes working directory)
  --timings        Print runtime or compiler timings to stderr
  --ffi-lib PATH   Load a native shared library in runtime mode (repeatable)
  --engine MODE    Runtime: auto (default) or ast
  -- ARGUMENTS     Forward program arguments in runtime mode

Compiler options (after FILE.dev):
  -o, --output PATH  Set executable/archive/generated-source output
  --debug          Use -O0 with DWARF symbols (Windows MSVC Clang needs LLD)
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
    let mut selected_package = None;
    let mut timings = false;
    let mut at = 0;
    while at < raw.len() {
        if args.first().is_some_and(|a| a == "exec") && args.len() >= 2 {
            args.extend_from_slice(&raw[at..]);
            break;
        }
        match raw[at].to_str().unwrap_or("") {
            "--" => {
                args.extend_from_slice(&raw[at..]);
                break;
            }
            "--help" | "-h" => {
                println!("{HELP}");
                return Ok(0);
            }
            "--version"
                if args.first().is_some_and(|a| a == "pkg")
                    && args.get(1).is_some_and(|a| a == "add") =>
            {
                args.push(raw[at].clone());
                at += 1;
                args.push(raw.get(at).ok_or("--version needs a requirement")?.clone());
            }
            "--package" => {
                if selected_package.is_some() {
                    return Err("duplicate --package".into());
                }
                at += 1;
                selected_package = Some(
                    raw.get(at)
                        .and_then(|s| s.to_str())
                        .ok_or("--package needs a UTF-8 name")?
                        .to_owned(),
                );
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
    if let Some(dir) = &cwd {
        env::set_current_dir(dir)
            .map_err(|e| format!("working directory {}: {e}", dir.display()))?;
    }
    if let Some(name) = selected_package {
        let selected =
            packages::select_project(&env::current_dir().map_err(|e| e.to_string())?, &name)?;
        env::set_current_dir(selected).map_err(|e| e.to_string())?;
    }
    if matches!(selector, "new" | "pkg" | "fmt" | "lsp" | "debug" | "don" | "exec") {
        let values = args
            .iter()
            .skip(1)
            .map(|a| {
                a.to_str()
                    .map(str::to_owned)
                    .ok_or("tooling arguments must be UTF-8")
            })
            .collect::<Result<Vec<_>, _>>()?;
        return match selector {
            "new" if values.len() == 1 => packages::new(std::path::Path::new(&values[0])),
            "new" => Err("usage: d new NAME".into()),
            "pkg" => packages::command(&values),
            "exec" => packages::exec_bin(&values),
            "don" => don::command(&values),
            "fmt" => format::command(&values),
            "lsp" => lsp::command(&values),
            "debug" => debug::command(&values),
            _ => unreachable!(),
        };
    }
    if matches!(selector, "--help" | "-h" | "help") {
        println!("{HELP}");
        return Ok(0);
    }
    if matches!(selector, "--version" | "-V") {
        println!("d {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    let inline_mode = matches!(selector, "-e" | "--eval");
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
        if args
            .first()
            .is_none_or(|a| a.to_string_lossy().starts_with('-') && a != "--eval" && a != "-e")
        {
            let here = env::current_dir().map_err(|e| e.to_string())?;
            let project = packages::root(&here)
                .ok_or("expected a .dev entry file or package.don/dev.toml project")?;
            args.insert(
                0,
                project
                    .join(packages::manifest(&project)?.package.entry)
                    .into_os_string(),
            );
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
    if !inline_mode {
        let entry_index = usize::from(command.is_some());
        if let Some(entry) = args.get(entry_index) {
            let entry = std::path::PathBuf::from(entry);
            if let Ok(entry) = entry.canonicalize() {
                if let Some(project) = packages::root(entry.parent().unwrap()) {
                    let boundary = args.iter().position(|a| a == "--").unwrap_or(args.len());
                    let mut maps = Vec::<OsString>::new();
                    for (name, path) in packages::mappings(&project)? {
                        maps.push("--module-dir".into());
                        maps.push(format!("{name}={}", path.display()).into());
                    }
                    args.splice(boundary..boundary, maps);
                }
            }
        }
    }
    child.args(args);
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
