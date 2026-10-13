impl Emitter<'_> {
    fn basic_array_method(
        &mut self,
        value: Value,
        element: &Type,
        op: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        if !self.hosted {
            return self.fail(span, "basic array methods require hosted mode");
        }
        if (op == "toString" && !args.is_empty())
            || (op == "delete" && args.len() != 1)
            || (op == "copyWithin" && !(2..=3).contains(&args.len()))
            || (["splice", "toSpliced"].contains(&op) && args.is_empty())
        {
            return self.fail(span, "wrong array method argument count");
        }
        let vector = Type::Vector(Box::new(element.clone()));
        let helper = self.collection(&vector);
        let target = if ["delete", "copyWithin", "splice"].contains(&op) {
            if !value.lvalue || !matches!(value.ty, Type::Vector(_)) {
                return self.fail(span, "mutable array method requires a writable Vec");
            }
            let target = format!("dev_basic_target_{}", self.next_var);
            self.next_var += 1;
            self.prelude.push_str(&format!(
                "{} *{target}=&({});\n",
                c_type(&value.ty),
                value.code
            ));
            Some(target)
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
        let r = &receiver.code;
        if op == "toString" {
            let item = format!("{r}.data[{r}.offset+dev_text_i]");
            let text = match element {
                Type::Str => item,
                Type::Bool => format!("({item}?\"true\":\"false\")"),
                Type::Int { signed: true, .. } | Type::Size { signed: true } => {
                    format!("dev_interpolate(\"%lld\",(long long){item})")
                }
                Type::Int { signed: false, .. } | Type::Size { signed: false } => {
                    format!("dev_interpolate(\"%llu\",(unsigned long long){item})")
                }
                Type::Float(_) => format!("dev_interpolate(\"%.17g\",(double){item})"),
                _ => return self.fail(span, "toString requires numeric, bool or str elements"),
            };
            self.uses_templates = true;
            let id = self.next_var;
            self.next_var += 1;
            let result = format!("dev_basic_text_{id}");
            self.prelude.push_str(&format!("if({r}.len>SIZE_MAX/sizeof(const char *))abort();\nconst char **dev_text_buf_{id}={r}.len?(const char **)malloc({r}.len*sizeof(const char *)):NULL;if({r}.len&&!dev_text_buf_{id})abort();\nfor(size_t dev_text_i=0;dev_text_i<{r}.len;++dev_text_i)dev_text_buf_{id}[dev_text_i]={text};\nconst char *{result}=dev_array_join(dev_text_buf_{id},{r}.len,\",\");free(dev_text_buf_{id});\n"));
            return Ok(Value {
                code: result,
                ty: Type::Str,
                lvalue: false,
            });
        }
        if op == "flat" {
            let depth = array_flat_depth(args).map_err(|message| {
                crate::ast::error(&self.program.modules[self.module].path, span, message)
            })?;
            let flat_element = array_flat_element(element.clone(), depth);
            let flat_ty = Type::Vector(Box::new(flat_element));
            let flat_helper = self.collection(&flat_ty);
            let output = self.owned(Value {
                code: format!("{flat_helper}_make(NULL,0)"),
                ty: flat_ty,
                lvalue: false,
            });
            let id = self.next_var;
            self.next_var += 1;
            self.prelude.push_str(&format!(
                "for(size_t dev_flat_i_{id}=0;dev_flat_i_{id}<{r}.len;++dev_flat_i_{id}){{\n"
            ));
            self.emit_flat_item(
                element,
                &format!("{r}.data[{r}.offset+dev_flat_i_{id}]"),
                depth,
                &output.code,
                &flat_helper,
            );
            self.prelude.push_str("}\n");
            return Ok(output);
        }
        let mut indices = Vec::new();
        let count = if op == "delete" {
            1
        } else if op == "copyWithin" {
            args.len()
        } else {
            args.len().min(2)
        };
        for arg in &args[..count] {
            let value = self.expr(arg, Some(&Type::i64()))?;
            self.compatible(&value, &Type::i64(), arg.span)?;
            indices.push(self.sequence_save(value).code);
        }
        let mut replacements = Vec::new();
        if op == "splice" || op == "toSpliced" {
            for arg in args.iter().skip(2) {
                let value = self.expr(arg, Some(element))?;
                self.compatible(&value, element, arg.span)?;
                replacements.push(self.sequence_save(value).code);
            }
        }
        let id = self.next_var;
        self.next_var += 1;
        let start = format!("dev_basic_start_{id}");
        let amount = format!("dev_basic_count_{id}");
        let mut deleted = None;
        if op == "delete" {
            let value = Value {
                code: format!(
                    "*{helper}_at_signed(({}){{{r}.len,{r}.offset,{r}.data}}, {}, {})",
                    c_type(&vector),
                    indices[0],
                    self.location(span)
                ),
                ty: element.clone(),
                lvalue: false,
            };
            deleted = Some(self.sequence_save(value));
        }
        let start_index = if op == "copyWithin" {
            &indices[1]
        } else {
            &indices[0]
        };
        self.prelude.push_str(&format!(
            "size_t {start}=dev_array_normalize({r}.len,{start_index});\n"
        ));
        if op == "copyWithin" {
            let end = if indices.len() == 3 {
                format!("dev_array_normalize({r}.len,{})", indices[2])
            } else {
                format!("{r}.len")
            };
            self.prelude.push_str(&format!("size_t dev_basic_target_pos_{id}=dev_array_normalize({r}.len,{}),dev_basic_end_{id}={end};size_t {amount}=dev_basic_end_{id}>{start}?dev_basic_end_{id}-{start}:0;if({amount}>{r}.len-dev_basic_target_pos_{id}){amount}={r}.len-dev_basic_target_pos_{id};\n",indices[0]));
        } else {
            let count = if op == "delete" {
                "1".into()
            } else if indices.len() == 2 {
                format!(
                    "({}<=0?0:((uint64_t){}>{r}.len-{start}?{r}.len-{start}:(size_t){}))",
                    indices[1], indices[1], indices[1]
                )
            } else {
                format!("{r}.len-{start}")
            };
            self.prelude
                .push_str(&format!("size_t {amount}={count};\n"));
        }
        let removed = if op == "splice" {
            Some(self.owned(Value {
                code: format!("{helper}_make({amount}?{r}.data+{r}.offset+{start}:NULL,{amount})"),
                ty: vector.clone(),
                lvalue: false,
            }))
        } else {
            None
        };
        let output = self.owned(Value {
            code: if op == "copyWithin" {
                format!("{helper}_make({r}.len?{r}.data+{r}.offset:NULL,{r}.len)")
            } else {
                format!("{helper}_make(NULL,0)")
            },
            ty: vector.clone(),
            lvalue: false,
        });
        let out = &output.code;
        if op == "copyWithin" {
            let keep = self.helper(element, true);
            let drop = self.helper(element, false);
            self.prelude.push_str(&format!("for(size_t dev_basic_i_{id}=0;dev_basic_i_{id}<{amount};++dev_basic_i_{id}){{{} dev_basic_value={r}.data[{r}.offset+{start}+dev_basic_i_{id}];{keep}(&dev_basic_value);{drop}(&{out}.data[dev_basic_target_pos_{id}+dev_basic_i_{id}]);{out}.data[dev_basic_target_pos_{id}+dev_basic_i_{id}]=dev_basic_value;}}\n",c_type(element)));
        } else {
            self.prelude.push_str(&format!("for(size_t dev_basic_i_{id}=0;dev_basic_i_{id}<{start};++dev_basic_i_{id}){helper}_push(&{out},{r}.data[{r}.offset+dev_basic_i_{id}]);\n"));
            for replacement in replacements {
                self.prelude
                    .push_str(&format!("{helper}_push(&{out},{replacement});\n"));
            }
            self.prelude.push_str(&format!("for(size_t dev_basic_i_{id}={start}+{amount};dev_basic_i_{id}<{r}.len;++dev_basic_i_{id}){helper}_push(&{out},{r}.data[{r}.offset+dev_basic_i_{id}]);\n"));
        }
        if let Some(target) = target {
            let keep = self.manage(&vector, out, true);
            let drop = self.manage(&vector, &format!("(*{target})"), false);
            self.prelude
                .push_str(&format!("{keep}{drop}*{target}={out};\n"));
        }
        Ok(if let Some(deleted) = deleted {
            deleted
        } else if let Some(removed) = removed {
            removed
        } else if op == "copyWithin" {
            Value {
                code: "((void)0)".into(),
                ty: Type::Void,
                lvalue: false,
            }
        } else {
            output
        })
    }
    fn emit_flat_item(&mut self, ty: &Type, access: &str, depth: usize, out: &str, helper: &str) {
        if depth > 0 {
            let (inner, len, item) = match ty {
                Type::Vector(t) | Type::Slice(t) => (
                    t,
                    format!("({access}).len"),
                    format!("({access}).data[({access}).offset+dev_flat_inner_ID]"),
                ),
                Type::Array(t, n) => (
                    t,
                    n.to_string(),
                    format!("({access}).data[dev_flat_inner_ID]"),
                ),
                _ => {
                    self.prelude
                        .push_str(&format!("{helper}_push(&{out},{access});\n"));
                    return;
                }
            };
            let id = self.next_var;
            self.next_var += 1;
            let item = item.replace("ID", &id.to_string());
            self.prelude.push_str(&format!("for(size_t dev_flat_inner_{id}=0;dev_flat_inner_{id}<{len};++dev_flat_inner_{id}){{\n"));
            self.emit_flat_item(inner, &item, depth - 1, out, helper);
            self.prelude.push_str("}\n");
        } else {
            self.prelude
                .push_str(&format!("{helper}_push(&{out},{access});\n"));
        }
    }
}
