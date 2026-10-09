impl Expand {
    fn closure(&mut self, id: usize, e: &mut Expr, env: &Bindings) -> Result<(), String> {
        let ExprKind::Closure(params, ret, body) = &e.kind else {
            unreachable!()
        };
        let params = params
            .iter()
            .map(|(n, t)| Ok((n.clone(), self.ty(id, t, env, e.span)?)))
            .collect::<Result<Vec<_>, String>>()?;
        unique(&params.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>())
            .map_err(|m| error(&self.paths[id], e.span, m))?;
        let ret = self.ty(id, ret, env, e.span)?;
        let mut names = HashSet::new();
        free_body(
            body,
            &mut params.iter().map(|(n, _)| n.clone()).collect(),
            &mut names,
        );
        let mut captures = names
            .into_iter()
            .filter_map(|n| {
                self.locals
                    .iter()
                    .rev()
                    .find_map(|s| s.get(&n))
                    .map(|t| (n, t.clone()))
            })
            .collect::<Vec<_>>();
        captures.sort_by(|a, b| a.0.cmp(&b.0));
        if captures.iter().any(|(_, t)| matches!(t, Type::Array(..))) {
            return Err(error(
                &self.paths[id],
                e.span,
                "capture arrays through Vec or a struct",
            ));
        }
        let mut instance = self.queue.len();
        let name = loop {
            let n = format!("closure_{id}_{instance}");
            if !self.functions[id].contains_key(&n) {
                break n;
            }
            instance += 1;
        };
        let mut context_name = "closure_env".to_string();
        while params
            .iter()
            .chain(captures.iter())
            .any(|(n, _)| *n == context_name)
        {
            context_name.push('_');
        }
        let envtype = Type::Record(format!("env_{name}"), captures.clone());
        self.concrete[id].insert(envtype.name(), envtype.clone());
        let context = if captures.is_empty() {
            None
        } else {
            Some(Box::new(Expr {
                span: e.span,
                kind: ExprKind::Reference(Box::new(Expr {
                    span: e.span,
                    kind: ExprKind::Record(
                        envtype.clone(),
                        captures
                            .iter()
                            .map(|(n, _)| Expr {
                                span: e.span,
                                kind: ExprKind::Name(vec![n.clone()]),
                            })
                            .collect(),
                    ),
                })),
            }))
        };
        let mut lifted_params = params.clone();
        let mut lifted_body = vec![];
        if context.is_some() {
            lifted_params.insert(0, (context_name.clone(), Type::Ref(Box::new(envtype))));
            for (n, t) in &captures {
                lifted_body.push(Stmt::Let {
                    name: n.clone(),
                    ty: Some(t.clone()),
                    span: e.span,
                    value: Expr {
                        span: e.span,
                        kind: ExprKind::Field(
                            Box::new(Expr {
                                span: e.span,
                                kind: ExprKind::Dereference(Box::new(Expr {
                                    span: e.span,
                                    kind: ExprKind::Name(vec![context_name.clone()]),
                                })),
                            }),
                            n.clone(),
                        ),
                    },
                });
            }
        }
        lifted_body.extend(body.clone());
        let f = Function {
            constraints: vec![],
            variadic: false,
            name: name.clone(),
            generics: vec![],
            main_default: false,
            params: lifted_params,
            ret: ret.clone(),
            body: Some(lifted_body),
            exported: false,
            span: e.span,
        };
        self.functions[id].insert(name.clone(), f.clone());
        self.queue.push((id, f, env.clone()));
        let t = Type::Function(params.into_iter().map(|(_, t)| t).collect(), Box::new(ret));
        self.concrete[id].insert(t.name(), t.clone());
        e.kind = ExprKind::Callable(vec![name], context, t);
        Ok(())
    }
}
fn free_expr(e: &Expr, bound: &HashSet<String>, free: &mut HashSet<String>) {
    match &e.kind {
        ExprKind::Name(n) => {
            if !bound.contains(&n[0]) {
                free.insert(n[0].clone());
            }
        }
        ExprKind::Call(n, args) | ExprKind::GenericCall(n, _, args) => {
            if !bound.contains(&n[0]) {
                free.insert(n[0].clone());
            }
            for a in args {
                free_expr(a, bound, free)
            }
        }
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            free_expr(a, bound, free);
            free_expr(b, bound, free)
        }
        ExprKind::Unary(_, a)
        | ExprKind::Field(a, _)
        | ExprKind::Cast(a, _)
        | ExprKind::Reference(a)
        | ExprKind::Dereference(a) => free_expr(a, bound, free),
        ExprKind::Array(args)
        | ExprKind::Record(_, args)
        | ExprKind::Vector(_, args)
        | ExprKind::Enum(_, _, args) => {
            for a in args {
                free_expr(a, bound, free)
            }
        }
        ExprKind::Collection(a, _, args) | ExprKind::Invoke(a, args) => {
            free_expr(a, bound, free);
            for a in args {
                free_expr(a, bound, free)
            }
        }
        ExprKind::Closure(ps, _, body) => {
            let mut nested = bound.clone();
            nested.extend(ps.iter().map(|(n, _)| n.clone()));
            free_body(body, &mut nested, free)
        }
        _ => {}
    }
}
fn free_body(body: &[Stmt], bound: &mut HashSet<String>, free: &mut HashSet<String>) {
    for s in body {
        match s {
            Stmt::Let { name, value, .. } => {
                free_expr(value, bound, free);
                bound.insert(name.clone());
            }
            Stmt::Expr(e) | Stmt::Return(Some(e), _) => free_expr(e, bound, free),
            Stmt::Assign { target, value, .. } => {
                free_expr(target, bound, free);
                free_expr(value, bound, free)
            }
            Stmt::Block(b) | Stmt::Unsafe(b) => free_body(b, &mut bound.clone(), free),
            Stmt::If { cond, yes, no } => {
                free_expr(cond, bound, free);
                free_body(yes, &mut bound.clone(), free);
                free_body(no, &mut bound.clone(), free)
            }
            Stmt::While { cond, body } => {
                free_expr(cond, bound, free);
                free_body(body, &mut bound.clone(), free)
            }
            Stmt::Match { value, arms } => {
                free_expr(value, bound, free);
                for arm in arms {
                    let mut b = bound.clone();
                    fn add(p: &Pattern, b: &mut HashSet<String>) {
                        match p {
                            Pattern::Bind(n) => {
                                b.insert(n.clone());
                            }
                            Pattern::Variant(_, ps) => {
                                for p in ps {
                                    add(p, b)
                                }
                            }
                        }
                    }
                    for p in &arm.bindings {
                        add(p, &mut b)
                    }
                    if let Some(g) = &arm.guard {
                        free_expr(g, &b, free)
                    }
                    free_body(&arm.body, &mut b, free)
                }
            }
            _ => {}
        }
    }
}
