//! Hosted native DON objects. Values are immutable; updates copy only the path.
use crate::ast::*;
use std::path::Path;

pub(crate) fn native_module(path: &Path) -> bool {
    path.parent()
        .is_some_and(|p| p.ends_with("__devlang_intrinsics__"))
        && path
            .file_stem()
            .is_some_and(|n| n == "std_json" || n == "std_don")
}

// Inference/ABI declarations only. Calls lower to the C data backend; these
// placeholder bodies are never emitted or exposed as callable function values.
pub(crate) const NATIVE_SIGNATURES: &str = r#"
struct Value {}
fn parse(text str) Value {return Value()}
fn fromJSON(text str) Value {return Value()}
fn valid(text str) bool {return false}
fn get(value Value,key str) Value {return Value()}
fn at(value Value,index i64) Value {return Value()}
fn object() Value {return Value()}
fn array() Value {return Value()}
fn null_value() Value {return Value()}
fn set(value Value,key str,item Value) Value {return Value()}
fn push(value Value,item Value) Value {return Value()}
fn remove(value Value,key str) Value {return Value()}
fn value(item Value) Value {return Value()}
fn stringify(value Value) str {return ""}
fn pretty(value Value) str {return ""}
fn toJSON(value Value) str {return ""}
fn kind(value Value) str {return ""}
fn len(value Value) usize {return 0 as usize}
fn keys(value Value) Vec<str> {return Vec<str>()}
fn has(value Value,key str) bool {return false}
fn is_null(value Value) bool {return false}
fn string(value Value) str {return ""}
fn int(value Value) i64 {return 0}
fn uint(value Value) u64 {return 0 as u64}
fn float(value Value) f64 {return 0.0}
fn bool(value Value) bool {return false}
"#;
use crate::codegen::{c_string, c_type, Emitter, Value};

pub(crate) fn is_dynamic(t: &Type) -> bool {
    matches!(t, Type::Record(n, _) | Type::Nominal(n) if n == "dev_dynamic_value" || n == "dev_dynamic_don_value")
}
fn ty() -> Type {
    Type::Record("dev_dynamic_value".into(), vec![])
}
fn value(code: String, t: Type) -> Value {
    Value {
        code,
        ty: t,
        lvalue: false,
    }
}

impl Emitter<'_> {
    pub(crate) fn dynamic_enable(&mut self, span: Span) -> Result<(), String> {
        if !self.hosted {
            return self.fail(span, "DON objects require hosted allocation");
        }
        self.uses_dynamic = true;
        Ok(())
    }
    pub(crate) fn dynamic_save(&mut self, v: Value) -> Value {
        let name = format!("dev_dynamic_tmp_{}", self.next_var);
        self.next_var += 1;
        self.prelude
            .push_str(&format!("{} {name} = {};\n", c_type(&v.ty), v.code));
        value(name, v.ty)
    }
    pub(crate) fn dynamic_box(&mut self, v: Value, span: Span) -> Result<Value, String> {
        self.dynamic_box_depth(v, span, 0)
    }
    fn dynamic_box_depth(&mut self, v: Value, span: Span, depth: usize) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        if depth > 64 {
            return self.fail(span,"native DON encoding type nesting exceeds 64; flatten recursive references before encoding");
        }
        let v = if matches!(v.ty,Type::Nominal(_)) && !is_dynamic(&v.ty) {Value{code:v.code,ty:self.resolve_collection_element(&v.ty),lvalue:v.lvalue}}else{v};
        if is_dynamic(&v.ty) {if self.object_boxing {return Ok(v);}return Ok(value(format!("dev_object_data_snapshot({}, {})",v.code,self.location(span)),v.ty));}
        if matches!(v.ty,Type::Function(..)) {if self.object_boxing {return self.object_function(v,span);}return self.fail(span,"functions require a prototype Object; use Object({...})");}
        match &v.ty {
            Type::Record(_, fields) => {
                let fields = fields.clone();
                let source = self.dynamic_save(v);
                let mut keys = vec![];
                let mut values = vec![];
                for (i, (key, t)) in fields.iter().enumerate() {
                    keys.push(c_string(key));
                    let item = value(format!("{}.dev_f_{i}", source.code), t.clone());
                    let item = self.dynamic_box_depth(item, span, depth + 1)?;
                    values.push(self.dynamic_save(item).code);
                }
                let keys = if keys.is_empty() {
                    "NULL".into()
                } else {
                    format!("(const char*[]){{{}}}", keys.join(","))
                };
                let data = if values.is_empty() {
                    "NULL".into()
                } else {
                    format!("(dev_dynamic_value[]){{{}}}", values.join(","))
                };
                return Ok(value(
                    format!("dev_dynamic_object({}, {keys}, {data})", fields.len()),
                    ty(),
                ));
            }
            Type::Ref(inner) => {
                let inner = self.resolve_collection_element(inner);
                return self.dynamic_box_depth(
                    value(format!("(*{})", v.code), inner),
                    span,
                    depth + 1,
                );
            }
            Type::Array(inner, _)
            | Type::Vector(inner)
            | Type::Slice(inner)
            | Type::Map(_, inner) => {
                let element = self.resolve_collection_element(inner);
                let array = matches!(v.ty, Type::Array(..));
                let map = matches!(v.ty, Type::Map(..));
                if matches!(&v.ty,Type::Map(key,_) if **key!=Type::Str) {
                    return self.fail(span, "DON map encoding requires str keys");
                }
                let count = if let Type::Array(_, n) = &v.ty {
                    n.to_string()
                } else if map {
                    format!("{}.entries.len", v.code)
                } else {
                    format!("{}.len", v.code)
                };
                let source = if array { v } else { self.dynamic_save(v) };
                let index = format!("dev_encode_index_{}", self.next_var);
                self.next_var += 1;
                let items = format!("dev_encode_items_{}", self.next_var);
                self.next_var += 1;
                let keys = format!("dev_encode_keys_{}", self.next_var);
                self.next_var += 1;
                // Recompute count using the saved value so input expressions run once.
                let count = if array {
                    count
                } else if map {
                    format!("{}.entries.len", source.code)
                } else {
                    format!("{}.len", source.code)
                };
                self.prelude.push_str(&format!("dev_dynamic_value *{items}=dev_dynamic_alloc({count}*sizeof(dev_dynamic_value));\n"));
                if map {
                    self.prelude.push_str(&format!(
                        "const char **{keys}=dev_dynamic_alloc({count}*sizeof(char*));\n"
                    ));
                }
                self.prelude.push_str(&format!(
                    "for(size_t {index}=0;{index}<{count};{index}++) {{\n"
                ));
                let access = if array {
                    format!("({})[{index}]", source.code)
                } else if map {
                    format!(
                        "{}.entries.data[{}.entries.offset+{index}].dev_f_1",
                        source.code, source.code
                    )
                } else {
                    format!("{}.data[{}.offset+{index}]", source.code, source.code)
                };
                let item = self.dynamic_box_depth(value(access, element), span, depth + 1)?;
                self.prelude
                    .push_str(&format!("{items}[{index}]={};\n", item.code));
                if map {
                    self.prelude.push_str(&format!(
                        "{keys}[{index}]={}.entries.data[{}.entries.offset+{index}].dev_f_0;\n",
                        source.code, source.code
                    ));
                }
                self.prelude.push_str("}\n");
                return Ok(value(
                    if map {
                        format!("dev_dynamic_object({count},{keys},{items})")
                    } else {
                        format!("dev_dynamic_array({count},{items})")
                    },
                    ty(),
                ));
            }
            _ => {}
        }
        let function = match v.ty {
            Type::Str => "string",
            Type::Bool => "bool",
            Type::Int { signed: true, .. } | Type::Size { signed: true } => "int",
            Type::Int { signed: false, .. } | Type::Size { signed: false } => "uint",
            Type::Float(_) => "float",
            _ => {
                return self.fail(
                    span,
                    "DON objects accept strings, numbers, bool, null, objects and arrays",
                )
            }
        };
        let code=format!("dev_dynamic_{function}({})",v.code);
        let scalar=match v.ty {Type::Bool=>1,Type::Str=>5,Type::Int{bits:64,signed:true}=>2,Type::Int{bits:64,signed:false}=>3,Type::Int{bits,signed}=>10+bits as u32+if signed {128}else{0},Type::Float(64)=>4,Type::Float(32)=>8,Type::Size{signed:true}=>6,Type::Size{signed:false}=>7,_=>0};
        Ok(value(if self.object_boxing&&scalar!=0 {format!("dev_dynamic_typed({code},{scalar})")}else{code},ty()))
    }
    pub(crate) fn dynamic_cast(&mut self, v: Value, t: &Type, span: Span) -> Result<Value, String> {
        let v = self.dynamic_save(v);
        let location = self.location(span);
        let code = match t {
            t if t.integer() => {
                self.uses_checks = true;
                format!("(dev_dynamic_tag({})==4 ? dev_checked_float_cast_{}(dev_dynamic_cast_float({}, {location}), {location}) : (({})dev_dynamic_cast_integer({}, {location})))",v.code,t.name(),v.code,c_type(t),v.code)
            }
            Type::Float(_) => format!(
                "(({})dev_dynamic_cast_float({}, {location}))",
                c_type(t),
                v.code
            ),
            Type::Bool => format!("dev_dynamic_cast_bool({}, {location})", v.code),
            Type::Str => return self.dynamic_coerce(v, t, span),
            _ => return self.fail(span, "DON casts require scalar types"),
        };
        Ok(value(code, t.clone()))
    }
    pub(crate) fn dynamic_coerce(
        &mut self,
        v: Value,
        t: &Type,
        span: Span,
    ) -> Result<Value, String> {
        let code = match t {
            Type::Bool => 1,
            Type::Int {
                bits: 64,
                signed: true,
            } => 2,
            Type::Int {
                bits: 64,
                signed: false,
            } => 3,
            Type::Float(64) => 4,
            Type::Str => 5,
            Type::Int { bits, signed } => 10 + *bits as u32 + if *signed { 128 } else { 0 },
            Type::Size { signed: true } => 6,
            Type::Size { signed: false } => 7,
            Type::Float(32) => 8,
            _ => 0,
        };
        let checked = value(
            format!(
                "dev_dynamic_require({}, {code}, {})",
                v.code,
                self.location(span)
            ),
            ty(),
        );
        self.dynamic_unbox(checked, t, span)
    }
    pub(crate) fn dynamic_unbox(
        &mut self,
        v: Value,
        t: &Type,
        span: Span,
    ) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        let function = match t {
            Type::Str => "as_string",
            Type::Bool => "as_bool",
            Type::Int { signed: true, .. } | Type::Size { signed: true } => "as_int",
            Type::Int { signed: false, .. } | Type::Size { signed: false } => "as_uint",
            Type::Float(_) => "as_float",
            _ => return self.fail(span, "cannot convert DON value to this type"),
        };
        // Check narrowing before the C cast, rather than silently truncating.
        let code = if t.integer() {
            let (bits, signed) = match t {
                Type::Int { bits, signed } => (*bits as u32, *signed),
                Type::Size { signed } => (usize::BITS, *signed),
                _ => unreachable!(),
            };
            format!(
                "(({})dev_dynamic_checked_integer({}, {bits}, {}, {}))",
                c_type(t),
                v.code,
                signed,
                self.location(span)
            )
        } else {
            format!(
                "dev_dynamic_{function}({}, {})",
                v.code,
                self.location(span)
            )
        };
        Ok(value(code, t.clone()))
    }
    pub(crate) fn dynamic_expr(&mut self, e: &Expr) -> Result<Value, String> {
        self.dynamic_enable(e.span)?;
        if let ExprKind::Array(items) = &e.kind {
            let mut values = vec![];
            for item in items {
                let v = self.dynamic_expr(item)?;
                values.push(self.dynamic_save(v).code);
            }
            let data = if values.is_empty() {
                "NULL".into()
            } else {
                format!("(dev_dynamic_value[]){{{}}}", values.join(","))
            };
            return Ok(value(
                format!("dev_dynamic_array({}, {data})", values.len()),
                ty(),
            ));
        }
        if matches!(&e.kind, ExprKind::Name(n) if n.as_slice() == ["null"]) {
            return Ok(value("((dev_dynamic_value){NULL,1})".into(), ty()));
        }
        let v=self.expr(e,None)?;
        if self.object_boxing {self.object_argument_box(v,e.span)} else {self.dynamic_box(v,e.span)}
    }
    pub(crate) fn dynamic_object(
        &mut self,
        t: &Type,
        fields: &[(String, Expr)],
        span: Span,
    ) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        let mut keys = vec![];
        let mut values = vec![];
        for (key, e) in fields {
            keys.push(c_string(key));
            let v = self.dynamic_expr(e)?;
            values.push(self.dynamic_save(v).code);
        }
        let k = if keys.is_empty() {
            "NULL".into()
        } else {
            format!("(const char*[]){{{}}}", keys.join(","))
        };
        let v = if values.is_empty() {
            "NULL".into()
        } else {
            format!("(dev_dynamic_value[]){{{}}}", values.join(","))
        };
        Ok(value(
            format!("{}({}, {k}, {v})",if self.object_boxing {"dev_object_literal"} else {"dev_dynamic_object"},fields.len()),
            t.clone(),
        ))
    }
    pub(crate) fn dynamic_field(
        &mut self,
        v: Value,
        name: &str,
        span: Span,
    ) -> Result<Value, String> {
        self.dynamic_enable(span)?;
        let t = v.ty.clone();
        let v = self.dynamic_save(v);
        Ok(value(
            format!(
                "dev_dynamic_read_field({}, {}, {})",
                v.code,
                c_string(name),
                self.location(span)
            ),
            t,
        ))
    }
    pub(crate) fn dynamic_index(
        &mut self,
        v: Value,
        key: Value,
        span: Span,
    ) -> Result<Value, String> {
        if key.ty != Type::Str && !key.ty.integer() && !is_dynamic(&key.ty) {
            return self.fail(
                span,
                "object key requires str; array index requires integer",
            );
        }
        if key.ty == Type::Str {
            let t = v.ty.clone();
            let v = self.dynamic_save(v);
            return Ok(value(
                format!(
                    "dev_dynamic_read_field({}, {}, {})",
                    v.code,
                    key.code,
                    self.location(span)
                ),
                t,
            ));
        }
        let result_ty = v.ty.clone();
        let v = self.dynamic_save(v);
        let key = self.dynamic_operand(key, span)?;
        let key = self.dynamic_save(key);
        Ok(value(
            format!(
                "dev_dynamic_read({}, {}, {})",
                v.code,
                key.code,
                self.location(span)
            ),
            result_ty,
        ))
    }
    pub(crate) fn dynamic_assignment(
        &mut self,
        target: &Expr,
        op: &str,
        rhs: &Expr,
    ) -> Result<Option<String>, String> {
        fn path<'a>(e: &'a Expr, keys: &mut Vec<Result<&'a Expr, &'a str>>) -> &'a Expr {
            match &e.kind {
                ExprKind::Field(b, n) => {
                    let root = path(b, keys);
                    keys.push(Err(n));
                    root
                }
                ExprKind::Index(b, i) => {
                    let root = path(b, keys);
                    keys.push(Ok(i));
                    root
                }
                _ => e,
            }
        }
        let normalized;
        let target=if let ExprKind::Name(names)=&target.kind {if names.len()>1 {let mut e=Expr{span:target.span,kind:ExprKind::Name(vec![names[0].clone()])};for name in &names[1..]{e=Expr{span:target.span,kind:ExprKind::Field(Box::new(e),name.clone())};}normalized=e;&normalized}else{target}}else{target};
        let mut keys = vec![];
        let root = path(target, &mut keys);

        let root = if let ExprKind::Name(names)=&root.kind {
            if names.len()!=1 {return Ok(None);}
            if let Some(variable)=self.variable(&names[0]) {
            if is_dynamic(&variable.ty) {variable.name.clone()} else if !keys.is_empty() && matches!(variable.ty,Type::Function(..)) {let v=self.expr(root,None)?;let v=self.object_function(v,target.span)?;self.dynamic_save(v).code} else {return Ok(None);}
            } else if !keys.is_empty() {let v=self.expr(root,None)?;if !matches!(v.ty,Type::Function(..)){return Ok(None);}let v=self.object_function(v,target.span)?;self.dynamic_save(v).code} else {return Ok(None);}
        } else if !keys.is_empty() && (matches!(root.kind,ExprKind::Callable(..)) || matches!(&root.kind,ExprKind::Call(names,_) if names.first().is_some_and(|n|n=="$object"))) {
            let v=self.expr(root,None)?;let v=self.dynamic_operand(v,target.span)?;self.dynamic_save(v).code
        } else {return Ok(None);};
        self.dynamic_enable(target.span)?;
        if keys.is_empty() {
            let wide = wide_literal(rhs);
            let v = self.expr(rhs, wide.as_ref())?;
            let v = self.dynamic_operand(v, rhs.span)?;
            let v = self.dynamic_save(v);
            let code = v.code;
            let code = if number_literal(rhs) {
                format!(
                    "dev_dynamic_literal({code}, {root}, {})",
                    self.location(target.span)
                )
            } else {
                code
            };
            return Ok(Some(format!(
                "{root}=dev_dynamic_assign({root}, {code}, {}, {});\n",
                c_string(op),
                self.location(target.span)
            )));
        }
        let mut codes = vec![];
        for key in keys {
            let v = match key {
                Ok(e) => {
                    let value = self.expr(e, None)?;
                    self.dynamic_operand(value, e.span)?
                }
                Err(n) => value(
                    format!("dev_dynamic_unboxed(dev_dynamic_string({}))", c_string(n)),
                    ty(),
                ),
            };
            codes.push(self.dynamic_save(v).code);
        }
        let path_name = format!("dev_dynamic_path_{}", self.next_var);
        self.next_var += 1;
        let data = if codes.is_empty() {
            "NULL".into()
        } else {
            self.prelude.push_str(&format!(
                "dev_dynamic_value {path_name}[]={{ {} }};\n",
                codes.join(",")
            ));
            path_name
        };
        let v = if op == "=" {
            let previous=self.object_boxing;self.object_boxing=true;let result=self.dynamic_expr(rhs);self.object_boxing=previous;result?
        } else {
            let wide = wide_literal(rhs);
            let v = self.expr(rhs, wide.as_ref())?;
            self.dynamic_operand(v, rhs.span)?
        };
        let v = self.dynamic_save(v);
        let rhs_code = if op != "=" && number_literal(rhs) {
            format!(
                "dev_dynamic_literal({}, dev_dynamic_read_path({root}, {}, {data}, {}), {})",
                v.code,
                codes.len(),
                self.location(target.span),
                self.location(rhs.span)
            )
        } else {
            v.code
        };
        Ok(Some(format!(
            "{root}=dev_dynamic_update({root}, {}, {data}, {}, {}, {});\n",
            codes.len(),
            rhs_code,
            c_string(op),
            self.location(target.span)
        )))
    }
    pub(crate) fn dynamic_operand(&mut self, v: Value, span: Span) -> Result<Value, String> {
        if is_dynamic(&v.ty) {return Ok(v);}
        if matches!(v.ty,Type::Function(..)) {return self.object_function(v,span);}
        let scalar = !is_dynamic(&v.ty);
        let code = match v.ty {
            Type::Bool => 1,
            Type::Int {
                bits: 64,
                signed: true,
            } => 2,
            Type::Int {
                bits: 64,
                signed: false,
            } => 3,
            Type::Float(64) => 4,
            Type::Str => 5,
            Type::Size { signed: true } => 6,
            Type::Size { signed: false } => 7,
            Type::Float(32) => 8,
            Type::Int { bits, signed } => 10 + bits as u32 + if signed { 128 } else { 0 },
            _ => 0,
        };
        let mut v = self.dynamic_box(v, span)?;
        if scalar {
            v.code = format!("dev_dynamic_typed({}, {code})", v.code);
        }
        Ok(v)
    }
    pub(crate) fn dynamic_binary(
        &mut self,
        op: &str,
        a: Value,
        b: Value,
        rhs_literal: bool,
        span: Span,
    ) -> Result<Value, String> {
        let a = self.dynamic_operand(a, span)?;
        let a = self.dynamic_save(a);
        let b = self.dynamic_operand(b, span)?;
        let b = self.dynamic_save(b);
        let b = if rhs_literal {
            value(
                format!(
                    "dev_dynamic_literal({}, {}, {})",
                    b.code,
                    a.code,
                    self.location(span)
                ),
                ty(),
            )
        } else {
            b
        };
        let v = value(
            format!(
                "dev_dynamic_typed(dev_dynamic_binary({}, {}, {}, {}), {})",
                c_string(op),
                a.code,
                b.code,
                self.location(span),
                if ["==", "!=", "<", "<=", ">", ">="].contains(&op) {
                    "1".into()
                } else {
                    format!("{}.scalar_type", a.code)
                }
            ),
            ty(),
        );
        if ["==", "!=", "<", "<=", ">", ">="].contains(&op) {
            self.dynamic_unbox(v, &Type::Bool, span)
        } else {
            Ok(v)
        }
    }
    pub(crate) fn dynamic_unary(
        &mut self,
        op: &str,
        v: Value,
        span: Span,
    ) -> Result<Value, String> {
        let v = self.dynamic_save(v);
        let v = value(
            format!(
                "dev_dynamic_typed(dev_dynamic_unary({}, {}, {}), {})",
                c_string(op),
                v.code,
                self.location(span),
                if op == "!" {
                    "1".into()
                } else {
                    format!("{}.scalar_type", v.code)
                }
            ),
            ty(),
        );
        if op == "!" {
            self.dynamic_unbox(v, &Type::Bool, span)
        } else {
            Ok(v)
        }
    }
    pub(crate) fn dynamic_call(
        &mut self,
        names: &[String],
        args: &[Expr],
        span: Span,
    ) -> Result<Option<Value>, String> {
        if names.len() != 2 {
            return Ok(None);
        };
        let Some(&id) = self.program.aliases[self.module].get(&names[0]) else {
            return Ok(None);
        };
        let module = &self.program.modules[id].path;
        if !module
            .parent()
            .is_some_and(|p| p.ends_with("__devlang_intrinsics__"))
            || !module
                .file_stem()
                .is_some_and(|n| n == "std_json" || n == "std_don")
        {
            return Ok(None);
        };
        let name = names[1].as_str();
        if module.file_stem().is_some_and(|n| n == "std_json")
            && ["fromJSON", "toJSON"].contains(&name)
        {
            return self.fail(span, "unknown JSON intrinsic");
        }
        let result_ty = if module.file_stem().is_some_and(|n| n == "std_don") {
            Type::Record("dev_dynamic_don_value".into(), vec![])
        } else {
            ty()
        };
        if ["parse", "fromJSON", "valid"].contains(&name) {
            if args.len() != 1 {
                return self.fail(span, "parse/valid expects one string");
            }
            let input = self.expr(&args[0], Some(&Type::Str))?;
            self.compatible(&input, &Type::Str, args[0].span)?;
            let don = module.file_stem().is_some_and(|n| n == "std_don");
            if don && name != "fromJSON" {
                if self.diagnostics.is_some() {
                    return Ok(Some(value(
                        if name == "valid" {
                            "false".into()
                        } else {
                            "((dev_dynamic_value){NULL,1})".into()
                        },
                        if name == "valid" {
                            Type::Bool
                        } else {
                            result_ty.clone()
                        },
                    )));
                }
                return self.fail(
                    span,
                    "DON document parsing still requires the source runtime",
                );
            }
            self.dynamic_enable(span)?;
            return Ok(Some(value(
                if name == "valid" {
                    format!("dev_dynamic_valid({})", input.code)
                } else {
                    format!("dev_dynamic_parse({}, {})", input.code, self.location(span))
                },
                if name == "valid" {
                    Type::Bool
                } else {
                    result_ty.clone()
                },
            )));
        }
        let count = match name {
            "object" | "array" | "null_value" => 0,
            "get" | "at" | "has" | "push" | "remove" => 2,
            "set" => 3,
            "value" | "stringify" | "pretty" | "toJSON" | "kind" | "is_null" | "len" | "keys"
            | "string" | "int" | "uint" | "float" | "bool" => 1,
            _ => return self.fail(span, format!("native {}.{name} is not supported", names[0])),
        };
        if args.len() != count {
            return self.fail(span, format!("{name} expects {count} arguments"));
        };
        self.dynamic_enable(span)?;
        let mut v = vec![];
        for (i, arg) in args.iter().enumerate() {
            let x = if i == 1 && ["get", "has", "set", "remove"].contains(&name) {
                let key = self.expr(arg, Some(&Type::Str))?;
                self.compatible(&key, &Type::Str, arg.span)?;
                self.dynamic_operand(key, arg.span)?
            } else if i == 1 && name == "at" {
                let index = self.expr(arg, None)?;
                self.dynamic_operand(index, arg.span)?
            } else if i == 0 && !["value", "stringify", "pretty", "toJSON"].contains(&name) {
                let x = self.expr(arg, None)?;
                if !is_dynamic(&x.ty) {
                    return self.fail(arg.span, "expected json.Value");
                }
                value(
                    format!(
                        "dev_object_data_snapshot(dev_dynamic_require_boxed({}, {}), {})",
                        x.code,
                        self.location(arg.span),
                        self.location(arg.span)
                    ),
                    result_ty.clone(),
                )
            } else {
                self.dynamic_expr(arg)?
            };
            v.push(self.dynamic_save(x).code)
        }
        let loc = self.location(span);
        let (code, t) = match name {
            "object" => ("dev_dynamic_object(0,NULL,NULL)".into(), result_ty.clone()),
            "array" => ("dev_dynamic_array(0,NULL)".into(), result_ty.clone()),
            "null_value" => ("((dev_dynamic_value){NULL,1})".into(), result_ty.clone()),
            "value" => (format!("dev_dynamic_boxed({})", v[0]), result_ty.clone()),
            "stringify" if module.file_stem().is_some_and(|n| n == "std_don") => (
                format!("dev_dynamic_don_serialize({}, {})", v[0], loc),
                Type::Str,
            ),
            "stringify" | "pretty" | "toJSON" => (
                format!(
                    "dev_dynamic_serialize({}, {}, {})",
                    v[0],
                    name == "pretty",
                    loc
                ),
                Type::Str,
            ),
            "kind" => (format!("dev_dynamic_kind({})", v[0]), Type::Str),
            "is_null" => (format!("({}.node==NULL)", v[0]), Type::Bool),
            "keys" => {
                let t = Type::Vector(Box::new(Type::Str));
                let collection = self.collection(&t);
                let count = format!("dev_dynamic_len({}, {})", v[0], loc);
                let input = format!("dev_dynamic_keys({}, {})", v[0], loc);
                return Ok(Some(
                    self.owned(value(format!("{collection}_make({input}, {count})"), t)),
                ));
            }
            "len" => (
                format!("dev_dynamic_len({}, {loc})", v[0]),
                Type::Size { signed: false },
            ),
            "string" | "int" | "uint" | "float" | "bool" => {
                let t = match name {
                    "string" => Type::Str,
                    "int" => Type::i64(),
                    "uint" => Type::Int {
                        bits: 64,
                        signed: false,
                    },
                    "float" => Type::Float(64),
                    _ => Type::Bool,
                };
                return Ok(Some(self.dynamic_unbox(
                    value(v[0].clone(), result_ty.clone()),
                    &t,
                    span,
                )?));
            }
            "get" | "at" => (
                format!("dev_dynamic_get({}, {}, {loc})", v[0], v[1]),
                result_ty.clone(),
            ),
            "has" => (
                format!("dev_dynamic_has({}, {}, {loc})", v[0], v[1]),
                Type::Bool,
            ),
            "set" => (
                format!(
                    "dev_dynamic_update({},1,&{}, {},\"=\",{loc})",
                    v[0], v[1], v[2]
                ),
                result_ty.clone(),
            ),
            "push" => (
                format!("dev_dynamic_push({}, {}, {loc})", v[0], v[1]),
                result_ty.clone(),
            ),
            "remove" => (
                format!("dev_dynamic_remove({}, {}, {loc})", v[0], v[1]),
                result_ty.clone(),
            ),
            _ => unreachable!(),
        };
        Ok(Some(value(code, t)))
    }
}

fn number_literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Number(_) => true,
        ExprKind::Unary(op, a) if op == "+" || op == "-" => number_literal(a),
        _ => false,
    }
}

// An untyped dynamic scalar can be u64 at execution time. Preserve a large
// positive RHS literal until the runtime context checks its numeric type.
pub(crate) fn wide_literal(e: &Expr) -> Option<Type> {
    let ExprKind::Number(n) = &e.kind else {
        return None;
    };
    let value = if let Some(hex) = n.strip_prefix("0x").or_else(|| n.strip_prefix("0X")) {
        u128::from_str_radix(hex, 16).ok()
    } else {
        n.parse::<u128>().ok()
    }?;
    (value > i64::MAX as u128 && value <= u64::MAX as u128).then_some(Type::Int {
        bits: 64,
        signed: false,
    })
}
