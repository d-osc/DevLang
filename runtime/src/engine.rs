use dev_syntax::{ast::*, parser};
use std::{collections::HashMap, path::Path, time::Instant};

#[derive(Clone, Debug)]
enum Value {
    Int(i128, Type),
    Float(f64, Type),
    Bool(bool),
    Str(String),
    Array(Vec<Value>, Type),
    Void,
}
impl Value {
    fn ty(&self) -> Type {
        match self {
            Self::Int(_, t) | Self::Float(_, t) | Self::Array(_, t) => t.clone(),
            Self::Bool(_) => Type::Bool,
            Self::Str(_) => Type::Str,
            Self::Void => Type::Void,
        }
    }
    fn display(&self) -> String {
        match self {
            Self::Int(n, _) => n.to_string(),
            Self::Float(n, _) => n.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Str(s) => s.clone(),
            Self::Array(a, _) => format!("{:?}", a),
            Self::Void => String::new(),
        }
    }
    fn integer(&self) -> Result<i128, String> {
        if let Self::Int(n, _) = self {
            Ok(*n)
        } else {
            Err("expected integer".into())
        }
    }
    fn boolean(&self) -> Result<bool, String> {
        if let Self::Bool(b) = self {
            Ok(*b)
        } else {
            Err("condition must be bool".into())
        }
    }
    fn string(&self) -> Result<&str, String> {
        if let Self::Str(s) = self {
            Ok(s)
        } else {
            Err("expected str".into())
        }
    }
}
fn wrap(n: i128, t: &Type) -> i128 {
    let (bits, signed) = match t {
        Type::Int { bits, signed } => (*bits as u32, *signed),
        Type::Size { signed } => (usize::BITS, *signed),
        _ => return n,
    };
    let mask = (1i128 << bits) - 1;
    let n = n & mask;
    if signed && n & (1i128 << (bits - 1)) != 0 {
        n - (1i128 << bits)
    } else {
        n
    }
}
fn convert(v: Value, t: &Type, explicit: bool) -> Result<Value, String> {
    if &v.ty() == t {
        return Ok(v);
    }
    if explicit {
        match (&v, t) {
            (Value::Int(n, _), t) if t.integer() => return Ok(Value::Int(wrap(*n, t), t.clone())),
            (Value::Int(n, _), Type::Float(bits)) => {
                return Ok(Value::Float(
                    if *bits == 32 {
                        *n as f32 as f64
                    } else {
                        *n as f64
                    },
                    t.clone(),
                ))
            }
            (Value::Float(n, _), t) if t.integer() => {
                return Ok(Value::Int(wrap(*n as i128, t), t.clone()))
            }
            (Value::Float(n, _), Type::Float(bits)) => {
                return Ok(Value::Float(
                    if *bits == 32 { *n as f32 as f64 } else { *n },
                    t.clone(),
                ))
            }
            _ => {}
        }
    }
    Err(format!("expected {}, got {}", t.name(), v.ty().name()))
}
struct Loaded {
    module: Module,
    imports: HashMap<String, String>,
}
struct Frame {
    module: String,
    ret: Type,
    scopes: Vec<HashMap<String, Value>>,
}
enum Flow {
    Next,
    Return(Value),
    Break,
    Continue,
}
pub struct Engine {
    modules: HashMap<String, Loaded>,
    entry: String,
    args: Vec<String>,
    start: Instant,
    depth: usize,
}
impl Engine {
    pub fn load(path: &Path, source: Option<&str>, args: Vec<String>) -> Result<Self, String> {
        let mut e = Self {
            modules: HashMap::new(),
            entry: String::new(),
            args,
            start: Instant::now(),
            depth: 0,
        };
        e.entry = e.load_module(path, source)?;
        Ok(e)
    }
    fn load_module(&mut self, path: &Path, source: Option<&str>) -> Result<String, String> {
        let path = if source.is_some() {
            path.to_path_buf()
        } else {
            path.canonicalize()
                .map_err(|e| format!("{}: {e}", path.display()))?
        };
        let key = path.to_string_lossy().into_owned();
        if self.modules.contains_key(&key) {
            return Ok(key);
        }
        let text = match source {
            Some(s) => s.to_owned(),
            None => {
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?
            }
        };
        let module = parser::parse(path.clone(), &text)?;
        let imports = module.imports.clone();
        let mut names = HashMap::new();
        for f in &module.functions {
            if names.insert(&f.name, ()).is_some() {
                return Err(error(&path, f.span, "duplicate function"));
            }
        }
        self.modules.insert(
            key.clone(),
            Loaded {
                module,
                imports: HashMap::new(),
            },
        );
        for import in imports {
            let target = if ["std/io", "std/strings", "std/time", "std/args"]
                .contains(&import.path.as_str())
            {
                import.path.clone()
            } else {
                let mut p = path.parent().unwrap_or(Path::new(".")).join(&import.path);
                if p.extension().is_none() {
                    p.set_extension("dev");
                }
                self.load_module(&p, None)?
            };
            if self
                .modules
                .get_mut(&key)
                .unwrap()
                .imports
                .insert(import.alias, target)
                .is_some()
            {
                return Err(error(&path, import.span, "duplicate module alias"));
            }
        }
        Ok(key)
    }
    pub fn run(&mut self) -> Result<i32, String> {
        let key = self.entry.clone();
        let v = self.call(&key, &["main".into()], vec![], Span { line: 1, col: 1 })?;
        match v {
            Value::Int(n, _) => Ok(n as i32),
            Value::Void => Ok(0),
            _ => Err("main must return an integer".into()),
        }
    }
    fn located(&self, key: &str, span: Span, msg: impl AsRef<str>) -> String {
        error(&self.modules[key].module.path, span, msg)
    }
    fn resolve(&self, key: &str, name: &[String]) -> Result<(String, String), String> {
        match name {
            [n] => Ok((key.into(), n.clone())),
            [alias, n] => Ok((
                self.modules[key]
                    .imports
                    .get(alias)
                    .ok_or_else(|| format!("unknown module {alias}"))?
                    .clone(),
                n.clone(),
            )),
            _ => Err("invalid function name".into()),
        }
    }
    fn call(
        &mut self,
        key: &str,
        name: &[String],
        args: Vec<Value>,
        span: Span,
    ) -> Result<Value, String> {
        let (target, n) = self
            .resolve(key, name)
            .map_err(|e| self.located(key, span, e))?;
        if target.starts_with("std/") {
            return self
                .builtin(&target, &n, args)
                .map_err(|e| self.located(key, span, e));
        }
        if name == ["print"] {
            for a in args {
                println!("{}", a.display());
            }
            return Ok(Value::Void);
        }
        let f = self.modules[&target]
            .module
            .functions
            .iter()
            .find(|f| f.name == n)
            .cloned()
            .ok_or_else(|| self.located(key, span, format!("unknown function {n}")))?;
        let body = f.body.as_ref().ok_or_else(|| {
            self.located(
                key,
                span,
                "extern C calls require devc; devrun has no native FFI",
            )
        })?;
        if args.len() != f.params.len() {
            return Err(self.located(key, span, "wrong argument count"));
        }
        if self.depth >= 128 {
            return Err(self.located(key, span, "recursion limit exceeded (128 calls)"));
        }
        let mut locals = HashMap::new();
        for ((name, t), v) in f.params.iter().zip(args) {
            locals.insert(
                name.clone(),
                convert(v, t, false).map_err(|e| self.located(key, span, e))?,
            );
        }
        let mut frame = Frame {
            module: target.clone(),
            ret: f.ret.clone(),
            scopes: vec![locals],
        };
        self.depth += 1;
        let result = self.block(&mut frame, body);
        self.depth -= 1;
        match result? {
            Flow::Return(v) => {
                convert(v, &f.ret, false).map_err(|e| self.located(&target, f.span, e))
            }
            Flow::Next if f.ret == Type::Void => Ok(Value::Void),
            Flow::Next if n == "main" => Ok(Value::Int(0, Type::i32())),
            Flow::Next => Err(self.located(&target, f.span, "missing return value")),
            _ => Err(self.located(&target, f.span, "break/continue outside loop")),
        }
    }
    fn block(&mut self, f: &mut Frame, stmts: &[Stmt]) -> Result<Flow, String> {
        f.scopes.push(HashMap::new());
        let r = self.statements(f, stmts);
        f.scopes.pop();
        r
    }
    fn statements(&mut self, f: &mut Frame, stmts: &[Stmt]) -> Result<Flow, String> {
        for s in stmts {
            match s {
                Stmt::Let {
                    name,
                    ty,
                    value,
                    span,
                } => {
                    let v = self.eval(f, value, ty.as_ref())?;
                    if f.scopes
                        .last_mut()
                        .unwrap()
                        .insert(name.clone(), v)
                        .is_some()
                    {
                        return Err(self.located(&f.module, *span, "duplicate local variable"));
                    }
                }
                Stmt::Expr(e) => {
                    self.eval(f, e, None)?;
                }
                Stmt::Return(e, _) => {
                    let ret = f.ret.clone();
                    let v = if let Some(e) = e {
                        self.eval(f, e, Some(&ret))?
                    } else {
                        Value::Void
                    };
                    return Ok(Flow::Return(v));
                }
                Stmt::Assign { target, op, value } => {
                    let old = self.eval(f, target, None)?;
                    let rhs = self.eval(f, value, Some(&old.ty()))?;
                    let v = if op == "=" {
                        rhs
                    } else {
                        self.binary(op.trim_end_matches('='), old, rhs)
                            .map_err(|e| self.located(&f.module, target.span, e))?
                    };
                    self.assign(f, target, v)?;
                }
                Stmt::If { cond, yes, no } => {
                    let b = self
                        .eval(f, cond, None)?
                        .boolean()
                        .map_err(|e| self.located(&f.module, cond.span, e))?;
                    let flow = self.block(f, if b { yes } else { no })?;
                    if !matches!(flow, Flow::Next) {
                        return Ok(flow);
                    }
                }
                Stmt::While { cond, body } => {
                    while self
                        .eval(f, cond, None)?
                        .boolean()
                        .map_err(|e| self.located(&f.module, cond.span, e))?
                    {
                        match self.block(f, body)? {
                            Flow::Break => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            _ => {}
                        }
                    }
                }
                Stmt::Break(_) => return Ok(Flow::Break),
                Stmt::Continue(_) => return Ok(Flow::Continue),
            }
        }
        Ok(Flow::Next)
    }
    fn assign(&mut self, f: &mut Frame, e: &Expr, v: Value) -> Result<(), String> {
        match &e.kind {
            ExprKind::Name(n) if n.len() == 1 => {
                let cell = f
                    .scopes
                    .iter_mut()
                    .rev()
                    .find_map(|s| s.get_mut(&n[0]))
                    .ok_or_else(|| "unknown variable".to_string())?;
                *cell = v;
                Ok(())
            }
            ExprKind::Index(base, index) => {
                let idx = self.eval(f, index, None)?.integer()?;
                let idx = usize::try_from(idx)
                    .map_err(|_| self.located(&f.module, e.span, "invalid index"))?;
                if let ExprKind::Name(n) = &base.kind {
                    if n.len() == 1 {
                        if let Some(Value::Array(a, _)) =
                            f.scopes.iter_mut().rev().find_map(|s| s.get_mut(&n[0]))
                        {
                            if let Some(cell) = a.get_mut(idx) {
                                *cell = v;
                                return Ok(());
                            }
                        }
                    }
                }
                Err(self.located(
                    &f.module,
                    e.span,
                    "invalid array assignment or index out of bounds",
                ))
            }
            _ => Err(self.located(&f.module, e.span, "unsupported assignment target")),
        }
    }
    fn eval(&mut self, f: &mut Frame, e: &Expr, hint: Option<&Type>) -> Result<Value, String> {
        let result = (|| {
            let v = match &e.kind {
                ExprKind::Number(s) => {
                    let t = hint.filter(|t| t.numeric()).cloned().unwrap_or_else(|| {
                        if !s.trim_start_matches('-').starts_with("0x")
                            && !s.trim_start_matches('-').starts_with("0X")
                            && (s.contains('.') || s.contains('e') || s.contains('E'))
                        {
                            Type::Float(64)
                        } else {
                            Type::i64()
                        }
                    });
                    if let Type::Float(bits) = t {
                        let n = s.parse::<f64>().map_err(|_| "invalid float")?;
                        Value::Float(if bits == 32 { n as f32 as f64 } else { n }, t)
                    } else {
                        let raw = s.replace('_', "");
                        let n = if let Some(h) =
                            raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X"))
                        {
                            i128::from_str_radix(h, 16)
                        } else {
                            raw.parse::<i128>()
                        }
                        .map_err(|_| "invalid integer")?;
                        if wrap(n, &t) != n {
                            return Err("integer literal out of range".into());
                        }
                        Value::Int(n, t)
                    }
                }
                ExprKind::String(s) => Value::Str(s.clone()),
                ExprKind::Bool(b) => Value::Bool(*b),
                ExprKind::Name(n) => {
                    if n.len() != 1 {
                        return Err("expected local variable".into());
                    }
                    f.scopes
                        .iter()
                        .rev()
                        .find_map(|s| s.get(&n[0]))
                        .cloned()
                        .ok_or_else(|| format!("unknown variable {}", n[0]))?
                }
                ExprKind::Cast(a, t) => convert(self.eval(f, a, None)?, t, true)?,
                ExprKind::Unary(op, a) => {
                    if op == "-" {
                        if let ExprKind::Number(n) = &a.kind {
                            let text = if let Some(hex) =
                                n.strip_prefix("0x").or_else(|| n.strip_prefix("0X"))
                            {
                                format!(
                                    "-{}",
                                    i128::from_str_radix(hex, 16).map_err(|_| "invalid integer")?
                                )
                            } else {
                                format!("-{n}")
                            };
                            return self.eval(
                                f,
                                &Expr {
                                    kind: ExprKind::Number(text),
                                    span: e.span,
                                },
                                hint,
                            );
                        }
                    }
                    let v = self.eval(f, a, hint)?;
                    match (op.as_str(),v){("!",v)=>Value::Bool(!v.boolean()?),("-",Value::Int(n,t))=>Value::Int(wrap(-n,&t),t),("~",Value::Int(n,t))=>Value::Int(wrap(!n,&t),t),("-",Value::Float(n,t))=>Value::Float(-n,t),_=>return Err("raw pointers or unsupported unary operation; use devc for hardware access".into())}
                }
                ExprKind::Binary(op, a, b) => {
                    let left = self.eval(f, a, hint.filter(|t| t.numeric()))?;
                    if op == "&&" && !left.boolean()? {
                        return Ok(Value::Bool(false));
                    }
                    if op == "||" && left.boolean()? {
                        return Ok(Value::Bool(true));
                    }
                    let right = self.eval(f, b, Some(&left.ty()))?;
                    self.binary(op, left, right)?
                }
                ExprKind::Call(n, arguments) => {
                    let (target, name) = self.resolve(&f.module, n)?;
                    let params = self
                        .modules
                        .get(&target)
                        .and_then(|m| m.module.functions.iter().find(|fun| fun.name == name))
                        .map(|fun| fun.params.clone());
                    let mut args = vec![];
                    for (i, a) in arguments.iter().enumerate() {
                        args.push(self.eval(
                            f,
                            a,
                            params.as_ref().and_then(|p| p.get(i).map(|(_, t)| t)),
                        )?)
                    }
                    self.call(&f.module, n, args, e.span)?
                }
                ExprKind::Array(items) => {
                    let element = if let Some(Type::Array(t, _)) = hint {
                        Some(t.as_ref())
                    } else {
                        None
                    };
                    let mut a = vec![];
                    for item in items {
                        let t = element.cloned().or_else(|| a.first().map(Value::ty));
                        a.push(self.eval(f, item, t.as_ref())?)
                    }
                    let t = Type::Array(
                        Box::new(
                            element
                                .cloned()
                                .or_else(|| a.first().map(Value::ty))
                                .ok_or("empty array needs a type")?,
                        ),
                        a.len(),
                    );
                    Value::Array(a, t)
                }
                ExprKind::Index(a, i) => {
                    let a = self.eval(f, a, None)?;
                    let i = self.eval(f, i, None)?.integer()?;
                    let i = usize::try_from(i).map_err(|_| "negative or excessive index")?;
                    match a {
                        Value::Array(a, _) => {
                            a.get(i).cloned().ok_or("array index out of bounds")?
                        }
                        Value::Str(s) => Value::Int(
                            *s.as_bytes().get(i).ok_or("string index out of bounds")? as i128,
                            Type::Int {
                                bits: 8,
                                signed: false,
                            },
                        ),
                        _ => return Err("value cannot be indexed".into()),
                    }
                }
            };
            if let Some(t) = hint {
                convert(v, t, false)
            } else {
                Ok(v)
            }
        })();
        result.map_err(|msg: String| {
            if msg.contains(": ") {
                msg
            } else {
                self.located(&f.module, e.span, msg)
            }
        })
    }
    fn binary(&self, op: &str, a: Value, b: Value) -> Result<Value, String> {
        if a.ty() != b.ty() {
            return Err("binary operands must have matching types".into());
        }
        let t = a.ty();
        match (a, b) {
            (Value::Int(a, _), Value::Int(b, _)) => {
                let n = match op {
                    "+" => a.wrapping_add(b),
                    "-" => a.wrapping_sub(b),
                    "*" => a.wrapping_mul(b),
                    "/" | "%" if b == 0 => return Err("division by zero".into()),
                    "/" => a / b,
                    "%" => a % b,
                    "&" => a & b,
                    "|" => a | b,
                    "^" => a ^ b,
                    "<<" | ">>" => {
                        let bits = match &t {
                            Type::Int { bits, .. } => *bits as i128,
                            _ => usize::BITS as i128,
                        };
                        if b < 0 || b >= bits {
                            return Err("shift count out of range".into());
                        }
                        if op == "<<" {
                            a << b
                        } else {
                            a >> b
                        }
                    }
                    "==" => return Ok(Value::Bool(a == b)),
                    "!=" => return Ok(Value::Bool(a != b)),
                    "<" => return Ok(Value::Bool(a < b)),
                    "<=" => return Ok(Value::Bool(a <= b)),
                    ">" => return Ok(Value::Bool(a > b)),
                    ">=" => return Ok(Value::Bool(a >= b)),
                    _ => return Err("unsupported integer operator".into()),
                };
                Ok(Value::Int(wrap(n, &t), t))
            }
            (Value::Float(a, _), Value::Float(b, _)) => {
                let n = match op {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b,
                    "%" => a % b,
                    "==" => return Ok(Value::Bool(a == b)),
                    "!=" => return Ok(Value::Bool(a != b)),
                    "<" => return Ok(Value::Bool(a < b)),
                    "<=" => return Ok(Value::Bool(a <= b)),
                    ">" => return Ok(Value::Bool(a > b)),
                    ">=" => return Ok(Value::Bool(a >= b)),
                    _ => return Err("unsupported float operator".into()),
                };
                Ok(Value::Float(
                    if t == Type::Float(32) {
                        n as f32 as f64
                    } else {
                        n
                    },
                    t,
                ))
            }
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(match op {
                "&&" => a && b,
                "||" => a || b,
                "==" => a == b,
                "!=" => a != b,
                _ => return Err("unsupported bool operator".into()),
            })),
            (Value::Str(a), Value::Str(b)) => match op {
                "+" => Ok(Value::Str(a + &b)),
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                _ => Err("unsupported string operator".into()),
            },
            _ => Err("unsupported operands".into()),
        }
    }
    fn builtin(&mut self, module: &str, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let a = |i: usize| args.get(i).ok_or_else(|| "missing argument".to_string());
        let count = match (module, name) {
            ("std/io", "read_line") | ("std/time", "now_ns" | "now_ms") | ("std/args", "len") => 0,
            ("std/strings", "concat" | "equal") | ("std/io", "write_file") => 2,
            _ => 1,
        };
        if args.len() != count {
            return Err("wrong argument count".into());
        }
        let size = |n: usize| Value::Int(n as i128, Type::Size { signed: false });
        match (module, name) {
            ("std/io", "write" | "writeln") => {
                use std::io::Write;
                let s = a(0)?.string()?;
                let mut out = std::io::stdout().lock();
                if name == "writeln" {
                    writeln!(out, "{s}")
                } else {
                    write!(out, "{s}")
                }
                .map_err(|e| e.to_string())?;
                out.flush().map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            ("std/io", "read_line") => {
                let mut s = String::new();
                std::io::stdin()
                    .read_line(&mut s)
                    .map_err(|e| e.to_string())?;
                if s.ends_with('\n') {
                    s.pop();
                    if s.ends_with('\r') {
                        s.pop();
                    }
                }
                Ok(Value::Str(s))
            }
            ("std/io", "read_file") => Ok(Value::Str(
                std::fs::read_to_string(a(0)?.string()?).map_err(|e| e.to_string())?,
            )),
            ("std/io", "write_file") => {
                std::fs::write(a(0)?.string()?, a(1)?.string()?).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            ("std/strings", "len") => Ok(size(a(0)?.string()?.len())),
            ("std/strings", "concat") => {
                Ok(Value::Str(a(0)?.string()?.to_owned() + a(1)?.string()?))
            }
            ("std/strings", "equal") => Ok(Value::Bool(a(0)?.string()? == a(1)?.string()?)),
            ("std/time", "now_ns" | "now_ms") => Ok(Value::Int(
                if name == "now_ns" {
                    self.start.elapsed().as_nanos()
                } else {
                    self.start.elapsed().as_millis()
                } as i128,
                Type::Int {
                    bits: 64,
                    signed: false,
                },
            )),
            ("std/time", "sleep_ms") => {
                let n = u64::try_from(a(0)?.integer()?).map_err(|_| "invalid duration")?;
                std::thread::sleep(std::time::Duration::from_millis(n));
                Ok(Value::Bool(true))
            }
            ("std/args", "len") => Ok(size(self.args.len())),
            ("std/args", "get") => {
                let i = usize::try_from(a(0)?.integer()?).map_err(|_| "invalid argument index")?;
                Ok(Value::Str(
                    self.args
                        .get(i)
                        .ok_or("argument index out of bounds")?
                        .clone(),
                ))
            }
            _ => Err(format!("unknown intrinsic {module}.{name}")),
        }
    }
}
