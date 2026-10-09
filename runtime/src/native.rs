use crate::engine::Value;
use dev_syntax::ast::{Function, Type};
use libffi::middle::{arg, ret, Arg, Cif, CodePtr, Type as FfiType};
use libloading::Library;
use std::{
    collections::{HashMap, HashSet},
    ffi::{c_void, CStr, CString},
    path::PathBuf,
};

pub struct Native {
    pub(crate) callbacks: std::sync::Arc<std::sync::Mutex<Vec<CallbackStorage>>>,
    libraries: Vec<Library>,
    signatures: HashMap<String, (Vec<Type>, Type, bool)>,
    bindings: HashMap<String, (Cif, CodePtr)>,
    system_loaded: bool,
    flushers: Vec<unsafe extern "C" fn(*mut c_void) -> i32>,
    directories: Vec<PathBuf>,
    tried: HashSet<PathBuf>,
    discovery_errors: Vec<String>,
    prepared: HashSet<PathBuf>,
}

fn ffi_type(t: &Type) -> Result<FfiType, String> {
    Ok(match t {
        Type::Void => FfiType::void(),
        Type::Bool => FfiType::u8(),
        Type::Int {
            bits: 8,
            signed: true,
        } => FfiType::i8(),
        Type::Int {
            bits: 8,
            signed: false,
        } => FfiType::u8(),
        Type::Int {
            bits: 16,
            signed: true,
        } => FfiType::i16(),
        Type::Int {
            bits: 16,
            signed: false,
        } => FfiType::u16(),
        Type::Int {
            bits: 32,
            signed: true,
        } => FfiType::i32(),
        Type::Int {
            bits: 32,
            signed: false,
        } => FfiType::u32(),
        Type::Int {
            bits: 64,
            signed: true,
        } => FfiType::i64(),
        Type::Int {
            bits: 64,
            signed: false,
        } => FfiType::u64(),
        Type::Size { signed: true } => FfiType::isize(),
        Type::Size { signed: false } => FfiType::usize(),
        Type::Float(32) => FfiType::f32(),
        Type::Float(64) => FfiType::f64(),
        Type::Record(_, fields) => {
            let types = if fields.is_empty() {
                vec![FfiType::u8()]
            } else {
                fields
                    .iter()
                    .map(|(_, t)| ffi_type(t))
                    .collect::<Result<Vec<_>, _>>()?
            };
            FfiType::structure(types)
        }
        Type::Array(t, n) if *n <= 1024 => FfiType::structure(
            (0..*n)
                .map(|_| ffi_type(t))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Type::Enum(_, vs) if vs.iter().all(|(_, ts)| ts.is_empty()) => FfiType::i32(),
        Type::Callback(..) | Type::Str | Type::Ptr(_) => FfiType::pointer(),
        _ => {
            return Err(format!(
                "unsupported native FFI type {}; pass arrays through pointers",
                t.name()
            ))
        }
    })
}

enum Slot {
    Aggregate(crate::memory::Buffer),
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Size(usize),
    SignedSize(isize),
    F32(f32),
    F64(f64),
    Pointer(*mut c_void),
    String {
        _owned: CString,
        pointer: *const std::ffi::c_char,
    },
}
impl Slot {
    fn from_value(v: &Value) -> Result<Self, String> {
        Ok(match v {
            Value::Record(..) => Self::Aggregate(crate::memory::Buffer::from_value(v)?),
            Value::Enum(n, _, payload) if payload.is_empty() => Self::I32(*n as i32),
            Value::Int(n, t) => match t {
                Type::Int {
                    bits: 8,
                    signed: true,
                } => Self::I8(*n as i8),
                Type::Int {
                    bits: 8,
                    signed: false,
                } => Self::U8(*n as u8),
                Type::Int {
                    bits: 16,
                    signed: true,
                } => Self::I16(*n as i16),
                Type::Int {
                    bits: 16,
                    signed: false,
                } => Self::U16(*n as u16),
                Type::Int {
                    bits: 32,
                    signed: true,
                } => Self::I32(*n as i32),
                Type::Int {
                    bits: 32,
                    signed: false,
                } => Self::U32(*n as u32),
                Type::Int {
                    bits: 64,
                    signed: true,
                } => Self::I64(*n as i64),
                Type::Int {
                    bits: 64,
                    signed: false,
                } => Self::U64(*n as u64),
                Type::Size { signed: true } => Self::SignedSize(*n as isize),
                Type::Size { signed: false } => Self::Size(*n as usize),
                _ => return Err("unsupported integer ABI".into()),
            },
            Value::Float(n, Type::Float(32)) => Self::F32(*n as f32),
            Value::Float(n, _) => Self::F64(*n),
            Value::Bool(b) => Self::U8(u8::from(*b)),
            Value::Ptr(p, _) => Self::Pointer(*p as *mut c_void),
            Value::Str(s) => {
                let owned = CString::new(s.as_bytes())
                    .map_err(|_| "native str arguments cannot contain NUL bytes")?;
                let pointer = owned.as_ptr();
                Self::String {
                    _owned: owned,
                    pointer,
                }
            }
            _ => return Err("unsupported native argument".into()),
        })
    }
    fn argument(&self) -> Arg<'_> {
        match self {
            Self::Aggregate(b) => arg(&b.words[0]),
            Self::I8(v) => arg(v),
            Self::U8(v) => arg(v),
            Self::I16(v) => arg(v),
            Self::U16(v) => arg(v),
            Self::I32(v) => arg(v),
            Self::U32(v) => arg(v),
            Self::I64(v) => arg(v),
            Self::U64(v) => arg(v),
            Self::Size(v) => arg(v),
            Self::SignedSize(v) => arg(v),
            Self::F32(v) => arg(v),
            Self::F64(v) => arg(v),
            Self::Pointer(v) => arg(v),
            Self::String { pointer, .. } => arg(pointer),
        }
    }
}
impl Native {
    pub fn flush_stdio(&self) {
        for flush in &self.flushers {
            // These pointers come only from the system CRTs and outlive every call.
            unsafe {
                flush(std::ptr::null_mut());
            }
        }
    }
    pub fn new(paths: &[PathBuf]) -> Result<Self, String> {
        let mut libraries = Vec::new();
        let mut tried = HashSet::new();
        for path in paths {
            let absolute = path
                .canonicalize()
                .map_err(|e| format!("cannot load native library {}: {e}", path.display()))?;
            // A library can run initialization code. Paths here are explicit CLI inputs.
            libraries.push(
                unsafe { Library::new(&absolute) }.map_err(|e| {
                    format!("cannot load native library {}: {e}", absolute.display())
                })?,
            );
            tried.insert(absolute);
        }
        Ok(Self {
            callbacks: std::sync::Arc::new(std::sync::Mutex::new(vec![])),
            libraries,
            signatures: HashMap::new(),
            bindings: HashMap::new(),
            system_loaded: false,
            flushers: Vec::new(),
            directories: Vec::new(),
            tried,
            discovery_errors: Vec::new(),
            prepared: HashSet::new(),
        })
    }
    pub fn add_directory(&mut self, path: &std::path::Path) {
        if !self.directories.iter().any(|p| p == path) {
            self.directories.push(path.to_path_buf());
        }
    }
    fn discover(&mut self) -> Result<(), String> {
        for directory in &self.directories {
            let entries = std::fs::read_dir(directory).map_err(|e| {
                format!(
                    "cannot search native libraries in {}: {e}",
                    directory.display()
                )
            })?;
            let mut paths = Vec::new();
            for entry in entries {
                let path = entry.map_err(|e| e.to_string())?.path();
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let eligible = if cfg!(windows) {
                    name.to_ascii_lowercase().ends_with(".dll")
                } else if cfg!(target_os = "macos") {
                    name.ends_with(".dylib")
                } else {
                    name.ends_with(".so") || name.contains(".so.")
                };
                if eligible && path.is_file() {
                    paths.push(path);
                }
            }
            paths.sort();
            for path in paths {
                let path = path.canonicalize().map_err(|e| e.to_string())?;
                if !self.tried.insert(path.clone()) {
                    continue;
                }
                // Native FFI execution authorizes loading libraries adjacent to the
                // declaring source module. Discovery never compiles source files.
                match unsafe { Library::new(&path) } {
                    Ok(library) => self.libraries.push(library),
                    Err(e) => self
                        .discovery_errors
                        .push(format!("{}: {e}", path.display())),
                }
            }
        }
        Ok(())
    }
    pub fn declare(&mut self, f: &Function) -> Result<(), String> {
        if matches!(f.ret, Type::Array(..)) {
            return Err("unsupported native FFI type: by-value array return".into());
        }
        ffi_type(&f.ret)?;
        let params: Vec<Type> = f.params.iter().map(|(_, t)| t.clone()).collect();
        for t in &params {
            if matches!(t, Type::Array(..)) {
                return Err("unsupported native FFI type: pass arrays through pointers".into());
            }
            if *t == Type::Void {
                return Err("native parameters cannot be void".into());
            }
            ffi_type(t)?;
        }
        if let Some(old) = self.signatures.get(&f.name) {
            if old != &(params.clone(), f.ret.clone(), f.variadic) {
                return Err(format!("conflicting native signatures for {}", f.name));
            }
        }
        self.signatures
            .insert(f.name.clone(), (params, f.ret.clone(), f.variadic));
        Ok(())
    }
    fn prepare_sources(&mut self, name: &str) -> Result<(), String> {
        for directory in self.directories.clone() {
            if self.prepared.contains(&directory) {
                continue;
            }
            let mut relevant = false;
            for entry in std::fs::read_dir(&directory).map_err(|e| e.to_string())? {
                let path = entry.map_err(|e| e.to_string())?.path();
                if path.is_file() && path.extension().is_some_and(|e| e == "c") {
                    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
                    if text
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .any(|word| word == name)
                    {
                        relevant = true;
                    }
                }
            }
            if !relevant {
                continue;
            }
            self.prepared.insert(directory.clone());
            let executable = std::env::current_exe().map_err(|e| e.to_string())?;
            let builder = executable
                .parent()
                .ok_or("cannot find native dependency builder")?
                .join(if cfg!(windows) { "devc.exe" } else { "devc" });
            let mut command = std::process::Command::new(&builder);
            command.arg("native-build").arg(&directory);
            let mut symbols = self.signatures.keys().collect::<Vec<_>>();
            symbols.sort();
            for symbol in symbols {
                command.arg("--symbol").arg(symbol);
            }
            let result=command.output().map_err(|e|format!("cannot prepare C dependency automatically: {e}; place devc next to devrun, or supply --ffi-lib PATH"))?;
            if !result.status.success() {
                return Err(String::from_utf8_lossy(&result.stderr).trim().to_owned());
            }
            let path = PathBuf::from(
                String::from_utf8(result.stdout)
                    .map_err(|_| "invalid native builder output")?
                    .trim(),
            );
            self.tried.insert(path.clone());
            self.libraries
                .push(unsafe { Library::new(&path) }.map_err(|e| {
                    format!(
                        "cannot load automatically prepared C dependency {}: {e}",
                        path.display()
                    )
                })?);
        }
        Ok(())
    }
    fn resolve(&mut self, name: &str) -> Result<CodePtr, String> {
        if !self.system_loaded {
            #[cfg(windows)]
            let systems = ["ucrtbase.dll", "msvcrt.dll"];
            #[cfg(target_os = "linux")]
            let systems = ["libc.so.6"];
            #[cfg(target_os = "macos")]
            let systems = ["/usr/lib/libSystem.B.dylib"];
            #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
            let systems = ["libc.so"];
            // Keep the handle alive for every cached function pointer.
            for system in systems {
                let library = unsafe { Library::new(system) }
                    .map_err(|e| format!("cannot load system C library: {e}"))?;
                if let Ok(flush) =
                    unsafe { library.get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"fflush\0") }
                {
                    self.flushers.push(*flush);
                }
                self.libraries.push(library);
            }
            self.system_loaded = true;
        }
        let symbol = CString::new(name).map_err(|_| "invalid native symbol")?;
        for library in &self.libraries {
            // Only called with extern declarations; the caller supplies the C ABI signature.
            if let Ok(pointer) =
                unsafe { library.get::<unsafe extern "C" fn()>(symbol.as_bytes_with_nul()) }
            {
                return Ok(CodePtr::from_fun(*pointer));
            }
        }
        self.discover()?;
        for library in &self.libraries {
            if let Ok(pointer) =
                unsafe { library.get::<unsafe extern "C" fn()>(symbol.as_bytes_with_nul()) }
            {
                return Ok(CodePtr::from_fun(*pointer));
            }
        }
        self.prepare_sources(name)?;
        for library in &self.libraries {
            if let Ok(pointer) =
                unsafe { library.get::<unsafe extern "C" fn()>(symbol.as_bytes_with_nul()) }
            {
                return Ok(CodePtr::from_fun(*pointer));
            }
        }
        let extra = if self.discovery_errors.is_empty() {
            String::new()
        } else {
            format!(
                "; adjacent libraries failed to load: {}",
                self.discovery_errors.join("; ")
            )
        };
        Err(format!(
            "native symbol '{name}' not found; place its C source or a shared library next to its .dev module, or load a library with --ffi-lib PATH{extra}"
        ))
    }
    pub fn call(&mut self, f: &Function, values: &[Value]) -> Result<Value, String> {
        let binding = if f.variadic {
            format!(
                "{}:{:?}",
                f.name,
                values.iter().map(Value::ty).collect::<Vec<_>>()
            )
        } else {
            f.name.clone()
        };
        if !self.bindings.contains_key(&binding) {
            let pointer = self.resolve(&f.name)?;
            let types = values
                .iter()
                .map(|v| ffi_type(&v.ty()))
                .collect::<Result<Vec<_>, _>>()?;
            let cif = (if f.variadic {
                Cif::try_new_variadic(types, f.params.len(), ffi_type(&f.ret)?)
            } else {
                Cif::try_new(types, ffi_type(&f.ret)?)
            })
            .map_err(|e| format!("invalid native ABI: {e:?}"))?;
            self.bindings.insert(binding.clone(), (cif, pointer));
        }
        let slots = values
            .iter()
            .map(Slot::from_value)
            .collect::<Result<Vec<_>, _>>()?;
        let arguments = slots.iter().map(Slot::argument).collect::<Vec<_>>();
        let (cif, pointer) = &self.bindings[&binding];
        // SAFETY: storage exactly matches the declared C ABI and survives the call.
        // Actual native function signatures and pointer validity are the user's contract.
        // No Rust transmute-based guess of register/stack layout is used.
        let result = unsafe {
            match &f.ret {
                Type::Record(..) => {
                    let mut buffer = crate::memory::Buffer::new(&f.ret)?;
                    cif.call_return_into(*pointer, &arguments, ret(&mut buffer.words[0]));
                    crate::memory::read(buffer.words.as_ptr() as usize, &f.ret)?
                }
                Type::Enum(_, vs) if vs.iter().all(|(_, ts)| ts.is_empty()) => {
                    let n = cif.call::<i32>(*pointer, &arguments);
                    if n < 0 || n as usize >= vs.len() {
                        return Err("invalid native enum discriminant".into());
                    }
                    Value::Enum(n as usize, f.ret.clone(), vec![])
                }
                Type::Void => {
                    cif.call::<()>(*pointer, &arguments);
                    Value::Void
                }
                Type::Bool => Value::Bool(cif.call::<u8>(*pointer, &arguments) != 0),
                Type::Int {
                    bits: 8,
                    signed: true,
                } => Value::Int(cif.call::<i8>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Int {
                    bits: 8,
                    signed: false,
                } => Value::Int(cif.call::<u8>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Int {
                    bits: 16,
                    signed: true,
                } => Value::Int(cif.call::<i16>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Int {
                    bits: 16,
                    signed: false,
                } => Value::Int(cif.call::<u16>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Int {
                    bits: 32,
                    signed: true,
                } => Value::Int(cif.call::<i32>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Int {
                    bits: 32,
                    signed: false,
                } => Value::Int(cif.call::<u32>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Int {
                    bits: 64,
                    signed: true,
                } => Value::Int(cif.call::<i64>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Int {
                    bits: 64,
                    signed: false,
                } => Value::Int(cif.call::<u64>(*pointer, &arguments) as i128, f.ret.clone()),
                Type::Size { signed: true } => Value::Int(
                    cif.call::<isize>(*pointer, &arguments) as i128,
                    f.ret.clone(),
                ),
                Type::Size { signed: false } => Value::Int(
                    cif.call::<usize>(*pointer, &arguments) as i128,
                    f.ret.clone(),
                ),
                Type::Float(32) => {
                    Value::Float(cif.call::<f32>(*pointer, &arguments) as f64, f.ret.clone())
                }
                Type::Float(64) => {
                    Value::Float(cif.call::<f64>(*pointer, &arguments), f.ret.clone())
                }
                Type::Ptr(_) => Value::Ptr(
                    cif.call::<*mut c_void>(*pointer, &arguments) as usize,
                    f.ret.clone(),
                ),
                Type::Callback(..) => {
                    Value::Ptr(cif.call::<usize>(*pointer, &arguments), f.ret.clone())
                }
                Type::Str => {
                    let p = cif.call::<*const std::ffi::c_char>(*pointer, &arguments);
                    if p.is_null() {
                        return Err("native function returned null for str; declare *u8 for nullable strings".into());
                    }
                    Value::Str(
                        CStr::from_ptr(p)
                            .to_str()
                            .map_err(|_| "native function returned invalid UTF-8 str")?
                            .to_owned(),
                    )
                }
                _ => return Err("unsupported native return type".into()),
            }
        };
        self.flush_stdio();
        let callbacks = self
            .callbacks
            .lock()
            .map_err(|_| "callback registry lock poisoned")?;
        for callback in callbacks.iter() {
            if let Some(error) = callback
                .data
                .error
                .lock()
                .map_err(|_| "callback error lock poisoned")?
                .remove(&std::thread::current().id())
            {
                return Err(error);
            }
        }
        Ok(result)
    }
}

include!("callbacks.rs");
