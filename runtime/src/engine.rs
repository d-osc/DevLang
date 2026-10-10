use crate::native::Native;
use dev_syntax::{ast::*, parser};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Shared(Arc<SharedResource>, Type),
    Json(Arc<serde_json::Value>, Type),
    Task(Arc<TaskState>, Type),
    Callable(String, String, Option<Arc<Value>>, Type),
    Int(i128, Type),
    Float(f64, Type),
    Bool(bool),
    Str(String),
    Array(Vec<Value>, Type),
    Void,
    Ptr(usize, Type),
    Record(Vec<(String, Value)>, Type),
    Enum(usize, Type, Vec<Value>),
    Ref(Arc<Value>, Type),
    Vector(Arc<Vec<Value>>, Type),
    Map(Arc<MapStorage>, Type),
    Slice(Arc<Vec<Value>>, usize, usize, Type),
}
include!("map.rs");
include!("server.rs");
include!("core.rs");
include!("network.rs");
include!("basics.rs");
include!("data_libs.rs");
include!("system_libs.rs");
include!("control_libs.rs");
include!("child_process.rs");
include!("storage_libs.rs");
include!("secure_network.rs");
include!("sync_lib.rs");

impl Value {
    pub(crate) fn ty(&self) -> Type {
        match self {
            Self::Json(_, t)
            | Self::Shared(_, t)
            | Self::Task(_, t)
            | Self::Callable(_, _, _, t)
            | Self::Int(_, t)
            | Self::Float(_, t)
            | Self::Array(_, t)
            | Self::Ptr(_, t)
            | Self::Record(_, t)
            | Self::Map(_, t)
            | Self::Vector(_, t)
            | Self::Slice(_, _, _, t)
            | Self::Ref(_, t)
            | Self::Enum(_, t, _) => t.clone(),
            Self::Bool(_) => Type::Bool,
            Self::Str(_) => Type::Str,
            Self::Void => Type::Void,
        }
    }
    fn display(&self) -> String {
        match self {
            Self::Shared(..) => "<shared>".into(),
            Self::Json(value, _) => value.to_string(),
            Self::Int(n, _) => n.to_string(),
            Self::Float(n, _) => n.to_string(),
            Self::Bool(b) => b.to_string(),
            Self::Str(s) => s.clone(),
            Self::Array(a, _) => format!("{:?}", a),
            Self::Void => String::new(),
            Self::Ptr(p, _) => format!("0x{p:x}"),
            Self::Map(xs, _) => format!(
                "{{{}}}",
                xs.entries
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k.display(), v.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Vector(xs, _) => format!(
                "[{}]",
                xs.iter().map(Value::display).collect::<Vec<_>>().join(", ")
            ),
            Self::Slice(xs, offset, len, _) => format!(
                "[{}]",
                xs[*offset..*offset + *len]
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Task(..) => "<task>".into(),
            Self::Callable(..) => "<fn>".into(),
            Self::Ref(_, _) => "<ref>".into(),
            Self::Record(fields, _) => format!(
                "{{{}}}",
                fields
                    .iter()
                    .map(|(n, v)| format!("{n}: {}", v.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Enum(n, Type::Enum(_, variants), _) => variants[*n].0.clone(),
            Self::Enum(..) => unreachable!(),
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
    pub(crate) fn string(&self) -> Result<&str, String> {
        if let Self::Str(s) = self {
            Ok(s)
        } else {
            Err("expected str".into())
        }
    }
}
pub(crate) fn wrap(n: i128, t: &Type) -> i128 {
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
pub(crate) fn convert(v: Value, t: &Type, explicit: bool) -> Result<Value, String> {
    if &v.ty() == t {
        return Ok(v);
    }
    if explicit {
        if matches!(&v,Value::Enum(_,Type::Enum(_,vs),_) if vs.iter().any(|(_,ts)| !ts.is_empty()))
            && t.integer()
        {
            return Err("payload enums cannot be cast to integers".into());
        }
        match (&v, t) {
            (Value::Enum(n, Type::Enum(_, vs), _), t)
                if t.integer() && vs.iter().all(|(_, ts)| ts.is_empty()) =>
            {
                return Ok(Value::Int(wrap(*n as i128, t), t.clone()))
            }
            (Value::Int(n, _), Type::Ptr(_)) => return Ok(Value::Ptr(*n as usize, t.clone())),
            (Value::Ptr(p, _), t) if t.integer() => {
                return Ok(Value::Int(wrap(*p as i128, t), t.clone()))
            }
            (Value::Ptr(p, _), Type::Ptr(_)) => return Ok(Value::Ptr(*p, t.clone())),
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
#[derive(Clone)]
struct Loaded {
    module: Module,
    imports: HashMap<String, String>,
    functions: HashMap<String, Arc<Function>>,
    numeric: HashMap<String, Rc<crate::numeric::Program>>,
}
struct Frame {
    module: String,
    ret: Type,
    scopes: Vec<HashMap<String, Value>>,
    unsafe_depth: usize,
}
enum Flow {
    Next,
    Return(Value),
    Break,
    Continue,
}
type TaskResult = Result<Value, String>;
type TaskStatus = (
    Option<std::thread::JoinHandle<TaskResult>>,
    Option<TaskResult>,
);
#[derive(Debug)]
pub(crate) struct TaskState {
    state: Mutex<TaskStatus>,
}
impl TaskState {
    fn ready(&self) -> bool {
        // A concurrent join must not make polling wait on its mutex.
        let Ok(s) = self.state.try_lock() else {
            return false;
        };
        s.1.is_some()
            || s.0
                .as_ref()
                .is_some_and(std::thread::JoinHandle::is_finished)
    }
    fn join(&self) -> Result<Value, String> {
        let mut s = self.state.lock().map_err(|_| "task lock poisoned")?;
        if let Some(h) = s.0.take() {
            s.1 = Some(h.join().unwrap_or_else(|_| Err("task panicked".into())))
        }
        s.1.as_ref().unwrap().clone()
    }
}
pub struct Engine {
    module_dirs: HashMap<String, PathBuf>,
    libraries: Vec<PathBuf>,
    modules: HashMap<String, Loaded>,
    entry: String,
    args: Vec<String>,
    start: Instant,
    depth: usize,
    native: Native,
    numeric: bool,
    http: Option<ureq::Agent>,
    servers: HashMap<i64, DevServer>,
    responses: HashMap<i64, DevResponse>,
    core: CoreState,
}
impl Engine {
    pub fn load_with_modules(
        path: &Path,
        source: Option<&str>,
        args: Vec<String>,
        libraries: &[PathBuf],
        numeric: bool,
        module_dirs: HashMap<String, PathBuf>,
    ) -> Result<Self, String> {
        let mut e = Self {
            module_dirs,
            libraries: libraries.to_vec(),
            modules: HashMap::new(),
            entry: String::new(),
            args,
            start: Instant::now(),
            depth: 0,
            native: Native::new(libraries)?,
            numeric,
            http: None,
            servers: HashMap::new(),
            responses: HashMap::new(),
            core: CoreState::default(),
        };
        e.entry = e.load_module(path, source)?;
        let mut keys = e.modules.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        let indices = keys
            .iter()
            .enumerate()
            .map(|(i, k)| (k.clone(), i))
            .collect::<HashMap<_, _>>();
        let mut modules = keys
            .iter()
            .map(|k| {
                let loaded = &e.modules[k];
                let mut module = loaded.module.clone();
                module.functions = loaded
                    .functions
                    .values()
                    .map(|f| f.as_ref().clone())
                    .collect();
                module.functions.sort_by_key(|f| f.span.line);
                module
            })
            .collect::<Vec<_>>();
        let aliases = keys
            .iter()
            .map(|k| {
                e.modules[k]
                    .imports
                    .iter()
                    .filter_map(|(alias, target)| {
                        indices.get(target).map(|id| (alias.clone(), *id))
                    })
                    .collect()
            })
            .collect::<Vec<_>>();
        dev_syntax::expand::modules(&mut modules, &aliases)?;
        for (key, mut module) in keys.into_iter().zip(modules) {
            let functions = std::mem::take(&mut module.functions)
                .into_iter()
                .map(|f| (f.name.clone(), Arc::new(f)))
                .collect::<HashMap<_, _>>();
            for f in functions.values().filter(|f| f.body.is_none()) {
                e.native
                    .declare(f)
                    .map_err(|msg| error(&module.path, f.span, msg))?;
            }
            let loaded = e.modules.get_mut(&key).unwrap();
            loaded.module = module;
            loaded.functions = functions;
        }
        if numeric {
            let mut plans = vec![];
            for (key, loaded) in &e.modules {
                for (name, f) in &loaded.functions {
                    if let Some(plan) = crate::numeric::Program::lower(f, &|n| e.signature(key, n))
                    {
                        plans.push((key.clone(), name.clone(), Rc::new(plan)));
                    }
                }
            }
            for (key, name, plan) in plans {
                e.modules.get_mut(&key).unwrap().numeric.insert(name, plan);
            }
        }
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
        let mut module = parser::parse(path.clone(), &text)?;
        if !self.modules.is_empty() && !module.statements.is_empty() {
            return Err(error(&path,Span{line:1,col:1},"imported modules contain declarations only; call their functions from the entry file"));
        }
        let imports = module.imports.clone();
        let mut names = HashMap::new();
        for f in &module.functions {
            fn unresolved(t: &Type) -> bool {
                match t {
                    Type::Named(..) => true,
                    Type::Ptr(t) | Type::Array(t, _) => unresolved(t),
                    _ => false,
                }
            }
            if f.body.is_none()
                && !unresolved(&f.ret)
                && !f.params.iter().any(|(_, t)| unresolved(t))
            {
                // Preserve early ABI validation and conflicting-signature diagnostics.
                self.native
                    .declare(f)
                    .map_err(|msg| error(&path, f.span, msg))?;
            }
            if f.body.is_none() && source.is_none() {
                if let Some(directory) = path.parent() {
                    self.native.add_directory(directory);
                }
            }
            if names.insert(&f.name, ()).is_some() {
                return Err(error(&path, f.span, "duplicate function"));
            }
        }
        let functions = std::mem::take(&mut module.functions)
            .into_iter()
            .map(|f| (f.name.clone(), Arc::new(f)))
            .collect();
        self.modules.insert(
            key.clone(),
            Loaded {
                functions,
                numeric: HashMap::new(),
                module,
                imports: HashMap::new(),
            },
        );
        for import in imports {
            let target = if import.path == "std/json" {
                self.load_module(Path::new("std/json"), Some(crate::json::MODULE))?
            } else if let Some(source) = dev_syntax::intrinsics::module(&import.path) {
                self.load_module(Path::new(&import.path), Some(source))?
            } else if ["std/io", "std/strings", "std/time", "std/args"]
                .contains(&import.path.as_str())
            {
                import.path.clone()
            } else {
                let p = dev_syntax::modules::resolve(&path, &import.path, &self.module_dirs)
                    .map_err(|e| error(&path, import.span, e))?;
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
    fn signature(&self, key: &str, name: &[String]) -> Option<(Vec<Type>, Type)> {
        let (target, n) = self.resolve(key, name).ok()?;
        let f = self.modules.get(&target)?.functions.get(&n)?;
        Some((
            f.params.iter().map(|(_, t)| t.clone()).collect(),
            f.ret.clone(),
        ))
    }
    pub fn run(&mut self) -> Result<i32, String> {
        let code = self.run_script()?;
        if code == 0 {
            self.serve_http()?;
        }
        Ok(code)
    }
    fn run_script(&mut self) -> Result<i32, String> {
        let key = self.entry.clone();
        let statements = self.modules[&key].module.statements.clone();
        if self.numeric {
            let script = Function {
                constraints: vec![],
                variadic: false,
                name: "<script>".into(),
                main_default: true,
                generics: vec![],
                params: vec![],
                ret: Type::i32(),
                body: Some(statements.clone()),
                exported: false,
                span: Span { line: 1, col: 1 },
            };
            if let Some(plan) =
                crate::numeric::Program::lower(&script, &|n| self.signature(&key, n))
            {
                let result = plan.run(vec![], &mut |n, args, span| self.call(&key, n, args, span));
                self.native.flush_stdio();
                return match result {
                    Ok(Value::Int(n, _)) => Ok(n as i32),
                    Ok(_) => unreachable!("script plan returns i32"),
                    Err((span, msg)) => Err(self.plan_error(&key, span, msg)),
                };
            }
        }
        let mut frame = Frame {
            unsafe_depth: 0,
            module: key.clone(),
            ret: Type::i32(),
            scopes: vec![HashMap::new()],
        };
        let result = self.statements(&mut frame, &statements);
        self.native.flush_stdio();
        match result? {
            Flow::Next => Ok(0),
            Flow::Return(Value::Int(n, _)) => Ok(n as i32),
            Flow::Return(_) => Err(self.located(
                &key,
                Span { line: 1, col: 1 },
                "top-level return must be i32",
            )),
            _ => Err(self.located(
                &key,
                Span { line: 1, col: 1 },
                "break/continue outside loop",
            )),
        }
    }
    fn located(&self, key: &str, span: Span, msg: impl AsRef<str>) -> String {
        error(&self.modules[key].module.path, span, msg)
    }
    fn plan_error(&self, key: &str, span: Span, msg: String) -> String {
        if msg.contains(": ") {
            msg
        } else {
            self.located(key, span, msg)
        }
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
        if target.starts_with("std/") && target != "std/fs/promises" {
            return self
                .builtin(&target, &n, args)
                .map_err(|e| self.located(key, span, e));
        }
        if name == ["volatile_load"] || name == ["volatile_store"] {
            let store = name == ["volatile_store"];
            if args.len() != if store { 2 } else { 1 } {
                return Err("wrong volatile argument count".into());
            }
            let Value::Ptr(p, Type::Ptr(t)) = &args[0] else {
                return Err("volatile access requires a pointer".into());
            };
            if store {
                let v = convert(args[1].clone(), t, false)?;
                unsafe { crate::memory::volatile_write(*p, &v) }?;
                return Ok(Value::Void);
            }
            return unsafe { crate::memory::volatile_read(*p, t) };
        }
        if name == ["callback"] || name == ["callback_context"] {
            let [Value::Callable(module, function, context, Type::Function(ps, r))] =
                args.as_slice()
            else {
                return Err(self.located(key, span, "callback expects one function value"));
            };
            let context_mode = name == ["callback_context"];
            let owner = context_mode.then(|| Arc::new(args[0].clone()));
            let weak_owner = owner.as_ref().map(Arc::downgrade);
            let userdata = owner.as_ref().map_or(0, |v| Arc::as_ptr(v) as usize);
            let record_type = callback_context_type(&args[0].ty()).unwrap();
            let callback_type = if context_mode {
                let Type::Record(_, fields) = &record_type else {
                    unreachable!()
                };
                fields[0].1.clone()
            } else {
                Type::Callback(ps.clone(), r.clone())
            };
            let context = if context_mode { None } else { context.clone() };
            let callback_key = format!(
                "{module}:{function}:{}",
                context.as_ref().map_or(0, |v| Arc::as_ptr(v) as usize)
            );
            let module = module.clone();
            let function = function.clone();
            let libraries = self.libraries.clone();
            let arguments = self.args.clone();
            let snapshots = self
                .modules
                .iter()
                .map(|(k, m)| {
                    (
                        k.clone(),
                        m.module.clone(),
                        m.imports.clone(),
                        m.functions.clone(),
                    )
                })
                .collect::<Vec<_>>();
            let callback_registry = Arc::downgrade(&self.native.callbacks);
            let callback = self.native.callback(
                if context_mode {
                    String::new()
                } else {
                    callback_key
                },
                &callback_type,
                Box::new(move |mut values| {
                    // The C API borrows userdata. Upgrade the managed owner for the
                    // whole invocation; expired owners cannot revive through cache reuse.
                    let owner = weak_owner
                        .as_ref()
                        .map(|w| w.upgrade().ok_or("context callback owner has expired"))
                        .transpose()?;
                    let context = if let Some(owner) = &owner {
                        if !matches!(values.pop(),Some(Value::Ptr(p,_)) if p==userdata) {
                            return Err("context callback userdata mismatch".into());
                        }
                        let Value::Callable(_, _, context, _) = owner.as_ref() else {
                            unreachable!()
                        };
                        context.clone()
                    } else {
                        context.clone()
                    };
                    let modules = snapshots
                        .iter()
                        .cloned()
                        .map(|(k, module, imports, functions)| {
                            (
                                k,
                                Loaded {
                                    module,
                                    imports,
                                    functions,
                                    numeric: HashMap::new(),
                                },
                            )
                        })
                        .collect::<HashMap<_, _>>();
                    let mut native = Native::new(&libraries)?;
                    native.callbacks = callback_registry
                        .upgrade()
                        .ok_or("callback engine has shut down")?;
                    for m in modules.values() {
                        if let Some(p) = m.module.path.parent() {
                            native.add_directory(p)
                        }
                        for f in m.functions.values().filter(|f| f.body.is_none()) {
                            native.declare(f)?;
                        }
                    }
                    let mut worker = Engine {
                        http: None,
                        servers: HashMap::new(),
                        responses: HashMap::new(),
                        core: CoreState::default(),
                        module_dirs: HashMap::new(),
                        libraries: libraries.clone(),
                        modules,
                        entry: module.clone(),
                        args: arguments.clone(),
                        start: Instant::now(),
                        depth: 0,
                        native,
                        numeric: false,
                    };
                    let mut values = values;
                    if let Some(v) = &context {
                        let inner = match v.ty() {
                            Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                            t => t,
                        };
                        values.insert(0, Value::Ref(v.clone(), Type::Ref(Box::new(inner))));
                    }
                    let result =
                        worker.call(&module, std::slice::from_ref(&function), values, span);
                    worker.native.flush_stdio();
                    result
                }),
            )?;
            return if let Some(owner) = owner {
                Ok(Value::Record(
                    vec![
                        ("call".into(), callback),
                        (
                            "data".into(),
                            Value::Ptr(userdata, Type::Ptr(Box::new(Type::Void))),
                        ),
                        (
                            "owner".into(),
                            Value::Ref(owner, Type::Ref(Box::new(args[0].ty()))),
                        ),
                    ],
                    record_type,
                ))
            } else {
                Ok(callback)
            };
        }
        if name == ["spawn"] {
            if args.len() != 1 {
                return Err(self.located(key, span, "spawn expects a zero-argument function"));
            }
            let Value::Callable(module, function, context, Type::Function(ps, r)) = args[0].clone()
            else {
                return Err(self.located(key, span, "spawn expects a function value"));
            };
            if !ps.is_empty() {
                return Err(self.located(key, span, "spawn expects a zero-argument function"));
            }
            let snapshots = self
                .modules
                .iter()
                .map(|(k, m)| {
                    (
                        k.clone(),
                        m.module.clone(),
                        m.imports.clone(),
                        m.functions.clone(),
                    )
                })
                .collect::<Vec<_>>();
            let libraries = self.libraries.clone();
            let arguments = self.args.clone();
            let numeric = self.numeric;
            let callback_registry = self.native.callbacks.clone();
            let handle = std::thread::spawn(move || {
                let modules = snapshots
                    .into_iter()
                    .map(|(k, module, imports, functions)| {
                        (
                            k,
                            Loaded {
                                module,
                                imports,
                                functions,
                                numeric: HashMap::new(),
                            },
                        )
                    })
                    .collect::<HashMap<_, _>>();
                let mut native = Native::new(&libraries)?;
                native.callbacks = callback_registry;
                for loaded in modules.values() {
                    if let Some(directory) = loaded.module.path.parent() {
                        native.add_directory(directory);
                    }
                    for f in loaded.functions.values().filter(|f| f.body.is_none()) {
                        native.declare(f)?;
                    }
                }
                let mut worker = Engine {
                    http: None,
                    servers: HashMap::new(),
                    responses: HashMap::new(),
                    core: CoreState::default(),
                    module_dirs: HashMap::new(),
                    libraries,
                    modules,
                    entry: module.clone(),
                    args: arguments,
                    start: Instant::now(),
                    depth: 0,
                    native,
                    numeric,
                };
                if numeric {
                    let mut plans = vec![];
                    for (key, m) in &worker.modules {
                        for (name, f) in &m.functions {
                            if let Some(p) =
                                crate::numeric::Program::lower(f, &|n| worker.signature(key, n))
                            {
                                plans.push((key.clone(), name.clone(), Rc::new(p)));
                            }
                        }
                    }
                    for (k, n, p) in plans {
                        worker.modules.get_mut(&k).unwrap().numeric.insert(n, p);
                    }
                }
                let mut values = vec![];
                if let Some(v) = context {
                    let inner = match v.ty() {
                        Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                        t => t,
                    };
                    values.push(Value::Ref(v, Type::Ref(Box::new(inner))))
                }
                let result = worker.call(&module, &[function], values, span);
                worker.native.flush_stdio();
                result
            });
            return Ok(Value::Task(
                Arc::new(TaskState {
                    state: Mutex::new((Some(handle), None)),
                }),
                Type::Task(r),
            ));
        }
        if name == ["ready"] {
            let [Value::Task(task, _)] = args.as_slice() else {
                return Err(self.located(key, span, "ready expects one Task"));
            };
            return Ok(Value::Bool(task.ready()));
        }
        if name == ["await"] {
            if args.len() != 1 {
                return Err(self.located(key, span, "await expects one Task"));
            }
            let Value::Task(task, _) = &args[0] else {
                return Err(self.located(key, span, "await expects Task"));
            };
            return task.join().map_err(|m| self.located(key, span, m));
        }
        if name == ["print"] {
            self.native.flush_stdio();
            for a in args {
                println!("{}", a.display());
            }
            return Ok(Value::Void);
        }
        let f = self.modules[&target]
            .functions
            .get(&n)
            .cloned()
            .ok_or_else(|| self.located(key, span, format!("unknown function {n}")))?;
        if args.len() < f.params.len() || (!f.variadic && args.len() != f.params.len()) {
            return Err(self.located(key, span, "wrong argument count"));
        }
        if f.body.is_none() {
            let values = args
                .into_iter()
                .enumerate()
                .map(|(i, v)| {
                    let t = if let Some((_, t)) = f.params.get(i) {
                        t.clone()
                    } else {
                        variadic_type(&v.ty())?
                    };
                    if i >= f.params.len() {
                        if let Value::Bool(b) = v {
                            return Ok(Value::Int(i128::from(b), t));
                        }
                        convert(v, &t, true)
                    } else {
                        convert(v, &t, false)
                    }
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| self.located(key, span, e))?;
            return self
                .native
                .call(&f, &values)
                .map_err(|e| self.located(key, span, e));
        }
        let body = f.body.as_ref().unwrap();
        if self.depth >= 128 {
            return Err(self.located(key, span, "recursion limit exceeded (128 calls)"));
        }
        if let Some(plan) = self.modules[&target].numeric.get(&n).cloned() {
            let args = f
                .params
                .iter()
                .zip(args)
                .map(|((_, t), v)| convert(v, t, false))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| self.located(key, span, e))?;
            self.depth += 1;
            let result = plan.run(args, &mut |n, args, span| self.call(&target, n, args, span));
            self.depth -= 1;
            return result.map_err(|(span, msg)| self.plan_error(&target, span, msg));
        }
        let mut locals = HashMap::new();
        for ((name, t), v) in f.params.iter().zip(args) {
            locals.insert(
                name.clone(),
                convert(v, t, false).map_err(|e| self.located(key, span, e))?,
            );
        }
        let mut frame = Frame {
            unsafe_depth: 0,
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
            Flow::Next if f.main_default && f.ret == Type::i32() => Ok(Value::Int(0, Type::i32())),
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
                Stmt::Match { value, arms } => {
                    let v = self.eval(f, value, None)?;
                    let indices = match_variants(&v.ty(), arms)
                        .map_err(|e| self.located(&f.module, value.span, e))?;
                    let Value::Enum(index, _, payload) = v else {
                        unreachable!()
                    };
                    for (arm, i) in arms.iter().zip(indices) {
                        if i != usize::MAX && i != index {
                            continue;
                        }
                        let mut bindings = HashMap::new();
                        if i != usize::MAX
                            && !arm
                                .bindings
                                .iter()
                                .zip(&payload)
                                .all(|(p, v)| pattern_value(p, v, &mut bindings))
                        {
                            continue;
                        }
                        f.scopes.push(bindings);
                        let result = (|| {
                            if let Some(g) = &arm.guard {
                                if !self.eval(f, g, Some(&Type::Bool))?.boolean()? {
                                    return Ok(None);
                                }
                            }
                            self.statements(f, &arm.body).map(Some)
                        })();
                        f.scopes.pop();
                        if let Some(flow) = result? {
                            if !matches!(flow, Flow::Next) {
                                return Ok(flow);
                            }
                            break;
                        }
                    }
                }
                Stmt::Unsafe(body) => {
                    f.unsafe_depth += 1;
                    let result = self.block(f, body);
                    f.unsafe_depth -= 1;
                    let flow = result?;
                    if !matches!(flow, Flow::Next) {
                        return Ok(flow);
                    }
                }
                Stmt::Block(body) => {
                    let flow = self.block(f, body)?;
                    if !matches!(flow, Flow::Next) {
                        return Ok(flow);
                    }
                }
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
                    if self.json_assign(f, target, op, value)? {
                        continue;
                    }
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
    fn native_target(&mut self, f: &mut Frame, e: &Expr) -> Result<Option<(usize, Type)>, String> {
        if let ExprKind::Field(base, name) = &e.kind {
            if let Some((p, t)) = self.native_target(f, base)? {
                let (offset, t) = crate::memory::field_offset(&t, name)?;
                return Ok(Some((
                    p.checked_add(offset).ok_or("native address overflow")?,
                    t,
                )));
            }
            return Ok(None);
        }
        let (pointer, index) = match &e.kind {
            ExprKind::Unary(op, a) if op == "*" => (self.eval(f, a, None)?, 0i128),
            ExprKind::Index(a, i) => {
                if let ExprKind::Name(n) = &a.kind {
                    if n.len() == 1
                        && f.scopes
                            .iter()
                            .rev()
                            .find_map(|s| s.get(&n[0]))
                            .is_some_and(|v| !matches!(v, Value::Ptr(..)))
                    {
                        return Ok(None);
                    }
                }
                let a = self.eval(f, a, None)?;
                if !matches!(a, Value::Ptr(..)) {
                    return Ok(None);
                };
                (a, self.eval(f, i, None)?.integer()?)
            }
            _ => return Ok(None),
        };
        let Value::Ptr(p, Type::Ptr(t)) = pointer else {
            return Err("dereference requires a raw pointer".into());
        };
        if f.unsafe_depth == 0 {
            return Err("raw pointer access requires an unsafe block".into());
        }
        let t = if let Type::Nominal(n) = &*t {
            self.modules
                .values()
                .flat_map(|m| &m.module.concrete_types)
                .find(|t| t.name() == *n)
                .cloned()
                .ok_or("unknown pointer type")?
        } else {
            *t
        };
        let (size, _) = crate::memory::layout(&t)?;
        let p = (p as i128)
            .checked_add(
                index
                    .checked_mul(size as i128)
                    .ok_or("native address overflow")?,
            )
            .ok_or("native address overflow")?;
        Ok(Some((
            usize::try_from(p).map_err(|_| "native address overflow")?,
            t,
        )))
    }
    fn writable<'a>(
        &mut self,
        f: &'a mut Frame,
        e: &Expr,
    ) -> Result<Option<&'a mut Value>, String> {
        fn path(
            engine: &mut Engine,
            f: &mut Frame,
            e: &Expr,
            steps: &mut Vec<Result<String, usize>>,
        ) -> Result<Option<String>, String> {
            match &e.kind {
                ExprKind::Name(n) if n.len() == 1 => Ok(Some(n[0].clone())),
                ExprKind::Field(base, n) => {
                    let name = path(engine, f, base, steps)?;
                    steps.push(Ok(n.clone()));
                    Ok(name)
                }
                ExprKind::Index(base, i) => {
                    let name = path(engine, f, base, steps)?;
                    let i = usize::try_from(engine.eval(f, i, None)?.integer()?)
                        .map_err(|_| "invalid index")?;
                    steps.push(Err(i));
                    Ok(name)
                }
                _ => Ok(None),
            }
        }
        let mut steps = vec![];
        let Some(name) = path(self, f, e, &mut steps)? else {
            return Ok(None);
        };
        let mut dest = f.scopes.iter_mut().rev().find_map(|s| s.get_mut(&name));
        for step in steps {
            dest = match (dest, step) {
                (Some(Value::Record(fs, _)), Ok(n)) => {
                    fs.iter_mut().find(|(name, _)| *name == n).map(|(_, v)| v)
                }
                (Some(Value::Vector(xs, _)), Err(i)) => Arc::make_mut(xs).get_mut(i),
                (Some(Value::Array(xs, _)), Err(i)) => xs.get_mut(i),
                _ => None,
            }
        }
        Ok(dest)
    }
    fn assign(&mut self, f: &mut Frame, e: &Expr, v: Value) -> Result<(), String> {
        if let Some((p, _)) = self.native_target(f, e)? {
            fn strings(v: &Value) -> bool {
                match v {
                    Value::Str(_) => true,
                    Value::Record(fs, _) => fs.iter().any(|(_, v)| strings(v)),
                    Value::Array(xs, _) => xs.iter().any(strings),
                    _ => false,
                }
            }
            if strings(&v) {
                return Err("native str writes require an explicit native allocation".into());
            }
            // The enclosing unsafe block provides allocation validity; layout is checked.
            return unsafe { crate::memory::write(p, &v, &mut vec![]) }
                .map_err(|message| self.located(&f.module, e.span, message));
        }
        enum Access {
            Field(String),
            Index(usize),
        }
        fn path(
            engine: &mut Engine,
            frame: &mut Frame,
            expr: &Expr,
            access: &mut Vec<Access>,
        ) -> Result<String, String> {
            match &expr.kind {
                ExprKind::Name(n) if n.len() == 1 => Ok(n[0].clone()),
                ExprKind::Field(base, name) => {
                    let root = path(engine, frame, base, access)?;
                    access.push(Access::Field(name.clone()));
                    Ok(root)
                }
                ExprKind::Index(base, index) => {
                    let root = path(engine, frame, base, access)?;
                    let index = usize::try_from(engine.eval(frame, index, None)?.integer()?)
                        .map_err(|_| engine.located(&frame.module, expr.span, "invalid index"))?;
                    access.push(Access::Index(index));
                    Ok(root)
                }
                _ => Err(engine.located(
                    &frame.module,
                    expr.span,
                    "assignment requires a local variable",
                )),
            }
        }
        if matches!(e.kind, ExprKind::Field(..) | ExprKind::Index(..)) {
            let mut access = vec![];
            let root = path(self, f, e, &mut access)?;
            let mut dest = f.scopes.iter_mut().rev().find_map(|s| s.get_mut(&root));
            for step in access {
                dest = match (dest, step) {
                    (Some(Value::Record(fields, _)), Access::Field(name)) => {
                        fields.iter_mut().find(|(n, _)| *n == name).map(|(_, v)| v)
                    }
                    (Some(Value::Vector(items, _)), Access::Index(index)) => {
                        Arc::make_mut(items).get_mut(index)
                    }
                    (Some(Value::Array(items, _)), Access::Index(index)) => items.get_mut(index),
                    _ => None,
                };
            }
            if let Some(dest) = dest {
                *dest = v;
                return Ok(());
            }
            return Err(self.located(
                &f.module,
                e.span,
                "invalid array assignment or index out of bounds",
            ));
        }
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
    fn resolve_collection_element(&self, t: &Type) -> Result<Type, String> {
        if let Type::Nominal(n) = t {
            self.modules
                .values()
                .flat_map(|m| &m.module.concrete_types)
                .find(|t| t.name() == *n)
                .cloned()
                .ok_or_else(|| "unknown collection element type".into())
        } else {
            Ok(t.clone())
        }
    }
    fn map_call(
        &mut self,
        f: &mut Frame,
        base: &Expr,
        op: &str,
        args: &[Expr],
        value: Value,
    ) -> Result<Value, String> {
        let ty = value.ty();
        let Type::Map(k, v) = &ty else { unreachable!() };
        let value_type = self.resolve_collection_element(v)?;
        let count = match op {
            "len" | "clear" => 0,
            "set" => 2,
            "get" | "contains" | "remove" => 1,
            _ => return Err("unknown Map method".into()),
        };
        if args.len() != count {
            return Err("wrong collection argument count".into());
        }
        let key = if count > 0 {
            Some(self.eval(f, &args[0], Some(k))?)
        } else {
            None
        };
        let Value::Map(entries, _) = &value else {
            unreachable!()
        };
        let index = key.as_ref().and_then(|key| entries.find(key));
        match op {
            "len" => {
                return Ok(Value::Int(
                    entries.entries.len() as i128,
                    Type::Size { signed: false },
                ))
            }
            "contains" => return Ok(Value::Bool(index.is_some())),
            "get" => {
                return index
                    .map(|i| entries.entries[i].1.clone())
                    .ok_or_else(|| "Map key not found".into())
            }
            _ => {}
        }
        drop(value);
        let new = if op == "set" {
            Some(self.eval(f, &args[1], Some(&value_type))?)
        } else {
            None
        };
        let Some(Value::Map(entries, _)) = self.writable(f, base)? else {
            return Err("mutable Map method requires writable Map".into());
        };
        let entries = Arc::make_mut(entries);
        match op {
            "set" => {
                entries.set(key.unwrap(), new.unwrap())?;
                Ok(Value::Void)
            }
            "clear" => {
                entries.entries.clear();
                entries.buckets.clear();
                Ok(Value::Void)
            }
            "remove" => Ok(Value::Bool(entries.remove(key.as_ref().unwrap()))),
            _ => unreachable!(),
        }
    }

    fn collection_call(
        &mut self,
        f: &mut Frame,
        base: &Expr,
        op: &str,
        args: &[Expr],
    ) -> Result<Value, String> {
        let value = self.eval(f, base, None)?;
        let ty = value.ty();
        if matches!(ty, Type::Map(..)) {
            return self.map_call(f, base, op, args, value);
        }
        let (Type::Vector(inner) | Type::Slice(inner)) = &ty else {
            return Err("collection method requires Vec or Slice".into());
        };
        let element = self.resolve_collection_element(inner)?;
        let count = match op {
            "push" => 1,
            "slice" => 2,
            "len" | "pop" | "clear" => 0,
            _ => return Err("unknown collection method".into()),
        };
        if args.len() != count {
            return Err("wrong collection argument count".into());
        }
        let len = match &value {
            Value::Vector(xs, _) => xs.len(),
            Value::Slice(_, _, n, _) => *n,
            _ => unreachable!(),
        };
        if op == "len" {
            return Ok(Value::Int(len as i128, Type::Size { signed: false }));
        }
        if op == "slice" {
            let a = usize::try_from(self.eval(f, &args[0], None)?.integer()?)
                .map_err(|_| "slice range out of bounds")?;
            let b = usize::try_from(self.eval(f, &args[1], None)?.integer()?)
                .map_err(|_| "slice range out of bounds")?;
            if a > b || b > len {
                return Err("slice range out of bounds".into());
            }
            let (xs, offset) = match value {
                Value::Vector(xs, _) => (xs, 0),
                Value::Slice(xs, o, _, _) => (xs, o),
                _ => unreachable!(),
            };
            return Ok(Value::Slice(
                xs,
                offset + a,
                b - a,
                Type::Slice(inner.clone()),
            ));
        }
        if !matches!(ty, Type::Vector(_)) {
            return Err("mutable collection method requires a writable Vec".into());
        }
        drop(value);
        let item = if op == "push" {
            Some(self.eval(f, &args[0], Some(&element))?)
        } else {
            None
        };
        let Some(Value::Vector(xs, _)) = self.writable(f, base)? else {
            return Err("mutable collection method requires a writable Vec".into());
        };
        let xs = Arc::make_mut(xs);
        match op {
            "push" => {
                xs.try_reserve(1)
                    .map_err(|_| "collection allocation failed")?;
                xs.push(item.unwrap());
                Ok(Value::Void)
            }
            "clear" => {
                xs.clear();
                Ok(Value::Void)
            }
            "pop" => xs.pop().ok_or_else(|| "pop from empty Vec".into()),
            _ => unreachable!(),
        }
    }
    fn json_expr(
        &mut self,
        f: &mut Frame,
        e: &Expr,
        depth: usize,
    ) -> Result<serde_json::Value, String> {
        if depth > 128 {
            return Err("JSON nesting limit exceeded".into());
        }
        match &e.kind {
            ExprKind::Object(_, fields) => {
                let mut result = serde_json::Map::new();
                for (key, value) in fields {
                    result.insert(key.clone(), self.json_expr(f, value, depth + 1)?);
                }
                Ok(serde_json::Value::Object(result))
            }
            ExprKind::Array(values) => {
                let mut result = Vec::new();
                for value in values {
                    result.push(self.json_expr(f, value, depth + 1)?);
                }
                Ok(serde_json::Value::Array(result))
            }
            _ => crate::json::encode(&self.eval(f, e, None)?, depth),
        }
    }
    fn json_assign(
        &mut self,
        f: &mut Frame,
        target: &Expr,
        op: &str,
        rhs: &Expr,
    ) -> Result<bool, String> {
        fn root(e: &Expr) -> Option<&str> {
            match &e.kind {
                ExprKind::Name(n) if n.len() == 1 => Some(&n[0]),
                ExprKind::Field(base, _) | ExprKind::Index(base, _) => root(base),
                _ => None,
            }
        }
        if !matches!(target.kind, ExprKind::Field(..) | ExprKind::Index(..)) {
            return Ok(false);
        }
        let Some(name) = root(target) else {
            return Ok(false);
        };
        let Some(Value::Json(node, ty)) = f.scopes.iter().rev().find_map(|s| s.get(name)).cloned()
        else {
            return Ok(false);
        };
        fn path(
            engine: &mut Engine,
            f: &mut Frame,
            e: &Expr,
            steps: &mut Vec<crate::json::Access>,
        ) -> Result<(), String> {
            match &e.kind {
                ExprKind::Field(base, key) => {
                    path(engine, f, base, steps)?;
                    steps.push(crate::json::Access::Key(key.clone()));
                }
                ExprKind::Index(base, index) => {
                    path(engine, f, base, steps)?;
                    steps.push(crate::json::access(engine.eval(f, index, None)?)?);
                }
                _ => {}
            }
            Ok(())
        }
        let result = (|| {
            let mut steps = Vec::new();
            path(self, f, target, &mut steps)?;
            let replacement = if op == "=" {
                self.json_expr(f, rhs, 0)?
            } else {
                let old =
                    crate::json::unwrap(crate::json::read(&node, &steps)?.clone(), ty.clone());
                let value = self.eval(f, rhs, Some(&old.ty()))?;
                crate::json::encode(&self.binary(op.trim_end_matches('='), old, value)?, 0)?
            };
            let mut updated = (*node).clone();
            crate::json::write(&mut updated, &steps, replacement)?;
            *f.scopes
                .iter_mut()
                .rev()
                .find_map(|s| s.get_mut(name))
                .unwrap() = Value::Json(Arc::new(updated), ty);
            Ok(true)
        })();
        result.map_err(|e: String| self.located(&f.module, target.span, e))
    }
    fn eval(&mut self, f: &mut Frame, e: &Expr, hint: Option<&Type>) -> Result<Value, String> {
        let result = (|| {
            let v = match &e.kind {
                ExprKind::Object(ty, _) => {
                    Value::Json(Arc::new(self.json_expr(f, e, 0)?), ty.clone())
                }
                ExprKind::SizeOf(t) => Value::Int(
                    crate::memory::object_layout(t)?.0 as i128,
                    Type::Size { signed: false },
                ),
                ExprKind::Closure(..) => unreachable!("unexpanded closure"),
                ExprKind::Callable(names, context, t) => {
                    let (module, name) = self.resolve(&f.module, names)?;
                    let context = if let Some(e) = context {
                        let Value::Ref(v, _) = self.eval(f, e, None)? else {
                            unreachable!()
                        };
                        Some(v)
                    } else {
                        None
                    };
                    Value::Callable(module, name, context, t.clone())
                }
                ExprKind::Invoke(base, args) => {
                    let callable = self.eval(f, base, None)?;
                    if let Value::Ptr(p, t @ Type::Callback(ps, _)) = &callable {
                        if f.unsafe_depth == 0 {
                            return Err("callback invocation requires an unsafe block".into());
                        }
                        if ps.len() != args.len() {
                            return Err("wrong argument count".into());
                        }
                        let mut values = vec![];
                        for (a, t) in args.iter().zip(ps) {
                            values.push(self.eval(f, a, Some(t))?)
                        }
                        return self.native.call_pointer(*p, t, &values);
                    }
                    let Value::Callable(module, name, context, Type::Function(ps, _)) = callable
                    else {
                        return Err("value is not callable".into());
                    };
                    if ps.len() != args.len() {
                        return Err("wrong argument count".into());
                    }
                    let mut values = vec![];
                    if let Some(v) = context {
                        let inner = match v.ty() {
                            Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                            t => t,
                        };
                        let t = Type::Ref(Box::new(inner));
                        values.push(Value::Ref(v, t));
                    }
                    for (a, t) in args.iter().zip(&ps) {
                        values.push(self.eval(f, a, Some(t))?);
                    }
                    self.call(&module, &[name], values, e.span)?
                }
                ExprKind::Map(t) => Value::Map(Arc::new(MapStorage::default()), t.clone()),
                ExprKind::Vector(t, args) => {
                    let Type::Vector(inner) = t else {
                        unreachable!()
                    };
                    let element = self.resolve_collection_element(inner)?;
                    let mut values = vec![];
                    values
                        .try_reserve(args.len())
                        .map_err(|_| "collection allocation failed")?;
                    for a in args {
                        values.push(self.eval(f, a, Some(&element))?);
                    }
                    Value::Vector(Arc::new(values), t.clone())
                }
                ExprKind::Collection(base, op, args) => self.collection_call(f, base, op, args)?,
                ExprKind::Reference(a) => {
                    let v = self.eval(f, a, None)?;
                    if matches!(v.ty(), Type::Void | Type::Array(..)) {
                        return Err("ref requires a non-array value".into());
                    }
                    let inner = match v.ty() {
                        Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                        t => t,
                    };
                    Value::Ref(Arc::new(v), Type::Ref(Box::new(inner)))
                }
                ExprKind::Dereference(a) => {
                    let Value::Ref(value, _) = self.eval(f, a, None)? else {
                        return Err("deref requires a managed Ref".into());
                    };
                    (*value).clone()
                }
                ExprKind::Record(ty, args) => {
                    let Type::Record(_, fields) = ty else {
                        unreachable!()
                    };
                    if args.len() != fields.len() {
                        return Err("wrong struct field count".into());
                    }
                    let values = fields
                        .iter()
                        .zip(args)
                        .map(|((n, t), e)| Ok((n.clone(), self.eval(f, e, Some(t))?)))
                        .collect::<Result<Vec<_>, String>>()?;
                    Value::Record(values, ty.clone())
                }
                ExprKind::Enum(ty, n, args) => {
                    let Type::Enum(_, vs) = ty else {
                        unreachable!()
                    };
                    let ts = &vs[*n].1;
                    if ts.len() != args.len() {
                        return Err("wrong enum payload argument count".into());
                    }
                    let mut payload = vec![];
                    for (t, a) in ts.iter().zip(args) {
                        payload.push(self.eval(f, a, Some(t))?);
                    }
                    Value::Enum(*n, ty.clone(), payload)
                }
                ExprKind::Field(base, name) => {
                    if let Some((p, t)) = self.native_target(f, e)? {
                        unsafe { crate::memory::read(p, &t) }?
                    } else {
                        match self.eval(f, base, None)? {
                            Value::Json(node, ty) => crate::json::unwrap(
                                crate::json::read(
                                    &node,
                                    &[crate::json::Access::Key(name.clone())],
                                )?
                                .clone(),
                                ty,
                            ),
                            Value::Record(fields, _) => fields
                                .into_iter()
                                .find(|(n, _)| n == name)
                                .map(|(_, v)| v)
                                .ok_or_else(|| format!("unknown struct field {name}"))?,
                            _ => return Err("field access requires a struct".into()),
                        }
                    }
                }
                ExprKind::GenericCall(..) => return Err("unresolved generic call".into()),
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
                    if n == &["null"] {
                        return Ok(Value::Ptr(
                            0,
                            hint.filter(|t| matches!(t, Type::Ptr(_)))
                                .cloned()
                                .unwrap_or(Type::Ptr(Box::new(Type::Void))),
                        ));
                    }
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
                ExprKind::Cast(a, t) => {
                    let value = self.eval(f, a, None)?;
                    if matches!(value, Value::Ptr(..)) && f.unsafe_depth == 0 {
                        return Err("raw pointer casts require an unsafe block".into());
                    }
                    convert(value, t, true)?
                }
                ExprKind::Unary(op, a) => {
                    if op == "*" {
                        let (p, t) = self
                            .native_target(f, e)?
                            .ok_or("dereference requires pointer")?;
                        return unsafe { crate::memory::read(p, &t) };
                    }
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
                    let pointer_math =
                        matches!(left, Value::Ptr(..)) && ["+", "-"].contains(&op.as_str());
                    if pointer_math && f.unsafe_depth == 0 {
                        return Err("pointer arithmetic requires an unsafe block".into());
                    }
                    let left_type = left.ty();
                    let right =
                        self.eval(f, b, if pointer_math { None } else { Some(&left_type) })?;
                    self.binary(op, left, right)?
                }
                ExprKind::Call(n, arguments) => {
                    let (target, name) = self.resolve(&f.module, n)?;
                    let params = self
                        .modules
                        .get(&target)
                        .and_then(|m| m.functions.get(&name))
                        .map(|fun| fun.params.clone());
                    let mut args = vec![];
                    for (i, a) in arguments.iter().enumerate() {
                        args.push(self.eval(
                            f,
                            a,
                            if n.as_slice() == ["volatile_store"] && i == 1 {
                                if let Some(Value::Ptr(_, Type::Ptr(t))) = args.first() {
                                    Some(t.as_ref())
                                } else {
                                    None
                                }
                            } else {
                                params.as_ref().and_then(|p| p.get(i).map(|(_, t)| t))
                            },
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
                    // Reading one named element must not deep-copy the entire array.
                    // Probe the base first, preserving base-before-index evaluation.
                    if let ExprKind::Name(name) = &a.kind {
                        if name.len() == 1
                            && f.scopes
                                .iter()
                                .rev()
                                .find_map(|s| s.get(&name[0]))
                                .is_some_and(|v| {
                                    matches!(
                                        v,
                                        Value::Array(..)
                                            | Value::Str(_)
                                            | Value::Vector(..)
                                            | Value::Slice(..)
                                    )
                                })
                        {
                            let index = self.eval(f, i, None)?.integer()?;
                            let index = usize::try_from(index)
                                .map_err(|_| "negative or excessive index")?;
                            let base = f.scopes.iter().rev().find_map(|s| s.get(&name[0])).unwrap();
                            let value = match base {
                                Value::Array(items, _) => items
                                    .get(index)
                                    .cloned()
                                    .ok_or("array index out of bounds")?,
                                Value::Vector(xs, _) => {
                                    xs.get(index).cloned().ok_or("array index out of bounds")?
                                }
                                Value::Slice(xs, offset, len, _) => {
                                    if index >= *len {
                                        return Err("array index out of bounds".into());
                                    }
                                    xs[*offset + index].clone()
                                }
                                Value::Str(text) => Value::Int(
                                    *text
                                        .as_bytes()
                                        .get(index)
                                        .ok_or("string index out of bounds")?
                                        as i128,
                                    Type::Int {
                                        bits: 8,
                                        signed: false,
                                    },
                                ),
                                _ => unreachable!(),
                            };
                            return if let Some(t) = hint {
                                convert(value, t, false)
                            } else {
                                Ok(value)
                            };
                        }
                    }
                    let a = self.eval(f, a, None)?;
                    let index = self.eval(f, i, None)?;
                    if let Value::Json(node, ty) = a {
                        let value = crate::json::unwrap(
                            crate::json::read(&node, &[crate::json::access(index)?])?.clone(),
                            ty,
                        );
                        return if let Some(t) = hint {
                            convert(value, t, false)
                        } else {
                            Ok(value)
                        };
                    }
                    let i = index.integer()?;
                    let i = usize::try_from(i).map_err(|_| "negative or excessive index")?;
                    match a {
                        Value::Ptr(p, Type::Ptr(t)) => {
                            if f.unsafe_depth == 0 {
                                return Err("raw pointer access requires an unsafe block".into());
                            }
                            let t = if let Type::Nominal(n) = &*t {
                                self.modules
                                    .values()
                                    .flat_map(|m| &m.module.concrete_types)
                                    .find(|t| t.name() == *n)
                                    .cloned()
                                    .ok_or("unknown pointer type")?
                            } else {
                                *t
                            };
                            let (size, _) = crate::memory::layout(&t)?;
                            let p = p
                                .checked_add(i.checked_mul(size).ok_or("native address overflow")?)
                                .ok_or("native address overflow")?;
                            unsafe { crate::memory::read(p, &t) }?
                        }
                        Value::Vector(xs, _) => {
                            xs.get(i).cloned().ok_or("array index out of bounds")?
                        }
                        Value::Slice(xs, offset, len, _) => {
                            if i >= len {
                                return Err("array index out of bounds".into());
                            }
                            xs[offset + i].clone()
                        }
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
        if let (Value::Ptr(p, Type::Ptr(t)), Value::Int(offset, _)) = (&a, &b) {
            if op == "+" || op == "-" {
                let t = if let Type::Nominal(n) = &**t {
                    self.modules
                        .values()
                        .flat_map(|m| &m.module.concrete_types)
                        .find(|t| t.name() == *n)
                        .cloned()
                        .ok_or("unknown pointer type")?
                } else {
                    (**t).clone()
                };
                let (size, _) = crate::memory::layout(&t)?;
                let delta = offset
                    .checked_mul(size as i128)
                    .ok_or("pointer arithmetic overflow")?;
                let address = if op == "+" {
                    (*p as i128).checked_add(delta)
                } else {
                    (*p as i128).checked_sub(delta)
                }
                .ok_or("pointer arithmetic overflow")?;
                return Ok(Value::Ptr(
                    usize::try_from(address).map_err(|_| "pointer arithmetic overflow")?,
                    a.ty(),
                ));
            }
        }
        if a.ty() != b.ty() {
            return Err("binary operands must have matching types".into());
        }
        let t = a.ty();
        match (a, b) {
            (Value::Enum(a,Type::Enum(_,vs),_),Value::Enum(b,_,_)) if vs.iter().all(|(_,ts)| ts.is_empty()) =>match op {"=="=>Ok(Value::Bool(a==b)),"!="=>Ok(Value::Bool(a!=b)),_=>Err("enums only support equality".into())},
            (Value::Ptr(a,_),Value::Ptr(b,_))=>match op {
                "=="=>Ok(Value::Bool(a==b)),"!="=>Ok(Value::Bool(a!=b)),
                _=>Err("opaque native pointers only support equality; use native functions for memory access".into()),
            },
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
        if [
            "std/sync",
            "std/tls", "std/websocket",
            "std/sqlite", "std/csv", "std/toml", "std/yaml",
            "std/result", "std/timers", "std/child_process",
            "std/dns", "std/cli",
            "std/regex", "std/encoding", "std/crypto", "std/compression", "std/archive", "std/uuid",
            "std/math", "std/random", "std/strings", "std/datetime", "std/test", "std/log",
            "std/net",
            "std/path",
            "std/os",
            "std/stream",
            "std/url",
            "std/module",
            "std/process",
            "std/events",
            "std/buffer",
            "std/dgram",
        ]
        .contains(&module)
        {
            return self
                .core_call(module, name, args)
                .map_err(|e| format!("{module}.{name}: {e}"));
        }
        if module == "std/fs" {
            return crate::filesystem::call(name, args);
        }
        if module == "std/http" {
            if name == "createServer"
                || name.starts_with("method_Server_")
                || name.starts_with("method_ServerResponse_")
            {
                return self.server_call(name, args);
            }
            let ty = self.modules[module].functions["get"].ret.clone();
            return crate::http::call(name, args, ty, &mut self.http);
        }
        if module == "std/json" || module == "std/don" {
            let ty = self.modules[module]
                .module
                .concrete_types
                .first()
                .ok_or("JSON value type unavailable")?
                .clone();
            if module == "std/don" {
                let name = if name.starts_with("g_stringify_") {
                    "stringify"
                } else if name.starts_with("g_toJSON_") {
                    "toJSON"
                } else {
                    name
                };
                if matches!(name, "parse" | "valid" | "fromJSON") {
                    let [Value::Str(text)] = args.as_slice() else {
                        return Err("don function expects one string".into());
                    };
                    if name == "fromJSON" {
                        if text.len() > 8 * 1024 * 1024 { return Err("DON input exceeds 8 MiB".into()); }
                        return crate::json::call("parse", args, ty);
                    }
                    let value = dev_syntax::don::parse(text);
                    if name == "valid" {
                        return Ok(Value::Bool(value.is_ok()));
                    }
                    return Ok(Value::Json(Arc::new(value?), ty));
                }
                if matches!(name, "stringify" | "toJSON") {
                    if args.len() != 1 {
                        return Err("don stringify expects one value".into());
                    }
                    let value = crate::json::encode(&args[0], 0)?;
                    return Ok(Value::Str(if name == "toJSON" {
                        serde_json::to_string(&value).map_err(|e| e.to_string())?
                    } else {
                        dev_syntax::don::stringify(&value)?
                    }));
                }
                return crate::json::call(name, args, ty);
            }
            return crate::json::call(name, args, ty);
        }
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
                self.native.flush_stdio();
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

fn pattern_value(p: &Pattern, v: &Value, bindings: &mut HashMap<String, Value>) -> bool {
    match p {
        Pattern::Bind(n) => {
            if let Value::Enum(i, Type::Enum(_, vs), _) = v {
                if vs.iter().any(|(name, _)| name == n) {
                    return &vs[*i].0 == n;
                }
            }
            if n != "_" {
                bindings.insert(n.clone(), v.clone());
            }
            true
        }
        Pattern::Variant(name, ps) => {
            let Value::Enum(i, Type::Enum(_, vs), xs) = v else {
                return false;
            };
            &vs[*i].0 == name
                && ps
                    .iter()
                    .zip(xs)
                    .all(|(p, v)| pattern_value(p, v, bindings))
        }
    }
}
