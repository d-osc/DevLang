use serde_json::json;
use std::{path::Path, process::Command};

pub fn command(args: &[String]) -> Result<i32, String> {
    if args == ["--vscode"] {
        return vscode();
    }
    let file=args.first().filter(|s|!s.starts_with('-')).ok_or("d debug FILE.dev [--debugger lldb|gdb] [--no-launch] [-- program arguments]; d debug --vscode")?;
    let mut debugger = "lldb";
    let mut no_launch = false;
    let mut batch = false;
    let mut commands = Vec::new();
    let mut build_args = Vec::new();
    let mut program_args = Vec::new();
    let mut at = 1;
    while at < args.len() {
        match args[at].as_str() {
            "--" => {
                program_args.extend_from_slice(&args[at + 1..]);
                break;
            }
            "--debugger" => {
                at += 1;
                debugger = args.get(at).ok_or("--debugger needs lldb or gdb")?;
                if !["lldb", "gdb"].contains(&debugger) {
                    return Err("--debugger needs lldb or gdb".into());
                }
            }
            "--no-launch" => no_launch = true,
            "--batch" => batch = true,
            "--command" => {
                at += 1;
                commands.push(
                    args.get(at)
                        .ok_or("--command needs a debugger command")?
                        .clone(),
                );
            }
            "--cc" | "--module-dir" | "--link" | "--cflag" | "--ldflag" => {
                build_args.push(args[at].clone());
                at += 1;
                build_args.push(args.get(at).ok_or("build option needs value")?.clone());
            }
            _ => return Err(format!("unsupported debug option {}", args[at])),
        }
        at += 1;
    }
    let output = Path::new("out/debug").join(if cfg!(windows) { "app.exe" } else { "app" });
    std::fs::create_dir_all(output.parent().unwrap()).map_err(|e| e.to_string())?;
    let status = Command::new(std::env::current_exe().map_err(|e| e.to_string())?)
        .args(["build", file, "--debug", "-o"])
        .arg(&output)
        .args(build_args)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Ok(status.code().unwrap_or(1));
    }
    let output = dunce::canonicalize(output).map_err(|e| e.to_string())?;
    if no_launch {
        println!("{}", output.display());
        return Ok(0);
    }
    let mut child = Command::new(debugger);
    if batch {
        child.arg("--batch");
    }
    for command in commands {
        child
            .arg(if debugger == "gdb" { "-ex" } else { "-o" })
            .arg(command);
    }
    let status = child
        .arg(if debugger == "gdb" { "--args" } else { "--" })
        .arg(output)
        .args(program_args)
        .status()
        .map_err(|e| format!("cannot launch {debugger}: {e}; install it or use --no-launch"))?;
    Ok(status.code().unwrap_or(1))
}
fn vscode() -> Result<i32, String> {
    let dir = Path::new(".vscode");
    if dir.join("tasks.json").exists() || dir.join("launch.json").exists() {
        return Err(".vscode/tasks.json or launch.json already exists; merge the documented templates manually".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tasks = json!({"version":"2.0.0","tasks":[{"label":"DevLang: debug build","type":"process","command":"d","args":["debug","${file}","--no-launch"],"options":{"cwd":"${workspaceFolder}"},"problemMatcher":[]} ]});
    let launch = json!({"version":"0.2.0","configurations":[{"name":"DevLang native (LLDB)","type":"lldb","request":"launch","program":"${workspaceFolder}/out/debug/app","windows":{"program":"${workspaceFolder}/out/debug/app.exe"},"cwd":"${workspaceFolder}","args":[],"preLaunchTask":"DevLang: debug build"}]});
    for (name, value) in [("tasks.json", tasks), ("launch.json", launch)] {
        std::fs::write(
            dir.join(name),
            serde_json::to_string_pretty(&value).unwrap() + "\n",
        )
        .map_err(|e| e.to_string())?;
    }
    println!("created native debug configuration; install CodeLLDB (vadimcn.vscode-lldb) and DevLang language support for .dev breakpoints");
    Ok(0)
}
