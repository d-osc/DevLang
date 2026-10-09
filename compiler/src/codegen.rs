use crate::ast::*;
use crate::program::{module_name, valid_type, Program};
use std::collections::HashMap;

pub const TYPES_HEADER: &str = "#ifndef DEV_TYPES_H\n#define DEV_TYPES_H\n#include <stdint.h>\n#include <stddef.h>\n#include <stdbool.h>\n#if defined(__SSE4_1__)\n#define DEV_LOW32_SELECT(narrow, wide) (narrow)\n#else\n#define DEV_LOW32_SELECT(narrow, wide) (wide)\n#endif\n#endif\n";
pub struct Generated {
    pub name: String,
    pub header: String,
    pub source: String,
    pub dependencies: Vec<usize>,
}

pub fn c_type(ty: &Type) -> String {
    match ty {
        Type::Void => "void".into(),
        Type::Bool => "bool".into(),
        Type::Str => "const char *".into(),
        Type::Int { bits, signed } => format!("{}int{bits}_t", if *signed { "" } else { "u" }),
        Type::Size { signed } => if *signed { "intptr_t" } else { "uintptr_t" }.into(),
        Type::Float(32) => "float".into(),
        Type::Float(_) => "double".into(),
        Type::Ptr(inner) => format!("{} *", c_type(inner)),
        Type::Array(..) => unreachable!("array requires a declarator"),
    }
}
fn declaration(ty: &Type, name: &str) -> String {
    match ty {
        Type::Array(t, n) => format!("{} {name}[{n}]", c_type(t)),
        _ => format!("{} {name}", c_type(ty)),
    }
}
fn prototype(program: &Program, id: usize, function: &Function) -> String {
    let signature = &program.signatures[&(id, function.name.clone())];
    let params: Vec<String> = function
        .params
        .iter()
        .enumerate()
        .map(|(i, (_, t))| declaration(t, &format!("dev_p_{i}")))
        .collect();
    format!(
        "{} {}({})",
        c_type(&function.ret),
        signature.c_name,
        if params.is_empty() {
            "void".into()
        } else {
            params.join(", ")
        }
    )
}
pub fn generate(program: &Program, hosted: bool) -> Result<Vec<Generated>, String> {
    let mut result = Vec::new();
    for (id, module) in program.modules.iter().enumerate() {
        let name = module_name(&module.path);
        let mut header = format!(
            "#ifndef DEV_{}_H\n#define DEV_{}_H\n#include \"dev_types.h\"\n",
            name.to_uppercase(),
            name.to_uppercase()
        );
        for f in &module.functions {
            header.push_str(&format!("{};\n", prototype(program, id, f)));
        }
        header.push_str("#endif\n");
        let mut dependencies: Vec<usize> = program.aliases[id].values().copied().collect();
        dependencies.push(id);
        dependencies.sort_unstable();
        dependencies.dedup();
        let mut source = String::new();
        for dep in &dependencies {
            source.push_str(&format!(
                "#include \"{}.h\"\n",
                module_name(&program.modules[*dep].path)
            ));
        }
        let mut emitter = Emitter {
            program,
            module: id,
            scopes: Vec::new(),
            next_var: 0,
            ret: Type::Void,
            loop_depth: 0,
            uses_print: false,
            hosted,
        };
        let mut functions = String::new();
        for f in &module.functions {
            let Some(body) = &f.body else {
                continue;
            };
            emitter.ret = f.ret.clone();
            emitter.scopes = vec![HashMap::new()];
            emitter.next_var = 0;
            for (i, (name, ty)) in f.params.iter().enumerate() {
                emitter.scopes[0].insert(
                    name.clone(),
                    Variable {
                        name: format!("dev_p_{i}"),
                        ty: ty.clone(),
                    },
                );
            }
            let text = emitter.block(body, false)?;
            let is_entry = id == 0 && f.name == "main";
            if f.ret != Type::Void && !is_entry && !returns(body) {
                return Err(error(
                    &module.path,
                    f.span,
                    "function must return a value on every path",
                ));
            }
            functions.push_str(&format!(
                "#line {} {}\n{} {{\n{}",
                f.span.line,
                c_string(&module.path.to_string_lossy()),
                prototype(program, id, f),
                text
            ));
            if is_entry {
                functions.push_str("return 0;\n");
            }
            functions.push_str("}\n");
        }
        if emitter.uses_print {
            source.push_str("#include <stdio.h>\n");
        }
        source.push_str(&functions);
        result.push(Generated {
            name,
            header,
            source,
            dependencies,
        });
    }
    Ok(result)
}

fn returns(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Return(..) => true,
        Stmt::If { yes, no, .. } => returns(yes) && returns(no),
        _ => false,
    })
}
#[derive(Clone)]
struct Variable {
    name: String,
    ty: Type,
}
struct Value {
    code: String,
    ty: Type,
    lvalue: bool,
}
struct Emitter<'a> {
    program: &'a Program,
    module: usize,
    scopes: Vec<HashMap<String, Variable>>,
    next_var: usize,
    ret: Type,
    loop_depth: usize,
    uses_print: bool,
    hosted: bool,
}
impl Emitter<'_> {
    fn fail<T>(&self, span: Span, message: impl AsRef<str>) -> Result<T, String> {
        Err(error(
            &self.program.modules[self.module].path,
            span,
            message,
        ))
    }
    fn compatible(&self, value: &Value, expected: &Type, span: Span) -> Result<(), String> {
        if &value.ty == expected {
            Ok(())
        } else {
            self.fail(
                span,
                format!(
                    "expected {}, found {}; use 'as {}' to convert",
                    expected.name(),
                    value.ty.name(),
                    expected.name()
                ),
            )
        }
    }
    fn variable(&self, name: &str) -> Option<&Variable> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
    fn block(&mut self, body: &[Stmt], scoped: bool) -> Result<String, String> {
        if scoped {
            self.scopes.push(HashMap::new());
        }
        let mut text = String::new();
        for statement in body {
            text.push_str(&self.statement(statement)?);
        }
        if scoped {
            self.scopes.pop();
        }
        Ok(text)
    }
    fn statement(&mut self, stmt: &Stmt) -> Result<String, String> {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                span,
            } => {
                if self.scopes.last().unwrap().contains_key(name) {
                    return self.fail(*span, "duplicate variable in this scope");
                }
                if let Some(ty) = ty {
                    valid_type(ty, false)
                        .map_err(|e| error(&self.program.modules[self.module].path, *span, e))?;
                }
                let v = self.expr(value, ty.as_ref())?;
                if let Some(t) = ty {
                    self.compatible(&v, t, value.span)?;
                }
                if v.ty == Type::Void {
                    return self.fail(*span, "cannot assign a void expression");
                }
                if matches!(v.ty, Type::Array(..)) && !matches!(value.kind, ExprKind::Array(..)) {
                    return self.fail(*span, "initialize arrays with an array literal");
                }
                let c_name = format!("dev_v_{}", self.next_var);
                self.next_var += 1;
                let decl = declaration(&v.ty, &c_name);
                self.scopes.last_mut().unwrap().insert(
                    name.clone(),
                    Variable {
                        name: c_name,
                        ty: v.ty,
                    },
                );
                Ok(format!("{decl} = {};\n", v.code))
            }
            Stmt::Assign { target, op, value } => {
                let t = self.expr(target, None)?;
                if !t.lvalue || matches!(t.ty, Type::Array(..)) {
                    return self.fail(
                        target.span,
                        "assignment requires a writable variable, pointer, or array element",
                    );
                }
                let v = self.expr(value, Some(&t.ty))?;
                self.compatible(&v, &t.ty, value.span)?;
                if op != "=" && !t.ty.numeric() {
                    return self.fail(target.span, "compound assignment requires a number");
                }
                if ["%=", "&=", "|=", "^="].contains(&op.as_str()) && !t.ty.integer() {
                    return self.fail(target.span, "operator requires integers");
                }
                Ok(format!("{} {op} {};\n", t.code, v.code))
            }
            Stmt::Expr(e) => {
                let v = self.expr(e, None)?;
                if matches!(v.ty, Type::Array(..)) {
                    return self.fail(e.span, "array literal is only valid in an initializer");
                }
                Ok(format!("(void)({});\n", v.code))
            }
            Stmt::Return(value, span) => {
                if let Some(value) = value {
                    if self.ret == Type::Void {
                        return self.fail(*span, "void function cannot return a value");
                    }
                    let ret = self.ret.clone();
                    let v = self.expr(value, Some(&ret))?;
                    self.compatible(&v, &ret, value.span)?;
                    Ok(format!("return {};\n", v.code))
                } else if self.ret == Type::Void {
                    Ok("return;\n".into())
                } else {
                    self.fail(*span, "expected a return value")
                }
            }
            Stmt::If { cond, yes, no } => {
                let c = self.expr(cond, Some(&Type::Bool))?;
                self.compatible(&c, &Type::Bool, cond.span)?;
                let yes = self.block(yes, true)?;
                let no = self.block(no, true)?;
                Ok(format!("if ({}) {{\n{yes}}} else {{\n{no}}}\n", c.code))
            }
            Stmt::While { cond, body } => {
                let c = self.expr(cond, Some(&Type::Bool))?;
                self.compatible(&c, &Type::Bool, cond.span)?;
                self.loop_depth += 1;
                let body = self.block(body, true)?;
                self.loop_depth -= 1;
                Ok(format!("while ({}) {{\n{body}}}\n", c.code))
            }
            Stmt::Break(span) | Stmt::Continue(span) => {
                if self.loop_depth == 0 {
                    return self.fail(*span, "break/continue requires a while loop");
                }
                Ok(if matches!(stmt, Stmt::Break(..)) {
                    "break;\n"
                } else {
                    "continue;\n"
                }
                .into())
            }
        }
    }
    fn expr(&mut self, expr: &Expr, expected: Option<&Type>) -> Result<Value, String> {
        let mut lvalue = false;
        let (code, ty) = match &expr.kind {
            ExprKind::Number(n) => {
                let is_float = !n.starts_with("0x")
                    && !n.starts_with("0X")
                    && (n.contains('.') || n.contains('e') || n.contains('E'));
                let ty = expected
                    .filter(|t| t.numeric())
                    .cloned()
                    .unwrap_or_else(|| {
                        if is_float {
                            Type::Float(64)
                        } else {
                            Type::i64()
                        }
                    });
                if is_float && ty.integer() {
                    return self.fail(
                        expr.span,
                        "floating literal needs a floating type or explicit cast",
                    );
                }
                if is_float {
                    let number = n.parse::<f64>().map_err(|_| {
                        error(
                            &self.program.modules[self.module].path,
                            expr.span,
                            "invalid floating literal",
                        )
                    })?;
                    if !number.is_finite()
                        || (ty == Type::Float(32) && !(number as f32).is_finite())
                    {
                        return self.fail(expr.span, "floating literal is out of range");
                    }
                    (format!("(({})({n}))", c_type(&ty)), ty)
                } else {
                    let number = if n.starts_with("0x") || n.starts_with("0X") {
                        u128::from_str_radix(&n[2..], 16)
                    } else {
                        n.parse::<u128>()
                    }
                    .map_err(|_| {
                        error(
                            &self.program.modules[self.module].path,
                            expr.span,
                            "integer literal is out of range",
                        )
                    })?;
                    let max = match ty {
                        Type::Int { bits, signed } => (1u128 << (bits - u8::from(signed))) - 1,
                        Type::Size { signed } => (1u128 << (usize::BITS - u32::from(signed))) - 1,
                        Type::Float(_) => u64::MAX as u128,
                        _ => unreachable!(),
                    };
                    if number > max {
                        return self.fail(
                            expr.span,
                            format!("literal is out of range for {}", ty.name()),
                        );
                    }
                    (format!("(({})UINT64_C({number}))", c_type(&ty)), ty)
                }
            }
            ExprKind::String(s) => (c_string(s), Type::Str),
            ExprKind::Bool(b) => (if *b { "true" } else { "false" }.into(), Type::Bool),
            ExprKind::Name(names) if names.as_slice() == ["null"] => {
                let ty = expected
                    .filter(|t| matches!(t, Type::Ptr(_) | Type::Str))
                    .cloned()
                    .unwrap_or_else(|| Type::Ptr(Box::new(Type::Void)));
                (format!("(({})0)", c_type(&ty)), ty)
            }
            ExprKind::Name(names) => {
                if names.len() != 1 {
                    return self.fail(expr.span, "module members are functions in v0.1");
                }
                let v = self.variable(&names[0]).ok_or_else(|| {
                    error(
                        &self.program.modules[self.module].path,
                        expr.span,
                        format!("unknown variable '{}'", names[0]),
                    )
                })?;
                lvalue = true;
                (v.name.clone(), v.ty.clone())
            }
            ExprKind::Unary(op, inner) => {
                // Parse the magnitude separately so the minimum signed integer is representable.
                if op == "-" {
                    if let ExprKind::Number(n) = &inner.kind {
                        let target = expected.cloned().unwrap_or_else(Type::i64);
                        let signed = match target {
                            Type::Int { bits, signed: true } => {
                                Some((bits, format!("INT{bits}_MIN")))
                            }
                            Type::Size { signed: true } => {
                                Some((usize::BITS as u8, "INTPTR_MIN".into()))
                            }
                            _ => None,
                        };
                        if let Some((bits, minimum)) = signed {
                            let magnitude = if n.starts_with("0x") || n.starts_with("0X") {
                                u128::from_str_radix(&n[2..], 16).ok()
                            } else {
                                n.parse::<u128>().ok()
                            };
                            if magnitude == Some(1u128 << (bits - 1)) {
                                return Ok(Value {
                                    code: minimum,
                                    ty: target,
                                    lvalue: false,
                                });
                            }
                        }
                    }
                }
                let v = self.expr(
                    inner,
                    if op == "&" || op == "*" {
                        None
                    } else {
                        expected
                    },
                )?;
                match op.as_str() {
                    "&" if v.lvalue && !matches!(v.ty, Type::Array(..)) => {
                        (format!("(&{})", v.code), Type::Ptr(Box::new(v.ty)))
                    }
                    "*" => {
                        if let Type::Ptr(t) = &v.ty {
                            if **t == Type::Void {
                                return self.fail(
                                    expr.span,
                                    "cast *void to an element pointer before dereferencing",
                                );
                            }
                            lvalue = true;
                            (format!("(*{})", v.code), (**t).clone())
                        } else {
                            return self.fail(expr.span, "dereference requires a pointer");
                        }
                    }
                    "!" if v.ty == Type::Bool => (format!("(!{})", v.code), Type::Bool),
                    "~" if v.ty.integer() => (format!("(({})(~{}))", c_type(&v.ty), v.code), v.ty),
                    "+" | "-" if v.ty.numeric() => {
                        (format!("(({})({op}{}))", c_type(&v.ty), v.code), v.ty)
                    }
                    _ => {
                        return self.fail(
                            expr.span,
                            format!("invalid unary '{op}' for {}", v.ty.name()),
                        )
                    }
                }
            }
            ExprKind::Binary(op, left, right) => {
                let comparison = ["==", "!=", "<", "<=", ">", ">="].contains(&op.as_str());
                let logic = op == "&&" || op == "||";
                let context = if comparison {
                    None
                } else {
                    expected.filter(|t| t.numeric() || **t == Type::Bool)
                };
                let (a, b) = if literal(left) && !literal(right) {
                    let b = self.expr(right, context)?;
                    let a = self.expr(left, Some(&b.ty))?;
                    (a, b)
                } else {
                    let a = self.expr(left, context)?;
                    let right_type = if matches!(a.ty, Type::Ptr(_)) && (op == "+" || op == "-") {
                        None
                    } else {
                        Some(&a.ty)
                    };
                    let b = self.expr(right, right_type)?;
                    (a, b)
                };
                let ty = if logic {
                    self.compatible(&a, &Type::Bool, left.span)?;
                    self.compatible(&b, &Type::Bool, right.span)?;
                    Type::Bool
                } else if matches!(a.ty, Type::Ptr(_)) && (op == "+" || op == "-") {
                    if matches!(a.ty, Type::Ptr(ref inner) if **inner == Type::Void) {
                        return self.fail(expr.span, "pointer arithmetic requires an element type");
                    }
                    if b.ty.integer() {
                        a.ty.clone()
                    } else if op == "-" && a.ty == b.ty {
                        Type::Size { signed: true }
                    } else {
                        return self.fail(expr.span, "pointer arithmetic requires an integer offset or matching pointer subtraction");
                    }
                } else {
                    self.compatible(&b, &a.ty, right.span)?;
                    if comparison {
                        if !a.ty.scalar() {
                            return self.fail(expr.span, "comparison requires scalar values");
                        }
                        if a.ty == Type::Str {
                            return self.fail(expr.span, "compare string contents through C strcmp; str equality is not defined");
                        }
                        if a.ty == Type::Bool && op != "==" && op != "!=" {
                            return self.fail(expr.span, "bool supports == and !=");
                        }
                        Type::Bool
                    } else {
                        if !a.ty.numeric() {
                            return self.fail(expr.span, "arithmetic requires numeric operands");
                        }
                        if ["%", "&", "|", "^", "<<", ">>"].contains(&op.as_str())
                            && !a.ty.integer()
                        {
                            return self.fail(expr.span, "operator requires integers");
                        }
                        a.ty.clone()
                    }
                };
                // Only the low 32 bits are observable here. Lower unsigned
                // modular arithmetic before the mask so native SIMD backends
                // can use 32-bit multiplies instead of emulating 64-bit ones.
                // Division, right shifts and casts retain full-width evaluation.
                // SSE4.1 provides native 32-bit SIMD multiplication. Preserve
                // the original expression on other targets, including SSE2.
                let masked = if op == "&"
                    && ty
                        == (Type::Int {
                            bits: 64,
                            signed: false,
                        }) {
                    if low32_mask(right) {
                        Some(self.low32(left, &ty)?)
                    } else if low32_mask(left) {
                        Some(self.low32(right, &ty)?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let code = if let Some(code) = masked {
                    format!(
                        "DEV_LOW32_SELECT(((uint64_t)({code})), ((uint64_t)({} & {})))",
                        a.code, b.code
                    )
                } else {
                    format!("(({})({} {op} {}))", c_type(&ty), a.code, b.code)
                };
                (code, ty)
            }
            ExprKind::Cast(inner, ty) => {
                valid_type(ty, false)
                    .map_err(|e| error(&self.program.modules[self.module].path, expr.span, e))?;
                let v = self.expr(inner, None)?;
                if !v.ty.scalar() || !ty.scalar() {
                    return self.fail(expr.span, "cast requires scalar types");
                }
                let from_pointer = matches!(v.ty, Type::Ptr(_) | Type::Str);
                let to_pointer = matches!(ty, Type::Ptr(_) | Type::Str);
                if (from_pointer && !(to_pointer || ty.integer() || *ty == Type::Bool))
                    || (to_pointer && !(from_pointer || v.ty.integer()))
                {
                    return self.fail(
                        expr.span,
                        "pointer casts require another pointer or integer",
                    );
                }
                if matches!(v.ty, Type::Str) && matches!(ty, Type::Ptr(_)) {
                    return self.fail(
                        expr.span,
                        "str is read-only; use a writable byte array for mutable memory",
                    );
                }
                (format!("(({})({}))", c_type(ty), v.code), ty.clone())
            }
            ExprKind::Index(inner, index) => {
                let v = self.expr(inner, None)?;
                let i = self.expr(index, None)?;
                if !i.ty.integer() {
                    return self.fail(index.span, "index requires an integer");
                }
                let (ty, writable) = match v.ty {
                    Type::Array(t, n) => {
                        if let ExprKind::Number(s) = &index.kind {
                            let number = if s.starts_with("0x") || s.starts_with("0X") {
                                usize::from_str_radix(&s[2..], 16).ok()
                            } else {
                                s.parse::<usize>().ok()
                            };
                            if number.is_some_and(|i| i >= n) {
                                return self
                                    .fail(index.span, "constant array index is out of bounds");
                            }
                        }
                        (*t, v.lvalue)
                    }
                    Type::Ptr(t) if *t != Type::Void => (*t, true),
                    Type::Str => (
                        Type::Int {
                            bits: 8,
                            signed: false,
                        },
                        false,
                    ),
                    _ => {
                        return self.fail(
                            expr.span,
                            "indexing requires an array, element pointer, or str",
                        )
                    }
                };
                if matches!(inner.kind, ExprKind::Array(..)) {
                    return self.fail(expr.span, "bind the array to a variable before indexing");
                }
                lvalue = writable;
                let access = format!("({}[{}])", v.code, i.code);
                (
                    if writable {
                        access
                    } else {
                        format!("(({}){access})", c_type(&ty))
                    },
                    ty,
                )
            }
            ExprKind::Array(values) => {
                if values.is_empty() {
                    return self.fail(expr.span, "array cannot be empty");
                }
                let inner_expected = match expected {
                    Some(Type::Array(t, n)) => {
                        if *n != values.len() {
                            return self
                                .fail(expr.span, "array initializer length does not match type");
                        }
                        Some(t.as_ref())
                    }
                    _ => None,
                };
                let first = self.expr(&values[0], inner_expected)?;
                if !first.ty.scalar() {
                    return self.fail(expr.span, "array elements require scalar values");
                }
                let mut codes = vec![first.code];
                for value in &values[1..] {
                    let v = self.expr(value, Some(&first.ty))?;
                    self.compatible(&v, &first.ty, value.span)?;
                    codes.push(v.code);
                }
                (
                    format!("{{ {} }}", codes.join(", ")),
                    Type::Array(Box::new(first.ty), values.len()),
                )
            }
            ExprKind::Call(names, args) => return self.call(names, args, expr.span),
        };
        Ok(Value { code, ty, lvalue })
    }
    fn low32(&mut self, expr: &Expr, wide: &Type) -> Result<String, String> {
        if let ExprKind::Binary(op, left, right) = &expr.kind {
            if ["+", "-", "*", "&", "|", "^"].contains(&op.as_str()) {
                let left = self.low32(left, wide)?;
                let right = self.low32(right, wide)?;
                return Ok(format!("((uint32_t)({left} {op} {right}))"));
            }
        }
        let value = self.expr(expr, Some(wide))?;
        Ok(format!("((uint32_t)({}))", value.code))
    }
    fn call(&mut self, names: &[String], args: &[Expr], span: Span) -> Result<Value, String> {
        if names.len() == 1 {
            match names[0].as_str() {
                "print" => {
                    if args.len() != 1 {
                        return self.fail(span, "print expects one value");
                    }
                    if !self.hosted {
                        return self.fail(span, "print is unavailable in freestanding mode; call your device's C driver instead");
                    }
                    let v = self.expr(&args[0], None)?;
                    self.uses_print = true;
                    let code = match v.ty {
                        Type::Str => format!("((void)printf(\"%s\\n\", {}))", v.code),
                        Type::Bool => format!("((void)puts({} ? \"true\" : \"false\"))", v.code),
                        Type::Int { signed: true, .. } | Type::Size { signed: true } => {
                            format!("((void)printf(\"%lld\\n\", (long long)({})))", v.code)
                        }
                        Type::Int { signed: false, .. } | Type::Size { signed: false } => format!(
                            "((void)printf(\"%llu\\n\", (unsigned long long)({})))",
                            v.code
                        ),
                        Type::Float(_) => {
                            format!("((void)printf(\"%.17g\\n\", (double)({})))", v.code)
                        }
                        Type::Ptr(_) => format!("((void)printf(\"%p\\n\", (void *)({})))", v.code),
                        _ => {
                            return self
                                .fail(span, "print supports strings, numbers, bools and pointers")
                        }
                    };
                    return Ok(Value {
                        code,
                        ty: Type::Void,
                        lvalue: false,
                    });
                }
                "sizeof" => {
                    if args.len() != 1 {
                        return self
                            .fail(span, "sizeof expects one expression (it is not evaluated)");
                    }
                    let v = self.expr(&args[0], None)?;
                    if v.ty == Type::Void {
                        return self.fail(span, "sizeof cannot accept void");
                    }
                    if matches!(args[0].kind, ExprKind::Array(..)) {
                        return self.fail(span, "bind array literals before using sizeof");
                    }
                    let operand = if matches!(v.ty, Type::Array(..)) {
                        v.code
                    } else {
                        c_type(&v.ty)
                    };
                    return Ok(Value {
                        code: format!("((uintptr_t)sizeof({operand}))"),
                        ty: Type::Size { signed: false },
                        lvalue: false,
                    });
                }
                "volatile_load" | "volatile_store" => {
                    let store = names[0] == "volatile_store";
                    if args.len() != if store { 2 } else { 1 } {
                        return self.fail(span, "volatile_load expects a pointer; volatile_store expects a pointer and value");
                    }
                    let p = self.expr(&args[0], None)?;
                    let inner = match p.ty {
                        Type::Ptr(t) if t.numeric() || *t == Type::Bool => *t,
                        _ => {
                            return self.fail(
                                span,
                                "volatile access requires a pointer to a number or bool",
                            )
                        }
                    };
                    let access = format!("(*(volatile {} *)({}))", c_type(&inner), p.code);
                    if store {
                        let v = self.expr(&args[1], Some(&inner))?;
                        self.compatible(&v, &inner, args[1].span)?;
                        return Ok(Value {
                            code: format!("((void)({access} = {}))", v.code),
                            ty: Type::Void,
                            lvalue: false,
                        });
                    }
                    return Ok(Value {
                        code: access,
                        ty: inner,
                        lvalue: false,
                    });
                }
                _ => {}
            }
        }
        let signature = self.program.resolve(self.module, names, span)?.clone();
        if signature.params.len() != args.len() {
            return self.fail(
                span,
                format!(
                    "{} expects {} arguments, found {}",
                    names.join("."),
                    signature.params.len(),
                    args.len()
                ),
            );
        }
        let mut codes = Vec::new();
        for (arg, ty) in args.iter().zip(&signature.params) {
            let v = self.expr(arg, Some(ty))?;
            self.compatible(&v, ty, arg.span)?;
            codes.push(v.code);
        }
        Ok(Value {
            code: format!("{}({})", signature.c_name, codes.join(", ")),
            ty: signature.ret,
            lvalue: false,
        })
    }
}
fn low32_mask(expr: &Expr) -> bool {
    if let ExprKind::Number(number) = &expr.kind {
        let value = if number.starts_with("0x") || number.starts_with("0X") {
            u64::from_str_radix(&number[2..], 16).ok()
        } else {
            number.parse::<u64>().ok()
        };
        return value == Some(u32::MAX as u64);
    }
    false
}
fn literal(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::Number(_) => true,
        ExprKind::Unary(op, e) if op == "+" || op == "-" => literal(e),
        ExprKind::Name(n) => n.as_slice() == ["null"],
        _ => false,
    }
}
pub fn c_string(value: &str) -> String {
    let mut out = String::from("\"");
    for byte in value.bytes() {
        match byte {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'?' => out.push_str("\\?"),
            32..=126 => out.push(byte as char),
            _ => out.push_str(&format!("\\{byte:03o}")),
        }
    }
    out.push('"');
    out
}
