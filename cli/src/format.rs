use dev_syntax::{
    lexer::{lex, Kind},
    parser,
};
use std::path::{Path, PathBuf};

// Preserve tokens and line boundaries; never reconstruct strings or comments.
pub fn source(text: &str) -> Result<String, String> {
    parser::parse(PathBuf::from("<format>"), text)?;
    let tokens = lex(text).map_err(|(_, e)| e)?;
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    let mut output = String::new();
    for (line_index, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        let in_string = quoted;
        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if quoted {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    quoted = false;
                }
            } else if ch == '#' || (ch == '/' && chars.peek() == Some(&'/')) {
                break;
            } else if ch == '"' {
                quoted = true;
            }
        }
        let line_tokens: Vec<_> = tokens
            .iter()
            .filter(|t| t.span.line == line_index + 1)
            .collect();
        let leading = line_tokens
            .iter()
            .take_while(
                |t| matches!(&t.kind, Kind::Symbol(s) if ["}", "]", ")"].contains(&s.as_str())),
            )
            .count();
        if in_string || quoted {
            output.push_str(line);
        } else if !line.trim().is_empty() {
            output.push_str(&"    ".repeat(depth.saturating_sub(leading)));
            output.push_str(line.trim());
        }
        output.push('\n');
        for token in line_tokens {
            if let Kind::Symbol(s) = &token.kind {
                match s.as_str() {
                    "{" | "[" | "(" => depth += 1,
                    "}" | "]" | ")" => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
        }
    }
    let kinds = |ts: Vec<dev_syntax::lexer::Token>| {
        let mut kinds = ts.into_iter().map(|t| t.kind).collect::<Vec<_>>();
        kinds.pop(); // EOF
        while kinds.last() == Some(&Kind::Newline) {
            kinds.pop();
        }
        kinds
    };
    if kinds(tokens) != kinds(lex(&output).map_err(|(_, e)| e)?) {
        return Err("formatting would change tokens; file left unchanged".into());
    }
    parser::parse(PathBuf::from("<format>"), &output)?;
    Ok(output)
}

fn collect(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        if path.extension().is_some_and(|e| e == "dev") {
            files.push(path.to_owned());
        } else {
            return Err(format!("expected .dev file: {}", path.display()));
        }
    } else {
        for item in std::fs::read_dir(path).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            if item.file_type().map_err(|e| e.to_string())?.is_symlink() {
                continue;
            }
            let p = item.path();
            if p.is_dir() {
                if ![".git", ".dev", "target", "dist", "node_modules", "out"]
                    .contains(&item.file_name().to_string_lossy().as_ref())
                {
                    collect(&p, files)?;
                }
            } else if p.extension().is_some_and(|e| e == "dev") {
                files.push(p);
            }
        }
    }
    Ok(())
}
pub fn command(args: &[String]) -> Result<i32, String> {
    let check = args.iter().any(|a| a == "--check");
    let stdout = args.iter().any(|a| a == "--stdout");
    let mut files = Vec::new();
    for arg in args {
        if arg == "--check" || arg == "--stdout" {
            continue;
        }
        if arg.starts_with('-') {
            return Err(format!("unknown fmt option {arg}"));
        }
        collect(Path::new(arg), &mut files)?;
    }
    if files.is_empty() {
        if args.iter().any(|a| !a.starts_with('-')) {
            return Err("no .dev files found".into());
        }
        let here = std::env::current_dir().map_err(|e| e.to_string())?;
        let folder = if let Some(root) = crate::packages::root(&here) {
            root.join(crate::packages::manifest(&root)?.package.modules)
        } else {
            here.join("src")
        };
        collect(&folder, &mut files)?;
    }
    files.sort();
    files.dedup();
    if stdout && (check || files.len() != 1) {
        return Err("--stdout requires one file and cannot combine with --check".into());
    }
    // Validate every input before writing any file.
    let changes = files
        .iter()
        .map(|p| {
            let old = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
            let new = source(&old).map_err(|e| format!("{}: {e}", p.display()))?;
            Ok((p, old, new))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut changed = false;
    for (p, old, new) in changes {
        if stdout {
            print!("{new}");
        } else if old != new {
            changed = true;
            if check {
                eprintln!("needs formatting: {}", p.display());
            } else {
                std::fs::write(p, new).map_err(|e| e.to_string())?;
                println!("formatted {}", p.display());
            }
        }
    }
    Ok(if check && changed { 1 } else { 0 })
}
