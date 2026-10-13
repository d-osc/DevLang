use crate::ast::*;
use std::collections::HashMap;

pub fn validate(module: &Module, constants: &[Span]) -> Result<(), String> {
    let checker = Checker { path: &module.path, constants };
    let mut globals = HashMap::new();
    for statement in &module.statements {
        if let Stmt::Let { name, span, .. } = statement {
            globals.insert(name.clone(), checker.constant(*span));
        }
    }
    checker.block(&module.statements, &mut HashMap::new())?;
    for function in &module.functions {
        let mut scope = globals.clone();
        for (name, _) in &function.params { scope.insert(name.clone(), false); }
        if let Some(body) = &function.body { checker.block(body, &mut scope)?; }
    }
    Ok(())
}
struct Checker<'a> { path: &'a std::path::Path, constants: &'a [Span] }
impl Checker<'_> {
    fn constant(&self, span: Span) -> bool {
        self.constants.iter().any(|s| s.line == span.line && s.col == span.col)
    }
    fn root(expr: &Expr) -> Option<&str> {
        match &expr.kind {
            ExprKind::Name(names) => names.first().map(String::as_str),
            ExprKind::Field(base, _) | ExprKind::Index(base, _) => Self::root(base),
            _ => None,
        }
    }
    fn writable(&self, expr: &Expr, scope: &HashMap<String, bool>) -> Result<(), String> {
        if let Some(name) = Self::root(expr) {
            if scope.get(name) == Some(&true) {
                return Err(format!("{}:{}:{}: cannot modify const binding '{}'", self.path.display(), expr.span.line, expr.span.col, name));
            }
        }
        Ok(())
    }
    fn pattern(pattern: &Pattern, scope: &mut HashMap<String, bool>) {
        match pattern {
            Pattern::Bind(name) => { scope.insert(name.clone(), false); }
            Pattern::Variant(_, children) => for child in children { Self::pattern(child, scope); },
        }
    }
    fn block(&self, body: &[Stmt], scope: &mut HashMap<String, bool>) -> Result<(), String> {
        for stmt in body {
            match stmt {
                Stmt::Let { name, value, span, .. } => {
                    self.expr(value, scope)?;
                    scope.insert(name.clone(), self.constant(*span));
                }
                Stmt::Assign { target, value, .. } => {
                    self.writable(target, scope)?;
                    self.expr(target, scope)?; self.expr(value, scope)?;
                }
                Stmt::Expr(expr) | Stmt::Return(Some(expr), _) => self.expr(expr, scope)?,
                Stmt::Unsafe(body) | Stmt::Block(body) => self.block(body, &mut scope.clone())?,
                Stmt::If { cond, yes, no } => {
                    self.expr(cond, scope)?;
                    self.block(yes, &mut scope.clone())?; self.block(no, &mut scope.clone())?;
                }
                Stmt::While { cond, body } => { self.expr(cond, scope)?; self.block(body, &mut scope.clone())?; }
                Stmt::Match { value, arms } => {
                    self.expr(value, scope)?;
                    for arm in arms {
                        let mut local = scope.clone();
                        for pattern in &arm.bindings { Self::pattern(pattern, &mut local); }
                        if let Some(guard) = &arm.guard { self.expr(guard, &local)?; }
                        self.block(&arm.body, &mut local)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn expr(&self, expr: &Expr, scope: &HashMap<String, bool>) -> Result<(), String> {
        if let ExprKind::Call(names, _) | ExprKind::GenericCall(names, _, _) = &expr.kind {
            if names.len() > 1 && matches!(names.last().map(String::as_str), Some("push" | "pop" | "set" | "insert" | "remove" | "clear" | "resize" | "shift" | "unshift" | "reverse" | "sort")) {
                let receiver = Expr { kind: ExprKind::Name(vec![names[0].clone()]), span: expr.span };
                self.writable(&receiver, scope)?;
            }
        }
        match &expr.kind {
            ExprKind::Repeat(value, _) => self.expr(value, scope)?,
            ExprKind::Spread(items) => for (_,value) in items {self.expr(value,scope)?;},
            ExprKind::Conditional(cond, yes, no, _) => {
                self.expr(cond, scope)?; self.expr(yes, scope)?; self.expr(no, scope)?;
            }
            ExprKind::Closure(params, _, body) => {
                let mut local = scope.clone();
                for (name, _) in params { local.insert(name.clone(), false); }
                self.block(body, &mut local)?;
            }
            ExprKind::Collection(base, method, args) => {
                if matches!(method.as_str(), "push" | "pop" | "set" | "insert" | "remove" | "clear" | "resize" | "shift" | "unshift" | "reverse" | "sort") { self.writable(base, scope)?; }
                self.expr(base, scope)?;
                for arg in args { self.expr(arg, scope)?; }
            }
            ExprKind::Unary(op, base) => {
                // A writable raw address would bypass binding immutability.
                if op == "&" { self.writable(base, scope)?; }
                self.expr(base, scope)?;
            }
            ExprKind::Field(base, _) | ExprKind::Cast(base, _) | ExprKind::Reference(base) | ExprKind::Dereference(base) => self.expr(base, scope)?,
            ExprKind::Binary(_, left, right) | ExprKind::Index(left, right) => { self.expr(left, scope)?; self.expr(right, scope)?; }
            ExprKind::Object(_, fields) => for (_, value) in fields { self.expr(value, scope)?; },
            ExprKind::Call(_, args) | ExprKind::GenericCall(_, _, args) | ExprKind::Array(args) | ExprKind::Record(_, args) | ExprKind::Enum(_, _, args) | ExprKind::Vector(_, args) => for arg in args { self.expr(arg, scope)?; },
            ExprKind::Invoke(base, args) => { self.expr(base, scope)?; for arg in args { self.expr(arg, scope)?; } }
            ExprKind::Callable(_, Some(base), _) => self.expr(base, scope)?,
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;
    use std::path::PathBuf;
    #[test]
    fn rejects_mutations_and_preserves_scope() {
        for source in [
            "const x=1\nx=2", "const x=1\nx+=2",
            "const x={a:1}\nx.a=2", "const x={a:[1]}\nx.a[0]=2",
            "const {a}= {a:1}\na=2", "const {a:b}= {a:1}\nb=2",
            "const x=Vec<i64>()\nx.push(1)",
            "const x=1\nlet f=fn():void { x=2 }",
            r#"print("${fn():i64 { const x=1; x=2; return x }}")"#,
        ] {
            let err=parse(PathBuf::from("test.dev"), source).unwrap_err();
            assert!(err.contains("cannot modify const"), "{source}: {err}");
        }
        parse(PathBuf::from("test.dev"), "const x=1\nif true { let x=2; x=3 }\nprint(x)").unwrap();
        parse(PathBuf::from("test.dev"), "let x=1\nif true {const x=2}\nx=3").unwrap();
    }
}

pub(crate) fn immutable_receiver(path: &std::path::Path, body: &[Stmt]) -> Result<(), String> {
    let checker = Checker { path, constants: &[] };
    let mut scope = HashMap::from([("this".to_string(), true)]);
    checker.block(body, &mut scope)
}
