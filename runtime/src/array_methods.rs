impl Engine {
    fn array_method(
        &mut self,
        frame: &mut Frame,
        value: Value,
        element: &Type,
        op: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        let arity = match op {
            "toVec" | "toReversed" => 0,
            "reduce" => 2,
            _ => 1,
        };
        if args.len() != arity {
            return Err("wrong array method argument count".into());
        }
        let items = match &value {
            Value::Vector(xs, _) => &xs[..],
            Value::Slice(xs, offset, len, _) => &xs[*offset..*offset + *len],
            _ => unreachable!(),
        };
        let vector = Type::Vector(Box::new(element.clone()));
        if op == "at" {
            let index = self.eval(frame, &args[0], Some(&Type::i64()))?;
            if index.ty() != Type::i64() {
                return Err("at index must be i64".into());
            }
            let index = index.integer()?;
            let index = if index < 0 {
                items.len() as i128 + index
            } else {
                index
            };
            return usize::try_from(index)
                .ok()
                .and_then(|i| items.get(i))
                .cloned()
                .ok_or_else(|| "array index out of bounds".into());
        }
        if op == "join" {
            if *element != Type::Str {
                return Err("join requires str elements".into());
            }
            let separator = self.eval(frame, &args[0], Some(&Type::Str))?;
            let separator = separator.string()?;
            let strings = items
                .iter()
                .map(Value::string)
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(Value::Str(strings.join(separator)));
        }
        if ["concat", "toVec", "toReversed"].contains(&op) {
            let mut result = items.to_vec();
            if op == "concat" {
                let other = self.eval(frame, &args[0], None)?;
                let (other_items, ty) = match &other {
                    Value::Array(xs, Type::Array(t, _)) => (&xs[..], t),
                    Value::Vector(xs, Type::Vector(t)) => (&xs[..], t),
                    Value::Slice(xs, offset, len, Type::Slice(t)) => {
                        (&xs[*offset..*offset + *len], t)
                    }
                    _ => return Err("concat requires Array, Vec or Slice".into()),
                };
                if self.resolve_collection_element(ty)? != *element {
                    return Err("concat element type mismatch".into());
                }
                result
                    .try_reserve(other_items.len())
                    .map_err(|_| "collection allocation failed")?;
                result.extend_from_slice(other_items);
            }
            if op == "toReversed" {
                result.reverse();
            }
            return Ok(Value::Vector(Arc::new(result), vector));
        }
        let callable = self.eval(frame, &args[0], None)?;
        let Value::Callable(module, name, context, Type::Function(params, ret)) = callable else {
            return Err("array callback must be a Dev function".into());
        };
        let ret = self.resolve_collection_element(&ret)?;
        if op == "reduce" {
            if params.len() != 2
                || self.resolve_collection_element(&params[0])? != ret
                || self.resolve_collection_element(&params[1])? != *element
            {
                return Err(
                    "reduce callback requires fn(accumulator, element): accumulator".into(),
                );
            }
            if matches!(ret, Type::Void | Type::Array(..)) {
                return Err("reduce accumulator cannot be void or fixed array".into());
            }
        } else {
            if params.len() != 1 || self.resolve_collection_element(&params[0])? != *element {
                return Err(
                    "array callback requires one parameter matching the element type".into(),
                );
            }
            if op == "map" && matches!(ret, Type::Void | Type::Array(..)) {
                return Err("map callback cannot return void or fixed array".into());
            }
            if op == "forEach" && ret != Type::Void {
                return Err("forEach callback must return void".into());
            }
            if ["filter", "some", "every", "findIndex", "findLastIndex"].contains(&op)
                && ret != Type::Bool
            {
                return Err("array predicate must return bool".into());
            }
        }
        let mut accumulator = if op == "reduce" {
            let initial = self.eval(frame, &args[1], Some(&ret))?;
            if initial.ty() != ret {
                return Err("reduce initial value type mismatch".into());
            }
            initial
        } else {
            Value::Void
        };
        let mut result = Vec::new();
        let mut found = -1i128;
        for step in 0..items.len() {
            let index = if op == "findLastIndex" {
                items.len() - 1 - step
            } else {
                step
            };
            let item = &items[index];
            let mut values = Vec::new();
            if let Some(v) = &context {
                let inner = match v.ty() {
                    Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                    t => t,
                };
                values.push(Value::Ref(v.clone(), Type::Ref(Box::new(inner))));
            }
            if op == "reduce" {
                values.push(accumulator.clone());
            }
            values.push(item.clone());
            let output = self.call(&module, &[name.clone()], values, span)?;
            match op {
                "map" => result.push(output),
                "filter" => {
                    if output.boolean()? {
                        result.push(item.clone());
                    }
                }
                "reduce" => accumulator = output,
                "forEach" => {}
                "some" => {
                    if output.boolean()? {
                        return Ok(Value::Bool(true));
                    }
                }
                "every" => {
                    if !output.boolean()? {
                        return Ok(Value::Bool(false));
                    }
                }
                "findIndex" | "findLastIndex" => {
                    if output.boolean()? {
                        found = index as i128;
                        break;
                    }
                }
                _ => unreachable!(),
            }
        }
        Ok(match op {
            "map" => Value::Vector(Arc::new(result), Type::Vector(Box::new(ret))),
            "filter" => Value::Vector(Arc::new(result), vector),
            "reduce" => accumulator,
            "some" => Value::Bool(false),
            "every" => Value::Bool(true),
            "findIndex" | "findLastIndex" => Value::Int(found, Type::i64()),
            _ => Value::Void,
        })
    }
}

impl Engine {
    fn sort_target(&mut self, frame: &mut Frame, target: &Expr) -> Result<Expr, String> {
        let kind = match &target.kind {
            ExprKind::Field(base, name) => {
                ExprKind::Field(Box::new(self.sort_target(frame, base)?), name.clone())
            }
            ExprKind::Index(base, index) => {
                let base = self.sort_target(frame, base)?;
                let value = self.eval(frame, index, None)?.integer()?;
                ExprKind::Index(
                    Box::new(base),
                    Box::new(Expr {
                        span: index.span,
                        kind: ExprKind::Number(value.to_string()),
                    }),
                )
            }
            _ => return Ok(target.clone()),
        };
        Ok(Expr {
            span: target.span,
            kind,
        })
    }
    fn array_sort(
        &mut self,
        frame: &mut Frame,
        value: Value,
        element: &Type,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        if args.len() > 1 {
            return Err("sort expects zero arguments or one comparator".into());
        }
        let callback = if let Some(arg) = args.first() {
            let cb = self.eval(frame, arg, None)?;
            let Value::Callable(_, _, _, Type::Function(params, ret)) = &cb else {
                return Err("sort comparator must be a Dev function".into());
            };
            if params.len() != 2
                || self.resolve_collection_element(&params[0])? != *element
                || self.resolve_collection_element(&params[1])? != *element
                || **ret != Type::i64()
            {
                return Err("sort comparator requires fn(T, T): i64".into());
            }
            Some(cb)
        } else {
            if !(element.numeric() || *element == Type::Str) {
                return Err(
                    "default sort requires numeric or str elements; provide a comparator".into(),
                );
            }
            None
        };
        let mut items = match value {
            Value::Vector(xs, _) => xs.as_ref().clone(),
            Value::Slice(xs, o, n, _) => xs[o..o + n].to_vec(),
            _ => unreachable!(),
        };
        let n = items.len();
        let mut width = 1;
        let mut buffer = items.clone();
        while width < n {
            let mut start = 0;
            while start < n {
                let middle = start + (n - start).min(width);
                let end = middle + (n - middle).min(width);
                let (mut a, mut b, mut out) = (start, middle, start);
                while a < middle && b < end {
                    let order = if let Some(Value::Callable(module, name, context, _)) = &callback {
                        let mut values = Vec::new();
                        if let Some(v) = context {
                            let inner = match v.ty() {
                                Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                                t => t,
                            };
                            values.push(Value::Ref(v.clone(), Type::Ref(Box::new(inner))));
                        }
                        values.push(items[a].clone());
                        values.push(items[b].clone());
                        self.call(module, &[name.clone()], values, span)?
                            .integer()?
                    } else {
                        match (&items[a], &items[b]) {
                            (Value::Int(a, _), Value::Int(b, _)) => {
                                if a < b {
                                    -1
                                } else if a > b {
                                    1
                                } else {
                                    0
                                }
                            }
                            (Value::Str(a), Value::Str(b)) => {
                                if a < b {
                                    -1
                                } else if a > b {
                                    1
                                } else {
                                    0
                                }
                            }
                            (Value::Float(a, _), Value::Float(b, _)) => {
                                if a.is_nan() {
                                    if b.is_nan() {
                                        0
                                    } else {
                                        1
                                    }
                                } else if b.is_nan() {
                                    -1
                                } else if a < b {
                                    -1
                                } else if a > b {
                                    1
                                } else {
                                    0
                                }
                            }
                            _ => return Err("unsupported sort element".into()),
                        }
                    };
                    if order <= 0 {
                        buffer[out] = items[a].clone();
                        a += 1;
                    } else {
                        buffer[out] = items[b].clone();
                        b += 1;
                    }
                    out += 1;
                }
                while a < middle {
                    buffer[out] = items[a].clone();
                    a += 1;
                    out += 1;
                }
                while b < end {
                    buffer[out] = items[b].clone();
                    b += 1;
                    out += 1;
                }
                start = end;
            }
            std::mem::swap(&mut items, &mut buffer);
            if width > n / 2 {
                break;
            }
            width *= 2;
        }
        Ok(Value::Vector(
            Arc::new(items),
            Type::Vector(Box::new(element.clone())),
        ))
    }
}
