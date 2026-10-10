use crate::ast::*;
use crate::program::{module_name, valid_type, Program};
use crate::tasks::THREADS;
use std::collections::HashMap;

pub const TYPES_HEADER: &str = "#ifndef DEV_TYPES_H\n#define DEV_TYPES_H\n#include <stdint.h>\n#include <stddef.h>\n#include <stdbool.h>\n#if defined(__SSE4_1__)\n#define DEV_LOW32_SELECT(narrow, wide) (narrow)\n#else\n#define DEV_LOW32_SELECT(narrow, wide) (wide)\n#endif\n#endif\n";
pub struct Generated {
    pub name: String,
    pub header: String,
    pub source: String,
    pub dependencies: Vec<usize>,
}

pub fn c_type(ty: &Type) -> String {
    match ty {
        Type::Callback(..) => format!("dev_callback_{}", managed_key(ty)),
        Type::Task(_) => format!("dev_task_{}", managed_key(ty)),
        Type::Function(..) => format!("dev_callable_{}", managed_key(ty)),
        Type::Void => "void".into(),
        Type::Bool => "bool".into(),
        Type::Str => "const char *".into(),
        Type::Int { bits, signed } => format!("{}int{bits}_t", if *signed { "" } else { "u" }),
        Type::Size { signed } => if *signed { "intptr_t" } else { "uintptr_t" }.into(),
        Type::Float(32) => "float".into(),
        Type::Float(_) => "double".into(),
        Type::Ref(inner) => format!("{} const *", c_type(inner)),
        Type::Ptr(inner) => format!("{} *", c_type(inner)),
        Type::Array(..) => unreachable!("array requires a declarator"),
        Type::Nominal(n) | Type::Record(n, _) | Type::Enum(n, _) => n.clone(),
        Type::Map(..) | Type::Vector(_) | Type::Slice(_) => {
            format!("dev_collection_{}", managed_key(ty))
        }
        Type::Named(..) | Type::Const(_) | Type::ArrayConst(..) => unreachable!("unresolved type"),
    }
}
pub(crate) fn declaration(ty: &Type, name: &str) -> String {
    match ty {
        Type::Array(t, n) => format!("{} {name}[{n}]", c_type(t)),
        _ => format!("{} {name}", c_type(ty)),
    }
}
fn prototype(program: &Program, id: usize, function: &Function) -> String {
    let signature = &program.signatures[&(id, function.name.clone())];
    let mut params: Vec<String> = function
        .params
        .iter()
        .enumerate()
        .map(|(i, (_, t))| declaration(t, &format!("dev_p_{i}")))
        .collect();
    if function.variadic {
        params.push("...".into());
    }
    format!(
        "{} {}({})",
        c_type(&function.ret),
        signature.c_name,
        if params.is_empty() {
            "void".into()
        } else {
            params.join(", ")
        }
    )
}
pub fn generate(program: &Program, hosted: bool) -> Result<Vec<Generated>, String> {
    generate_inner(program, hosted, None)
}
pub fn diagnostics(program: &Program) -> Vec<String> {
    let mut errors = Vec::new();
    if let Err(error) = generate_inner(program, true, Some(&mut errors)) {
        errors.push(error);
    }
    errors.sort();
    errors.dedup();
    errors.truncate(100);
    errors
}
fn generate_inner(
    program: &Program,
    hosted: bool,
    mut diagnostics: Option<&mut Vec<String>>,
) -> Result<Vec<Generated>, String> {
    fn definitions(
        t: &Type,
        seen: &mut std::collections::HashSet<String>,
        out: &mut String,
        registry: &HashMap<String, Type>,
    ) {
        match t {
            Type::Callback(ps, r) => {
                let name = c_type(t);
                if !seen.insert(name.clone()) {
                    return;
                }
                for p in ps {
                    definitions(p, seen, out, registry);
                }
                definitions(r, seen, out, registry);
                let params = if ps.is_empty() {
                    "void".into()
                } else {
                    ps.iter().map(c_type).collect::<Vec<_>>().join(",")
                };
                out.push_str(&format!("#ifndef {name}_DEFINED\n#define {name}_DEFINED\ntypedef {} (*{name})({params});\n#endif\n",c_type(r)));
            }
            Type::Task(r) => {
                let name = c_type(t);
                if !seen.insert(name.clone()) {
                    return;
                }
                definitions(r, seen, out, registry);
                out.push_str(&format!("#ifndef {name}_DEFINED\n#define {name}_DEFINED\ntypedef struct {{ void *state; }} {name};\n#endif\n"));
            }
            Type::Function(ps, r) => {
                let name = c_type(t);
                if !seen.insert(name.clone()) {
                    return;
                }
                for p in ps {
                    definitions(p, seen, out, registry)
                }
                definitions(r, seen, out, registry);
                let mut args = vec!["const void *".into()];
                args.extend(ps.iter().map(|p| declaration(p, "")));
                out.push_str(&format!("#ifndef {name}_DEFINED\n#define {name}_DEFINED\ntypedef struct {{ {} (*call)({}); const void *context; }} {name};\n#endif\n",c_type(r),args.join(",")));
            }
            Type::Map(..) => {
                let name = c_type(t);
                if !seen.insert(name.clone()) {
                    return;
                }
                let entries = Type::Vector(Box::new(map_entry(t)));
                definitions(&entries, seen, out, registry);
                let buckets = Type::Vector(Box::new(Type::Size { signed: false }));
                definitions(&buckets, seen, out, registry);
                out.push_str(&format!("#ifndef {name}_DEFINED\n#define {name}_DEFINED\ntypedef struct {{ {} entries; {} buckets; }} {name};\n#endif\n",c_type(&entries),c_type(&buckets)));
            }
            Type::Vector(inner) | Type::Slice(inner) => {
                let name = c_type(t);
                if !seen.insert(name.clone()) {
                    return;
                }
                if !matches!(
                    **inner,
                    Type::Record(..) | Type::Enum(..) | Type::Nominal(_)
                ) {
                    definitions(inner, seen, out, registry);
                }
                out.push_str(&format!("#ifndef {name}_DEFINED\n#define {name}_DEFINED\ntypedef struct {{ size_t len; size_t offset; {} *data; }} {name};\n#endif\n",c_type(inner)));
            }
            Type::Nominal(name) => {
                if let Some(t) = registry.get(name) {
                    definitions(t, seen, out, registry);
                }
            }
            Type::Record(name, fields) => {
                if !seen.insert(name.clone()) {
                    return;
                }
                for (_, t) in fields {
                    definitions(t, seen, out, registry);
                }
                out.push_str(&format!(
                    "#ifndef {name}_DEFINED\n#define {name}_DEFINED\ntypedef struct {name} {{\n"
                ));
                if fields.is_empty() {
                    out.push_str("unsigned char dev_empty;\n");
                }
                for (i, (_, t)) in fields.iter().enumerate() {
                    out.push_str(&format!("{};\n", declaration(t, &format!("dev_f_{i}"))));
                }
                out.push_str(&format!("}} {name};\n#endif\n"));
            }
            Type::Enum(name, variants) => {
                if !seen.insert(name.clone()) {
                    return;
                }
                let payload = variants.iter().any(|(_, ts)| !ts.is_empty());
                for (_, ts) in variants {
                    for t in ts {
                        definitions(t, seen, out, registry);
                    }
                }
                out.push_str(&format!("#ifndef {name}_DEFINED\n#define {name}_DEFINED\n"));
                if payload {
                    out.push_str(&format!("struct {name} {{ int32_t tag; union {{\n"));
                    for (i, (_, ts)) in variants.iter().enumerate() {
                        if !ts.is_empty() {
                            out.push_str("struct { ");
                            for (j, t) in ts.iter().enumerate() {
                                out.push_str(&format!(
                                    "{}; ",
                                    declaration(t, &format!("dev_p_{j}"))
                                ));
                            }
                            out.push_str(&format!("}} dev_v_{i};\n"));
                        }
                    }
                    out.push_str("} data; };\n");
                } else {
                    out.push_str(&format!("typedef int32_t {name};\n"));
                }
                out.push_str(&format!(
                    "static inline const char *{name}_name({name} value) {{ switch({}) {{\n",
                    if payload { "value.tag" } else { "value" }
                ));
                for (i, (v, _)) in variants.iter().enumerate() {
                    out.push_str(&format!("case {i}: return {};\n", c_string(v)));
                }
                out.push_str("default: return \"invalid enum\"; }}\n#endif\n");
            }
            Type::Ref(t) | Type::Ptr(t) => {
                if !matches!(**t, Type::Nominal(_)) {
                    definitions(t, seen, out, registry);
                }
            }
            Type::Array(t, _) => definitions(t, seen, out, registry),
            _ => {}
        }
    }
    let registry = program
        .modules
        .iter()
        .flat_map(|m| &m.concrete_types)
        .map(|t| (t.name(), t.clone()))
        .collect::<HashMap<_, _>>();
    let mut nominal = String::new();
    for module in &program.modules {
        for t in &module.concrete_types {
            if matches!(t, Type::Record(..))
                || matches!(t, Type::Enum(_,v) if v.iter().any(|(_,ts)| !ts.is_empty()))
            {
                let name = t.name();
                nominal.push_str(&format!("#ifndef {name}_FORWARD\n#define {name}_FORWARD\ntypedef struct {name} {name};\n#endif\n"));
            } else if matches!(t, Type::Enum(..)) {
                let name = t.name();
                nominal.push_str(&format!("#ifndef {name}_FORWARD\n#define {name}_FORWARD\ntypedef int32_t {name};\n#endif\n"));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    for module in &program.modules {
        for t in &module.concrete_types {
            definitions(t, &mut seen, &mut nominal, &registry);
        }
    }
    let mut result = Vec::new();
    for (id, module) in program.modules.iter().enumerate() {
        let name = module_name(&module.path);
        let mut header = format!(
            "#ifndef DEV_{}_H\n#define DEV_{}_H\n#include \"dev_types.h\"\n",
            name.to_uppercase(),
            name.to_uppercase()
        );
        header.push_str(&nominal);
        for f in &module.functions {
            header.push_str(&format!("{};\n", prototype(program, id, f)));
        }
        header.push_str("#endif\n");
        let mut dependencies: Vec<usize> = program.aliases[id].values().copied().collect();
        dependencies.push(id);
        dependencies.sort_unstable();
        dependencies.dedup();
        let mut source = String::new();
        for dep in &dependencies {
            source.push_str(&format!(
                "#include \"{}.h\"\n",
                module_name(&program.modules[*dep].path)
            ));
        }
        let mut emitter = Emitter {
            program,
            module: id,
            scopes: Vec::new(),
            next_var: 0,
            ret: Type::Void,
            loop_depth: 0,
            uses_print: false,
            uses_refs: false,
            unsafe_depth: 0,
            uses_checks: false,
            prelude: String::new(),
            temporaries: vec![],
            loop_scopes: vec![],
            loop_temporaries: vec![],
            managed_helpers: HashMap::new(),
            collection_types: HashMap::new(),
            adapters: HashMap::new(),
            task_types: HashMap::new(),
            callback_types: HashMap::new(),
            context_callback_types: HashMap::new(),
            writing: false,
            hosted,
            diagnostics: diagnostics.as_deref_mut(),
        };
        let mut functions = String::new();
        for f in &module.functions {
            let Some(body) = &f.body else {
                continue;
            };
            emitter.ret = f.ret.clone();
            emitter.scopes = vec![HashMap::new()];
            emitter.next_var = 0;
            for (i, (name, ty)) in f.params.iter().enumerate() {
                emitter.scopes[0].insert(
                    name.clone(),
                    Variable {
                        name: format!("dev_p_{i}"),
                        ty: ty.clone(),
                    },
                );
            }
            let parameter_retains = emitter.scope_cleanup(0, true);
            let text = emitter.block(body, false)?;
            let text = format!("{parameter_retains}{text}");
            let is_entry = f.main_default && f.ret == Type::i32();
            if f.ret != Type::Void && !is_entry && !returns(body) {
                let message = error(
                    &module.path,
                    f.span,
                    "function must return a value on every path",
                );
                if let Some(errors) = &mut emitter.diagnostics {
                    errors.push(message);
                } else {
                    return Err(message);
                }
            }
            functions.push_str(&format!(
                "#line {} {}\n{} {{\n{}",
                f.span.line,
                c_string(&module.path.to_string_lossy()),
                prototype(program, id, f),
                text
            ));
            if is_entry {
                functions.push_str("return 0;\n");
            }
            functions.push_str("}\n");
        }
        if id == 0 {
            emitter.ret = Type::i32();
            emitter.scopes = vec![HashMap::new()];
            emitter.next_var = 0;
            let text = emitter.block(&module.statements, false)?;
            functions.push_str(&format!(
                "int32_t dev_script_{name}(void) {{\n{text}return 0;\n}}\n"
            ));
        } else if !module.statements.is_empty() {
            return Err(error(&module.path,Span{line:1,col:1},"imported modules contain declarations only; call their functions from the entry file"));
        }
        if emitter.uses_print {
            source.push_str("#include <stdio.h>\n");
        }
        if !emitter.collection_types.is_empty() {
            source.push_str(&emitter.collection_typedefs());
        }
        if hosted
            && program
                .modules
                .iter()
                .flat_map(|m| &m.concrete_types)
                .any(|t| matches!(t, Type::Task(_) | Type::Callback(..)))
        {
            source.push_str(THREADS);
        }
        if emitter.uses_refs {
            source.push_str(MANAGED_REFS);
            source.push_str(&emitter.helper_definitions());
        }
        if emitter.uses_checks {
            source.push_str(&checked_helpers(hosted));
        }
        if !emitter.collection_types.is_empty() {
            source.push_str(&emitter.collection_definitions());
        }
        source.push_str(&emitter.task_definitions());
        source.push_str(&emitter.callback_definitions());
        let mut adapters = emitter.adapters.iter().collect::<Vec<_>>();
        adapters.sort_by_key(|(n, _)| *n);
        for (_, code) in adapters {
            source.push_str(code);
        }
        source.push_str(&functions);
        result.push(Generated {
            name,
            header,
            source,
            dependencies,
        });
    }
    Ok(result)
}

fn returns(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Return(..) => true,
        Stmt::Unsafe(body) | Stmt::Block(body) => returns(body),
        Stmt::Match { arms, .. } => !arms.is_empty() && arms.iter().all(|a| returns(&a.body)),
        Stmt::If { yes, no, .. } => returns(yes) && returns(no),
        _ => false,
    })
}
#[derive(Clone)]
pub(crate) struct Variable {
    name: String,
    ty: Type,
}
pub(crate) struct Value {
    pub(crate) code: String,
    pub(crate) ty: Type,
    pub(crate) lvalue: bool,
}
pub(crate) struct Emitter<'a> {
    pub(crate) program: &'a Program,
    pub(crate) module: usize,
    pub(crate) scopes: Vec<HashMap<String, Variable>>,
    pub(crate) next_var: usize,
    pub(crate) ret: Type,
    pub(crate) loop_depth: usize,
    pub(crate) uses_print: bool,
    pub(crate) uses_refs: bool,
    pub(crate) unsafe_depth: usize,
    pub(crate) uses_checks: bool,
    pub(crate) prelude: String,
    pub(crate) temporaries: Vec<Variable>,
    pub(crate) loop_scopes: Vec<usize>,
    pub(crate) loop_temporaries: Vec<usize>,
    pub(crate) managed_helpers: HashMap<String, Type>,
    pub(crate) collection_types: HashMap<String, Type>,
    adapters: HashMap<String, String>,
    pub(crate) task_types: HashMap<String, Type>,
    pub(crate) callback_types: HashMap<String, Type>,
    pub(crate) context_callback_types: HashMap<String, Type>,
    pub(crate) writing: bool,
    pub(crate) hosted: bool,
    diagnostics: Option<&'a mut Vec<String>>,
}
impl Emitter<'_> {
    pub(crate) fn location(&self, span: Span) -> String {
        format!(
            "{}, {}, {}",
            c_string(&self.program.modules[self.module].path.to_string_lossy()),
            span.line,
            span.col
        )
    }
    pub(crate) fn fail<T>(&self, span: Span, message: impl AsRef<str>) -> Result<T, String> {
        Err(error(
            &self.program.modules[self.module].path,
            span,
            message,
        ))
    }
    pub(crate) fn compatible(
        &self,
        value: &Value,
        expected: &Type,
        span: Span,
    ) -> Result<(), String> {
        if &value.ty == expected {
            Ok(())
        } else {
            self.fail(
                span,
                format!(
                    "expected {}, found {}; use 'as {}' to convert",
                    expected.name(),
                    value.ty.name(),
                    expected.name()
                ),
            )
        }
    }
    fn variable(&self, name: &str) -> Option<&Variable> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
    pub(crate) fn helper(&mut self, t: &Type, retain: bool) -> String {
        self.uses_refs = true;
        let t = if let Type::Nominal(n) = t {
            self.program
                .modules
                .iter()
                .flat_map(|m| &m.concrete_types)
                .find(|t| t.name() == *n)
                .unwrap()
                .clone()
        } else {
            t.clone()
        };
        fn concrete(t: &Type, program: &Program) -> Type {
            match t {
                Type::Nominal(n) => concrete(
                    program
                        .modules
                        .iter()
                        .flat_map(|m| &m.concrete_types)
                        .find(|t| t.name() == *n)
                        .unwrap(),
                    program,
                ),
                Type::Record(n, fs) => Type::Record(
                    n.clone(),
                    fs.iter()
                        .map(|(n, t)| (n.clone(), concrete(t, program)))
                        .collect(),
                ),
                Type::Enum(n, vs) => Type::Enum(
                    n.clone(),
                    vs.iter()
                        .map(|(n, ts)| {
                            (n.clone(), ts.iter().map(|t| concrete(t, program)).collect())
                        })
                        .collect(),
                ),
                Type::Array(t, n) => Type::Array(Box::new(concrete(t, program)), *n),
                _ => t.clone(),
            }
        }
        let t = concrete(&t, self.program);
        let key = managed_key(&t);
        self.managed_helpers.insert(key.clone(), t);
        format!("dev_{}_{}", if retain { "keep" } else { "drop" }, key)
    }
    pub(crate) fn manage(&mut self, t: &Type, access: &str, retain: bool) -> String {
        if !contains_managed(t) {
            return String::new();
        }
        format!("{}((void *)&({access}));\n", self.helper(t, retain))
    }
    fn scope_cleanup(&mut self, from: usize, retain: bool) -> String {
        let vars = self
            .scopes
            .iter()
            .skip(from)
            .flat_map(|s| s.values().cloned())
            .collect::<Vec<_>>();
        let mut code = String::new();
        for v in vars {
            code.push_str(&self.manage(&v.ty, &v.name, retain));
        }
        code
    }
    fn temporary_cleanup(&mut self, from: usize) -> String {
        let vars = self.temporaries[from..].to_vec();
        let mut code = String::new();
        for v in vars.iter().rev() {
            code.push_str(&self.manage(&v.ty, &v.name, false));
        }
        code
    }
    pub(crate) fn owned(&mut self, value: Value) -> Value {
        if !contains_managed(&value.ty) {
            return value;
        }
        let name = format!("dev_temp_{}", self.next_var);
        self.next_var += 1;
        self.prelude.push_str(&format!(
            "{} = {};\n",
            declaration(&value.ty, &name),
            value.code
        ));
        self.temporaries.push(Variable {
            name: name.clone(),
            ty: value.ty.clone(),
        });
        Value {
            code: name,
            ty: value.ty,
            lvalue: false,
        }
    }
    fn helper_definitions(&self) -> String {
        let mut keys = self.managed_helpers.keys().collect::<Vec<_>>();
        keys.sort();
        let mut out = String::new();
        for key in keys {
            let t = &self.managed_helpers[key];
            for retain in [true, false] {
                let mut body = String::new();
                managed_visit(t, "(*p)", retain, &mut body);
                let pointer = if let Type::Array(inner, n) = t {
                    format!(
                        "{} (*p)[{n}]=({} (*)[{n}])value;",
                        c_type(inner),
                        c_type(inner)
                    )
                } else {
                    format!("{} *p=({} *)value;", c_type(t), c_type(t))
                };
                out.push_str(&format!(
                    "static void dev_{}_{key}(void *value) {{ {pointer} (void)p; {body} }}\n",
                    if retain { "keep" } else { "drop" }
                ));
            }
        }
        out
    }
    fn block(&mut self, body: &[Stmt], scoped: bool) -> Result<String, String> {
        if scoped {
            self.scopes.push(HashMap::new());
        }
        let mut text = String::new();
        for s in body {
            if self.diagnostics.is_none() {
                text.push_str(&self.statement(s)?);
                continue;
            }
            // Editor checks recover after a failed statement. Roll back scope/control
            // state so one malformed statement cannot corrupt later diagnostics.
            let scopes = self.scopes.clone();
            let prelude = self.prelude.clone();
            let temporaries = self.temporaries.clone();
            let loop_scopes = self.loop_scopes.clone();
            let loop_temporaries = self.loop_temporaries.clone();
            let loop_depth = self.loop_depth;
            let unsafe_depth = self.unsafe_depth;
            let ret = self.ret.clone();
            match self.statement(s) {
                Ok(code) => text.push_str(&code),
                Err(error) => {
                    let errors = self.diagnostics.as_mut().unwrap();
                    if errors.len() < 100 {
                        errors.push(error);
                    }
                    self.scopes = scopes;
                    self.prelude = prelude;
                    self.temporaries = temporaries;
                    self.loop_scopes = loop_scopes;
                    self.loop_temporaries = loop_temporaries;
                    self.loop_depth = loop_depth;
                    self.unsafe_depth = unsafe_depth;
                    self.ret = ret;
                    self.writing = false;
                    // Retain an explicit annotation to avoid unknown-name cascades.
                    if let Stmt::Let {
                        name, ty: Some(ty), ..
                    } = s
                    {
                        if valid_type(ty, false).is_ok() {
                            self.scopes
                                .last_mut()
                                .unwrap()
                                .entry(name.clone())
                                .or_insert(Variable {
                                    name: "dev_invalid".into(),
                                    ty: ty.clone(),
                                });
                        }
                    }
                }
            }
        }
        text.push_str(&self.scope_cleanup(self.scopes.len() - 1, false));
        if scoped {
            self.scopes.pop();
        }
        Ok(text)
    }
    fn statement(&mut self, stmt: &Stmt) -> Result<String, String> {
        let previous = std::mem::take(&mut self.prelude);
        let start = self.temporaries.len();
        let result = self.statement_inner(stmt)?;
        let prelude = std::mem::replace(&mut self.prelude, previous);
        let cleanup = self.temporary_cleanup(start);
        self.temporaries.truncate(start);
        let span = match stmt {
            Stmt::Let { span, .. }
            | Stmt::Return(_, span)
            | Stmt::Break(span)
            | Stmt::Continue(span) => Some(*span),
            Stmt::Expr(e)
            | Stmt::Assign { target: e, .. }
            | Stmt::Match { value: e, .. }
            | Stmt::If { cond: e, .. }
            | Stmt::While { cond: e, .. } => Some(e.span),
            _ => None,
        };
        let line = span
            .map(|s| {
                format!(
                    "#line {} {}\n",
                    s.line,
                    c_string(&self.program.modules[self.module].path.to_string_lossy())
                )
            })
            .unwrap_or_default();
        Ok(format!("{line}{prelude}{result}{cleanup}"))
    }
    fn statement_inner(&mut self, stmt: &Stmt) -> Result<String, String> {
        match stmt {
            Stmt::Match { value, arms } => {
                let v = self.expr(value, None)?;
                let indices = match_variants(&v.ty, arms)
                    .map_err(|e| error(&self.program.modules[self.module].path, value.span, e))?;
                let Type::Enum(_, variants) = &v.ty else {
                    unreachable!()
                };
                let temporary = format!("dev_match_{}", self.next_var);
                self.next_var += 1;
                let payload = variants.iter().any(|(_, ts)| !ts.is_empty());
                let mut code = format!("{{ {} {temporary} = {};\n", c_type(&v.ty), v.code);
                code.push_str("bool dev_matched=false;\n");
                for (arm, index) in arms.iter().zip(indices) {
                    let mut conditions = vec!["!dev_matched".to_string()];
                    let mut bindings = vec![];
                    if index != usize::MAX {
                        conditions.push(format!(
                            "{} == {index}",
                            if payload {
                                format!("{temporary}.tag")
                            } else {
                                temporary.clone()
                            }
                        ));
                        for (j, (p, t)) in arm.bindings.iter().zip(&variants[index].1).enumerate() {
                            pattern_c(
                                p,
                                t,
                                &format!("{temporary}.data.dev_v_{index}.dev_p_{j}"),
                                &mut conditions,
                                &mut bindings,
                            );
                        }
                    }
                    code.push_str(&format!("if ({}) {{\n", conditions.join(" && ")));
                    self.scopes.push(HashMap::new());
                    for (binding, access, t) in bindings {
                        if binding == "_" {
                            continue;
                        }
                        let name = format!("dev_v_{}", self.next_var);
                        self.next_var += 1;
                        code.push_str(&format!("{} = {access};\n", declaration(&t, &name)));
                        self.scopes
                            .last_mut()
                            .unwrap()
                            .insert(binding, Variable { name, ty: t });
                    }
                    let binding_keep = self.scope_cleanup(self.scopes.len() - 1, true);
                    if let Some(g) = &arm.guard {
                        let saved = std::mem::take(&mut self.prelude);
                        let start = self.temporaries.len();
                        let value = self.expr(g, Some(&Type::Bool))?;
                        self.compatible(&value, &Type::Bool, g.span)?;
                        let prefix = std::mem::replace(&mut self.prelude, saved);
                        let drop = self.temporary_cleanup(start);
                        self.temporaries.truncate(start);
                        code.push_str(&format!(
                            "{prefix}bool dev_guard={};\n{drop}if(dev_guard) {{\n",
                            value.code
                        ));
                    }
                    code.push_str(&binding_keep);
                    code.push_str("dev_matched=true;\n");
                    code.push_str(&self.block(&arm.body, false)?);
                    if arm.guard.is_some() {
                        code.push_str("}\n");
                    }
                    self.scopes.pop();
                    code.push_str("}\n");
                }
                code.push_str("}\n");
                Ok(code)
            }
            Stmt::Unsafe(body) => {
                self.unsafe_depth += 1;
                let result = self.block(body, true);
                self.unsafe_depth -= 1;
                Ok(format!("{{\n{}}}\n", result?))
            }
            Stmt::Block(body) => Ok(format!("{{\n{}}}\n", self.block(body, true)?)),
            Stmt::Let {
                name,
                ty,
                value,
                span,
            } => {
                if self.scopes.last().unwrap().contains_key(name) {
                    return self.fail(*span, "duplicate variable in this scope");
                }
                if let Some(ty) = ty {
                    valid_type(ty, false)
                        .map_err(|e| error(&self.program.modules[self.module].path, *span, e))?;
                }
                let v = self.expr(value, ty.as_ref())?;
                if let Some(t) = ty {
                    self.compatible(&v, t, value.span)?;
                }
                if v.ty == Type::Void {
                    return self.fail(*span, "cannot assign a void expression");
                }
                if matches!(v.ty, Type::Array(..)) && !matches!(value.kind, ExprKind::Array(..)) {
                    return self.fail(*span, "initialize arrays with an array literal");
                }
                let c_name = format!("dev_v_{}", self.next_var);
                self.next_var += 1;
                let decl = declaration(&v.ty, &c_name);
                let retain = self.manage(&v.ty, &c_name, true);
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Variable {
                        name: c_name,
                        ty: v.ty,
                    },
                );
                Ok(format!("{decl} = {};\n{retain}", v.code))
            }
            Stmt::Assign { target, op, value } => {
                self.writing = true;
                let t_result = self.expr(target, None);
                self.writing = false;
                let t = t_result?;
                if !t.lvalue || matches!(t.ty, Type::Array(..)) {
                    return self.fail(
                        target.span,
                        "assignment requires a writable variable, pointer, or array element",
                    );
                }
                let v = self.expr(value, Some(&t.ty))?;
                self.compatible(&v, &t.ty, value.span)?;
                if op == "=" && contains_managed(&t.ty) {
                    let pointer = format!("dev_target_{}", self.next_var);
                    self.next_var += 1;
                    let copy = format!("dev_new_{}", self.next_var);
                    self.next_var += 1;
                    let keep = self.manage(&t.ty, &copy, true);
                    let drop = self.manage(&t.ty, &format!("*{pointer}"), false);
                    return Ok(format!(
                        "{{ {} = {};\n{keep}{} *{pointer}=&{};\n{drop}*{pointer}={copy}; }}\n",
                        declaration(&t.ty, &copy),
                        v.code,
                        c_type(&t.ty),
                        t.code
                    ));
                }
                if op != "=" && !t.ty.numeric() {
                    return self.fail(target.span, "compound assignment requires a number");
                }
                if ["%=", "&=", "|=", "^="].contains(&op.as_str()) && !t.ty.integer() {
                    return self.fail(target.span, "operator requires integers");
                }
                if t.ty.integer() && ["/=", "%="].contains(&op.as_str()) {
                    self.uses_checks = true;
                    Ok(format!(
                        "dev_checked_{}_assign_{}(&{}, {}, {});\n",
                        op_name(op.trim_end_matches('=')),
                        t.ty.name(),
                        t.code,
                        v.code,
                        self.location(target.span)
                    ))
                } else {
                    Ok(format!("{} {op} {};\n", t.code, v.code))
                }
            }
            Stmt::Expr(e) => {
                let v = self.expr(e, None)?;
                if matches!(v.ty, Type::Array(..)) {
                    return self.fail(e.span, "array literal is only valid in an initializer");
                }
                Ok(format!("(void)({});\n", v.code))
            }
            Stmt::Return(value, span) => {
                if let Some(value) = value {
                    if self.ret == Type::Void {
                        return self.fail(*span, "void function cannot return a value");
                    }
                    let ret = self.ret.clone();
                    let v = self.expr(value, Some(&ret))?;
                    self.compatible(&v, &ret, value.span)?;
                    let name = format!("dev_return_{}", self.next_var);
                    self.next_var += 1;
                    let keep = self.manage(&v.ty, &name, true);
                    let temps = self.temporary_cleanup(0);
                    let locals = self.scope_cleanup(0, false);
                    Ok(format!(
                        "{} = {};\n{keep}{temps}{locals}return {name};\n",
                        declaration(&v.ty, &name),
                        v.code
                    ))
                } else if self.ret == Type::Void {
                    let temps = self.temporary_cleanup(0);
                    let locals = self.scope_cleanup(0, false);
                    Ok(format!("{temps}{locals}return;\n"))
                } else {
                    self.fail(*span, "expected a return value")
                }
            }
            Stmt::If { cond, yes, no } => {
                let c = self.expr(cond, Some(&Type::Bool))?;
                self.compatible(&c, &Type::Bool, cond.span)?;
                let yes = self.block(yes, true)?;
                let no = self.block(no, true)?;
                Ok(format!("if ({}) {{\n{yes}}} else {{\n{no}}}\n", c.code))
            }
            Stmt::While { cond, body } => {
                let saved = std::mem::take(&mut self.prelude);
                let start = self.temporaries.len();
                let c = self.expr(cond, Some(&Type::Bool))?;
                self.compatible(&c, &Type::Bool, cond.span)?;
                let prefix = std::mem::replace(&mut self.prelude, saved);
                let drop = self.temporary_cleanup(start);
                self.temporaries.truncate(start);
                self.loop_depth += 1;
                self.loop_scopes.push(self.scopes.len());
                self.loop_temporaries.push(self.temporaries.len());
                let body = self.block(body, true)?;
                self.loop_scopes.pop();
                self.loop_temporaries.pop();
                self.loop_depth -= 1;
                Ok(format!("while (true) {{\n{prefix}bool dev_condition={};\n{drop}if(!dev_condition) break;\n{body}}}\n",c.code))
            }
            Stmt::Break(span) | Stmt::Continue(span) => {
                if self.loop_depth == 0 {
                    return self.fail(*span, "break/continue requires a while loop");
                }
                let from = *self.loop_scopes.last().unwrap();
                let locals = self.scope_cleanup(from, false);
                let temps = self.temporary_cleanup(*self.loop_temporaries.last().unwrap());
                Ok(format!(
                    "{temps}{locals}{}\n",
                    if matches!(stmt, Stmt::Break(..)) {
                        "break;"
                    } else {
                        "continue;"
                    }
                ))
            }
        }
    }
    pub(crate) fn expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Result<Value, String> {
        let mut lvalue = false;
        let (code, ty) = match &expr.kind {
            ExprKind::SizeOf(t) => {
                let operand = if let Type::Array(inner, n) = t {
                    format!("{}[{n}]", c_type(inner))
                } else {
                    c_type(t)
                };
                (
                    format!("((uintptr_t)sizeof({operand}))"),
                    Type::Size { signed: false },
                )
            }
            ExprKind::Closure(..) => unreachable!("unexpanded closure"),
            ExprKind::Callable(names, context, ty) => {
                let signature = self.program.resolve(self.module, names, expr.span)?.clone();
                let Type::Function(ps, r) = ty else {
                    unreachable!()
                };
                let adapter = format!("adapter_{}", signature.c_name);
                let mut params = vec!["const void *ctx".into()];
                params.extend(
                    ps.iter()
                        .enumerate()
                        .map(|(i, t)| declaration(t, &format!("p{i}"))),
                );
                let mut args = vec![];
                let ctx = if let Some(e) = context {
                    let v = self.expr(e, None)?;
                    args.push(format!("(({})ctx)", c_type(&v.ty)));
                    v.code
                } else {
                    "NULL".into()
                };
                args.extend((0..ps.len()).map(|i| format!("p{i}")));
                self.adapters.insert(
                    adapter.clone(),
                    format!(
                        "static {} {adapter}({}) {{ (void)ctx; {}{}({}); }}\n",
                        c_type(r),
                        params.join(","),
                        if **r == Type::Void { "" } else { "return " },
                        signature.c_name,
                        args.join(",")
                    ),
                );
                (
                    format!("(({}){{ {adapter}, {ctx} }})", c_type(ty)),
                    ty.clone(),
                )
            }
            ExprKind::Invoke(base, args) => {
                let v = self.expr(base, None)?;
                if let Type::Callback(ps, r) = &v.ty {
                    if self.unsafe_depth == 0 {
                        return self
                            .fail(expr.span, "callback invocation requires an unsafe block");
                    }
                    if ps.len() != args.len() {
                        return self.fail(expr.span, "wrong argument count");
                    }
                    let mut codes = vec![];
                    for (a, t) in args.iter().zip(ps) {
                        let v = self.expr(a, Some(t))?;
                        self.compatible(&v, t, a.span)?;
                        codes.push(v.code)
                    }
                    return Ok(Value {
                        code: format!("({})({})", v.code, codes.join(",")),
                        ty: (**r).clone(),
                        lvalue: false,
                    });
                }
                let Type::Function(ps, r) = &v.ty else {
                    return self.fail(expr.span, "value is not callable");
                };
                if ps.len() != args.len() {
                    return self.fail(expr.span, "wrong argument count");
                }
                let name = format!("dev_callable_value_{}", self.next_var);
                self.next_var += 1;
                self.prelude
                    .push_str(&format!("{} {name}={};\n", c_type(&v.ty), v.code));
                let mut codes = vec![format!("{name}.context")];
                for (a, t) in args.iter().zip(ps) {
                    let value = self.expr(a, Some(t))?;
                    self.compatible(&value, t, a.span)?;
                    codes.push(value.code)
                }
                return Ok(self.owned(Value {
                    code: format!("{name}.call({})", codes.join(",")),
                    ty: (**r).clone(),
                    lvalue: false,
                }));
            }
            ExprKind::Object(..) => {
                return Err("JSON object literals currently require the source runtime".into())
            }
            ExprKind::Map(t) => return self.map_constructor(t, expr.span),
            ExprKind::Vector(t, args) => return self.vector(t, args, expr.span),
            ExprKind::Collection(base, op, args) => {
                return self.collection_call(base, op, args, expr.span)
            }
            ExprKind::Reference(a) => {
                if !self.hosted {
                    return self.fail(expr.span, "managed references require hosted mode");
                }
                let v = self.expr(a, None)?;
                if matches!(v.ty, Type::Void | Type::Array(..)) {
                    return self.fail(expr.span, "ref requires a non-array value");
                }
                self.uses_refs = true;
                let inner = match &v.ty {
                    Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n.clone()),
                    t => t.clone(),
                };
                let keep = self.helper(&v.ty, true);
                let drop = self.helper(&v.ty, false);
                return Ok(self.owned(Value{code:format!("(({} const *)dev_ref_copy(&((struct {{ {} value; }}){{{}}}).value, sizeof({}), {keep}, {drop}))",c_type(&v.ty),c_type(&v.ty),v.code,c_type(&v.ty)),ty:Type::Ref(Box::new(inner)),lvalue:false}));
            }
            ExprKind::Dereference(a) => {
                let v = self.expr(a, None)?;
                let Type::Ref(t) = &v.ty else {
                    return self.fail(expr.span, "deref requires a managed Ref");
                };
                let t = if let Type::Nominal(n) = &**t {
                    self.program
                        .modules
                        .iter()
                        .flat_map(|m| &m.concrete_types)
                        .find(|t| t.name() == *n)
                        .cloned()
                        .ok_or_else(|| "missing nominal reference type".to_string())?
                } else {
                    (**t).clone()
                };
                (format!("(*{})", v.code), t)
            }
            ExprKind::GenericCall(..) => return self.fail(expr.span, "unresolved generic call"),
            ExprKind::Record(ty, args) => {
                let Type::Record(_, fields) = ty else {
                    unreachable!()
                };
                if fields.len() != args.len() {
                    return self.fail(expr.span, "wrong struct field count");
                }
                if fields.iter().any(|(_, t)| matches!(t, Type::Array(..))) {
                    let name = format!("dev_record_value_{}", self.next_var);
                    self.next_var += 1;
                    self.prelude
                        .push_str(&format!("{} {name}={{0}};\n", c_type(ty)));
                    for (i, ((_, t), a)) in fields.iter().zip(args).enumerate() {
                        let v = self.expr(a, Some(t))?;
                        self.compatible(&v, t, a.span)?;
                        if matches!(t, Type::Array(..)) {
                            let source = if matches!(a.kind, ExprKind::Array(_)) {
                                let n = format!("dev_array_value_{}", self.next_var);
                                self.next_var += 1;
                                self.prelude.push_str(&format!(
                                    "{}={};\n",
                                    declaration(t, &n),
                                    v.code
                                ));
                                n
                            } else {
                                v.code
                            };
                            self.prelude.push_str(&format!(
                                "memcpy({name}.dev_f_{i},{source},sizeof({name}.dev_f_{i}));\n"
                            ));
                            self.uses_refs = true;
                        } else {
                            self.prelude
                                .push_str(&format!("{name}.dev_f_{i}={};\n", v.code));
                        }
                    }
                    return Ok(Value {
                        code: name,
                        ty: ty.clone(),
                        lvalue: false,
                    });
                }
                let values = fields
                    .iter()
                    .zip(args)
                    .map(|((_, t), a)| {
                        let v = self.expr(a, Some(t))?;
                        self.compatible(&v, t, a.span)?;
                        Ok(v.code)
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                (
                    format!(
                        "(({}){{{}}})",
                        c_type(ty),
                        if values.is_empty() {
                            "0".into()
                        } else {
                            values.join(", ")
                        }
                    ),
                    ty.clone(),
                )
            }
            ExprKind::Enum(ty, n, args) => {
                let Type::Enum(_, variants) = ty else {
                    unreachable!()
                };
                let ts = &variants[*n].1;
                if ts.len() != args.len() {
                    return self.fail(expr.span, "wrong enum payload argument count");
                }
                let mut values = vec![];
                for (t, a) in ts.iter().zip(args) {
                    let v = self.expr(a, Some(t))?;
                    self.compatible(&v, t, a.span)?;
                    values.push(v.code);
                }
                let code = if variants.iter().any(|(_, ts)| !ts.is_empty()) {
                    if values.is_empty() {
                        format!("(({}){{.tag={n}}})", c_type(ty))
                    } else {
                        format!(
                            "(({}){{.tag={n}, .data.dev_v_{n}={{{}}}}})",
                            c_type(ty),
                            values.join(",")
                        )
                    }
                } else {
                    format!("(({}){n})", c_type(ty))
                };
                (code, ty.clone())
            }
            ExprKind::Field(base, name) => {
                let v = self.expr(base, None)?;
                let Type::Record(_, fields) = &v.ty else {
                    return self.fail(expr.span, "field access requires a struct");
                };
                let (i, (_, t)) = fields
                    .iter()
                    .enumerate()
                    .find(|(_, (n, _))| n == name)
                    .ok_or_else(|| {
                        error(
                            &self.program.modules[self.module].path,
                            expr.span,
                            format!("unknown struct field {name}"),
                        )
                    })?;
                lvalue = v.lvalue;
                (format!("({}).dev_f_{i}", v.code), t.clone())
            }
            ExprKind::Number(n) => {
                let is_float = !n.starts_with("0x")
                    && !n.starts_with("0X")
                    && (n.contains('.') || n.contains('e') || n.contains('E'));
                let ty = expected
                    .filter(|t| t.numeric())
                    .cloned()
                    .unwrap_or_else(|| {
                        if is_float {
                            Type::Float(64)
                        } else {
                            Type::i64()
                        }
                    });
                if is_float && ty.integer() {
                    return self.fail(
                        expr.span,
                        "floating literal needs a floating type or explicit cast",
                    );
                }
                if is_float {
                    let number = n.parse::<f64>().map_err(|_| {
                        error(
                            &self.program.modules[self.module].path,
                            expr.span,
                            "invalid floating literal",
                        )
                    })?;
                    if !number.is_finite()
                        || (ty == Type::Float(32) && !(number as f32).is_finite())
                    {
                        return self.fail(expr.span, "floating literal is out of range");
                    }
                    (format!("(({})({n}))", c_type(&ty)), ty)
                } else {
                    let number = if n.starts_with("0x") || n.starts_with("0X") {
                        u128::from_str_radix(&n[2..], 16)
                    } else {
                        n.parse::<u128>()
                    }
                    .map_err(|_| {
                        error(
                            &self.program.modules[self.module].path,
                            expr.span,
                            "integer literal is out of range",
                        )
                    })?;
                    let max = match ty {
                        Type::Int { bits, signed } => (1u128 << (bits - u8::from(signed))) - 1,
                        Type::Size { signed } => (1u128 << (usize::BITS - u32::from(signed))) - 1,
                        Type::Float(_) => u64::MAX as u128,
                        _ => unreachable!(),
                    };
                    if number > max {
                        return self.fail(
                            expr.span,
                            format!("literal is out of range for {}", ty.name()),
                        );
                    }
                    (format!("(({})UINT64_C({number}))", c_type(&ty)), ty)
                }
            }
            ExprKind::String(s) => (c_string(s), Type::Str),
            ExprKind::Bool(b) => (if *b { "true" } else { "false" }.into(), Type::Bool),
            ExprKind::Name(names) if names.as_slice() == ["null"] => {
                let ty = expected
                    .filter(|t| matches!(t, Type::Ptr(_) | Type::Str))
                    .cloned()
                    .unwrap_or_else(|| Type::Ptr(Box::new(Type::Void)));
                (format!("(({})0)", c_type(&ty)), ty)
            }
            ExprKind::Name(names) => {
                if names.len() != 1 {
                    return self.fail(expr.span, "module members are functions in v0.1");
                }
                let v = self.variable(&names[0]).ok_or_else(|| {
                    error(
                        &self.program.modules[self.module].path,
                        expr.span,
                        format!("unknown variable '{}'", names[0]),
                    )
                })?;
                lvalue = true;
                (v.name.clone(), v.ty.clone())
            }
            ExprKind::Unary(op, inner) => {
                // Parse the magnitude separately so the minimum signed integer is representable.
                if op == "-" {
                    if let ExprKind::Number(n) = &inner.kind {
                        let target = expected.cloned().unwrap_or_else(Type::i64);
                        let signed = match target {
                            Type::Int { bits, signed: true } => {
                                Some((bits, format!("INT{bits}_MIN")))
                            }
                            Type::Size { signed: true } => {
                                Some((usize::BITS as u8, "INTPTR_MIN".into()))
                            }
                            _ => None,
                        };
                        if let Some((bits, minimum)) = signed {
                            let magnitude = if n.starts_with("0x") || n.starts_with("0X") {
                                u128::from_str_radix(&n[2..], 16).ok()
                            } else {
                                n.parse::<u128>().ok()
                            };
                            if magnitude == Some(1u128 << (bits - 1)) {
                                return Ok(Value {
                                    code: minimum,
                                    ty: target,
                                    lvalue: false,
                                });
                            }
                        }
                    }
                }
                let v = self.expr(
                    inner,
                    if op == "&" || op == "*" {
                        None
                    } else {
                        expected
                    },
                )?;
                match op.as_str() {
                    "&" if v.lvalue && !matches!(v.ty, Type::Array(..)) => (
                        format!("(&{})", v.code),
                        Type::Ptr(Box::new(match v.ty {
                            Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                            t => t,
                        })),
                    ),
                    "*" => {
                        if let Type::Ptr(t) = &v.ty {
                            if **t == Type::Void {
                                return self.fail(
                                    expr.span,
                                    "cast *void to an element pointer before dereferencing",
                                );
                            }
                            lvalue = true;
                            self.uses_checks = true;
                            (
                                format!("(*(({} *)dev_checked_pointer((uintptr_t)({}),_Alignof({}),{})))",c_type(t),v.code,c_type(t),self.location(expr.span)),
                                if let Type::Nominal(n) = &**t {
                                    self.program
                                        .modules
                                        .iter()
                                        .flat_map(|m| &m.concrete_types)
                                        .find(|t| t.name() == *n)
                                        .cloned()
                                        .ok_or_else(|| "missing nominal pointer type".to_string())?
                                } else {
                                    (**t).clone()
                                },
                            )
                        } else {
                            return self.fail(expr.span, "dereference requires a pointer");
                        }
                    }
                    "!" if v.ty == Type::Bool => (format!("(!{})", v.code), Type::Bool),
                    "~" if v.ty.integer() => (format!("(({})(~{}))", c_type(&v.ty), v.code), v.ty),
                    "+" | "-" if v.ty.numeric() => {
                        (format!("(({})({op}{}))", c_type(&v.ty), v.code), v.ty)
                    }
                    _ => {
                        return self.fail(
                            expr.span,
                            format!("invalid unary '{op}' for {}", v.ty.name()),
                        )
                    }
                }
            }
            ExprKind::Binary(op, left, right) => {
                if op == "&&" || op == "||" {
                    let a = self.expr(left, Some(&Type::Bool))?;
                    self.compatible(&a, &Type::Bool, left.span)?;
                    let saved = std::mem::take(&mut self.prelude);
                    let start = self.temporaries.len();
                    let b = self.expr(right, Some(&Type::Bool))?;
                    self.compatible(&b, &Type::Bool, right.span)?;
                    let rhs_prefix = std::mem::replace(&mut self.prelude, saved);
                    let drop = self.temporary_cleanup(start);
                    self.temporaries.truncate(start);
                    if !rhs_prefix.is_empty() {
                        let name = format!("dev_logic_{}", self.next_var);
                        self.next_var += 1;
                        self.prelude.push_str(&format!(
                            "bool {name}={};\nif({}{name}) {{\n{rhs_prefix}{name}={};\n{drop}}}\n",
                            a.code,
                            if op == "||" { "!" } else { "" },
                            b.code
                        ));
                        return Ok(Value {
                            code: name,
                            ty: Type::Bool,
                            lvalue: false,
                        });
                    }
                    return Ok(Value {
                        code: format!("({} {op} {})", a.code, b.code),
                        ty: Type::Bool,
                        lvalue: false,
                    });
                }
                let comparison = ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str());
                let logic = op == "&&" || op == "||";
                let context = if comparison {
                    None
                } else {
                    expected.filter(|t| t.numeric() || **t == Type::Bool)
                };
                let (a, b) = if literal(left) && !literal(right) {
                    let b = self.expr(right, context)?;
                    let a = self.expr(left, Some(&b.ty))?;
                    (a, b)
                } else {
                    let a = self.expr(left, context)?;
                    let right_type = if matches!(a.ty, Type::Ptr(_)) && (op == "+" || op == "-") {
                        None
                    } else {
                        Some(&a.ty)
                    };
                    let b = self.expr(right, right_type)?;
                    (a, b)
                };
                let ty = if logic {
                    self.compatible(&a, &Type::Bool, left.span)?;
                    self.compatible(&b, &Type::Bool, right.span)?;
                    Type::Bool
                } else if matches!(a.ty, Type::Ptr(_)) && (op == "+" || op == "-") {
                    if self.unsafe_depth == 0 {
                        return self.fail(expr.span, "pointer arithmetic requires an unsafe block");
                    }
                    if matches!(a.ty, Type::Ptr(ref inner) if **inner == Type::Void) {
                        return self.fail(expr.span, "pointer arithmetic requires an element type");
                    }
                    if b.ty.integer() {
                        a.ty.clone()
                    } else if op == "-" && a.ty == b.ty {
                        Type::Size { signed: true }
                    } else {
                        return self.fail(expr.span, "pointer arithmetic requires an integer offset or matching pointer subtraction");
                    }
                } else {
                    self.compatible(&b, &a.ty, right.span)?;
                    if comparison {
                        if !a.ty.scalar() && !matches!(a.ty, Type::Enum(..)) {
                            return self.fail(expr.span, "comparison requires scalar values");
                        }
                        if a.ty == Type::Str {
                            return self.fail(expr.span, "compare string contents through C strcmp; str equality is not defined");
                        }
                        if a.ty == Type::Bool && op != "==" && op != "!=" {
                            return self.fail(expr.span, "bool supports == and !=");
                        }
                        if matches!(&a.ty, Type::Enum(_,vs) if vs.iter().any(|(_,ts)| !ts.is_empty()))
                        {
                            return self.fail(
                                expr.span,
                                "payload enums require match; equality is unsupported",
                            );
                        }
                        if matches!(a.ty, Type::Enum(..)) && op != "==" && op != "!=" {
                            return self.fail(expr.span, "enums only support equality");
                        }
                        Type::Bool
                    } else {
                        if !a.ty.numeric() {
                            return self.fail(expr.span, "arithmetic requires numeric operands");
                        }
                        if ["%", "&", "|", "^", "<<", ">>"].contains(&op.as_str())
                            && !a.ty.integer()
                        {
                            return self.fail(expr.span, "operator requires integers");
                        }
                        a.ty.clone()
                    }
                };
                // Only the low 32 bits are observable here. Lower unsigned
                // modular arithmetic before the mask so native SIMD backends
                // can use 32-bit multiplies instead of emulating 64-bit ones.
                // Division, right shifts and casts retain full-width evaluation.
                // SSE4.1 provides native 32-bit SIMD multiplication. Preserve
                // the original expression on other targets, including SSE2.
                let masked = if op == "&"
                    && ty
                        == (Type::Int {
                            bits: 64,
                            signed: false,
                        }) {
                    if low32_mask(right) {
                        Some(self.low32(left, &ty)?)
                    } else if low32_mask(left) {
                        Some(self.low32(right, &ty)?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let code = if let Some(code) = masked {
                    format!(
                        "DEV_LOW32_SELECT(((uint64_t)({code})), ((uint64_t)({} & {})))",
                        a.code, b.code
                    )
                } else if ty.integer() && ["/", "%", "<<", ">>"].contains(&op.as_str()) {
                    self.uses_checks = true;
                    format!(
                        "dev_checked_{}_{}({}, {}, {})",
                        op_name(op),
                        ty.name(),
                        a.code,
                        b.code,
                        self.location(expr.span)
                    )
                } else {
                    format!("(({})({} {op} {}))", c_type(&ty), a.code, b.code)
                };
                (code, ty)
            }
            ExprKind::Cast(inner, ty) => {
                valid_type(ty, false)
                    .map_err(|e| error(&self.program.modules[self.module].path, expr.span, e))?;
                let v = self.expr(inner, None)?;
                if matches!(&v.ty, Type::Enum(_,vs) if vs.iter().any(|(_,ts)| !ts.is_empty())) {
                    return self.fail(expr.span, "payload enums cannot be cast to integers");
                }
                if matches!(v.ty, Type::Enum(..)) && ty.integer() {
                    return Ok(Value {
                        code: format!("(({})({}))", c_type(ty), v.code),
                        ty: ty.clone(),
                        lvalue: false,
                    });
                }
                if !v.ty.scalar() || !ty.scalar() {
                    return self.fail(expr.span, "cast requires scalar types");
                }
                if matches!(v.ty, Type::Ptr(_)) && self.unsafe_depth == 0 {
                    return self.fail(expr.span, "pointer casts require an unsafe block");
                }
                let from_pointer = matches!(v.ty, Type::Ptr(_) | Type::Str);
                let to_pointer = matches!(ty, Type::Ptr(_) | Type::Str);
                if (from_pointer && !(to_pointer || ty.integer() || *ty == Type::Bool))
                    || (to_pointer && !(from_pointer || v.ty.integer()))
                {
                    return self.fail(
                        expr.span,
                        "pointer casts require another pointer or integer",
                    );
                }
                if matches!(v.ty, Type::Str) && matches!(ty, Type::Ptr(_)) {
                    return self.fail(
                        expr.span,
                        "str is read-only; use a writable byte array for mutable memory",
                    );
                }
                (format!("(({})({}))", c_type(ty), v.code), ty.clone())
            }
            ExprKind::Index(inner, index) => {
                let v = self.expr(inner, None)?;
                let i = self.expr(index, None)?;
                if let Type::Vector(t) | Type::Slice(t) = &v.ty {
                    if !i.ty.integer() {
                        return self.fail(index.span, "index requires an integer");
                    }
                    let mutable = self.writing && v.lvalue && matches!(v.ty, Type::Vector(_));
                    if self.writing && !mutable {
                        return self.fail(expr.span, "slice is read-only");
                    }
                    let name = self.collection(&v.ty);
                    lvalue = mutable;
                    return Ok(Value {
                        code: format!(
                            "(*{name}_{}(&({}), (uint64_t)({}), {}))",
                            if mutable { "at_mut" } else { "at" },
                            v.code,
                            i.code,
                            self.location(expr.span)
                        ),
                        ty: self.resolve_collection_element(t),
                        lvalue,
                    });
                }
                if !i.ty.integer() {
                    return self.fail(index.span, "index requires an integer");
                }
                let array_size = if let Type::Array(_, n) = &v.ty {
                    Some(*n)
                } else {
                    None
                };
                let string_access = v.ty == Type::Str;
                let raw_pointer = matches!(v.ty, Type::Ptr(_));
                let (ty, writable) = match v.ty {
                    Type::Array(t, n) => {
                        if let ExprKind::Number(s) = &index.kind {
                            let number = if s.starts_with("0x") || s.starts_with("0X") {
                                usize::from_str_radix(&s[2..], 16).ok()
                            } else {
                                s.parse::<usize>().ok()
                            };
                            if number.is_some_and(|i| i >= n) {
                                return self
                                    .fail(index.span, "constant array index is out of bounds");
                            }
                        }
                        (*t, v.lvalue)
                    }
                    Type::Ptr(t) if *t != Type::Void => {
                        if self.unsafe_depth == 0 {
                            return self
                                .fail(expr.span, "raw pointer indexing requires an unsafe block");
                        }
                        (*t, true)
                    }
                    Type::Str => (
                        Type::Int {
                            bits: 8,
                            signed: false,
                        },
                        false,
                    ),
                    _ => {
                        return self.fail(
                            expr.span,
                            "indexing requires an array, element pointer, or str",
                        )
                    }
                };
                let ty = self.resolve_collection_element(&ty);
                if matches!(inner.kind, ExprKind::Array(..)) {
                    return self.fail(expr.span, "bind the array to a variable before indexing");
                }
                lvalue = writable;
                let index_code = if let Some(n) = array_size {
                    self.uses_checks = true;
                    format!(
                        "dev_checked_index((uint64_t)({}), {n}, {})",
                        i.code,
                        self.location(expr.span)
                    )
                } else {
                    i.code
                };
                let access = if string_access {
                    self.uses_checks = true;
                    format!(
                        "dev_checked_byte({}, (uint64_t)({index_code}), {})",
                        v.code,
                        self.location(expr.span)
                    )
                } else if raw_pointer {
                    self.uses_checks = true;
                    format!("(({} *)dev_checked_pointer((uintptr_t)({}),_Alignof({}),{}))[{index_code}]",c_type(&ty),v.code,c_type(&ty),self.location(expr.span))
                } else {
                    format!("({}[{index_code}])", v.code)
                };
                (
                    if writable {
                        access
                    } else {
                        format!("(({}){access})", c_type(&ty))
                    },
                    ty,
                )
            }
            ExprKind::Array(values) => {
                if values.is_empty() {
                    return self.fail(expr.span, "array cannot be empty");
                }
                let inner_expected = match expected {
                    Some(Type::Array(t, n)) => {
                        if *n != values.len() {
                            return self
                                .fail(expr.span, "array initializer length does not match type");
                        }
                        Some(t.as_ref())
                    }
                    _ => None,
                };
                let first = self.expr(&values[0], inner_expected)?;
                if !first.ty.scalar()
                    && !matches!(first.ty, Type::Ref(..) | Type::Record(..) | Type::Enum(..))
                {
                    return self.fail(expr.span, "array elements require scalar values");
                }
                let mut codes = vec![first.code];
                for value in &values[1..] {
                    let v = self.expr(value, Some(&first.ty))?;
                    self.compatible(&v, &first.ty, value.span)?;
                    codes.push(v.code);
                }
                (
                    format!("{{ {} }}", codes.join(", ")),
                    Type::Array(Box::new(first.ty), values.len()),
                )
            }
            ExprKind::Call(names, args) => return self.call(names, args, expr.span),
        };
        Ok(Value { code, ty, lvalue })
    }
    fn low32(&mut self, expr: &Expr, wide: &Type) -> Result<String, String> {
        if let ExprKind::Binary(op, left, right) = &expr.kind {
            if ["+", "-", "*", "&", "|", "^"].contains(&op.as_str()) {
                let left = self.low32(left, wide)?;
                let right = self.low32(right, wide)?;
                return Ok(format!("((uint32_t)({left} {op} {right}))"));
            }
        }
        let value = self.expr(expr, Some(wide))?;
        Ok(format!("((uint32_t)({}))", value.code))
    }
    fn call(&mut self, names: &[String], args: &[Expr], span: Span) -> Result<Value, String> {
        if names.len() == 1 {
            match names[0].as_str() {
                "print" => {
                    if args.len() != 1 {
                        return self.fail(span, "print expects one value");
                    }
                    if !self.hosted {
                        return self.fail(span, "print is unavailable in freestanding mode; call your device's C driver instead");
                    }
                    let v = self.expr(&args[0], None)?;
                    self.uses_print = true;
                    let code = match v.ty {
                        Type::Str => format!("((void)printf(\"%s\\n\", {}))", v.code),
                        Type::Bool => format!("((void)puts({} ? \"true\" : \"false\"))", v.code),
                        Type::Enum(ref n, _) => format!("((void)puts({n}_name({})))", v.code),
                        Type::Int { signed: true, .. } | Type::Size { signed: true } => {
                            format!("((void)printf(\"%lld\\n\", (long long)({})))", v.code)
                        }
                        Type::Int { signed: false, .. } | Type::Size { signed: false } => format!(
                            "((void)printf(\"%llu\\n\", (unsigned long long)({})))",
                            v.code
                        ),
                        Type::Float(_) => {
                            format!("((void)printf(\"%.17g\\n\", (double)({})))", v.code)
                        }
                        Type::Ptr(_) => format!("((void)printf(\"%p\\n\", (void *)({})))", v.code),
                        _ => {
                            return self
                                .fail(span, "print supports strings, numbers, bools and pointers")
                        }
                    };
                    return Ok(Value {
                        code,
                        ty: Type::Void,
                        lvalue: false,
                    });
                }
                "sizeof" => {
                    if args.len() != 1 {
                        return self
                            .fail(span, "sizeof expects one expression (it is not evaluated)");
                    }
                    let saved = self.prelude.clone();
                    let start = self.temporaries.len();
                    let v = self.expr(&args[0], None)?;
                    self.prelude = saved;
                    self.temporaries.truncate(start);
                    if v.ty == Type::Void {
                        return self.fail(span, "sizeof cannot accept void");
                    }
                    if matches!(args[0].kind, ExprKind::Array(..)) {
                        return self.fail(span, "bind array literals before using sizeof");
                    }
                    let operand = if matches!(v.ty, Type::Array(..)) {
                        v.code
                    } else {
                        c_type(&v.ty)
                    };
                    return Ok(Value {
                        code: format!("((uintptr_t)sizeof({operand}))"),
                        ty: Type::Size { signed: false },
                        lvalue: false,
                    });
                }
                "callback_context" => {
                    if !self.hosted {
                        return self.fail(span, "context callbacks require hosted mode");
                    }
                    if args.len() != 1 {
                        return self.fail(span, "callback_context expects one function value");
                    }
                    let v = self.expr(&args[0], None)?;
                    let Some(t) = callback_context_type(&v.ty) else {
                        return self.fail(span, "callback_context expects a function value");
                    };
                    self.context_callback_types.insert(c_type(&t), v.ty.clone());
                    self.helper(&v.ty, true);
                    self.helper(&v.ty, false);
                    self.uses_checks = true;
                    return Ok(self.owned(Value {
                        code: format!("{}_make({})", c_type(&t), v.code),
                        ty: t,
                        lvalue: false,
                    }));
                }
                "callback" => {
                    if args.len() != 1 {
                        return self.fail(span, "callback expects one function");
                    }
                    // Plain functions need no trampoline allocation.
                    if let ExprKind::Callable(n, None, Type::Function(ps, r)) = &args[0].kind {
                        let sig = self.program.resolve(self.module, n, span)?;
                        return Ok(Value {
                            code: sig.c_name.clone(),
                            ty: Type::Callback(ps.clone(), r.clone()),
                            lvalue: false,
                        });
                    }
                    if !self.hosted {
                        return self.fail(span, "capturing callbacks require hosted mode");
                    }
                    let v = self.expr(&args[0], None)?;
                    let Type::Function(ps, r) = &v.ty else {
                        return self.fail(span, "callback expects a function value");
                    };
                    let t = Type::Callback(ps.clone(), r.clone());
                    let name = c_type(&t);
                    self.callback_types.insert(name.clone(), t.clone());
                    self.helper(&v.ty, true);
                    self.uses_checks = true;
                    return Ok(Value {
                        code: format!("{name}_create({}, {})", v.code, self.location(span)),
                        ty: t,
                        lvalue: false,
                    });
                }

                "spawn" | "await" | "ready" => return self.task_call(&names[0], args, span),
                "volatile_load" | "volatile_store" => {
                    let store = names[0] == "volatile_store";
                    if args.len() != if store { 2 } else { 1 } {
                        return self.fail(span, "volatile_load expects a pointer; volatile_store expects a pointer and value");
                    }
                    let p = self.expr(&args[0], None)?;
                    let inner = match p.ty {
                        Type::Ptr(t) if t.numeric() || *t == Type::Bool => *t,
                        _ => {
                            return self.fail(
                                span,
                                "volatile access requires a pointer to a number or bool",
                            )
                        }
                    };
                    self.uses_checks = true;
                    let access = format!(
                        "(*(volatile {} *)dev_checked_pointer((uintptr_t)({}),_Alignof({}),{}))",
                        c_type(&inner),
                        p.code,
                        c_type(&inner),
                        self.location(span)
                    );
                    if store {
                        let v = self.expr(&args[1], Some(&inner))?;
                        self.compatible(&v, &inner, args[1].span)?;
                        return Ok(Value {
                            code: format!("((void)({access} = {}))", v.code),
                            ty: Type::Void,
                            lvalue: false,
                        });
                    }
                    return Ok(Value {
                        code: access,
                        ty: inner,
                        lvalue: false,
                    });
                }
                _ => {}
            }
        }
        let signature = self.program.resolve(self.module, names, span)?.clone();
        if args.len() < signature.params.len()
            || (!signature.variadic && signature.params.len() != args.len())
        {
            return self.fail(
                span,
                format!(
                    "{} expects {} arguments, found {}",
                    names.join("."),
                    signature.params.len(),
                    args.len()
                ),
            );
        }
        let mut codes = Vec::new();
        for (arg, ty) in args.iter().zip(&signature.params) {
            let v = self.expr(arg, Some(ty))?;
            self.compatible(&v, ty, arg.span)?;
            codes.push(v.code);
        }
        for arg in args.iter().skip(signature.params.len()) {
            let v = self.expr(arg, None)?;
            let ty = variadic_type(&v.ty)
                .map_err(|m| error(&self.program.modules[self.module].path, arg.span, m))?;
            codes.push(format!("(({})({}))", c_type(&ty), v.code));
        }
        let result = Value {
            code: format!("{}({})", signature.c_name, codes.join(", ")),
            ty: signature.ret,
            lvalue: false,
        };
        Ok(self.owned(result))
    }
}
fn low32_mask(expr: &Expr) -> bool {
    if let ExprKind::Number(number) = &expr.kind {
        let value = if number.starts_with("0x") || number.starts_with("0X") {
            u64::from_str_radix(&number[2..], 16).ok()
        } else {
            number.parse::<u64>().ok()
        };
        return value == Some(u32::MAX as u64);
    }
    false
}
fn literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Number(_) => true,
        ExprKind::Unary(op, e) if op == "+" || op == "-" => literal(e),
        ExprKind::Name(n) => n.as_slice() == ["null"],
        _ => false,
    }
}
pub fn c_string(value: &str) -> String {
    let mut out = String::from("\"");
    for byte in value.bytes() {
        match byte {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'?' => out.push_str("\\?"),
            32..=126 => out.push(byte as char),
            _ => out.push_str(&format!("\\{byte:03o}")),
        }
    }
    out.push('"');
    out
}

// Per-value reference counts; the compiler balances locals and expression temporaries.
const MANAGED_REFS:&str="#include <stdlib.h>\n#include <string.h>\n#ifndef DEV_COUNT\n#define DEV_COUNT size_t\n#endif\nstruct dev_ref_block { DEV_COUNT count; size_t bytes; size_t items; void (*drop)(void *); long double alignment; };\nstatic void dev_ref_retain(const void *v) { if(v) { struct dev_ref_block *b=((struct dev_ref_block *)v)-1; if(b->count==SIZE_MAX) abort(); ++b->count; } }\nstatic void dev_ref_release(const void *v) { if(v) { struct dev_ref_block *b=((struct dev_ref_block *)v)-1; if(--b->count==0) { if(b->drop) b->drop((void *)v); free(b); } } }\nstatic void *dev_ref_copy(const void *v,size_t n,void (*keep)(void *),void (*drop)(void *)) { if(n>SIZE_MAX-sizeof(struct dev_ref_block)) abort(); struct dev_ref_block *b=(struct dev_ref_block *)malloc(sizeof(*b)+n); if(!b) abort(); b->count=1; b->bytes=n; b->items=0; b->drop=drop; void *out=b+1; memcpy(out,v,n); if(keep) keep(out); return out; }\n";
pub(crate) fn managed_key(t: &Type) -> String {
    let text = format!("{t:?}");
    let hash = text.bytes().fold(0xcbf29ce484222325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    });
    format!("{hash:016x}")
}
fn managed_visit(t: &Type, access: &str, retain: bool, out: &mut String) {
    let op = if retain { "retain" } else { "release" };
    match t {
        Type::Task(_) => out.push_str(&format!("dev_ref_{op}(({access}).state);\n")),
        Type::Function(..) => out.push_str(&format!("dev_ref_{op}(({access}).context);\n")),
        Type::Map(..) => out.push_str(&format!(
            "dev_ref_{op}(({access}).entries.data);dev_ref_{op}(({access}).buckets.data);\n"
        )),
        Type::Vector(_) | Type::Slice(_) => {
            out.push_str(&format!("dev_ref_{op}(({access}).data);\n"))
        }
        Type::Ref(_) => out.push_str(&format!("dev_ref_{op}({access});\n")),
        Type::Record(_, fs) => {
            for (i, (_, t)) in fs.iter().enumerate() {
                managed_visit(t, &format!("({access}).dev_f_{i}"), retain, out);
            }
        }
        Type::Enum(_, vs) if vs.iter().any(|(_, ts)| !ts.is_empty()) => {
            out.push_str(&format!("switch(({access}).tag) {{\n"));
            for (i, (_, ts)) in vs.iter().enumerate() {
                out.push_str(&format!("case {i}:\n"));
                for (j, t) in ts.iter().enumerate() {
                    managed_visit(
                        t,
                        &format!("({access}).data.dev_v_{i}.dev_p_{j}"),
                        retain,
                        out,
                    );
                }
                out.push_str("break;\n");
            }
            out.push_str("}\n");
        }
        Type::Array(t, n) if contains_managed(t) => {
            out.push_str(&format!("for(size_t dev_i=0;dev_i<{n};++dev_i) {{\n"));
            managed_visit(t, &format!("({access})[dev_i]"), retain, out);
            out.push_str("}\n");
        }
        _ => {}
    }
}

fn op_name(op: &str) -> &str {
    match op {
        "/" => "div",
        "%" => "rem",
        "<<" => "shl",
        ">>" => "shr",
        _ => unreachable!(),
    }
}
fn checked_helpers(hosted: bool) -> String {
    let mut out = String::from("#include <string.h>\n");
    if hosted {
        out.push_str("#include <stdio.h>\n#include <stdlib.h>\nstatic void dev_checked_fail(const char *p, size_t l, size_t c, const char *m) { fprintf(stderr, \"%s:%zu:%zu: %s\\n\",p,l,c,m); exit(1); }\n");
    } else {
        out.push_str("static void dev_checked_fail(const char *p,size_t l,size_t c,const char *m) { (void)p;(void)l;(void)c;(void)m;for(;;){} }\n");
    }
    out.push_str("static size_t dev_checked_index(uint64_t i,size_t n,const char *p,size_t l,size_t c) { if(i>=n) dev_checked_fail(p,l,c,\"array index out of bounds\"); return (size_t)i; }\nstatic uint8_t dev_checked_byte(const char *s,uint64_t i,const char *p,size_t l,size_t c) { if(!s || i>=strlen(s)) dev_checked_fail(p,l,c,\"string index out of bounds\"); return (uint8_t)s[i]; }\n");
    out.push_str("static void *dev_checked_pointer(uintptr_t v,size_t a,const char *p,size_t l,size_t c) { if(!v)dev_checked_fail(p,l,c,\"null pointer dereference\");if(v%a)dev_checked_fail(p,l,c,\"misaligned native pointer\");return (void *)v; }\n");
    for signed in [false, true] {
        for bits in [8, 16, 32, 64] {
            let ty = Type::Int { bits, signed };
            let n = ty.name();
            let ct = c_type(&ty);
            let u = format!("uint{bits}_t");
            for op in ["div", "rem", "shl", "shr"] {
                let check = if op == "div" || op == "rem" {
                    "if(b==0) dev_checked_fail(p,l,c,\"division by zero\");".to_string()
                } else {
                    format!("if((uint64_t)b>={bits}) dev_checked_fail(p,l,c,\"shift count out of range\");")
                };
                let exceptional = if signed && (op == "div" || op == "rem") {
                    format!(
                        "if(a==INT{bits}_MIN && b==-1) return {};",
                        if op == "div" {
                            format!("INT{bits}_MIN")
                        } else {
                            "0".into()
                        }
                    )
                } else {
                    String::new()
                };
                let expr = match op {
                    "div" => "a/b".to_string(),
                    "rem" => "a%b".into(),
                    "shl" => format!("(({u})a)<<b"),
                    _ => "a>>b".into(),
                };
                out.push_str(&format!("static {ct} dev_checked_{op}_{n}({ct} a,{ct} b,const char *p,size_t l,size_t c) {{ {check}{exceptional} return ({ct})({expr}); }}\nstatic void dev_checked_{op}_assign_{n}({ct} *a,{ct} b,const char *p,size_t l,size_t c) {{ *a=dev_checked_{op}_{n}(*a,b,p,l,c); }}\n"));
            }
        }
    }
    // intptr_t and uintptr_t share their target's fixed-width checks.
    for (name, fixed) in [
        ("isize", format!("i{}", usize::BITS)),
        ("usize", format!("u{}", usize::BITS)),
    ] {
        for op in ["div", "rem", "shl", "shr"] {
            out.push_str(&format!("#define dev_checked_{op}_{name} dev_checked_{op}_{fixed}\n#define dev_checked_{op}_assign_{name} dev_checked_{op}_assign_{fixed}\n"));
        }
    }
    out
}

fn pattern_c(
    p: &Pattern,
    t: &Type,
    access: &str,
    conditions: &mut Vec<String>,
    bindings: &mut Vec<(String, String, Type)>,
) {
    match p {
        Pattern::Bind(n) => {
            if let Type::Enum(_, vs) = t {
                if let Some(i) = vs.iter().position(|(v, _)| v == n) {
                    conditions.push(format!(
                        "{} == {i}",
                        if vs.iter().any(|(_, ts)| !ts.is_empty()) {
                            format!("({access}).tag")
                        } else {
                            access.into()
                        }
                    ));
                    return;
                }
            }
            bindings.push((n.clone(), access.into(), t.clone()));
        }
        Pattern::Variant(n, ps) => {
            let Type::Enum(_, vs) = t else { unreachable!() };
            let i = vs.iter().position(|(v, _)| v == n).unwrap();
            conditions.push(format!(
                "{} == {i}",
                if vs.iter().any(|(_, ts)| !ts.is_empty()) {
                    format!("({access}).tag")
                } else {
                    access.into()
                }
            ));
            for (j, (p, t)) in ps.iter().zip(&vs[i].1).enumerate() {
                pattern_c(
                    p,
                    t,
                    &format!("({access}).data.dev_v_{i}.dev_p_{j}"),
                    conditions,
                    bindings,
                );
            }
        }
    }
}
