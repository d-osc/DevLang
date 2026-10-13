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
        self.dynamic_box(value, span)
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
                } else {
                    return self.fail(span,"native Object callback parameters require scalar types or json.Value/don.Value");
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
                        code: "result".into(),
                        ty: ret.clone(),
                        lvalue: false,
                    },
                    span,
                )?;
                let code = format!("dev_dynamic_value out={};\n", boxed.code);
                let drop = self.manage(&ret, "result", false);
                format!("{code}{drop}return out;")
            };
            let prelude = std::mem::take(&mut self.prelude);
            self.prelude = saved;
            self.object_boxing = old_box;
            self.adapters.insert(adapter.clone(),format!("static dev_dynamic_value {adapter}(const void *payload,size_t argc,const dev_dynamic_value *argv,const char *p,size_t l,size_t c){{(void)p;(void)l;(void)c;if(argc!={})dev_dynamic_error(\"wrong object method argument count\",p,l,c);const {ct} *fn=(const {ct}*)payload;{prelude}{body}}}\n",params.len()));
        }
        let value = self.sequence_save(value);
        let keep = self.helper(&function_type, true);
        Ok(Value{code:format!("dev_object_function_value({adapter},(void(*)(void)){}.call,{}.context,{}.identity,&{},sizeof({ct}),{keep},{},{})",value.code,value.code,value.code,value.code,params.len(),if ret==Type::Str {5}else{0}),ty:data_type(),lvalue:false})
    }
    fn object_args(&mut self, args: &[Expr]) -> Result<Vec<String>, String> {
        let previous = self.object_boxing;
        self.object_boxing = true;
        let result = (|| {
            let mut values = Vec::new();
            for arg in args {
                let v = self.dynamic_expr(arg)?;
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
