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
        let mut program = Self {
            modules: Vec::new(),
            aliases: Vec::new(),
            signatures: HashMap::new(),
        };
        program.load_module(entry, &mut HashMap::new(), module_dirs)?;
        let mut external_names: HashMap<String, (Vec<Type>, Type, bool)> = HashMap::new();
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
                let params: Vec<Type> = function.params.iter().map(|(_, t)| t.clone()).collect();
                if external || function.exported {
                    // C keywords and the implementation's reserved identifier space cannot be exported.
                    if function.name.starts_with('_') || c_keyword(&function.name) {
                        return Err(fail("this name cannot be used as a C symbol"));
                    }
                    if let Some((old_params, old_ret, definition)) =
                        external_names.get_mut(&function.name)
                    {
                        if *old_params != params
                            || *old_ret != function.ret
                            || (*definition && !external)
                        {
                            return Err(fail("conflicting C symbol signature or duplicate export"));
                        }
                        *definition |= !external;
                    } else {
                        external_names.insert(
                            function.name.clone(),
                            (params.clone(), function.ret.clone(), !external),
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
    ) -> Result<usize, String> {
        let path = std::fs::canonicalize(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(id) = seen.get(&path) {
            return Ok(*id);
        }
        if seen.len() >= 512 {
            return Err("a program can have at most 512 modules in v0.1".into());
        }
        let metadata = std::fs::metadata(&path).map_err(|e| e.to_string())?;
        if metadata.len() > 8 * 1024 * 1024 {
            return Err(format!("{}: source exceeds 8 MiB", path.display()));
        }
        let source =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
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
            let mapped = import
                .path
                .split_once('/')
                .and_then(|(name, suffix)| module_dirs.get(name).map(|root| (root, suffix)));
            let target = if let Some((root, suffix)) = mapped {
                if suffix.is_empty()
                    || Path::new(suffix)
                        .components()
                        .any(|part| !matches!(part, std::path::Component::Normal(_)))
                {
                    return Err(error(
                        &path,
                        import.span,
                        "module path cannot escape its namespace directory",
                    ));
                }
                let target = root.join(suffix);
                if target.extension().is_some() {
                    target
                } else {
                    target.with_extension("dev")
                }
            } else {
                path.parent().unwrap().join(&import.path)
            };
            let target_id = self.load_module(&target, seen, module_dirs).map_err(|e| {
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
