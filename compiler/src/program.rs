use crate::ast::*;
use crate::parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Program {
    pub modules: Vec<Module>,
    pub aliases: Vec<HashMap<String, usize>>,
    pub signatures: HashMap<(usize, String), Signature>,
}
#[derive(Clone)]
pub struct Signature {
    pub variadic: bool,
    pub c_name: String,
    pub params: Vec<Type>,
    pub ret: Type,
}

pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ *b as u64).wrapping_mul(0x100000001b3)
    })
}
pub fn module_name(path: &Path) -> String {
    format!("m{:016x}", hash(path.to_string_lossy().as_bytes()))
}

impl Program {
    pub fn load(entry: &Path) -> Result<Self, String> {
        Self::load_with_modules(entry, &HashMap::new())
    }
    pub fn load_with_modules(
        entry: &Path,
        module_dirs: &HashMap<String, PathBuf>,
    ) -> Result<Self, String> {
        Self::load_with_sources(entry, module_dirs, &HashMap::new())
    }
    pub fn load_with_sources(
        entry: &Path,
        module_dirs: &HashMap<String, PathBuf>,
        sources: &HashMap<PathBuf, String>,
    ) -> Result<Self, String> {
        let mut program = Self {
            modules: Vec::new(),
            aliases: Vec::new(),
            signatures: HashMap::new(),
        };
        program.load_module(entry, &mut HashMap::new(), module_dirs, sources)?;
        dev_syntax::expand::modules(&mut program.modules, &program.aliases)?;
        for module in &program.modules {
            for ty in &module.concrete_types {
                valid_type(ty, false)
                    .map_err(|msg| error(&module.path, Span { line: 1, col: 1 }, msg))?;
            }
        }
        let mut external_names: HashMap<String, (Vec<Type>, Type, bool, bool)> = HashMap::new();
        for (id, module) in program.modules.iter().enumerate() {
            for function in &module.functions {
                let fail = |message: &str| error(&module.path, function.span, message);
                if function.name.starts_with("dev_")
                    || ["print", "sizeof", "volatile_load", "volatile_store"]
                        .contains(&function.name.as_str())
                {
                    return Err(fail("function name is reserved by the compiler"));
                }
                valid_type(&function.ret, true).map_err(|e| fail(&e))?;
                if matches!(function.ret, Type::Array(..)) {
                    return Err(fail("return an array through a pointer"));
                }
                let mut names = std::collections::HashSet::new();
                for (name, ty) in &function.params {
                    if !names.insert(name) {
                        return Err(fail("duplicate parameter name"));
                    }
                    valid_type(ty, false).map_err(|e| fail(&e))?;
                    if matches!(ty, Type::Array(..)) {
                        return Err(fail("array parameters use pointers, for example *i32"));
                    }
                }
                let external = function.body.is_none();
                if external
                    && (matches!(function.ret, Type::Ref(..))
                        || function
                            .params
                            .iter()
                            .any(|(_, t)| matches!(t, Type::Ref(..))))
                {
                    return Err(fail(
                        "aggregate and enum FFI values are not supported; use scalar wrappers",
                    ));
                }
                let params: Vec<Type> = function.params.iter().map(|(_, t)| t.clone()).collect();
                if external || function.exported {
                    // C keywords and the implementation's reserved identifier space cannot be exported.
                    if function.name.starts_with('_') || c_keyword(&function.name) {
                        return Err(fail("this name cannot be used as a C symbol"));
                    }
                    if let Some((old_params, old_ret, definition, variadic)) =
                        external_names.get_mut(&function.name)
                    {
                        if *variadic != function.variadic
                            || *old_params != params
                            || *old_ret != function.ret
                            || (*definition && !external)
                        {
                            return Err(fail("conflicting C symbol signature or duplicate export"));
                        }
                        *definition |= !external;
                    } else {
                        external_names.insert(
                            function.name.clone(),
                            (
                                params.clone(),
                                function.ret.clone(),
                                !external,
                                function.variadic,
                            ),
                        );
                    }
                }
                let c_name = if external || function.exported {
                    function.name.clone()
                } else {
                    format!("dev_{}_{}", module_name(&module.path), function.name)
                };
                let key = (id, function.name.clone());
                if program
                    .signatures
                    .insert(
                        key,
                        Signature {
                            variadic: function.variadic,
                            c_name,
                            params,
                            ret: function.ret.clone(),
                        },
                    )
                    .is_some()
                {
                    return Err(fail("duplicate function name"));
                }
            }
        }
        Ok(program)
    }
    fn load_module(
        &mut self,
        path: &Path,
        seen: &mut HashMap<PathBuf, usize>,
        module_dirs: &HashMap<String, PathBuf>,
        sources: &HashMap<PathBuf, String>,
    ) -> Result<usize, String> {
        let path = std::fs::canonicalize(path)
            .or_else(|e| {
                if sources.contains_key(path) {
                    Ok(path.to_owned())
                } else {
                    Err(e)
                }
            })
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(id) = seen.get(&path) {
            return Ok(*id);
        }
        if seen.len() >= 512 {
            return Err("a program can have at most 512 modules in v0.1".into());
        }
        let source = match sources.get(&path) {
            Some(source) => source.clone(),
            None => {
                let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
                if metadata.len() > 8 * 1024 * 1024 {
                    return Err(format!("{}: source exceeds 8 MiB", path.display()));
                }
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?
            }
        };
        if source.len() > 8 * 1024 * 1024 {
            return Err(format!("{}: source exceeds 8 MiB", path.display()));
        }
        let module = parser::parse(path.clone(), &source)?;
        let id = self.modules.len();
        seen.insert(path.clone(), id);
        let imports = module.imports.clone();
        self.modules.push(module);
        self.aliases.push(HashMap::new());
        for import in imports {
            if self.aliases[id].contains_key(&import.alias) {
                return Err(error(&path, import.span, "duplicate module alias"));
            }
            // Editor overlays may check runtime intrinsics using shared signatures.
            // Native builds have no overlays and still require real native modules.
            if !sources.is_empty() {
                if let Some(source) = dev_syntax::intrinsics::module(&import.path) {
                    let virtual_path = PathBuf::from("__devlang_intrinsics__").join(format!("{}.dev", import.path.replace('/', "_")));
                    let mut overlays = sources.clone();
                    overlays.insert(virtual_path.clone(), source.into());
                    let target_id = self.load_module(&virtual_path, seen, module_dirs, &overlays)?;
                    self.aliases[id].insert(import.alias, target_id);
                    continue;
                }
            }
            let target = dev_syntax::modules::resolve(&path, &import.path, module_dirs)
                .map_err(|e| error(&path, import.span, e))?;
            let target_id = self
                .load_module(&target, seen, module_dirs, sources)
                .map_err(|e| {
                    error(
                        &path,
                        import.span,
                        format!("cannot import '{}': {e}", import.path),
                    )
                })?;
            self.aliases[id].insert(import.alias, target_id);
        }
        Ok(id)
    }
    pub fn resolve(
        &self,
        module: usize,
        names: &[String],
        span: Span,
    ) -> Result<&Signature, String> {
        let path = &self.modules[module].path;
        let (target, name) = match names {
            [name] => (module, name),
            [alias, name] => (
                *self.aliases[module]
                    .get(alias)
                    .ok_or_else(|| error(path, span, format!("unknown module '{alias}'")))?,
                name,
            ),
            _ => {
                return Err(error(
                    path,
                    span,
                    "use module.function to call an imported function",
                ))
            }
        };
        self.signatures.get(&(target, name.clone())).ok_or_else(|| {
            error(
                path,
                span,
                format!("unknown function '{}'", names.join(".")),
            )
        })
    }
}

pub fn valid_type(ty: &Type, allow_void: bool) -> Result<(), String> {
    match ty {
        Type::Void if !allow_void => Err("void is not a value type".into()),
        Type::Function(ps, r) => {
            for t in ps {
                if matches!(t, Type::Array(..)) {
                    return Err("function value parameters cannot be arrays".into());
                }
                valid_type(t, false)?;
            }
            if matches!(**r, Type::Array(..)) {
                return Err("function values cannot return arrays".into());
            }
            valid_type(r, true)
        }
        Type::Task(r) => valid_type(r, true),
        Type::Map(k, v) => {
            valid_type(k, false)?;
            valid_type(v, false)
        }
        Type::Vector(inner) | Type::Slice(inner) => valid_type(inner, false),
        Type::Ref(inner) => {
            if matches!(**inner, Type::Void | Type::Array(..)) {
                return Err("Ref requires a non-array value type".into());
            }
            valid_type(inner, false)
        }
        Type::Ptr(inner) => {
            if matches!(**inner, Type::Array(..)) {
                return Err("pointers to arrays are not supported; use an element pointer".into());
            }
            valid_type(inner, true)
        }
        Type::Array(inner, _) => {
            if matches!(**inner, Type::Array(..)) {
                return Err("nested arrays are not supported in v0.1".into());
            }
            valid_type(inner, false)
        }
        Type::Record(_, fields) => {
            for (_, t) in fields {
                valid_type(t, false)?;
            }
            Ok(())
        }
        Type::Enum(_, variants) => {
            for (_, ts) in variants {
                for t in ts {
                    valid_type(t, false)?;
                }
            }
            Ok(())
        }
        Type::Named(..) | Type::Const(_) | Type::ArrayConst(..) => {
            Err("unresolved or non-value type".into())
        }
        _ => Ok(()),
    }
}
fn c_keyword(s: &str) -> bool {
    [
        "auto",
        "break",
        "case",
        "char",
        "const",
        "continue",
        "default",
        "do",
        "double",
        "else",
        "enum",
        "extern",
        "float",
        "for",
        "goto",
        "if",
        "inline",
        "int",
        "long",
        "register",
        "restrict",
        "return",
        "short",
        "signed",
        "sizeof",
        "static",
        "struct",
        "switch",
        "typedef",
        "union",
        "unsigned",
        "void",
        "volatile",
        "while",
        "_Bool",
        "_Complex",
        "_Imaginary",
        "main",
    ]
    .contains(&s)
}
