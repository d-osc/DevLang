mod engine;
use std::path::PathBuf;
use std::time::Instant;

const HELP: &str = "devrun [run] FILE.dev [--timings] [-- arguments]
devrun -e|--eval 'print(40 + 2)' [--timings] [-- arguments]
  -h, --help       Show help
  -v, -V, --version  Show runtime version
  --timings        Print source-load and execution timings to stderr
Interprets Dev directly without a compiler or generated build artifacts.";

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
fn run() -> Result<i32, String> {
    let mut remaining = Vec::new();
    let mut timings = false;
    let mut raw = std::env::args().skip(1);
    while let Some(arg) = raw.next() {
        match arg.as_str() {
            "--" => {
                remaining.push(arg);
                remaining.extend(raw);
                break;
            }
            "--timings" => timings = true,
            "--help" | "-h" => {
                println!("{HELP}");
                return Ok(0);
            }
            "--version" | "-V" | "-v" => {
                println!("devrun {}", env!("CARGO_PKG_VERSION"));
                return Ok(0);
            }
            "-e" | "--eval" => {
                remaining.push(arg);
                remaining.push(raw.next().ok_or("--eval/-e needs code")?);
            }
            _ => remaining.push(arg),
        }
    }
    let mut args = remaining.into_iter();
    let Some(mut entry) = args.next() else {
        println!("{HELP}");
        return Ok(0);
    };
    if entry == "run" {
        entry = args.next().ok_or("run needs a .dev file")?;
    }
    let inline = entry == "-e" || entry == "--eval";
    let path = if inline {
        PathBuf::from("<eval>")
    } else {
        PathBuf::from(entry)
    };
    let source = if inline {
        Some(format!(
            "fn main() {{\n{}\n}}",
            args.next().ok_or("-e needs code")?
        ))
    } else {
        None
    };
    let mut program_args: Vec<String> = args.collect();
    if !program_args.is_empty() {
        if program_args[0] != "--" {
            return Err("program arguments must follow --".into());
        }
        program_args.remove(0);
    }
    let start = Instant::now();
    let mut engine = engine::Engine::load(&path, source.as_deref(), program_args)?;
    let loaded = start.elapsed();
    let executed = Instant::now();
    let result = engine.run();
    if timings {
        eprintln!(
            "runtime: load {:.3} ms, execute {:.3} ms",
            loaded.as_secs_f64() * 1000.0,
            executed.elapsed().as_secs_f64() * 1000.0
        );
    }
    result
}
