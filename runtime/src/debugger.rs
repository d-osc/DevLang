// Source debugger retains summaries only, never managed values or raw memory.
struct RuntimeDebugger {
    names: Vec<String>,
    locations: Vec<String>,
    breaks: Vec<(String, usize)>,
    stop_depth: Option<usize>,
    quit: bool,
}
fn debug_statement_span(s: &Stmt) -> Option<Span> {
    match s {
        Stmt::Let { span, .. }
        | Stmt::Return(_, span)
        | Stmt::Break(span)
        | Stmt::Continue(span) => Some(*span),
        Stmt::Assign { target, .. } => Some(target.span),
        Stmt::Expr(e) => Some(e.span),
        Stmt::If { cond, .. } | Stmt::While { cond, .. } => Some(cond.span),
        Stmt::Match { value, .. } => Some(value.span),
        _ => None,
    }
}
fn debug_value(v: &Value) -> String {
    match v {
        Value::Int(..) | Value::Float(..) | Value::Bool(..) | Value::Void => v.display(),
        Value::Str(s) => format!(
            "{:?}{}",
            s.chars().take(160).collect::<String>(),
            if s.chars().count() > 160 { "…" } else { "" }
        ),
        _ => format!("<{:?}>", v.ty()).chars().take(240).collect(),
    }
}
impl Engine {
    pub fn enable_debugger(&mut self) {
        self.debugger = Some(RuntimeDebugger {
            names: vec!["<entry>".into()],
            locations: Vec::new(),
            breaks: Vec::new(),
            stop_depth: Some(usize::MAX),
            quit: false,
        });
    }
    pub fn debugger_quit(&self) -> bool {
        self.debugger.as_ref().is_some_and(|d| d.quit)
    }
    fn debug_stop(&mut self, f: &Frame, span: Span) -> Result<(), String> {
        use std::io::Write;
        let Some(d) = &mut self.debugger else {
            return Ok(());
        };
        let depth = d.names.len() - 1;
        d.locations.resize(depth + 1, String::new());
        d.locations[depth] = format!("{}:{}:{}", f.module, span.line, span.col);
        if !d.stop_depth.is_some_and(|limit| depth <= limit)
            && !d
                .breaks
                .iter()
                .any(|(file, line)| file == &f.module && *line == span.line)
        {
            return Ok(());
        }
        eprintln!("paused {} ({})", d.locations[depth], d.names[depth]);
        loop {
            eprint!("(devdbg) ");
            std::io::stderr().flush().map_err(|e| e.to_string())?;
            let mut input = String::new();
            if std::io::stdin()
                .read_line(&mut input)
                .map_err(|e| e.to_string())?
                == 0
            {
                d.quit = true;
                return Err("debugger input closed".into());
            }
            let (command, arg) = input.trim().split_once(' ').unwrap_or((input.trim(), ""));
            match command {
                "s" | "step" => { d.stop_depth = Some(usize::MAX); return Ok(()); }
                "n" | "next" => { d.stop_depth = Some(depth); return Ok(()); }
                "finish" if depth > 0 => { d.stop_depth = Some(depth - 1); return Ok(()); }
                "c" | "continue" => { d.stop_depth = None; return Ok(()); }
                "q" | "quit" => { d.quit = true; return Err("debugger stopped".into()); }
                "locals" => {
                    let mut vars = std::collections::BTreeMap::new();
                    for scope in &f.scopes { for (name, value) in scope { vars.insert(name, debug_value(value)); } }
                    for (name, value) in vars { eprintln!("{name} = {value}"); }
                }
                "stack" => { for i in (0..=depth).rev() { eprintln!("#{} {} {}", depth-i, d.names[i], d.locations.get(i).map(String::as_str).unwrap_or("")); } }
                "break" | "b" => {
                    let (file, line) = arg.rsplit_once(':').unwrap_or((&f.module, arg));
                    if let Ok(line) = line.parse::<usize>() {
                        if line == 0 { eprintln!("line must be positive"); continue; }
                        let file = std::path::Path::new(file).canonicalize().map(|p| p.to_string_lossy().into_owned()).unwrap_or_else(|_| file.to_string());
                        if !self.modules.contains_key(&file) { eprintln!("unknown loaded source file"); continue; }
                        d.breaks.push((file, line)); eprintln!("breakpoint added");
                    } else { eprintln!("usage: break LINE or break FILE:LINE"); }
                }
                "clear" => { d.breaks.clear(); eprintln!("breakpoints cleared"); }
                "breakpoints" => { for (file, line) in &d.breaks { eprintln!("{file}:{line}"); } }
                _ => eprintln!("step/s, next/n, finish, continue/c, locals, stack, break/b LINE|FILE:LINE, breakpoints, clear, quit/q"),
            }
        }
    }
}
