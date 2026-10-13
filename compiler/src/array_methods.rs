impl Emitter<'_> {
    fn array_method(
        &mut self,
        value: Value,
        element: &Type,
        op: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        if op == "sort" || op == "toSorted" {
            return self.array_sort(value, element, op, args, span);
        }
        let arity = match op {
            "toVec" | "toReversed" | "shift" | "reverse" => 0,
            "reduce" => 2,
            _ => 1,
        };
        if args.len() != arity {
            return self.fail(span, "wrong array method argument count");
        }
        let location = self.location(span);
        let vector = Type::Vector(Box::new(element.clone()));
        let helper = self.collection(&vector);
        if ["shift", "unshift", "reverse"].contains(&op) {
            if !value.lvalue || !matches!(value.ty, Type::Vector(_)) {
                return self.fail(span, "mutable array method requires a writable Vec");
            }
            // Resolve the receiver before evaluating arguments; keep its address, not its data buffer.
            let target = format!("dev_array_target_{}", self.next_var);
            self.next_var += 1;
            self.prelude.push_str(&format!(
                "{} *{target} = &({});\n",
                c_type(&value.ty),
                value.code
            ));
            if op == "unshift" {
                let x = self.expr(&args[0], Some(element))?;
                self.compatible(&x, element, args[0].span)?;
                let x = self.sequence_save(x);
                return Ok(Value {
                    code: format!("{helper}_unshift({target}, {})", x.code),
                    ty: Type::Void,
                    lvalue: false,
                });
            }
            let result = Value {
                code: format!(
                    "{helper}_{op}({target}{})",
                    if op == "shift" {
                        format!(", {location}")
                    } else {
                        String::new()
                    }
                ),
                ty: if op == "shift" {
                    element.clone()
                } else {
                    Type::Void
                },
                lvalue: false,
            };
            return Ok(self.owned(result));
        }
        let receiver = self.sequence_save(value);
        let r = &receiver.code;
        if op == "at" {
            let index = self.expr(&args[0], Some(&Type::i64()))?;
            self.compatible(&index, &Type::i64(), args[0].span)?;
            let index = self.sequence_save(index);
            let code = format!(
                "*{helper}_at_signed(({}){{{r}.len,{r}.offset,{r}.data}}, {}, {location})",
                c_type(&vector),
                index.code
            );
            return Ok(self.sequence_save(Value {
                code,
                ty: element.clone(),
                lvalue: false,
            }));
        }
        if op == "join" {
            if *element != Type::Str {
                return self.fail(span, "join requires str elements");
            }
            let sep = self.expr(&args[0], Some(&Type::Str))?;
            self.compatible(&sep, &Type::Str, args[0].span)?;
            let sep = self.sequence_save(sep);
            self.uses_templates = true;
            return Ok(Value {
                code: format!(
                    "dev_array_join({r}.len ? {r}.data + {r}.offset : NULL, {r}.len, {})",
                    sep.code
                ),
                ty: Type::Str,
                lvalue: false,
            });
        }
        if ["toVec", "toReversed", "concat"].contains(&op) {
            let mut second = None;
            if op == "concat" {
                let other = self.expr(&args[0], None)?;
                let (Type::Array(t, _) | Type::Vector(t) | Type::Slice(t)) = &other.ty else {
                    return self.fail(span, "concat requires Array, Vec or Slice");
                };
                if self.resolve_collection_element(t) != *element {
                    return self.fail(span, "concat element type mismatch");
                }
                second = Some(self.sequence_save(other));
            }
            let output = self.owned(Value {
                code: format!("{helper}_make({r}.len ? {r}.data + {r}.offset : NULL, {r}.len)"),
                ty: vector,
                lvalue: false,
            });
            if op == "toReversed" {
                self.prelude
                    .push_str(&format!("{helper}_reverse(&{});\n", output.code));
            }
            if let Some(other) = second {
                let (len, item) = match other.ty {
                    Type::Array(_, n) => (n.to_string(), format!("{}[dev_array_i]", other.code)),
                    _ => (
                        format!("{}.len", other.code),
                        format!("{}.data[{}.offset + dev_array_i]", other.code, other.code),
                    ),
                };
                self.prelude.push_str(&format!("for (size_t dev_array_i = 0; dev_array_i < {len}; ++dev_array_i) {helper}_push(&{}, {item});\n", output.code));
            }
            return Ok(output);
        }
        let callback = self.expr(&args[0], None)?;
        let Type::Function(params, ret) = &callback.ty else {
            return self.fail(span, "array callback must be a Dev function");
        };
        let params = params.clone();
        let ret = self.resolve_collection_element(ret);
        if op == "reduce" {
            if !(params.len() == 2 || params.len() == 3)
                || self.resolve_collection_element(&params[0]) != ret
                || self.resolve_collection_element(&params[1]) != *element
                || (params.len() == 3 && params[2] != Type::i64())
            {
                return self.fail(
                    span,
                    "reduce callback requires fn(accumulator, element[, index: i64]): accumulator",
                );
            }
            if matches!(ret, Type::Void | Type::Array(..)) {
                return self.fail(span, "reduce accumulator cannot be void or fixed array");
            }
        } else {
            if !(params.len() == 1 || params.len() == 2)
                || self.resolve_collection_element(&params[0]) != *element
                || (params.len() == 2 && params[1] != Type::i64())
            {
                return self.fail(
                    span,
                    "array callback requires an element parameter and optional index: i64",
                );
            }
            if op == "map" && matches!(ret, Type::Void | Type::Array(..)) {
                return self.fail(span, "map callback cannot return void or fixed array");
            }
            if op == "forEach" && ret != Type::Void {
                return self.fail(span, "forEach callback must return void");
            }
            if ["filter", "some", "every", "findIndex", "findLastIndex"].contains(&op)
                && ret != Type::Bool
            {
                return self.fail(span, "array predicate must return bool");
            }
        }
        let callback = self.sequence_save(callback);
        let output_ty = match op {
            "map" => Type::Vector(Box::new(ret.clone())),
            "filter" => vector,
            "some" | "every" => Type::Bool,
            "findIndex" | "findLastIndex" => Type::i64(),
            "reduce" => ret.clone(),
            _ => Type::Void,
        };
        let output_helper = if matches!(output_ty, Type::Vector(_)) {
            Some(self.collection(&output_ty))
        } else {
            None
        };
        let output = if op == "reduce" {
            let initial = self.expr(&args[1], Some(&ret))?;
            self.compatible(&initial, &ret, args[1].span)?;
            self.sequence_save(initial)
        } else if let Some(h) = &output_helper {
            self.owned(Value {
                code: format!("{h}_make(NULL, 0)"),
                ty: output_ty.clone(),
                lvalue: false,
            })
        } else if op == "forEach" {
            Value {
                code: "((void)0)".into(),
                ty: Type::Void,
                lvalue: false,
            }
        } else {
            self.sequence_save(Value {
                code: if op == "every" {
                    "true"
                } else if op == "some" {
                    "false"
                } else {
                    "-1"
                }
                .into(),
                ty: output_ty.clone(),
                lvalue: false,
            })
        };
        let id = self.next_var;
        self.next_var += 1;
        let item = format!("{r}.data[{r}.offset + dev_array_i_{id}]");
        let mut callback_args = if op == "reduce" {
            format!("{}, {item}", output.code)
        } else {
            item.clone()
        };
        if params.len() == if op == "reduce" { 3 } else { 2 } {
            callback_args.push_str(&format!(", (int64_t)dev_array_i_{id}"));
        }
        let call = format!(
            "{}.call({}.context, {callback_args})",
            callback.code, callback.code
        );
        let mut body = String::new();
        if op == "forEach" {
            body.push_str(&format!("{call};"));
        } else {
            let result = format!("dev_array_result_{id}");
            body.push_str(&format!("{} {result} = {call};\n", c_type(&ret)));
            match op {
                "map" => {
                    let h = output_helper.as_ref().unwrap();
                    let drop = self.helper(&ret, false);
                    body.push_str(&format!(
                        "{h}_push(&{}, {result}); {drop}(&{result});",
                        output.code
                    ));
                }
                "filter" => body.push_str(&format!(
                    "if ({result}) {}_push(&{}, {item});",
                    output_helper.as_ref().unwrap(),
                    output.code
                )),
                "reduce" => {
                    let drop = self.helper(&ret, false);
                    body.push_str(&format!(
                        "{drop}(&{}); {} = {result};",
                        output.code, output.code
                    ));
                }
                "some" => {
                    body.push_str(&format!("if ({result}) {{ {}=true; break; }}", output.code))
                }
                "every" => body.push_str(&format!(
                    "if (!{result}) {{ {}=false; break; }}",
                    output.code
                )),
                "findIndex" | "findLastIndex" => body.push_str(&format!(
                    "if ({result}) {{ {}=(int64_t)dev_array_i_{id}; break; }}",
                    output.code
                )),
                _ => unreachable!(),
            }
        }
        let iterator = if op == "findLastIndex" {
            format!("for (size_t dev_array_i_{id}={r}.len; dev_array_i_{id}-- > 0;)")
        } else {
            format!("for (size_t dev_array_i_{id}=0; dev_array_i_{id}<{r}.len; ++dev_array_i_{id})")
        };
        self.prelude.push_str(&format!("{iterator} {{ {body} }}\n"));
        Ok(output)
    }
}

impl Emitter<'_> {
    fn array_sort(
        &mut self,
        value: Value,
        element: &Type,
        op: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        if args.len() > 1 {
            return self.fail(span, "sort expects zero arguments or one comparator");
        }
        let vector = Type::Vector(Box::new(element.clone()));
        let helper = self.collection(&vector);
        let target = if op == "sort" {
            if !value.lvalue || !matches!(value.ty, Type::Vector(_)) {
                return self.fail(
                    span,
                    "sort requires a writable Vec; use toSorted for Array or Slice",
                );
            }
            let name = format!("dev_sort_target_{}", self.next_var);
            self.next_var += 1;
            self.prelude.push_str(&format!(
                "{} *{name}=&({});\n",
                c_type(&value.ty),
                value.code
            ));
            Some(name)
        } else {
            None
        };
        let value = if let Some(target) = &target {
            Value {
                code: format!("(*{target})"),
                ty: value.ty,
                lvalue: false,
            }
        } else {
            value
        };
        let receiver = self.sequence_save(value);
        let callback = if let Some(arg) = args.first() {
            let cb = self.expr(arg, None)?;
            let Type::Function(params, ret) = &cb.ty else {
                return self.fail(span, "sort comparator must be a Dev function");
            };
            if params.len() != 2
                || params
                    .iter()
                    .any(|p| self.resolve_collection_element(p) != *element)
                || **ret != Type::i64()
            {
                return self.fail(span, "sort comparator requires fn(T, T): i64");
            }
            Some(self.sequence_save(cb))
        } else {
            if !(element.numeric() || *element == Type::Str) {
                return self.fail(
                    span,
                    "default sort requires numeric or str elements; provide a comparator",
                );
            }
            None
        };
        let r = &receiver.code;
        let output = self.owned(Value {
            code: format!("{helper}_make({r}.len ? {r}.data+{r}.offset : NULL,{r}.len)"),
            ty: vector.clone(),
            lvalue: false,
        });
        let id = self.next_var;
        self.next_var += 1;
        let v = &output.code;
        let a = format!("{v}.data[dev_sort_a_{id}]");
        let b = format!("{v}.data[dev_sort_b_{id}]");
        let compare = if let Some(cb) = callback {
            format!("{}.call({}.context,{a},{b})", cb.code, cb.code)
        } else if *element == Type::Str {
            format!("strcmp({a},{b})")
        } else if matches!(element, Type::Float(_)) {
            format!("({a}!={a} ? ({b}!={b} ? 0 : 1) : ({b}!={b} ? -1 : ({a}<{b} ? -1 : ({a}>{b} ? 1 : 0))))")
        } else {
            format!("({a}<{b} ? -1 : ({a}>{b} ? 1 : 0))")
        };
        let ct = c_type(element);
        self.prelude.push_str(&format!("if ({v}.len>1) {{\n{ct} *dev_sort_buf_{id}=({ct} *)malloc({v}.len*sizeof({ct}));if(!dev_sort_buf_{id})abort();\nfor(size_t dev_sort_w_{id}=1;dev_sort_w_{id}<{v}.len;) {{\nfor(size_t dev_sort_start_{id}=0;dev_sort_start_{id}<{v}.len;) {{\nsize_t dev_sort_mid_{id}=dev_sort_start_{id}+({v}.len-dev_sort_start_{id}<dev_sort_w_{id}?{v}.len-dev_sort_start_{id}:dev_sort_w_{id});\nsize_t dev_sort_end_{id}=dev_sort_mid_{id}+({v}.len-dev_sort_mid_{id}<dev_sort_w_{id}?{v}.len-dev_sort_mid_{id}:dev_sort_w_{id});\nsize_t dev_sort_a_{id}=dev_sort_start_{id},dev_sort_b_{id}=dev_sort_mid_{id},dev_sort_out_{id}=dev_sort_start_{id};\nwhile(dev_sort_a_{id}<dev_sort_mid_{id} && dev_sort_b_{id}<dev_sort_end_{id}) {{int64_t dev_sort_cmp_{id}={compare};dev_sort_buf_{id}[dev_sort_out_{id}++]=dev_sort_cmp_{id}<=0 ? {v}.data[dev_sort_a_{id}++] : {v}.data[dev_sort_b_{id}++];}}\nwhile(dev_sort_a_{id}<dev_sort_mid_{id})dev_sort_buf_{id}[dev_sort_out_{id}++]={v}.data[dev_sort_a_{id}++];\nwhile(dev_sort_b_{id}<dev_sort_end_{id})dev_sort_buf_{id}[dev_sort_out_{id}++]={v}.data[dev_sort_b_{id}++];\ndev_sort_start_{id}=dev_sort_end_{id};}}\nmemcpy({v}.data,dev_sort_buf_{id},{v}.len*sizeof({ct}));if(dev_sort_w_{id}>{v}.len/2)break;dev_sort_w_{id}*=2;}}\nfree(dev_sort_buf_{id});}}\n"));
        if let Some(target) = target {
            let keep = self.manage(&vector, v, true);
            let drop = self.manage(&vector, &format!("(*{target})"), false);
            self.prelude
                .push_str(&format!("{keep}{drop}*{target}={v};\n"));
            Ok(Value {
                code: "((void)0)".into(),
                ty: Type::Void,
                lvalue: false,
            })
        } else {
            Ok(output)
        }
    }
}
