use crate::ast::*;
use crate::lexer::{lex, Kind, Token};
use std::path::PathBuf;

pub fn parse(path: PathBuf, source: &str) -> Result<Module, String> {
    let tokens = lex(source).map_err(|(span, msg)| error(&path, span, msg))?;
    Parser {
        tokens,
        at: 0,
        path,
        constraints: vec![],
        object_literals: false,
    }
    .module()
}

struct Parser {
    object_literals: bool,
    constraints: Vec<(String, Vec<String>)>,
    tokens: Vec<Token>,
    at: usize,
    path: PathBuf,
}
impl Parser {
    fn token(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn bump(&mut self) -> Token {
        let t = self.token().clone();
        if t.kind != Kind::Eof {
            self.at += 1;
        }
        t
    }
    fn is(&self, text: &str) -> bool {
        matches!(&self.token().kind, Kind::Word(s) | Kind::Symbol(s) if s == text)
    }
    fn eat(&mut self, text: &str) -> bool {
        if self.is(text) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn fail<T>(&self, msg: impl AsRef<str>) -> Result<T, String> {
        Err(error(&self.path, self.token().span, msg))
    }
    fn expect(&mut self, text: &str) -> Result<(), String> {
        if self.eat(text) {
            Ok(())
        } else {
            self.fail(format!("expected '{text}'"))
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.bump() {
            Token {
                kind: Kind::Word(s),
                span,
            } => {
                if [
                    "fn", "let", "return", "if", "else", "while", "break", "continue", "use", "as",
                    "extern", "export", "true", "false", "null", "and", "or", "not", "for", "in",
                    "struct", "enum", "match", "unsafe", "trait", "const", "async",
                ]
                .contains(&s.as_str())
                {
                    Err(error(&self.path, span, format!("'{s}' is reserved")))
                } else {
                    Ok(s)
                }
            }
            t => Err(error(&self.path, t.span, "expected a name")),
        }
    }
    fn separators(&mut self) {
        while matches!(self.token().kind, Kind::Newline) || self.is(";") {
            self.bump();
        }
    }
    fn end(&mut self) -> Result<(), String> {
        if self.is("}") || self.token().kind == Kind::Eof {
            return Ok(());
        }
        if matches!(self.token().kind, Kind::Newline) || self.is(";") {
            self.separators();
            Ok(())
        } else {
            self.fail("expected a newline or ';' between statements")
        }
    }
    fn module(mut self) -> Result<Module, String> {
        let mut imports = Vec::new();
        let mut traits = vec![];
        let mut functions = Vec::new();
        let mut statements = Vec::new();
        let mut definitions = Vec::new();
        self.separators();
        while self.token().kind != Kind::Eof {
            let span = self.token().span;
            if self.eat("trait") {
                let name = self.ident()?;
                self.expect("{")?;
                self.separators();
                let mut methods = vec![];
                while !self.eat("}") {
                    let span = self.token().span;
                    self.expect("fn")?;
                    let method = self.ident()?;
                    self.expect("(")?;
                    let mut params = vec![];
                    while !self.eat(")") {
                        let n = self.ident()?;
                        self.eat(":");
                        params.push((n, self.ty()?));
                        if !self.eat(",") {
                            self.expect(")")?;
                            break;
                        }
                    }
                    self.eat("->");
                    let ret = if self.is(";") || matches!(self.token().kind, Kind::Newline) {
                        Type::Void
                    } else {
                        self.ty()?
                    };
                    self.end()?;
                    methods.push(Function {
                        constraints: vec![],
                        variadic: false,
                        name: method,
                        generics: vec![],
                        main_default: false,
                        params,
                        ret,
                        body: None,
                        exported: false,
                        span,
                    });
                    self.separators();
                }
                traits.push(TraitDefinition {
                    name,
                    methods,
                    span,
                });
            } else if self.eat("use") {
                let (path, default_alias) = match self.bump() {
                    Token {
                        kind: Kind::Word(s),
                        ..
                    } => (format!("{s}.dev"), s),
                    Token {
                        kind: Kind::String(s),
                        ..
                    } => {
                        let alias = std::path::Path::new(&s)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        (s, alias)
                    }
                    t => {
                        return Err(error(
                            &self.path,
                            t.span,
                            "expected module name or quoted .dev path",
                        ))
                    }
                };
                let alias = if self.eat("as") {
                    self.ident()?
                } else {
                    default_alias
                };
                if alias.is_empty()
                    || !alias.chars().enumerate().all(|(i, c)| {
                        c == '_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit())
                    })
                {
                    return Err(error(
                        &self.path,
                        span,
                        "invalid module alias; use 'as name'",
                    ));
                }
                imports.push(Import { path, alias, span });
                self.end()?;
            } else if self.is("struct") || self.is("enum") {
                let enumeration = self.eat("enum");
                if !enumeration {
                    self.expect("struct")?;
                }
                let name = self.ident()?;
                let generics = self.generic_names()?;
                let constraints = std::mem::take(&mut self.constraints);
                self.separators();
                self.expect("{")?;
                self.separators();
                let mut fields = vec![];
                let mut variants = vec![];
                while !self.eat("}") {
                    let field = self.ident()?;
                    if enumeration {
                        let mut payload = vec![];
                        if self.eat("(") {
                            while !self.eat(")") {
                                payload.push(self.ty()?);
                                if !self.eat(",") {
                                    self.expect(")")?;
                                    break;
                                }
                            }
                        }
                        variants.push((field, payload));
                    } else {
                        self.eat(":");
                        fields.push((field, self.ty()?));
                    }
                    if self.eat(",") {
                        self.separators();
                    } else {
                        self.end()?;
                    }
                    self.separators();
                }
                definitions.push(TypeDefinition {
                    constraints,
                    name,
                    generics,
                    fields,
                    variants,
                    enumeration,
                    span,
                });
            } else if self.is("fn") || self.is("extern") || self.is("export") || self.is("async") {
                let asynchronous = self.eat("async");
                let external = self.eat("extern");
                let exported = if !external { self.eat("export") } else { false };
                self.expect("fn")?;
                let mut name = self.ident()?;
                if self.eat(".") {
                    let method = self.ident()?;
                    if external || exported {
                        return self.fail("struct methods cannot be extern or exported");
                    }
                    name = format!("method_{name}_{method}");
                }
                let generics = self.generic_names()?;
                let constraints = std::mem::take(&mut self.constraints);
                self.expect("(")?;
                let mut params = Vec::new();
                let mut variadic = false;
                while !self.eat(")") {
                    if self.eat("...") {
                        if !external || params.is_empty() {
                            return self.fail(
                                "variadic declarations require extern and a fixed parameter",
                            );
                        }
                        variadic = true;
                        self.expect(")")?;
                        break;
                    }
                    let param = self.ident()?;
                    self.eat(":");
                    let ty = self.ty()?;
                    params.push((param, ty));
                    if !self.eat(",") {
                        self.expect(")")?;
                        break;
                    }
                }
                self.eat("->");
                let mut ret = if self.is("{")
                    || matches!(self.token().kind, Kind::Newline | Kind::Eof)
                    || self.is(";")
                {
                    if name == "main" && !external {
                        Type::i32()
                    } else {
                        Type::Void
                    }
                } else {
                    self.ty()?
                };
                let mut body = if external {
                    self.end()?;
                    None
                } else {
                    Some(self.block()?)
                };
                if asynchronous {
                    if external || exported {
                        return self.fail("async functions cannot cross C ABI");
                    }
                    let closure = Expr {
                        span,
                        kind: ExprKind::Closure(vec![], ret.clone(), body.take().unwrap()),
                    };
                    body = Some(vec![Stmt::Return(
                        Some(Expr {
                            span,
                            kind: ExprKind::Call(vec!["spawn".into()], vec![closure]),
                        }),
                        span,
                    )]);
                    ret = Type::Named(vec!["Task".into()], vec![ret]);
                }
                functions.push(Function {
                    constraints,
                    variadic,
                    main_default: name == "main",
                    name,
                    generics,
                    params,
                    ret,
                    body,
                    exported,
                    span,
                });
            } else {
                statements.push(self.stmt()?);
            }
            self.separators();
        }
        if self.object_literals {
            imports.push(Import {
                path: "std/json".into(),
                alias: "$json".into(),
                span: Span { line: 1, col: 1 },
            });
        }
        Ok(Module {
            traits,
            path: self.path,
            imports,
            functions,
            statements,
            definitions,
            concrete_types: vec![],
        })
    }
    fn ty(&mut self) -> Result<Type, String> {
        if self.is("fn") || self.is("callback") {
            let callback = self.eat("callback");
            if !callback {
                self.expect("fn")?;
            }
            self.expect("(")?;
            let mut params = vec![];
            while !self.eat(")") {
                params.push(self.ty()?);
                if !self.eat(",") {
                    self.expect(")")?;
                    break;
                }
            }
            self.eat("->");
            let ret = self.ty()?;
            return Ok(if callback {
                Type::Callback(params, Box::new(ret))
            } else {
                Type::Function(params, Box::new(ret))
            });
        }
        if self.eat("*") {
            return Ok(Type::Ptr(Box::new(self.ty()?)));
        }
        if self.eat("[") {
            let inner = self.ty()?;
            self.expect(";")?;
            let token = self.bump();
            if let Kind::Word(n) = &token.kind {
                self.expect("]")?;
                return Ok(Type::ArrayConst(Box::new(inner), n.clone()));
            }
            let count = match token.kind {
                Kind::Number(s) => s.parse::<usize>().ok(),
                _ => None,
            }
            .filter(|n| *n > 0 && *n <= 1_000_000)
            .ok_or_else(|| error(&self.path, token.span, "array length must be 1..1000000"))?;
            self.expect("]")?;
            return Ok(Type::Array(Box::new(inner), count));
        }
        let t = self.bump();
        if let Kind::Number(n) = &t.kind {
            return n.parse::<usize>().map(Type::Const).map_err(|_| {
                error(
                    &self.path,
                    t.span,
                    "const argument must be a nonnegative integer",
                )
            });
        }
        let name = if let Kind::Word(s) = t.kind {
            s
        } else {
            return Err(error(&self.path, t.span, "expected a type"));
        };
        match name.as_str() {
            "Ref" => {
                self.expect("<")?;
                let inner = self.ty()?;
                self.close_type()?;
                Ok(Type::Ref(Box::new(inner)))
            }
            "void" => Ok(Type::Void),
            "bool" => Ok(Type::Bool),
            "str" => Ok(Type::Str),
            "usize" => Ok(Type::Size { signed: false }),
            "isize" => Ok(Type::Size { signed: true }),
            "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => Ok(Type::Int {
                bits: name[1..].parse().unwrap(),
                signed: name.starts_with('i'),
            }),
            "f32" => Ok(Type::Float(32)),
            "f64" => Ok(Type::Float(64)),
            _ => {
                let mut names = vec![name];
                while self.eat(".") {
                    names.push(self.ident()?);
                }
                let args = self.type_arguments()?;
                Ok(Type::Named(names, args))
            }
        }
    }
    fn close_type(&mut self) -> Result<(), String> {
        if self.is(">>") || self.is(">=") {
            self.tokens[self.at].kind = Kind::Symbol(if self.is(">=") { "=" } else { ">" }.into());
            Ok(())
        } else {
            self.expect(">")
        }
    }
    fn generic_names(&mut self) -> Result<Vec<String>, String> {
        let mut names = vec![];
        if self.eat("<") {
            loop {
                let constant = self.eat("const");
                let name = self.ident()?;
                names.push(name.clone());
                let mut bounds = vec![];
                if constant {
                    bounds.push("const".into());
                }
                if self.eat(":") {
                    loop {
                        let mut b = self.ident()?;
                        while self.eat(".") {
                            b.push('.');
                            b.push_str(&self.ident()?)
                        }
                        bounds.push(b);
                        if !self.eat("+") {
                            break;
                        }
                    }
                }
                if !bounds.is_empty() {
                    self.constraints.push((name, bounds));
                }

                if !self.eat(",") {
                    self.close_type()?;
                    break;
                }
            }
        }
        Ok(names)
    }
    fn type_arguments(&mut self) -> Result<Vec<Type>, String> {
        let mut args = vec![];
        if self.eat("<") {
            loop {
                args.push(self.ty()?);
                if !self.eat(",") {
                    self.close_type()?;
                    break;
                }
            }
        }
        Ok(args)
    }
    fn type_call_ahead(&self) -> bool {
        let (mut depth, mut brackets) = (0isize, 0usize);
        for (offset, token) in self.tokens[self.at..].iter().enumerate() {
            match &token.kind {
                Kind::Symbol(s) if s == "<" => depth += 1,
                Kind::Symbol(s) if s == ">" || s == ">>" => {
                    depth -= if s == ">>" { 2 } else { 1 };
                    if depth <= 0 {
                        return depth == 0
                            && self.tokens.get(self.at + offset + 1).is_some_and(|t| {
                                t.kind == Kind::Symbol("(".into())
                                    || t.kind == Kind::Symbol(".".into())
                            });
                    }
                }
                Kind::Symbol(s) if s == "[" => brackets += 1,
                Kind::Symbol(s) if s == "]" && brackets > 0 => brackets -= 1,
                Kind::Word(_) | Kind::Number(_) => {}
                Kind::Symbol(s)
                    if ["*", ".", ",", "(", ")", "->"].contains(&s.as_str())
                        || (s == ";" && brackets > 0) => {}
                _ => return false,
            }
        }
        false
    }
    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        self.separators();
        self.expect("{")?;
        self.separators();
        let mut body = Vec::new();
        while !self.eat("}") {
            if self.token().kind == Kind::Eof {
                return self.fail("expected '}' before end of file");
            }
            body.push(self.stmt()?);
            self.separators();
        }
        Ok(body)
    }
    fn pattern(&mut self) -> Result<Pattern, String> {
        let name = self.ident()?;
        if self.eat("(") {
            let mut ps = vec![];
            while !self.eat(")") {
                ps.push(self.pattern()?);
                if !self.eat(",") {
                    self.expect(")")?;
                    break;
                }
            }
            Ok(Pattern::Variant(name, ps))
        } else {
            Ok(Pattern::Bind(name))
        }
    }
    fn stmt(&mut self) -> Result<Stmt, String> {
        let span = self.token().span;
        if self.eat("unsafe") {
            return Ok(Stmt::Unsafe(self.block()?));
        }
        if self.eat("match") {
            let value = self.expr(0)?;
            self.separators();
            self.expect("{")?;
            self.separators();
            let mut arms = vec![];
            while !self.eat("}") {
                let variant = self.ident()?;
                let mut bindings = vec![];
                if self.eat("(") {
                    while !self.eat(")") {
                        bindings.push(self.pattern()?);
                        if !self.eat(",") {
                            self.expect(")")?;
                            break;
                        }
                    }
                }
                let guard = if self.eat("if") {
                    Some(self.expr(0)?)
                } else {
                    None
                };
                self.expect("=>")?;
                let body = self.block()?;
                arms.push(MatchArm {
                    variant,
                    bindings,
                    guard,
                    body,
                });
                self.eat(",");
                self.separators();
            }
            return Ok(Stmt::Match { value, arms });
        }
        if self.eat("let") {
            let name = self.ident()?;
            self.eat(":");
            let ty = if self.is("=") { None } else { Some(self.ty()?) };
            self.expect("=")?;
            let value = self.expr(0)?;
            self.end()?;
            return Ok(Stmt::Let {
                name,
                ty,
                value,
                span,
            });
        }
        if self.eat("return") {
            let value = if self.is("}")
                || self.is(";")
                || matches!(self.token().kind, Kind::Newline | Kind::Eof)
            {
                None
            } else {
                Some(self.expr(0)?)
            };
            self.end()?;
            return Ok(Stmt::Return(value, span));
        }
        if self.eat("if") {
            let cond = self.expr(0)?;
            let yes = self.block()?;
            self.separators();
            let no = if self.eat("else") {
                if self.is("if") {
                    vec![self.stmt()?]
                } else {
                    self.block()?
                }
            } else {
                Vec::new()
            };
            return Ok(Stmt::If { cond, yes, no });
        }
        if self.eat("while") {
            let cond = self.expr(0)?;
            return Ok(Stmt::While {
                cond,
                body: self.block()?,
            });
        }
        if self.eat("for") {
            let name = self.ident()?;
            self.expect("in")?;
            let start = self.expr(0)?;
            self.expect("..")?;
            let end = self.expr(0)?;
            let hidden = format!("$for_end_{}", self.at);
            let counter = format!("$for_counter_{}", self.at);
            let variable = Expr {
                kind: ExprKind::Name(vec![counter.clone()]),
                span,
            };
            let increment = Stmt::Assign {
                target: variable.clone(),
                op: "+=".into(),
                value: Expr {
                    kind: ExprKind::Number("1".into()),
                    span,
                },
            };
            fn continues(stmts: &mut Vec<Stmt>, increment: &Stmt) {
                for stmt in stmts {
                    match stmt {
                        Stmt::Continue(_) => {
                            *stmt = Stmt::Block(vec![increment.clone(), stmt.clone()])
                        }
                        Stmt::If { yes, no, .. } => {
                            continues(yes, increment);
                            continues(no, increment);
                        }
                        Stmt::Unsafe(body) | Stmt::Block(body) => continues(body, increment),
                        Stmt::Match { arms, .. } => {
                            for arm in arms {
                                continues(&mut arm.body, increment);
                            }
                        }
                        _ => {}
                    }
                }
            }
            let mut body = self.block()?;
            continues(&mut body, &increment);
            body.insert(
                0,
                Stmt::Let {
                    name,
                    ty: Some(Type::i64()),
                    value: variable.clone(),
                    span,
                },
            );
            body.push(increment);
            return Ok(Stmt::Block(vec![
                Stmt::Let {
                    name: counter,
                    ty: Some(Type::i64()),
                    value: start,
                    span,
                },
                Stmt::Let {
                    name: hidden.clone(),
                    ty: Some(Type::i64()),
                    value: end,
                    span,
                },
                Stmt::While {
                    cond: Expr {
                        kind: ExprKind::Binary(
                            "<".into(),
                            Box::new(variable),
                            Box::new(Expr {
                                kind: ExprKind::Name(vec![hidden]),
                                span,
                            }),
                        ),
                        span,
                    },
                    body,
                },
            ]));
        }
        if self.eat("break") {
            self.end()?;
            return Ok(Stmt::Break(span));
        }
        if self.eat("continue") {
            self.end()?;
            return Ok(Stmt::Continue(span));
        }
        let target = self.expr(0)?;
        if ["=", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^="]
            .iter()
            .any(|s| self.is(s))
        {
            let op = match self.bump().kind {
                Kind::Symbol(s) => s,
                _ => unreachable!(),
            };
            let value = self.expr(0)?;
            self.end()?;
            Ok(Stmt::Assign { target, op, value })
        } else {
            self.end()?;
            Ok(Stmt::Expr(target))
        }
    }
    fn expr(&mut self, min: u8) -> Result<Expr, String> {
        let token = self.bump();
        let span = token.span;
        let mut left = match token.kind {
            Kind::Number(s) => Expr {
                kind: ExprKind::Number(s),
                span,
            },
            Kind::String(s) => Expr {
                kind: ExprKind::String(s),
                span,
            },
            Kind::Word(s) if s == "true" || s == "false" => Expr {
                kind: ExprKind::Bool(s == "true"),
                span,
            },
            Kind::Word(s) if s == "fn" => {
                self.expect("(")?;
                let mut params = vec![];
                while !self.eat(")") {
                    let n = self.ident()?;
                    self.eat(":");
                    params.push((n, self.ty()?));
                    if !self.eat(",") {
                        self.expect(")")?;
                        break;
                    }
                }
                self.eat("->");
                let ret = if self.is("{") { Type::Void } else { self.ty()? };
                let body = self.block()?;
                Expr {
                    kind: ExprKind::Closure(params, ret, body),
                    span,
                }
            }
            Kind::Word(s) if s == "not" => Expr {
                kind: ExprKind::Unary("!".into(), Box::new(self.expr(12)?)),
                span,
            },
            Kind::Word(s) => {
                let mut names = vec![s];
                while self.eat(".") {
                    names.push(self.ident()?);
                }
                Expr {
                    kind: ExprKind::Name(names),
                    span,
                }
            }
            Kind::Symbol(s) if ["-", "+", "!", "~", "&", "*"].contains(&s.as_str()) => Expr {
                kind: ExprKind::Unary(s, Box::new(self.expr(12)?)),
                span,
            },
            Kind::Symbol(s) if s == "(" => {
                let e = self.expr(0)?;
                self.expect(")")?;
                e
            }
            Kind::Symbol(s) if s == "{" => {
                self.object_literals = true;
                self.separators();
                let mut fields = vec![];
                while !self.eat("}") {
                    let key = match self.bump().kind {
                        Kind::Word(key) | Kind::String(key) => key,
                        _ => return self.fail("expected object key"),
                    };
                    if fields.iter().any(|(name, _)| name == &key) {
                        return self.fail("duplicate object literal key");
                    }
                    self.expect(":")?;
                    self.separators();
                    fields.push((key, self.expr(0)?));
                    self.separators();
                    if !self.eat(",") {
                        self.expect("}")?;
                        break;
                    }
                    self.separators();
                }
                Expr {
                    kind: ExprKind::Object(
                        Type::Named(vec!["$json".into(), "Value".into()], vec![]),
                        fields,
                    ),
                    span,
                }
            }
            Kind::Symbol(s) if s == "[" => {
                let mut values = Vec::new();
                while !self.eat("]") {
                    values.push(self.expr(0)?);
                    if !self.eat(",") {
                        self.expect("]")?;
                        break;
                    }
                }
                Expr {
                    kind: ExprKind::Array(values),
                    span,
                }
            }
            _ => return Err(error(&self.path, span, "expected an expression")),
        };
        loop {
            if min <= 12
                && self.is("<")
                && matches!(left.kind, ExprKind::Name(_))
                && self.type_call_ahead()
            {
                let saved_at = self.at;
                let saved_tokens = self.tokens.clone();
                if let Ok(types) = self.type_arguments() {
                    let mut variant = None;
                    if self.eat(".") {
                        variant = Some(self.ident()?);
                    }
                    if self.eat("(") {
                        let mut args = vec![];
                        while !self.eat(")") {
                            args.push(self.expr(0)?);
                            if !self.eat(",") {
                                self.expect(")")?;
                                break;
                            }
                        }
                        let ExprKind::Name(mut names) = left.kind else {
                            unreachable!()
                        };
                        if let Some(variant) = variant {
                            names.push(variant);
                        }
                        left = Expr {
                            kind: ExprKind::GenericCall(names, types, args),
                            span,
                        };
                        continue;
                    }
                }
                self.at = saved_at;
                self.tokens = saved_tokens;
            }
            if min <= 12 && self.eat(".") {
                let field = self.ident()?;
                if self.eat("(") {
                    let mut args = vec![];
                    while !self.eat(")") {
                        args.push(self.expr(0)?);
                        if !self.eat(",") {
                            self.expect(")")?;
                            break;
                        }
                    }
                    left = Expr {
                        kind: ExprKind::Collection(Box::new(left), field, args),
                        span,
                    };
                    continue;
                }
                left = Expr {
                    kind: ExprKind::Field(Box::new(left), field),
                    span,
                };
                continue;
            }
            if min <= 12 && self.eat("(") {
                let mut args = Vec::new();
                while !self.eat(")") {
                    args.push(self.expr(0)?);
                    if !self.eat(",") {
                        self.expect(")")?;
                        break;
                    }
                }
                left = Expr {
                    kind: match left.kind {
                        ExprKind::Name(names) => ExprKind::Call(names, args),
                        _ => ExprKind::Invoke(Box::new(left), args),
                    },
                    span,
                };
                continue;
            }
            if min <= 12 && self.eat("[") {
                let index = self.expr(0)?;
                self.expect("]")?;
                left = Expr {
                    kind: ExprKind::Index(Box::new(left), Box::new(index)),
                    span,
                };
                continue;
            }
            if min <= 11 && self.eat("as") {
                left = Expr {
                    kind: ExprKind::Cast(Box::new(left), self.ty()?),
                    span,
                };
                continue;
            }
            let op = match &self.token().kind {
                Kind::Symbol(s) | Kind::Word(s) => s.as_str(),
                _ => break,
            };
            let (power, mapped) = match op {
                "||" | "or" => (1, "||"),
                "&&" | "and" => (2, "&&"),
                "|" => (3, "|"),
                "^" => (4, "^"),
                "&" => (5, "&"),
                "==" => (6, "=="),
                "!=" => (6, "!="),
                "<" => (7, "<"),
                "<=" => (7, "<="),
                ">" => (7, ">"),
                ">=" => (7, ">="),
                "<<" => (8, "<<"),
                ">>" => (8, ">>"),
                "+" => (9, "+"),
                "-" => (9, "-"),
                "*" => (10, "*"),
                "/" => (10, "/"),
                "%" => (10, "%"),
                _ => break,
            };
            if power < min {
                break;
            }
            let op = mapped.to_string();
            self.bump();
            let right = self.expr(power + 1)?;
            left = Expr {
                kind: ExprKind::Binary(op, Box::new(left), Box::new(right)),
                span,
            };
        }
        Ok(left)
    }
}
