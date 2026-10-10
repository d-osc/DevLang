use dev_syntax::ast::{Expr, ExprKind, Module, Pattern, Span, Stmt};
use std::collections::HashMap;

#[derive(Default)]
struct Analysis {
    scopes: Vec<HashMap<String, Option<usize>>>,
    bindings: Vec<(String, Span, bool)>,
}
impl Analysis {
    fn bind(&mut self, name: &str, span: Option<Span>) {
        let id = span.map(|span| {
            let id = self.bindings.len();
            self.bindings.push((name.into(), span, false));
            id
        });
        self.scopes.last_mut().unwrap().insert(name.into(), id);
    }
    fn read(&mut self, name: &str) {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                if let Some(id) = id {
                    self.bindings[*id].2 = true;
                }
                break;
            }
        }
    }
    fn pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Bind(name) => self.bind(name, None),
            Pattern::Variant(_, patterns) => {
                for p in patterns {
                    self.pattern(p);
                }
            }
        }
    }
    fn block(&mut self, statements: &[Stmt]) {
        self.scopes.push(HashMap::new());
        self.statements(statements);
        self.scopes.pop();
    }
    fn statements(&mut self, statements: &[Stmt]) {
        for stmt in statements {
            match stmt {
                Stmt::Let {
                    name, value, span, ..
                } => {
                    self.expr(value);
                    self.bind(name, Some(*span));
                }
                Stmt::Assign { target, op, value } => {
                    self.expr(value);
                    // A plain write to a local does not read its value.
                    if op != "=" || !matches!(target.kind, ExprKind::Name(_)) {
                        self.expr(target);
                    }
                }
                Stmt::Expr(e) | Stmt::Return(Some(e), _) => self.expr(e),
                Stmt::Block(body) | Stmt::Unsafe(body) => self.block(body),
                Stmt::If { cond, yes, no } => {
                    self.expr(cond);
                    self.block(yes);
                    self.block(no);
                }
                Stmt::While { cond, body } => {
                    self.expr(cond);
                    self.block(body);
                }
                Stmt::Match { value, arms } => {
                    self.expr(value);
                    for arm in arms {
                        self.scopes.push(HashMap::new());
                        for p in &arm.bindings {
                            self.pattern(p);
                        }
                        if let Some(guard) = &arm.guard {
                            self.expr(guard);
                        }
                        self.statements(&arm.body);
                        self.scopes.pop();
                    }
                }
                Stmt::Return(None, _) | Stmt::Break(_) | Stmt::Continue(_) => {}
            }
        }
    }
    fn expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Name(names) => {
                if let Some(name) = names.first() {
                    self.read(name);
                }
            }
            ExprKind::Call(names, args) | ExprKind::GenericCall(names, _, args) => {
                if let Some(name) = names.first() {
                    self.read(name);
                }
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::Closure(params, _, body) => {
                self.scopes.push(HashMap::new());
                for (name, _) in params {
                    self.bind(name, None);
                }
                self.statements(body);
                self.scopes.pop();
            }
            ExprKind::Callable(names, value, _) => {
                if let Some(name) = names.first() {
                    self.read(name);
                }
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            ExprKind::Unary(_, e)
            | ExprKind::Cast(e, _)
            | ExprKind::Field(e, _)
            | ExprKind::Reference(e)
            | ExprKind::Dereference(e) => self.expr(e),
            ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Invoke(e, args) | ExprKind::Collection(e, _, args) => {
                self.expr(e);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::Array(args)
            | ExprKind::Record(_, args)
            | ExprKind::Enum(_, _, args)
            | ExprKind::Vector(_, args) => {
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::Object(_, fields) => {
                for (_, value) in fields {
                    self.expr(value);
                }
            }
            ExprKind::Number(_)
            | ExprKind::String(_)
            | ExprKind::Bool(_)
            | ExprKind::SizeOf(_)
            | ExprKind::Map(_) => {}
        }
    }
}
pub fn variables(module: &Module) -> Vec<(String, Span)> {
    let mut analysis = Analysis::default();
    analysis.scopes.push(HashMap::new());
    analysis.statements(&module.statements);
    for function in &module.functions {
        if let Some(body) = &function.body {
            analysis.scopes.push(HashMap::new());
            for (name, _) in &function.params {
                analysis.bind(name, None);
            }
            analysis.statements(body);
            analysis.scopes.pop();
        }
    }
    analysis
        .bindings
        .into_iter()
        .filter_map(|(name, span, used)| (!used && !name.starts_with('_')).then_some((name, span)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn unused(source: &str) -> Vec<String> {
        let module = dev_syntax::parser::parse("test.dev".into(), source).unwrap();
        variables(&module)
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    }
    #[test]
    fn ignores_strings_comments_and_writes() {
        assert_eq!(
            unused("fn main() {\nlet x = 1\nx = 2\nprint(\"x\") // x\n}\nmain()"),
            ["x"]
        );
    }
    #[test]
    fn resolves_shadowed_names_and_initializers() {
        assert_eq!(
            unused("fn main() {\nlet x = 1\nif true {\nlet x = 2\nprint(x)\n}\n}\nmain()"),
            ["x"]
        );
        assert!(unused("fn main() {\nlet x = 1\nlet y = x\nprint(y)\n}\nmain()").is_empty());
    }
    #[test]
    fn resolves_parameters_and_global_reads() {
        assert_eq!(
            unused("let x = 1\nfn use_x(x i64) {\nprint(x)\n}\nuse_x(2)"),
            ["x"]
        );
        assert!(unused("let x = 1\nfn main() {\nprint(x)\n}\nmain()").is_empty());
    }
}
