impl Expand {
    /// Evaluate task and continuation once, then capture the two managed values
    /// in a worker. Waiting happens in that worker, never in the caller of then.
    fn continuation(&mut self, id: usize, args: Vec<Expr>, span: Span) -> Result<ExprKind, String> {
        let path = self.paths[id].clone();
        let fail = |msg| error(&path, span, msg);
        if args.len() != 2 {
            return Err(fail("then expects a task and a function value"));
        }
        let Some(task @ Type::Task(_)) = self.infer(id, &args[0], None)? else {
            return Err(fail("then expects Task<T> as its first argument"));
        };
        let Some(next @ Type::Function(..)) = self.infer(id, &args[1], None)? else {
            return Err(fail("then expects a function value as its second argument"));
        };
        let Type::Task(input) = &task else {
            unreachable!()
        };
        let Type::Function(params, result) = &next else {
            unreachable!()
        };
        if if **input == Type::Void {
            !params.is_empty()
        } else {
            params.len() != 1 || params[0].name() != input.name()
        } {
            return Err(fail(
                "then continuation parameters must match the task result",
            ));
        }
        let mut index = self.queue.len();
        let name = loop {
            let name = format!("then_internal_{id}_{index}");
            if !self.functions[id].contains_key(&name) {
                break name;
            }
            index += 1;
        };
        let expr = |kind| Expr { kind, span };
        let await_input = expr(ExprKind::Call(
            vec!["await".into()],
            vec![expr(ExprKind::Name(vec!["task".into()]))],
        ));
        let mut body = vec![];
        let values = if **input == Type::Void {
            body.push(Stmt::Expr(await_input));
            vec![]
        } else {
            body.push(Stmt::Let {
                name: "value".into(),
                ty: Some((**input).clone()),
                value: await_input,
                span,
            });
            vec![expr(ExprKind::Name(vec!["value".into()]))]
        };
        let invoke = expr(ExprKind::Invoke(
            Box::new(expr(ExprKind::Name(vec!["next".into()]))),
            values,
        ));
        if **result == Type::Void {
            body.push(Stmt::Expr(invoke));
            body.push(Stmt::Return(None, span));
        } else {
            body.push(Stmt::Return(Some(invoke), span));
        }
        let job = expr(ExprKind::Closure(vec![], (**result).clone(), body));
        let function = Function {
            constraints: vec![],
            variadic: false,
            name: name.clone(),
            generics: vec![],
            main_default: false,
            params: vec![("task".into(), task), ("next".into(), next.clone())],
            ret: Type::Task(result.clone()),
            body: Some(vec![Stmt::Return(
                Some(expr(ExprKind::Call(vec!["spawn".into()], vec![job]))),
                span,
            )]),
            exported: false,
            span,
        };
        self.functions[id].insert(name.clone(), function.clone());
        self.queue.push((id, function, HashMap::new()));
        Ok(ExprKind::Call(vec![name], args))
    }
}
