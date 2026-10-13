//! Native prototype objects share the hosted data ABI, with explicit function adapters.
use crate::ast::*;
use crate::codegen::{c_string, c_type, Emitter, Value};
pub(crate) const HELPERS: &str = include_str!("object.c");
fn data_type() -> Type {
    Type::Record("dev_dynamic_value".into(), vec![])
}
impl Emitter<'_> {
    pub(crate) fn object_expected(
        &mut self,
        value: Value,
        expected: Option<&Type>,
        span: Span,
    ) -> Result<Value, String> {
        if crate::dynamic::is_dynamic(&value.ty) {
            if let Some(expected) = expected {
                if self.object_aggregate_type(expected) {
                    return self.object_aggregate_unbox(value, expected, span, true);
                }
                if expected.numeric() || matches!(expected, Type::Bool | Type::Str) {
                    return self.dynamic_coerce(value, expected, span);
                }
            }
        }
        Ok(value)
    }
    pub(crate) fn object_argument_box(
        &mut self,
        value: Value,
        span: Span,
    ) -> Result<Value, String> {
        if matches!(value.ty, Type::Function(..)) {
            return self.object_function(value, span);
        }
        if value.ty.numeric() || matches!(value.ty, Type::Str | Type::Bool) {
            return self.dynamic_operand(value, span);
        }
        if self.object_aggregate_type(&value.ty) {
            return self.object_aggregate_box(value, span);
        }
        self.dynamic_box(value, span)
    }
    fn object_aggregate_type(&self, t: &Type) -> bool {
        !crate::dynamic::is_dynamic(t)
            && matches!(
                t,
                Type::Record(..)
                    | Type::Nominal(..)
                    | Type::Enum(..)
                    | Type::Array(..)
                    | Type::Vector(..)
                    | Type::Slice(..)
                    | Type::Map(..)
                    | Type::Ref(..)
                    | Type::Function(..)
            )
    }
    fn object_data_encodable(&self, t: &Type, seen: &mut Vec<String>, depth: usize) -> bool {
        if depth > 64 {
            return false;
        }
        if crate::dynamic::is_dynamic(t)
            || t.numeric()
            || matches!(t, Type::Str | Type::Bool | Type::Function(..))
        {
            return true;
        }
        let t = self.resolve_collection_element(t);
        match &t {
            Type::Record(name, fields) => {
                if seen.contains(name) {
                    return false;
                }
                seen.push(name.clone());
                let ok = fields
                    .iter()
                    .all(|(_, t)| self.object_data_encodable(t, seen, depth + 1));
                seen.pop();
                ok
            }
            Type::Ref(t) | Type::Array(t, _) | Type::Vector(t) | Type::Slice(t) => {
                self.object_data_encodable(t, seen, depth + 1)
            }
            Type::Map(k, v) => **k == Type::Str && self.object_data_encodable(v, seen, depth + 1),
            _ => false,
        }
    }
    fn object_aggregate_box(&mut self, value: Value, span: Span) -> Result<Value, String> {
        let value = self.sequence_save(value);
        let t = value.ty.clone();
        let encoder = if self.object_data_encodable(&t, &mut Vec::new(), 0) {
            self.object_aggregate_encoder(&t, span)?
        } else {
            "NULL".into()
        };
        let keep = self.helper(&t, true);
        Ok(Value {
            code: format!(
                "dev_object_aggregate_value({}, {}, &({}), sizeof({}), {keep})",
                encoder,
                c_string(&c_type(&t)),
                value.code,
                value.code
            ),
            ty: data_type(),
            lvalue: false,
        })
    }
    fn object_aggregate_encoder(&mut self, t: &Type, span: Span) -> Result<String, String> {
        self.helper(t, true);
        let name = format!("object_encoder_{}", crate::codegen::managed_key(t));
        if self.adapters.contains_key(&name) {
            return Ok(name);
        }
        let saved = std::mem::take(&mut self.prelude);
        let saved_temps = std::mem::take(&mut self.temporaries);
        let ct = c_type(t);
        self.prelude.push_str(&format!(
            "{ct} input;memcpy(&input,payload,sizeof(input));\n"
        ));
        let data = self.dynamic_box(
            Value {
                code: if matches!(t, Type::Array(..)) {
                    "input.data".into()
                } else {
                    "input".into()
                },
                ty: t.clone(),
                lvalue: false,
            },
            span,
        )?;
        let data = self.dynamic_save(data);
        let temps = std::mem::take(&mut self.temporaries);
        let cleanup = temps
            .iter()
            .rev()
            .map(|v| self.manage(&v.ty, &v.name, false))
            .collect::<String>();
        let prelude = std::mem::take(&mut self.prelude);
        self.prelude = saved;
        self.temporaries = saved_temps;
        self.adapters.insert(name.clone(),format!("static dev_dynamic_value {name}(const void *payload,size_t argc,const dev_dynamic_value *argv,const char *p,size_t l,size_t c){{(void)argc;(void)argv;(void)p;(void)l;(void)c;{prelude}{cleanup}return {};}}\n",data.code));
        Ok(name)
    }
    fn object_aggregate_borrow(&mut self, value: Value, span: Span) -> Result<Value, String> {
        let value = self.sequence_save(value);
        let t = value.ty.clone();
        let encoder = if self.object_data_encodable(&t, &mut Vec::new(), 0) {
            self.object_aggregate_encoder(&t, span)?
        } else {
            "NULL".into()
        };
        let name = format!("dev_object_borrow_{}", self.next_var);
        self.next_var += 1;
        self.prelude.push_str(&format!("dev_object_aggregate {name}_data={{{},(void*)&({}),sizeof({}),{encoder}}};dev_dynamic_node {name}={{0}};{name}.kind=10;{name}.data.opaque=&{name}_data;\n",c_string(&c_type(&t)),value.code,value.code));
        Ok(Value {
            code: format!("(dev_dynamic_value){{&{name},1,0}}"),
            ty: data_type(),
            lvalue: false,
        })
    }
    fn object_aggregate_unbox(
        &mut self,
        value: Value,
        t: &Type,
        span: Span,
        owned: bool,
    ) -> Result<Value, String> {
        let name = format!("dev_object_arg_{}", self.next_var);
        self.next_var += 1;
        let ct = c_type(t);
        self.prelude.push_str(&format!("{ct} {name};memcpy(&{name},dev_object_aggregate_require({}, {}, sizeof({ct}), {}),sizeof({ct}));\n",value.code,c_string(&ct),if owned {self.location(span)}else{"p,l,c".into()}));
        let code = if matches!(t, Type::Array(..)) {
            format!("{name}.data")
        } else {
            name
        };
        if owned {
            let keep = self.manage(t, &code, true);
            self.prelude.push_str(&keep);
            self.temporaries.push(crate::codegen::Variable {
                name: code.clone(),
                ty: t.clone(),
            });
        }
        Ok(Value {
            code,
            ty: t.clone(),
            lvalue: false,
        })
    }
    pub(crate) fn object_function(&mut self, value: Value, span: Span) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        let Type::Function(params, ret) = &value.ty else {
            return self.fail(span, "expected a Dev function");
        };
        let params = params.clone();
        let ret = (**ret).clone();
        let function_type = value.ty.clone();
        let ct = c_type(&function_type);
        let adapter = format!("object_adapter_{ct}");
        if !self.adapters.contains_key(&adapter) {
            let saved = std::mem::take(&mut self.prelude);
            let saved_temporaries = std::mem::take(&mut self.temporaries);
            let old_box = self.object_boxing;
            self.object_boxing = true;
            let mut arguments = vec!["fn->context".to_string()];
            for (i, t) in params.iter().enumerate() {
                let code = if crate::dynamic::is_dynamic(t) {
                    format!("dev_object_box_data(argv[{i}])")
                } else if t.numeric() || matches!(t, Type::Bool | Type::Str) {
                    let v = self.dynamic_coerce(
                        Value {
                            code: format!("argv[{i}]"),
                            ty: data_type(),
                            lvalue: false,
                        },
                        t,
                        span,
                    )?;
                    v.code
                } else if self.object_aggregate_type(t) {
                    let v = self.object_aggregate_unbox(
                        Value {
                            code: format!("argv[{i}]"),
                            ty: data_type(),
                            lvalue: false,
                        },
                        t,
                        span,
                        false,
                    )?;
                    if matches!(t, Type::Array(..)) {
                        v.code.strip_suffix(".data").unwrap().to_string()
                    } else {
                        v.code
                    }
                } else {
                    return self.fail(span,"native Object callback parameters require scalar, aggregate or json.Value/don.Value types");
                };
                arguments.push(code);
            }
            let call = format!("fn->call({})", arguments.join(","));
            let body = if ret == Type::Void {
                format!("{call};return (dev_dynamic_value){{NULL,1}};")
            } else {
                self.prelude
                    .push_str(&format!("{} result={call};\n", c_type(&ret)));
                let boxed = self.object_argument_box(
                    Value {
                        code: if matches!(ret, Type::Array(..)) {
                            "result.data".into()
                        } else {
                            "result".into()
                        },
                        ty: ret.clone(),
                        lvalue: false,
                    },
                    span,
                )?;
                let code = format!("dev_dynamic_value out={};\n", boxed.code);
                let drop = self.manage(
                    &ret,
                    if matches!(ret, Type::Array(..)) {
                        "result.data"
                    } else {
                        "result"
                    },
                    false,
                );
                format!("{code}{drop}return out;")
            };
            let temps = std::mem::take(&mut self.temporaries);
            let cleanup = temps
                .iter()
                .rev()
                .map(|v| self.manage(&v.ty, &v.name, false))
                .collect::<String>();
            self.temporaries = saved_temporaries;
            let body = body.replace("return out;", &format!("{cleanup}return out;"));
            let prelude = std::mem::take(&mut self.prelude);
            self.prelude = saved;
            self.object_boxing = old_box;
            self.adapters.insert(adapter.clone(),format!("static dev_dynamic_value {adapter}(const void *payload,size_t argc,const dev_dynamic_value *argv,const char *p,size_t l,size_t c){{(void)p;(void)l;(void)c;if(argc!={})dev_dynamic_error(\"wrong object method argument count\",p,l,c);const {ct} *fn=(const {ct}*)payload;{prelude}{body}}}\n",params.len()));
        }
        let value = self.sequence_save(value);
        let keep = self.helper(&function_type, true);
        Ok(Value{code:format!("dev_object_function_value({adapter},(void(*)(void)){}.call,{}.context,{}.identity,&{},sizeof({ct}),{keep},{},{},{})",value.code,value.code,value.code,value.code,params.len(),if ret==Type::Str {5}else{0},c_string(&ct)),ty:data_type(),lvalue:false})
    }
    fn object_args(&mut self, args: &[Expr]) -> Result<Vec<String>, String> {
        let previous = self.object_boxing;
        self.object_boxing = true;
        let result = (|| {
            let mut values = Vec::new();
            for arg in args {
                let v = if matches!(arg.kind, ExprKind::Object(..) | ExprKind::Array(..))
                    || matches!(&arg.kind,ExprKind::Name(n) if n.as_slice()==["null"])
                {
                    self.dynamic_expr(arg)?
                } else {
                    let v = self.expr(arg, None)?;
                    if self.object_aggregate_type(&v.ty) && !matches!(v.ty, Type::Function(..)) {
                        self.object_aggregate_borrow(v, arg.span)?
                    } else {
                        self.object_argument_box(v, arg.span)?
                    }
                };
                values.push(self.dynamic_save(v).code);
            }
            Ok(values)
        })();
        self.object_boxing = previous;
        result
    }
    pub(crate) fn object_api_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        let values = self.object_args(args)?;
        let list = if values.is_empty() {
            "NULL".into()
        } else {
            format!("(dev_dynamic_value[]){{{}}}", values.join(","))
        };
        let result = self.dynamic_save(Value {
            code: format!(
                "dev_object_api({}, {}, {list}, {})",
                c_string(name),
                values.len(),
                self.location(span)
            ),
            ty: data_type(),
            lvalue: false,
        });
        match name {
            "isExtensible" | "isFrozen" | "isSealed" | "delete" => {
                self.dynamic_unbox(result, &Type::Bool, span)
            }
            "toString" => self.dynamic_unbox(result, &Type::Str, span),
            "array" | "keys" | "getOwnPropertyNames" | "values" | "entries" => {
                let element = if name == "keys" || name == "getOwnPropertyNames" {
                    Type::Str
                } else {
                    data_type()
                };
                let vector = Type::Vector(Box::new(element.clone()));
                let helper = self.collection(&vector);
                let out = self.owned(Value {
                    code: format!("{helper}_make(NULL,0)"),
                    ty: vector,
                    lvalue: false,
                });
                let id = self.next_var;
                self.next_var += 1;
                let item = if element == Type::Str {
                    format!("dev_dynamic_as_string(dev_dynamic_node_of({})->data.list.values[dev_object_i_{id}],{})",result.code,self.location(span))
                } else {
                    format!(
                        "dev_dynamic_node_of({})->data.list.values[dev_object_i_{id}]",
                        result.code
                    )
                };
                self.prelude.push_str(&format!("for(size_t dev_object_i_{id}=0;dev_object_i_{id}<dev_dynamic_node_of({})->data.list.len;++dev_object_i_{id}){helper}_push(&{}, {item});\n",result.code,out.code));
                Ok(out)
            }
            _ => Ok(result),
        }
    }
    pub(crate) fn object_invoke_value(
        &mut self,
        callable: Value,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        let callable = self.dynamic_save(callable);
        let values = self.object_args(args)?;
        let list = if values.is_empty() {
            "NULL".into()
        } else {
            format!("(dev_dynamic_value[]){{{}}}", values.join(","))
        };
        Ok(Value {
            code: format!(
                "dev_object_invoke({}, {}, {list}, {})",
                callable.code,
                values.len(),
                self.location(span)
            ),
            ty: data_type(),
            lvalue: false,
        })
    }
    pub(crate) fn object_method_call(
        &mut self,
        receiver: Value,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        let receiver = self.dynamic_save(receiver);
        let values = self.object_args(args)?;
        let list = if values.is_empty() {
            "NULL".into()
        } else {
            format!("(dev_dynamic_value[]){{{}}}", values.join(","))
        };
        Ok(Value {
            code: format!(
                "dev_object_method({}, {}, {}, {list}, {})",
                receiver.code,
                c_string(name),
                values.len(),
                self.location(span)
            ),
            ty: data_type(),
            lvalue: false,
        })
    }
}
