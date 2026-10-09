//! Shared nominal type resolution and explicit generic specialization.
use crate::ast::*;
use std::collections::{HashMap, HashSet};

type Bindings = HashMap<String, Type>;
struct Expand {
    module_aliases: Vec<HashSet<String>>,
    import_paths: Vec<HashMap<String, String>>,
    traits: Vec<HashMap<String, TraitDefinition>>,
    paths: Vec<std::path::PathBuf>,
    aliases: Vec<HashMap<String, usize>>,
    defs: Vec<HashMap<String, TypeDefinition>>,
    functions: Vec<HashMap<String, Function>>,
    concrete: Vec<HashMap<String, Type>>,
    busy: HashSet<String>,
    pending: Vec<(usize, Type, Bindings, Span)>,
    instances: HashMap<String, String>,
    queue: Vec<(usize, Function, Bindings)>,
    locals: Vec<Bindings>,
    hint: Option<Type>,
    return_type: Type,
    nominals: HashMap<String, (usize, String, Vec<Type>)>,
}
fn hash(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100000001b3)
    })
}
pub fn modules(modules: &mut [Module], aliases: &[HashMap<String, usize>]) -> Result<(), String> {
    let mut e = Expand {
        paths: modules.iter().map(|m| m.path.clone()).collect(),
        aliases: aliases.to_vec(),
        import_paths: modules
            .iter()
            .map(|m| {
                m.imports
                    .iter()
                    .map(|i| (i.alias.clone(), i.path.clone()))
                    .collect()
            })
            .collect(),
        module_aliases: modules
            .iter()
            .map(|m| m.imports.iter().map(|i| i.alias.clone()).collect())
            .collect(),
        defs: vec![],
        functions: vec![],
        concrete: vec![HashMap::new(); modules.len()],
        busy: HashSet::new(),
        pending: vec![],
        instances: HashMap::new(),
        queue: vec![],
        locals: vec![HashMap::new()],
        hint: None,
        return_type: Type::i32(),
        nominals: HashMap::new(),
        traits: modules
            .iter()
            .map(|m| {
                m.traits
                    .iter()
                    .map(|t| (t.name.clone(), t.clone()))
                    .collect()
            })
            .collect(),
    };
    for (id, m) in modules.iter_mut().enumerate() {
        unique(&m.traits.iter().map(|t| t.name.clone()).collect::<Vec<_>>())
            .map_err(|msg| error(&m.path, Span { line: 1, col: 1 }, msg))?;
        for t in &m.traits {
            unique(&t.methods.iter().map(|f| f.name.clone()).collect::<Vec<_>>())
                .map_err(|msg| error(&m.path, t.span, msg))?;
        }
        let mut defs = HashMap::new();
        for d in &m.definitions {
            if primitive(&d.name) {
                return Err(error(&m.path, d.span, "type name is reserved"));
            }
            if d.generics.iter().any(|n| primitive(n)) {
                return Err(error(&m.path, d.span, "type parameter name is reserved"));
            }
            if defs.insert(d.name.clone(), d.clone()).is_some() {
                return Err(error(&m.path, d.span, "duplicate type name"));
            }
            unique(&d.generics).map_err(|msg| error(&m.path, d.span, msg))?;
            if d.enumeration {
                unique(
                    &d.variants
                        .iter()
                        .map(|(n, _)| n.clone())
                        .collect::<Vec<_>>(),
                )
                .map_err(|msg| error(&m.path, d.span, msg))?;
                if d.variants.is_empty() {
                    return Err(error(&m.path, d.span, "enum needs a variant"));
                }
            } else {
                unique(&d.fields.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>())
                    .map_err(|msg| error(&m.path, d.span, msg))?;
            }
        }
        let mut functions = HashMap::new();
        for f in std::mem::take(&mut m.functions) {
            if ["ref", "deref"].contains(&f.name.as_str())
                || functions.contains_key(&f.name)
                || defs.contains_key(&f.name)
            {
                return Err(error(&m.path, f.span, "duplicate function or type name"));
            }
            unique(&f.generics).map_err(|msg| error(&m.path, f.span, msg))?;
            if f.generics.iter().any(|n| primitive(n)) {
                return Err(error(&m.path, f.span, "type parameter name is reserved"));
            }
            if !f.generics.is_empty() && (f.exported || f.body.is_none()) {
                return Err(error(
                    &m.path,
                    f.span,
                    "generic functions cannot be extern or export",
                ));
            }
            if f.generics.is_empty() {
                e.queue.push((id, f.clone(), HashMap::new()));
            }
            functions.insert(f.name.clone(), f);
        }
        e.defs.push(defs);
        e.functions.push(functions);
    }
    // Validate even unused concrete declarations, including recursive layouts.
    for (id, module) in modules.iter_mut().enumerate() {
        let defs = e.defs[id]
            .values()
            .filter(|d| d.generics.is_empty())
            .cloned()
            .collect::<Vec<_>>();
        for d in defs {
            e.ty(
                id,
                &Type::Named(vec![d.name], vec![]),
                &HashMap::new(),
                d.span,
            )?;
        }
        e.locals = vec![HashMap::new()];
        e.return_type = Type::i32();
        e.stmts(id, &mut module.statements, &HashMap::new())?;
    }
    let mut at = 0;
    while at < e.queue.len() {
        let (id, mut f, bindings) = e.queue[at].clone();
        at += 1;
        for (_, ty) in &mut f.params {
            *ty = e.ty(id, ty, &bindings, f.span)?;
        }
        f.ret = e.ty(id, &f.ret, &bindings, f.span)?;
        if (f.exported || f.body.is_none())
            && (contains_managed(&f.ret) || f.params.iter().any(|(_, t)| contains_managed(t)))
        {
            return Err(error(
                &e.paths[id],
                f.span,
                "managed references cannot cross the C ABI; use raw pointer wrappers",
            ));
        }
        if f.body.is_none() || f.exported {
            foreign_value(&f.ret, false).map_err(|m| error(&e.paths[id], f.span, m))?;
            for (_, t) in &f.params {
                foreign_value(t, true).map_err(|m| error(&e.paths[id], f.span, m))?;
            }
        }
        e.locals = vec![f.params.iter().cloned().collect()];
        e.return_type = f.ret.clone();
        if let Some(body) = &mut f.body {
            e.stmts(id, body, &bindings)?;
        }
        f.generics.clear();
        modules[id].functions.push(f);
    }
    while let Some((id, ty, env, span)) = e.pending.pop() {
        e.ty(id, &ty, &env, span)?;
    }
    for (id, m) in modules.iter_mut().enumerate() {
        m.concrete_types = e.concrete[id].values().cloned().collect();
        m.concrete_types.sort_by_key(Type::name);
    }
    crate::safety::validate(modules, aliases)?;
    Ok(())
}
fn unique(names: &[String]) -> Result<(), &'static str> {
    let mut set = HashSet::new();
    if names.iter().any(|n| !set.insert(n)) {
        Err("duplicate field, variant or type parameter")
    } else {
        Ok(())
    }
}
fn primitive(name: &str) -> bool {
    [
        "Map", "Vec", "Slice", "ref", "deref", "Ref", "void", "bool", "str", "usize", "isize",
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    ]
    .contains(&name)
}
fn bounded(ty: &Type, depth: usize) -> bool {
    if depth > 64 {
        return false;
    }
    match ty {
        Type::Vector(t) | Type::Slice(t) | Type::Ref(t) | Type::Ptr(t) | Type::Array(t, _) => {
            bounded(t, depth + 1)
        }
        Type::Map(k, v) => bounded(k, depth + 1) && bounded(v, depth + 1),
        Type::Record(_, fields) => fields.iter().all(|(_, t)| bounded(t, depth + 1)),
        Type::Enum(_, variants) => variants
            .iter()
            .all(|(_, ts)| ts.iter().all(|t| bounded(t, depth + 1))),
        _ => true,
    }
}
impl Expand {
    fn target(&self, id: usize, name: &[String]) -> Option<(usize, String)> {
        match name {
            [n] => Some((id, n.clone())),
            [alias, n] => Some((*self.aliases[id].get(alias)?, n.clone())),
            _ => None,
        }
    }
    fn ty(&mut self, id: usize, ty: &Type, env: &Bindings, span: Span) -> Result<Type, String> {
        self.ty_inner(id, ty, env, span, false)
    }
    fn ty_inner(
        &mut self,
        id: usize,
        ty: &Type,
        env: &Bindings,
        span: Span,
        indirect: bool,
    ) -> Result<Type, String> {
        let path = self.paths[id].clone();
        let fail = |msg: &str| error(&path, span, msg);
        match ty {
            Type::Named(n, args) if n.as_slice() == ["ContextCallback"] => {
                if args.len() != 1 {
                    return Err(fail("ContextCallback expects one function type"));
                }
                let function = self.ty(id, &args[0], env, span)?;
                let t = callback_context_type(&function)
                    .ok_or_else(|| fail("ContextCallback expects a function type"))?;
                let Type::Record(_, fields) = &t else {
                    unreachable!()
                };
                self.ty(id, &fields[0].1, env, span)?;
                self.concrete[id].insert(t.name(), t.clone());
                Ok(t)
            }
            Type::Named(n, args) if n.as_slice() == ["Task"] => {
                if args.len() != 1 {
                    return Err(fail("Task expects one result type"));
                }
                let t = Type::Task(Box::new(self.ty(id, &args[0], env, span)?));
                self.concrete[id].insert(t.name(), t.clone());
                Ok(t)
            }
            Type::Task(r) => {
                let t = Type::Task(Box::new(self.ty(id, r, env, span)?));
                self.concrete[id].insert(t.name(), t.clone());
                Ok(t)
            }
            Type::ArrayConst(t, n) => {
                let Some(Type::Const(count)) = env.get(n) else {
                    return Err(fail("array length requires a const generic argument"));
                };
                if *count == 0 || *count > 1_000_000 {
                    return Err(fail("array length must be 1..1000000"));
                }
                Ok(Type::Array(Box::new(self.ty(id, t, env, span)?), *count))
            }
            Type::Callback(ps, r) => {
                let ps = ps
                    .iter()
                    .map(|t| self.ty(id, t, env, span))
                    .collect::<Result<Vec<_>, _>>()?;
                let r = self.ty(id, r, env, span)?;
                if ps.iter().any(|t| !t.scalar())
                    || r != Type::Void && !r.scalar()
                    || r == Type::Str
                {
                    return Err(fail("callbacks currently require C scalar parameters and a non-str scalar/void result"));
                }
                let t = Type::Callback(ps, Box::new(r));
                self.concrete[id].insert(t.name(), t.clone());
                Ok(t)
            }
            Type::Function(ps, r) => {
                let ps = ps
                    .iter()
                    .map(|t| self.ty(id, t, env, span))
                    .collect::<Result<Vec<_>, _>>()?;
                let r = self.ty(id, r, env, span)?;
                let t = Type::Function(ps, Box::new(r));
                self.concrete[id].insert(t.name(), t.clone());
                Ok(t)
            }
            Type::Ref(t) => {
                let t = self.ty_inner(id, t, env, span, true)?;
                let t = match t {
                    Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                    t => t,
                };
                if matches!(t, Type::Void | Type::Array(..)) {
                    return Err(fail("Ref requires a non-array value type"));
                }
                Ok(Type::Ref(Box::new(t)))
            }
            Type::Ptr(t) => {
                let t = self.ty_inner(id, t, env, span, true)?;
                let t = match t {
                    Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                    t => t,
                };
                Ok(Type::Ptr(Box::new(t)))
            }
            Type::Array(t, n) => Ok(Type::Array(Box::new(self.ty(id, t, env, span)?), *n)),
            Type::Named(names, args) if names.as_slice() == ["Map"] => {
                if args.len() != 2 {
                    return Err(fail("Map requires two type arguments"));
                }
                let k = self.ty(id, &args[0], env, span)?;
                if !(k.numeric()
                    || matches!(k, Type::Bool | Type::Str)
                    || matches!(&k,Type::Enum(_,vs) if vs.iter().all(|(_,ts)|ts.is_empty())))
                {
                    return Err(fail(
                        "Map key requires numeric, bool, str or payload-free enum",
                    ));
                }
                let v = self.ty_inner(id, &args[1], env, span, true)?;
                let v = match v {
                    Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                    t => t,
                };
                if matches!(v, Type::Void | Type::Array(..)) {
                    return Err(fail("Map value requires a non-array value"));
                }
                let ty = Type::Map(Box::new(k), Box::new(v));
                let entry = map_entry(&ty);
                let vector = Type::Vector(Box::new(entry.clone()));
                for t in [entry, vector, ty.clone()] {
                    self.concrete[id].insert(format!("collection:{t:?}"), t);
                }
                Ok(ty)
            }
            Type::Named(names, args)
                if names.len() == 1 && ["Vec", "Slice"].contains(&names[0].as_str()) =>
            {
                if args.len() != 1 {
                    return Err(fail("Vec/Slice requires one type argument"));
                }
                let element = self.ty_inner(id, &args[0], env, span, true)?;
                let element = match element {
                    Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                    t => t,
                };
                if matches!(element, Type::Void | Type::Array(..)) {
                    return Err(fail("collection element must be a non-array value"));
                }
                let ty = if names[0] == "Vec" {
                    Type::Vector(Box::new(element))
                } else {
                    Type::Slice(Box::new(element))
                };
                if let Type::Vector(element) = &ty {
                    let slice = Type::Slice(element.clone());
                    self.concrete[id].insert(format!("collection:{slice:?}"), slice);
                }
                self.concrete[id].insert(format!("collection:{ty:?}"), ty.clone());
                Ok(ty)
            }
            Type::Named(names, args) => {
                if let [n] = names.as_slice() {
                    if let Some(t) = env.get(n) {
                        if !args.is_empty() {
                            return Err(fail("type parameter cannot take type arguments"));
                        }
                        return Ok(t.clone());
                    }
                }
                let (owner, name) = self
                    .target(id, names)
                    .ok_or_else(|| fail("unknown type module"))?;
                let d = self.defs[owner]
                    .get(&name)
                    .cloned()
                    .ok_or_else(|| fail(&format!("unknown type '{name}'")))?;
                if args.len() != d.generics.len() {
                    return Err(fail("wrong type argument count"));
                }
                let args = args
                    .iter()
                    .map(|t| self.ty(id, t, env, span))
                    .collect::<Result<Vec<_>, _>>()?;
                if !args.iter().all(|t| bounded(t, 1)) {
                    return Err(fail("type nesting limit exceeded (64)"));
                }
                self.constraints(owner, &d.generics, &d.constraints, &args, span)?;
                let identity = format!("{}:{name}:{args:?}", self.paths[owner].display());
                let key = format!("dev_t_{:016x}", hash(&identity));
                self.nominals
                    .insert(key.clone(), (owner, name.clone(), args.clone()));
                if indirect {
                    if self.pending.len() + self.concrete.iter().map(|c| c.len()).sum::<usize>()
                        > 4096
                    {
                        return Err(fail("type specialization limit exceeded (4096)"));
                    }
                    if !self.concrete[owner].contains_key(&key) && !self.busy.contains(&key) {
                        self.pending.push((id, ty.clone(), env.clone(), span));
                    }
                    return Ok(Type::Nominal(key));
                }
                if let Some(t) = self.concrete[owner].get(&key) {
                    return Ok(t.clone());
                }
                if self.busy.len() >= 64 || !self.busy.insert(key.clone()) {
                    return Err(fail("recursive type layout or type nesting limit exceeded"));
                }
                let env = d.generics.iter().cloned().zip(args).collect();
                let ty = if d.enumeration {
                    Type::Enum(
                        key.clone(),
                        d.variants
                            .iter()
                            .map(|(n, ts)| {
                                let ts = ts
                                    .iter()
                                    .map(|t| self.ty(owner, t, &env, d.span))
                                    .collect::<Result<Vec<_>, _>>()?;
                                if ts.iter().any(|t| matches!(t, Type::Void | Type::Array(..))) {
                                    return Err(fail(
                                        "enum payload cannot be void or an array; use a struct",
                                    ));
                                }
                                Ok((n.clone(), ts))
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                    )
                } else {
                    let fields = d
                        .fields
                        .iter()
                        .map(|(n, t)| Ok((n.clone(), self.ty(owner, t, &env, d.span)?)))
                        .collect::<Result<Vec<_>, String>>()?;
                    if fields.iter().any(|(_, t)| *t == Type::Void) {
                        return Err(fail("struct fields cannot be void"));
                    }
                    Type::Record(key.clone(), fields)
                };
                self.busy.remove(&key);
                if !bounded(&ty, 0) {
                    return Err(fail("type nesting limit exceeded (64)"));
                }
                self.concrete[owner].insert(key, ty.clone());
                Ok(ty)
            }
            _ => Ok(ty.clone()),
        }
    }
    fn scoped_stmts(
        &mut self,
        id: usize,
        stmts: &mut [Stmt],
        env: &Bindings,
    ) -> Result<(), String> {
        self.locals.push(HashMap::new());
        let result = self.stmts(id, stmts, env);
        self.locals.pop();
        result
    }
    fn expr_hint(
        &mut self,
        id: usize,
        e: &mut Expr,
        env: &Bindings,
        hint: Option<Type>,
    ) -> Result<(), String> {
        self.hint = hint;
        self.expr(id, e, env)
    }
    fn stmts(&mut self, id: usize, stmts: &mut [Stmt], env: &Bindings) -> Result<(), String> {
        for stmt in stmts {
            match stmt {
                Stmt::Let {
                    name,
                    ty,
                    value,
                    span,
                } => {
                    if let Some(t) = ty {
                        *t = self.ty(id, t, env, *span)?;
                    }
                    self.expr_hint(id, value, env, ty.clone())?;
                    if let Some(t) = ty.clone().or(self.infer(id, value, None)?) {
                        self.locals.last_mut().unwrap().insert(name.clone(), t);
                    }
                }
                Stmt::Assign { target, value, .. } => {
                    self.expr(id, target, env)?;
                    let hint = self.infer(id, target, None)?;
                    self.expr_hint(id, value, env, hint)?;
                }
                Stmt::Return(Some(e), _) => {
                    self.expr_hint(id, e, env, Some(self.return_type.clone()))?
                }
                Stmt::Expr(e) => self.expr(id, e, env)?,
                Stmt::If { cond, yes, no } => {
                    self.expr(id, cond, env)?;
                    self.scoped_stmts(id, yes, env)?;
                    self.scoped_stmts(id, no, env)?;
                }
                Stmt::While { cond, body } => {
                    self.expr(id, cond, env)?;
                    self.scoped_stmts(id, body, env)?;
                }
                Stmt::Block(body) | Stmt::Unsafe(body) => self.scoped_stmts(id, body, env)?,
                Stmt::Match { value, arms } => {
                    self.expr(id, value, env)?;
                    let ty = self.infer(id, value, None)?;
                    for arm in arms {
                        let mut variables = HashMap::new();
                        if let Some(Type::Enum(_, vs)) = &ty {
                            if let Some((_, ts)) = vs.iter().find(|(n, _)| n == &arm.variant) {
                                for (p, t) in arm.bindings.iter().zip(ts) {
                                    infer_pattern(p, t, &mut variables);
                                }
                            }
                        }
                        self.locals.push(variables);
                        if let Some(g) = &mut arm.guard {
                            self.expr(id, g, env)?;
                        }
                        self.stmts(id, &mut arm.body, env)?;
                        self.locals.pop();
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn expr(&mut self, id: usize, e: &mut Expr, env: &Bindings) -> Result<(), String> {
        let span = e.span;
        let expected = self.hint.take();
        if matches!(e.kind, ExprKind::Closure(..)) {
            return self.closure(id, e, env);
        }
        if let ExprKind::Name(names) = &e.kind {
            if names.len() == 1 {
                if let Some(Type::Const(n)) = env.get(&names[0]) {
                    e.kind = ExprKind::Number(n.to_string());
                    return Ok(());
                }
            }
        }
        if let ExprKind::Name(names) = &e.kind {
            if !self.locals.iter().rev().any(|s| s.contains_key(&names[0])) {
                if let Some((owner, name)) = self.target(id, names) {
                    if let Some(f) = self.functions[owner].get(&name).cloned() {
                        if !f.generics.is_empty() || f.variadic || f.body.is_none() {
                            return Err(error(
                                &self.paths[id],
                                span,
                                "function values require a concrete Dev function",
                            ));
                        }
                        let params = f
                            .params
                            .iter()
                            .map(|(_, t)| self.ty(owner, t, &HashMap::new(), span))
                            .collect::<Result<Vec<_>, _>>()?;
                        let ret = self.ty(owner, &f.ret, &HashMap::new(), span)?;
                        let ty = Type::Function(params, Box::new(ret));
                        self.concrete[id].insert(ty.name(), ty.clone());
                        e.kind = ExprKind::Callable(names.clone(), None, ty);
                        return Ok(());
                    }
                }
            }
        }
        if let ExprKind::Call(names, args) = &e.kind {
            if self.locals.iter().rev().any(|s| s.contains_key(&names[0])) {
                let mut base = Expr {
                    kind: ExprKind::Name(names.clone()),
                    span,
                };
                self.expr(id, &mut base, env)?;
                if matches!(
                    self.infer(id, &base, None)?,
                    Some(Type::Function(..) | Type::Callback(..))
                ) {
                    e.kind = ExprKind::Invoke(Box::new(base), args.clone());
                }
            }
        }

        if matches!(&e.kind,ExprKind::GenericCall(n,_,_) if n.len()==1 && ["ref","deref"].contains(&n[0].as_str()))
        {
            return Err(error(
                &self.paths[id],
                span,
                "ref/deref do not take explicit type arguments",
            ));
        }
        // Normalize receiver calls before ordinary generic specialization.
        let receiver = match &e.kind {
            ExprKind::Collection(base, method, args) => {
                Some(((**base).clone(), method.clone(), args.clone()))
            }
            ExprKind::Call(names, args)
                if names.len() > 1 && !self.module_aliases[id].contains(&names[0]) =>
            {
                Some((
                    Expr {
                        kind: ExprKind::Name(names[..names.len() - 1].to_vec()),
                        span,
                    },
                    names.last().unwrap().clone(),
                    args.clone(),
                ))
            }
            _ => None,
        };
        if let Some((mut base, method, mut args)) = receiver {
            self.expr(id, &mut base, env)?;
            if let Some(t) = self.infer(id, &base, None)? {
                if let Type::Record(_, fields) = self.concrete(&t) {
                    if fields.iter().any(|(n, t)| {
                        n == &method && matches!(t, Type::Function(..) | Type::Callback(..))
                    }) {
                        e.kind = ExprKind::Invoke(
                            Box::new(Expr {
                                span,
                                kind: ExprKind::Field(Box::new(base), method),
                            }),
                            args,
                        );
                        self.hint = expected;
                        return self.expr(id, e, env);
                    }
                }
                let key = match t {
                    Type::Record(n, _) | Type::Enum(n, _) | Type::Nominal(n) => Some(n),
                    _ => None,
                };
                if let Some((owner, type_name, _)) =
                    key.as_ref().and_then(|n| self.nominals.get(n)).cloned()
                {
                    let function = format!("method_{type_name}_{method}");
                    if self.functions[owner].contains_key(&function) {
                        let names = if owner == id {
                            vec![function]
                        } else {
                            vec![
                                self.aliases[id]
                                    .iter()
                                    .find(|(_, v)| **v == owner)
                                    .map(|(k, _)| k.clone())
                                    .ok_or_else(|| {
                                        error(
                                            &self.paths[id],
                                            span,
                                            "import receiver module before calling its method",
                                        )
                                    })?,
                                function,
                            ]
                        };
                        args.insert(0, base);
                        e.kind = ExprKind::Call(names, args);
                        self.hint = expected;
                        return self.expr(id, e, env);
                    }
                }
            }
        }
        match &mut e.kind {
            ExprKind::Invoke(base, args) => {
                self.expr(id, base, env)?;
                for a in args {
                    self.expr(id, a, env)?;
                }
            }
            ExprKind::Cast(a, t) => {
                self.expr(id, a, env)?;
                *t = self.ty(id, t, env, span)?;
            }
            ExprKind::Reference(a)
            | ExprKind::Dereference(a)
            | ExprKind::Unary(_, a)
            | ExprKind::Field(a, _) => self.expr(id, a, env)?,
            ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                self.expr(id, a, env)?;
                self.expr(id, b, env)?;
            }
            ExprKind::Collection(base, _, args) => {
                self.expr(id, base, env)?;
                for a in args {
                    self.expr(id, a, env)?;
                }
            }
            ExprKind::Vector(_, items) | ExprKind::Array(items) | ExprKind::Record(_, items) => {
                for a in items {
                    self.expr(id, a, env)?;
                }
            }
            ExprKind::Name(names) if names.len() > 1 => {
                let type_name = &names[..names.len() - 1];
                if let Some((owner, name)) = self.target(id, type_name) {
                    if self.defs[owner].get(&name).is_some_and(|d| d.enumeration) {
                        let ty =
                            self.ty(id, &Type::Named(type_name.to_vec(), vec![]), env, span)?;
                        let Type::Enum(_, variants) = &ty else {
                            unreachable!()
                        };
                        let index = variants
                            .iter()
                            .position(|(v, _)| v == names.last().unwrap())
                            .ok_or_else(|| error(&self.paths[id], span, "unknown enum variant"))?;
                        if !variants[index].1.is_empty() {
                            return Err(error(
                                &self.paths[id],
                                span,
                                "enum variant requires payload arguments",
                            ));
                        }
                        e.kind = ExprKind::Enum(ty, index, vec![]);
                        return Ok(());
                    }
                }
                let mut base = Expr {
                    kind: ExprKind::Name(vec![names[0].clone()]),
                    span,
                };
                for field in &names[1..] {
                    base = Expr {
                        kind: ExprKind::Field(Box::new(base), field.clone()),
                        span,
                    };
                }
                e.kind = base.kind;
            }
            ExprKind::Call(names, args) | ExprKind::GenericCall(names, _, args) => {
                for a in args.iter_mut() {
                    self.expr(id, a, env)?;
                }
                if names.len() == 1 && ["ref", "deref"].contains(&names[0].as_str()) {
                    if args.len() != 1 {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "ref/deref expects one argument",
                        ));
                    }
                    let arg = Box::new(args[0].clone());
                    e.kind = if names[0] == "ref" {
                        ExprKind::Reference(arg)
                    } else {
                        ExprKind::Dereference(arg)
                    };
                    return Ok(());
                }
                let names = names.clone();
                let args = args.clone();
                if names.as_slice() == ["sizeof"] {
                    if args.len() != 1 {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "sizeof expects one expression",
                        ));
                    }
                    let t = self.infer(id, &args[0], None)?.ok_or_else(|| {
                        error(
                            &self.paths[id],
                            span,
                            "cannot determine sizeof operand type",
                        )
                    })?;
                    if t == Type::Void {
                        return Err(error(&self.paths[id], span, "sizeof cannot accept void"));
                    }
                    e.kind = ExprKind::SizeOf(t);
                    return Ok(());
                }
                if names.as_slice() == ["callback"] || names.as_slice() == ["callback_context"] {
                    if args.len() != 1 {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "callback expects a concrete function",
                        ));
                    }
                    let Some(Type::Function(ps, r)) = self.infer(id, &args[0], None)? else {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "callback expects a function value",
                        ));
                    };
                    self.ty(id, &Type::Callback(ps.clone(), r.clone()), env, span)?;
                    if names.as_slice() == ["callback_context"] {
                        let t = callback_context_type(&Type::Function(ps, r)).unwrap();
                        let Type::Record(_, fields) = &t else {
                            unreachable!()
                        };
                        self.ty(id, &fields[0].1, env, span)?;
                        self.concrete[id].insert(t.name(), t);
                    }
                    return Ok(());
                }
                if names.as_slice() == ["then"] {
                    e.kind = self.continuation(id, args, span)?;
                    return Ok(());
                }
                if names.as_slice() == ["ready"] {
                    return Ok(());
                }
                if names.as_slice() == ["spawn"] {
                    if let Some(a) = args.first() {
                        if let Some(Type::Function(ps, r)) = self.infer(id, a, None)? {
                            if ps.is_empty() {
                                let t = Type::Task(r);
                                self.concrete[id].insert(t.name(), t);
                            }
                        }
                    }
                    return Ok(());
                }

                let mut types = if let ExprKind::GenericCall(_, types, _) = &e.kind {
                    types.clone()
                } else {
                    vec![]
                };
                if types.is_empty() {
                    match (&expected, names.as_slice()) {
                        (Some(Type::Vector(t)), [name]) if name == "Vec" => {
                            types.push((**t).clone())
                        }
                        (Some(Type::Map(k, v)), [name]) if name == "Map" => {
                            types = vec![(**k).clone(), (**v).clone()]
                        }
                        _ => {}
                    }
                }
                if names.as_slice() == ["Map"] {
                    if !args.is_empty() {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "Map constructor expects no arguments",
                        ));
                    }
                    let ty = self.ty(id, &Type::Named(names, types), env, span)?;
                    e.kind = ExprKind::Map(ty);
                    return Ok(());
                }
                if names.as_slice() == ["Vec"] {
                    let ty = self.ty(id, &Type::Named(names, types), env, span)?;
                    e.kind = ExprKind::Vector(ty, args);
                    return Ok(());
                }
                if names.len() >= 2
                    && [
                        "set", "get", "contains", "remove", "push", "pop", "len", "slice", "clear",
                    ]
                    .contains(&names.last().unwrap().as_str())
                    && !self.module_aliases[id].contains(&names[0])
                {
                    if !types.is_empty() {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "collection methods do not take type arguments",
                        ));
                    }
                    let mut target = Expr {
                        kind: ExprKind::Name(names[..names.len() - 1].to_vec()),
                        span,
                    };
                    self.expr(id, &mut target, env)?;
                    e.kind =
                        ExprKind::Collection(Box::new(target), names.last().unwrap().clone(), args);
                    return Ok(());
                }
                if names.len() >= 2 {
                    let type_names = &names[..names.len() - 1];
                    if let Some((owner, name)) = self.target(id, type_names) {
                        if self.defs[owner].get(&name).is_some_and(|d| d.enumeration) {
                            let d = self.defs[owner][&name].clone();
                            if types.is_empty() && !d.generics.is_empty() {
                                let payload = d
                                    .variants
                                    .iter()
                                    .find(|(v, _)| v == names.last().unwrap())
                                    .map(|(_, ts)| ts.clone())
                                    .ok_or_else(|| {
                                        error(&self.paths[id], span, "unknown enum variant")
                                    })?;
                                types = self.infer_arguments(
                                    id,
                                    owner,
                                    &d.generics,
                                    &payload,
                                    &Type::Named(
                                        vec![name.clone()],
                                        d.generics
                                            .iter()
                                            .map(|n| Type::Named(vec![n.clone()], vec![]))
                                            .collect(),
                                    ),
                                    &args,
                                    expected.as_ref(),
                                    span,
                                )?;
                            }
                            let ty =
                                self.ty(id, &Type::Named(type_names.to_vec(), types), env, span)?;
                            let Type::Enum(_, variants) = &ty else {
                                unreachable!()
                            };
                            let index = variants
                                .iter()
                                .position(|(v, _)| v == names.last().unwrap())
                                .ok_or_else(|| {
                                    error(&self.paths[id], span, "unknown enum variant")
                                })?;
                            e.kind = ExprKind::Enum(ty, index, args);
                            return Ok(());
                        }
                    }
                }
                let Some((owner, name)) = self.target(id, &names) else {
                    return Ok(());
                };
                if let Some(d) = self.defs[owner].get(&name).cloned() {
                    if types.is_empty() && !d.generics.is_empty() {
                        types = self.infer_arguments(
                            id,
                            owner,
                            &d.generics,
                            &d.fields.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>(),
                            &Type::Named(
                                vec![name.clone()],
                                d.generics
                                    .iter()
                                    .map(|n| Type::Named(vec![n.clone()], vec![]))
                                    .collect(),
                            ),
                            &args,
                            expected.as_ref(),
                            span,
                        )?;
                    }
                    let ty = self.ty(id, &Type::Named(names, types), env, span)?;
                    if !matches!(ty, Type::Record(..)) {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "enum variants are referenced as Type.Variant",
                        ));
                    }
                    e.kind = ExprKind::Record(ty, args);
                } else if let Some(f) = self.functions[owner].get(&name).cloned() {
                    if types.is_empty() && !f.generics.is_empty() {
                        types = self.infer_arguments(
                            id,
                            owner,
                            &f.generics,
                            &f.params.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>(),
                            &f.ret,
                            &args,
                            expected.as_ref(),
                            span,
                        )?;
                    }
                    if types.len() != f.generics.len() {
                        return Err(error(
                            &self.paths[id],
                            span,
                            "wrong type argument count; generic calls need explicit types",
                        ));
                    }
                    if !types.is_empty() {
                        let types = types
                            .iter()
                            .map(|t| self.ty(id, t, env, span))
                            .collect::<Result<Vec<_>, _>>()?;
                        if types.contains(&Type::Void) {
                            return Err(error(
                                &self.paths[id],
                                span,
                                "void is not a generic type argument",
                            ));
                        }
                        self.constraints(owner, &f.generics, &f.constraints, &types, span)?;
                        let key = format!("{owner}:{name}:{types:?}");
                        let instance = if let Some(n) = self.instances.get(&key) {
                            n.clone()
                        } else {
                            if self.instances.len() >= 1024 {
                                return Err(error(
                                    &self.paths[id],
                                    span,
                                    "generic specialization limit exceeded (1024 functions)",
                                ));
                            }
                            let n = format!("g_{}_{:016x}", name, hash(&key));
                            self.instances.insert(key, n.clone());
                            let bindings = f.generics.iter().cloned().zip(types).collect();
                            let mut f = f;
                            f.name = n.clone();
                            self.queue.push((owner, f, bindings));
                            n
                        };
                        let mut names = names;
                        *names.last_mut().unwrap() = instance;
                        e.kind = ExprKind::Call(names, args);
                    }
                } else if !types.is_empty() {
                    return Err(error(&self.paths[id], span, "unknown generic function"));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

include!("inference.rs");

include!("closures.rs");

include!("constraints.rs");

include!("continuations.rs");
