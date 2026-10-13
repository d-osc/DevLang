impl Engine {
    fn basic_array_method(
        &mut self,
        frame: &mut Frame,
        base: &Expr,
        value: Value,
        element: &Type,
        op: &str,
        args: &[Expr],
    ) -> Result<Value, String> {
        let items = match &value {
            Value::Vector(xs, _) => &xs[..],
            Value::Slice(xs, o, n, _) => &xs[*o..*o + *n],
            _ => unreachable!(),
        };
        let vector = Type::Vector(Box::new(element.clone()));
        if op == "toString" {
            if !args.is_empty() {
                return Err("toString expects no arguments".into());
            }
            if !(element.numeric() || matches!(element, Type::Str | Type::Bool)) {
                return Err("toString requires numeric, bool or str elements".into());
            }
            return Ok(Value::Str(
                items
                    .iter()
                    .map(Value::display)
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if op == "flat" {
            let depth = array_flat_depth(args)?;
            fn flatten(value: &Value, depth: usize, out: &mut Vec<Value>) {
                if depth > 0 {
                    match value {
                        Value::Array(xs, _) => {
                            for value in xs {
                                flatten(value, depth - 1, out);
                            }
                            return;
                        }
                        Value::Vector(xs, _) => {
                            for value in xs.iter() {
                                flatten(value, depth - 1, out);
                            }
                            return;
                        }
                        Value::Slice(xs, o, n, _) => {
                            for value in &xs[*o..*o + *n] {
                                flatten(value, depth - 1, out);
                            }
                            return;
                        }
                        _ => {}
                    }
                }
                out.push(value.clone());
            }
            let mut out = Vec::new();
            for value in items {
                flatten(value, depth, &mut out);
            }
            return Ok(Value::Vector(
                Arc::new(out),
                Type::Vector(Box::new(array_flat_element(element.clone(), depth))),
            ));
        }
        if (op == "delete" && args.len() != 1)
            || (op == "copyWithin" && !(2..=3).contains(&args.len()))
            || (["splice", "toSpliced"].contains(&op) && args.is_empty())
        {
            return Err("wrong array method argument count".into());
        }
        let n = items.len();
        let mut indices = Vec::new();
        let index_count = if op == "delete" {
            1
        } else if op == "copyWithin" {
            args.len()
        } else {
            args.len().min(2)
        };
        for arg in &args[..index_count] {
            let index = self.eval(frame, arg, Some(&Type::i64()))?;
            if index.ty() != Type::i64() {
                return Err("array method index/count must be i64".into());
            }
            indices.push(index.integer()?);
        }
        fn normalize(index: i128, n: usize) -> usize {
            let n = n as i128;
            if index < 0 {
                (n + index).max(0) as usize
            } else {
                index.min(n) as usize
            }
        }
        let mut result = items.to_vec();
        let output = if op == "delete" {
            let i = if indices[0] < 0 {
                n as i128 + indices[0]
            } else {
                indices[0]
            };
            let i = usize::try_from(i)
                .ok()
                .filter(|i| *i < n)
                .ok_or("array index out of bounds")?;
            result.remove(i)
        } else if op == "copyWithin" {
            let target = normalize(indices[0], n);
            let start = normalize(indices[1], n);
            let end = if indices.len() == 3 {
                normalize(indices[2], n)
            } else {
                n
            };
            let count = end.saturating_sub(start).min(n - target);
            result[target..target + count].clone_from_slice(&items[start..start + count]);
            Value::Void
        } else {
            let start = normalize(indices[0], n);
            let count = if indices.len() == 2 {
                indices[1].max(0).min((n - start) as i128) as usize
            } else {
                n - start
            };
            let mut replacements = Vec::new();
            for arg in args.iter().skip(2) {
                let value = self.array_argument(frame, arg, element)?;
                if value.ty() != *element {
                    return Err("splice element type mismatch".into());
                }
                replacements.push(value);
            }
            let removed = result
                .splice(start..start + count, replacements)
                .collect::<Vec<_>>();
            if op == "toSpliced" {
                return Ok(Value::Vector(Arc::new(result), vector));
            }
            Value::Vector(Arc::new(removed), vector.clone())
        };
        let destination = self
            .writable(frame, base)?
            .ok_or("mutable array method requires a writable Vec")?;
        if !matches!(destination, Value::Vector(..)) {
            return Err("mutable array method requires a writable Vec".into());
        }
        *destination = Value::Vector(Arc::new(result), vector);
        Ok(output)
    }
}
