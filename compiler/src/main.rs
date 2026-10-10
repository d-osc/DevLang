#[cfg(test)]
use dev_syntax::lexer;
mod build;
mod native;
use dev_lang::{codegen, program};
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::time::Instant;

const HELP: &str = "Dev Lang 0.4.0 — small syntax, native code, C interoperability

  devc run   main.dev [options] [-- program arguments]
  devc build main.dev [options]
  devc check main.dev
  devc emit  main.dev -o generated [--lib] [--freestanding]

Options:
  -o, --output PATH  Executable, archive (--lib), or generated directory (emit)
  --debug          Use -O0 with DWARF symbols (default; Windows MSVC Clang needs LLD)
  --release        Optimize native code with -O3 (default: fast -O0 build)
  --native         Tune for this CPU (-march=native; binary may not run elsewhere)
  --module-dir NAME=DIR  Map a module namespace to a directory (repeatable)
  --fast           Use TinyCC for fast development builds (requires tcc)
  --cc PATH        C compiler, also configurable through DEV_CC
  --link PATH      Link a C source/object/archive (repeatable)
  --cflag FLAG     Additional compile flag (repeatable; also passed at link)
  --ldflag FLAG    Additional link flag (repeatable)
  --lib            Build a static C-compatible library (export fn for C symbols)
  --ar PATH        Archiver for --lib (default: ar, llvm-ar on Windows)
  --freestanding   Disable hosted features for device/library builds
  --jobs N         Parallel object compilation (default: CPU count, capped at 8)
  --cache-dir DIR  Object cache directory (default: .dev-cache next to entry)
  --timings        Print frontend/native build timings
  --version        Show compiler version
";

fn main() {
    match cli() {
        Ok(code) => std::process::exit(code),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}
fn cli() -> Result<i32, String> {
    let start = Instant::now();
    let mut args = std::env::args().skip(1);
    let Some(command) = args.next() else {
        print!("{HELP}");
        return Ok(0);
    };
    if command == "--help" || command == "-h" {
        print!("{HELP}");
        return Ok(0);
    }
    if command == "--version" {
        println!("devc {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    if command == "native-build" {
        let directory = PathBuf::from(args.next().ok_or("native-build needs a source directory")?);
        let mut symbols = Vec::new();
        while let Some(option) = args.next() {
            if option != "--symbol" {
                return Err(format!("unknown native-build option {option}"));
            }
            let symbol = args.next().ok_or("--symbol needs a C identifier")?;
            if symbol.is_empty()
                || !symbol.bytes().enumerate().all(|(i, b)| {
                    b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                })
            {
                return Err("invalid native symbol".into());
            }
            symbols.push(symbol);
        }
        symbols.sort();
        symbols.dedup();
        println!("{}", native::prepare(&directory, &symbols)?.display());
        return Ok(0);
    }
    if !["build", "run", "check", "emit"].contains(&command.as_str()) {
        return Err(format!("unknown command '{command}'\n{HELP}"));
    }
    let entry = PathBuf::from(args.next().ok_or("expected a .dev entry file")?);
    let stem = entry.file_stem().and_then(|s| s.to_str()).unwrap_or("app");
    let mut options = build::Options {
        output: PathBuf::from("out").join(if cfg!(windows) {
            format!("{stem}.exe")
        } else {
            stem.into()
        }),
        cache: entry
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(".dev-cache"),
        cc: None,
        ar: if cfg!(windows) { "llvm-ar" } else { "ar" }.into(),
        release: false,
        native: false,
        fast: false,
        library: false,
        freestanding: false,
        jobs: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .min(8),
        links: Vec::new(),
        cflags: Vec::new(),
        ldflags: Vec::new(),
        timings: false,
    };
    let mut explicit_output = false;
    let mut module_dirs = std::collections::HashMap::new();
    let mut run_args = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => {
                options.output = args.next().ok_or("-o needs a path")?.into();
                explicit_output = true;
            }
            "--release" => options.release = true,
            "--debug" => options.release = false,
            "--native" => options.native = true,
            "--module-dir" => {
                let value = args.next().ok_or("--module-dir needs NAME=DIR")?;
                let (name, path) = value.split_once('=').ok_or("--module-dir needs NAME=DIR")?;
                if name.is_empty()
                    || path.is_empty()
                    || !name.bytes().enumerate().all(|(i, b)| {
                        b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                    })
                {
                    return Err(
                        "--module-dir needs an identifier NAME and nonempty directory".into(),
                    );
                }
                if module_dirs
                    .insert(name.to_string(), PathBuf::from(path))
                    .is_some()
                {
                    return Err(format!("duplicate module namespace '{name}'"));
                }
            }
            "--fast" => options.fast = true,
            "--cc" => options.cc = Some(args.next().ok_or("--cc needs a compiler")?),
            "--ar" => options.ar = args.next().ok_or("--ar needs an archiver")?,
            "--link" => options
                .links
                .push(args.next().ok_or("--link needs a file")?.into()),
            "--cflag" => options
                .cflags
                .push(args.next().ok_or("--cflag needs a flag")?),
            "--ldflag" => options
                .ldflags
                .push(args.next().ok_or("--ldflag needs a flag")?),
            "--lib" => options.library = true,
            "--freestanding" => options.freestanding = true,
            "--cache-dir" => {
                options.cache = args.next().ok_or("--cache-dir needs a directory")?.into()
            }
            "--jobs" => {
                options.jobs = args
                    .next()
                    .ok_or("--jobs needs a number")?
                    .parse()
                    .map_err(|_| "invalid --jobs number")?;
                if options.jobs == 0 || options.jobs > 256 {
                    return Err("--jobs must be 1..256".into());
                }
            }
            "--timings" => options.timings = true,
            "--" if command == "run" => {
                run_args.extend(args);
                break;
            }
            "--help" | "-h" => {
                print!("{HELP}");
                return Ok(0);
            }
            _ => return Err(format!("unknown option '{arg}'")),
        }
    }
    if command == "run" && (options.library || options.freestanding) {
        return Err("run needs a hosted executable".into());
    }
    if options.fast
        && (options.release || options.native || options.freestanding || options.library)
    {
        return Err("--fast is for hosted development executables; use --release/--native or --lib for optimized/device builds".into());
    }
    if !explicit_output {
        if command == "emit" {
            options.output = PathBuf::from("out").join(format!("{stem}-c"));
        } else if options.library {
            options.output = PathBuf::from("out").join(format!("lib{stem}.a"));
        }
    }
    let program = if module_dirs.is_empty() {
        program::Program::load(&entry)?
    } else {
        program::Program::load_with_modules(&entry, &module_dirs)?
    };
    let mut modules = codegen::generate(&program, !options.freestanding)?;
    if !options.library && command != "check" {
        modules[0].source.push_str(&build::entry_wrapper(&program)?);
    }
    let frontend = start.elapsed();
    if options.timings {
        eprintln!(
            "frontend {:.3} ms; {} modules",
            frontend.as_secs_f64() * 1000.0,
            modules.len()
        );
    }
    match command.as_str() {
        "check" => {
            println!("checked {} modules", modules.len());
            Ok(0)
        }
        "emit" => {
            build::write_generated(&options.output, &modules)?;
            println!("generated C: {}", options.output.display());
            Ok(0)
        }
        _ => {
            let result = build::build(&options, &modules)?;
            eprintln!(
                "built {} ({} compiled, {} cached; link {})",
                options.output.display(),
                result.compiled,
                result.cached,
                if result.linked { "yes" } else { "cached" }
            );
            if options.timings {
                eprintln!("total {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
            }
            if command == "run" {
                let output = build::absolute(&options.output)?;
                let status = std::process::Command::new(&output)
                    .args(run_args)
                    .status()
                    .map_err(|e| format!("{}: {e}", output.display()))?;
                Ok(status.code().unwrap_or(1))
            } else {
                Ok(0)
            }
        }
    }
}
