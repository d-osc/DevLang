// Static expression types used only to select concrete generic instances.
// Execution and the native emitter retain their ordinary value/type validation.
impl Expand {
    fn concrete(&self, t: &Type) -> Type {
        if let Type::Nominal(n) = t {
            self.concrete
                .iter()
                .find_map(|c| c.get(n))
                .cloned()
                .unwrap_or_else(|| t.clone())
        } else {
            t.clone()
        }
    }
    fn infer(&mut self, id: usize, e: &Expr, hint: Option<&Type>) -> Result<Option<Type>, String> {
        let ty = match &e.kind {
            ExprKind::SizeOf(_) => Type::Size { signed: false },
            ExprKind::Callable(_, _, t) => t.clone(),
            ExprKind::Invoke(a, _) => {
                let Some(Type::Function(_, r) | Type::Callback(_, r)) = self.infer(id, a, None)?
                else {
                    return Ok(None);
                };
                *r
            }
            ExprKind::Number(n) => hint.filter(|t| t.numeric()).cloned().unwrap_or_else(|| {
                if !n.starts_with("0x")
                    && !n.starts_with("0X")
                    && (n.contains('.') || n.contains('e') || n.contains('E'))
                {
                    Type::Float(64)
                } else {
                    Type::i64()
                }
            }),
            ExprKind::Bool(_) => Type::Bool,
            ExprKind::String(_) => Type::Str,
            ExprKind::Name(n) if n.len() == 1 => {
                let Some(t) = self.locals.iter().rev().find_map(|s| s.get(&n[0])).cloned() else {
                    return Ok(None);
                };
                t
            }
            ExprKind::Object(t, _)
            | ExprKind::Record(t, _)
            | ExprKind::Enum(t, _, _)
            | ExprKind::Vector(t, _)
            | ExprKind::Map(t)
            | ExprKind::Cast(_, t) => t.clone(),
            ExprKind::Reference(a) => {
                let Some(t) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                Type::Ref(Box::new(match t {
                    Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                    t => t,
                }))
            }
            ExprKind::Dereference(a) => {
                let Some(Type::Ref(t)) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                self.concrete(&t)
            }
            ExprKind::Field(a, n) => {
                let Some(t) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                let Type::Record(_, fs) = self.concrete(&t) else {
                    return Ok(None);
                };
                let Some((_, t)) = fs.into_iter().find(|(name, _)| name == n) else {
                    return Ok(None);
                };
                t
            }
            ExprKind::Index(a, _) => {
                let Some(t) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                match t {
                    Type::Array(t, _) | Type::Vector(t) | Type::Slice(t) | Type::Ptr(t) => {
                        self.concrete(&t)
                    }
                    Type::Str => Type::Int {
                        bits: 8,
                        signed: false,
                    },
                    _ => return Ok(None),
                }
            }
            ExprKind::Array(xs) => {
                let Some(first) = xs.first() else {
                    return Ok(None);
                };
                let element_hint = if let Some(Type::Array(t, _)) = hint {
                    Some(t.as_ref())
                } else {
                    None
                };
                let Some(t) = self.infer(id, first, element_hint)? else {
                    return Ok(None);
                };
                Type::Array(Box::new(t), xs.len())
            }
            ExprKind::Unary(op, a) => {
                if op == "!" {
                    Type::Bool
                } else {
                    let Some(t) = self.infer(id, a, hint)? else {
                        return Ok(None);
                    };
                    match op.as_str() {
                        "&" => Type::Ptr(Box::new(match t {
                            Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                            t => t,
                        })),
                        "*" => {
                            if let Type::Ptr(t) = t {
                                self.concrete(&t)
                            } else {
                                return Ok(None);
                            }
                        }
                        _ => t,
                    }
                }
            }
            ExprKind::Binary(op, a, _) => {
                if ["==", "!=", "<", ">", "<=", ">=", "&&", "||"].contains(&op.as_str()) {
                    Type::Bool
                } else {
                    let Some(t) = self.infer(id, a, hint)? else {
                        return Ok(None);
                    };
                    t
                }
            }
            ExprKind::Collection(a, op, _) => {
                let Some(t) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                match op.as_str() {
                    "len" => Type::Size { signed: false },
                    "clear" | "push" | "set" => Type::Void,
                    "contains" | "remove" => Type::Bool,
                    "pop" => {
                        if let Type::Vector(t) = t {
                            self.concrete(&t)
                        } else {
                            return Ok(None);
                        }
                    }
                    "get" => {
                        if let Type::Map(_, t) = t {
                            self.concrete(&t)
                        } else {
                            return Ok(None);
                        }
                    }
                    "slice" => {
                        if let Type::Vector(t) | Type::Slice(t) = t {
                            Type::Slice(t)
                        } else {
                            return Ok(None);
                        }
                    }
                    _ => return Ok(None),
                }
            }
            ExprKind::Call(names, args) if names.as_slice() == ["callback_context"] => {
                let Some(a) = args.first() else {
                    return Ok(None);
                };
                let Some(t) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                let Some(t) = callback_context_type(&t) else {
                    return Ok(None);
                };
                t
            }
            ExprKind::Call(names, args) if names.as_slice() == ["callback"] => {
                let Some(a) = args.first() else {
                    return Ok(None);
                };
                let Some(Type::Function(ps, r)) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                Type::Callback(ps, r)
            }
            ExprKind::Call(names, args) if names.as_slice() == ["spawn"] => {
                let Some(a) = args.first() else {
                    return Ok(None);
                };
                let Some(Type::Function(ps, r)) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                if !ps.is_empty() {
                    return Ok(None);
                }
                Type::Task(r)
            }
            ExprKind::Call(names, _) if names.as_slice() == ["ready"] => Type::Bool,
            ExprKind::Call(names, args) if names.as_slice() == ["await"] => {
                let Some(a) = args.first() else {
                    return Ok(None);
                };
                let Some(Type::Task(r)) = self.infer(id, a, None)? else {
                    return Ok(None);
                };
                *r
            }
            ExprKind::Call(names, _) => {
                if names.len() == 2 && self.target(id, names).is_none() {
                    if let Some(module) = self.import_paths[id].get(&names[0]) {
                        let t = match (module.as_str(), names[1].as_str()) {
                            ("std/strings", "concat" | "view")
                            | ("std/io", "read_line" | "read_file")
                            | ("std/args", "get") => Some(Type::Str),
                            ("std/strings", "len") | ("std/args", "len") => {
                                Some(Type::Size { signed: false })
                            }
                            ("std/strings", "equal")
                            | ("std/io", "write" | "writeln" | "write_file") => Some(Type::Bool),
                            ("std/time", "now_ns" | "now_ms") => Some(Type::Int {
                                bits: 64,
                                signed: false,
                            }),
                            ("std/time", "sleep_ms") => Some(Type::Void),
                            _ => None,
                        };
                        return Ok(t);
                    }
                }
                let Some((owner, name)) = self.target(id, names) else {
                    return Ok(None);
                };
                if let Some(f) = self.functions[owner].get(&name).cloned() {
                    if !f.generics.is_empty() {
                        return Ok(None);
                    }
                    self.ty(owner, &f.ret, &HashMap::new(), e.span)?
                } else if let Some((_, f, env)) = self
                    .queue
                    .iter()
                    .find(|(i, f, _)| *i == owner && f.name == name)
                    .cloned()
                {
                    self.ty(owner, &f.ret, &env, e.span)?
                } else {
                    return Ok(None);
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(ty))
    }
    fn unify(
        &self,
        owner: usize,
        pattern: &Type,
        actual: &Type,
        generics: &[String],
        bindings: &mut Bindings,
    ) -> Result<(), String> {
        if let Type::Named(names, args) = pattern {
            if names.len() == 1 && args.is_empty() && generics.contains(&names[0]) {
                let t = self.concrete(actual);
                if let Some(old) = bindings.get(&names[0]) {
                    if old != &t {
                        return Err(
                            "conflicting inferred type arguments; use explicit types".into()
                        );
                    }
                } else {
                    bindings.insert(names[0].clone(), t);
                }
                return Ok(());
            }
            if names.as_slice() == ["Vec"] || names.as_slice() == ["Slice"] {
                if let Type::Vector(t) | Type::Slice(t) = actual {
                    if args.len() == 1 {
                        return self.unify(owner, &args[0], t, generics, bindings);
                    }
                }
            }
            if names.as_slice() == ["Map"] {
                if let Type::Map(k, v) = actual {
                    if args.len() == 2 {
                        self.unify(owner, &args[0], k, generics, bindings)?;
                        return self.unify(owner, &args[1], v, generics, bindings);
                    }
                }
            }
            let key = match actual {
                Type::Record(n, _) | Type::Enum(n, _) | Type::Nominal(n) => Some(n),
                _ => None,
            };
            if let Some((actual_owner, name, types)) = key.and_then(|key| self.nominals.get(key)) {
                if self.target(owner, names) == Some((*actual_owner, name.clone()))
                    && args.len() == types.len()
                {
                    for (p, t) in args.iter().zip(types) {
                        self.unify(owner, p, t, generics, bindings)?;
                    }
                    return Ok(());
                }
            }
            return Ok(());
        }
        match (pattern, actual) {
            (Type::ArrayConst(p, n), Type::Array(t, m)) => {
                self.unify(owner, p, t, generics, bindings)?;
                if let Some(old) = bindings.insert(n.clone(), Type::Const(*m)) {
                    if old != Type::Const(*m) {
                        return Err("conflicting const generic arguments".into());
                    }
                }
            }
            (Type::Ref(p), Type::Ref(t))
            | (Type::Ptr(p), Type::Ptr(t))
            | (Type::Vector(p), Type::Vector(t))
            | (Type::Slice(p), Type::Slice(t)) => self.unify(owner, p, t, generics, bindings)?,
            (Type::Array(p, n), Type::Array(t, m)) if n == m => {
                self.unify(owner, p, t, generics, bindings)?
            }
            _ => {}
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    fn infer_arguments(
        &mut self,
        id: usize,
        owner: usize,
        generics: &[String],
        parameters: &[Type],
        ret: &Type,
        args: &[Expr],
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Vec<Type>, String> {
        let mut bindings = HashMap::new();
        if let Some(t) = expected {
            self.unify(owner, ret, t, generics, &mut bindings)
                .map_err(|e| error(&self.paths[id], span, e))?;
        }
        for (p, arg) in parameters.iter().zip(args) {
            let hint = if let Type::Named(n, ts) = p {
                if n.len() == 1 && ts.is_empty() {
                    bindings.get(&n[0]).cloned()
                } else {
                    None
                }
            } else if p.numeric() {
                Some(p.clone())
            } else {
                None
            };
            if let Some(t) = self.infer(id, arg, hint.as_ref())? {
                self.unify(owner, p, &t, generics, &mut bindings)
                    .map_err(|e| error(&self.paths[id], span, e))?;
            }
        }
        generics.iter().map(|n|bindings.get(n).cloned().ok_or_else(||error(&self.paths[id],span,"wrong type argument count; cannot infer all types, supply explicit arguments"))).collect()
    }
}
fn infer_pattern(p: &Pattern, t: &Type, vars: &mut Bindings) {
    match p {
        Pattern::Bind(n) => {
            if n != "_" && !matches!(t,Type::Enum(_,vs) if vs.iter().any(|(v,_)|v==n)) {
                vars.insert(n.clone(), t.clone());
            }
        }
        Pattern::Variant(n, ps) => {
            if let Type::Enum(_, vs) = t {
                if let Some((_, ts)) = vs.iter().find(|(name, _)| name == n) {
                    for (p, t) in ps.iter().zip(ts) {
                        infer_pattern(p, t, vars)
                    }
                }
            }
        }
    }
}
