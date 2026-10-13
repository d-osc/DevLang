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
        array_newline_end: false,
        annotations: vec![],
        constants: vec![],
    }
    .module()
}

/// Ranges containing only the separator between a declared name/parameter list and its type.
pub fn type_annotations(path: PathBuf, source: &str) -> Result<Vec<(Span, Span)>, String> {
    let tokens = lex(source).map_err(|(span, msg)| error(&path, span, msg))?;
    let mut parser = Parser {
        tokens,
        at: 0,
        path,
        constraints: vec![],
        object_literals: false,
        array_newline_end: false,
        annotations: vec![],
        constants: vec![],
    };
    parser.module()?;
    Ok(parser.annotations)
}

struct Parser {
    constants: Vec<Span>,
    annotations: Vec<(Span, Span)>,
    object_literals: bool,
    array_newline_end: bool,
    constraints: Vec<(String, Vec<String>)>,
    tokens: Vec<Token>,
    at: usize,
    path: PathBuf,
}
impl Parser {
    // Destructuring expands into declarations in the surrounding scope.
    fn append_statement(body: &mut Vec<Stmt>, statement: Stmt) {
        if let Stmt::Block(entries) = &statement {
            if matches!(entries.first(), Some(Stmt::Let { name, .. }) if name.starts_with("$destructure_")) {
                body.extend(entries.clone());
                return;
            }
        }
        body.push(statement);
    }
    fn type_separator(&mut self, arrow: bool) -> Result<(), String> {
        let previous = self.tokens[self.at - 1].clone();
        let separator = self.eat(":") || (arrow && self.eat("->"));
        if self.is("=")
            || self.is("{")
            || self.is(";")
            || matches!(self.token().kind, Kind::Newline | Kind::Eof)
        {
            return if separator {
                self.fail("expected a type after annotation separator")
            } else {
                Ok(())
            };
        }
        let length = match &previous.kind {
            Kind::Word(s) | Kind::Symbol(s) => s.chars().count(),
            _ => return Ok(()),
        };
        self.annotations.push((
            Span {
                line: previous.span.line,
                col: previous.span.col + length,
            },
            self.token().span,
        ));
        Ok(())
    }
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
                ..
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
    fn module(&mut self) -> Result<Module, String> {
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
                        self.type_separator(false)?;
                        params.push((n, self.ty()?));
                        if !self.eat(",") {
                            self.expect(")")?;
                            break;
                        }
                    }
                    self.type_separator(true)?;
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
                        self.type_separator(false)?;
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
                    self.type_separator(false)?;
                    let ty = self.ty()?;
                    params.push((param, ty));
                    if !self.eat(",") {
                        self.expect(")")?;
                        break;
                    }
                }
                self.type_separator(true)?;
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
                let statement = self.stmt()?;
                Self::append_statement(&mut statements, statement);
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
        let module = Module {
            traits,
            path: self.path.clone(),
            imports,
            functions,
            statements,
            definitions,
            concrete_types: vec![],
        };
        crate::const_check::validate(&module, &self.constants)?;
        Ok(module)
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
            self.type_separator(true)?;
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
                    if ["*", ".", ",", "(", ")", "->", ":"].contains(&s.as_str())
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
            let statement = self.stmt()?;
            Self::append_statement(&mut body, statement);
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
        if self.eat("switch") {
            let hidden = format!("$switch_{}", self.at);
            let value = self.expr(0)?;
            self.separators();
            self.expect("{")?;
            self.separators();
            let mut cases = Vec::new();
            let mut fallback = Vec::new();
            let mut has_default = false;
            while !self.eat("}") {
                if self.eat("default") {
                    if has_default { return self.fail("duplicate switch default"); }
                    has_default = true;
                    self.expect(":")?;
                    fallback = self.block()?;
                } else {
                    if has_default { return self.fail("switch default must be last"); }
                    self.expect("case")?;
                    let mut condition: Option<Expr> = None;
                    loop {
                        let item = self.expr(0)?;
                        let comparison = Expr { span: item.span, kind: ExprKind::Binary(
                            "$switch_eq".into(), Box::new(Expr { kind: ExprKind::Name(vec![hidden.clone()]), span }), Box::new(item),
                        ) };
                        condition = Some(match condition {
                            Some(previous) => Expr { span, kind: ExprKind::Binary("||".into(), Box::new(previous), Box::new(comparison)) },
                            None => comparison,
                        });
                        if !self.eat(",") { break; }
                    }
                    self.expect(":")?;
                    cases.push((condition.unwrap(), self.block()?));
                }
                self.separators();
            }
            for (cond, yes) in cases.into_iter().rev() {
                fallback = vec![Stmt::If { cond, yes, no: fallback }];
            }
            let mut body = vec![Stmt::Let { name: hidden, ty: None, value, span }];
            body.extend(fallback);
            return Ok(Stmt::Block(body));
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
        let constant = self.is("const");
        if self.eat("let") || self.eat("const") {
            if constant { self.constants.push(span); }
            if self.eat("[") {
                let hidden = format!("$destructure_{}", self.at);
                let mut names = Vec::new();
                let mut rest = None;
                self.separators();
                while !self.eat("]") {
                    if self.eat("...") {
                        let rest_span = self.token().span;
                        let name = self.ident()?;
                        if names.iter().any(|(n, _)| n == &name) {return self.fail("duplicate array destructuring binding");}
                        if constant {self.constants.push(rest_span);}
                        rest = Some((name, rest_span, names.len()));
                        self.eat(","); self.separators(); self.expect("]")?; break;
                    }
                    let binding_span = self.token().span;
                    let name = self.ident()?;
                    if names.iter().any(|(n, _)| n == &name) { return self.fail("duplicate array destructuring binding"); }
                    if constant { self.constants.push(binding_span); }
                    names.push((name, binding_span));
                    if self.eat(",") { self.separators(); } else { self.expect("]")?; break; }
                }
                self.expect("=")?;
                let value = self.expr(0)?;
                self.end()?;
                let mut entries = vec![Stmt::Let {name: hidden.clone(), ty: None, value, span}];
                for (index, (name, binding_span)) in names.into_iter().enumerate() {
                    entries.push(Stmt::Let { name, ty: None, span: binding_span, value: Expr {
                        span: binding_span, kind: ExprKind::Index(Box::new(Expr {span, kind: ExprKind::Name(vec![hidden.clone()])}),
                        Box::new(Expr {span, kind: ExprKind::Number(index.to_string())})),
                    }});
                }
                if let Some((name, binding_span, count)) = rest {
                    let base = Expr {span, kind: ExprKind::Name(vec![hidden.clone()])};
                    let end = Expr {span, kind: ExprKind::Collection(Box::new(base.clone()), "len".into(), vec![])};
                    let slice = Expr {span, kind: ExprKind::Collection(Box::new(base), "slice".into(), vec![Expr {span, kind: ExprKind::Number(count.to_string())}, end])};
                    entries.push(Stmt::Let {name, ty: None, span: binding_span, value: Expr {span, kind: ExprKind::Spread(vec![(true,slice)])}});
                }
                return Ok(Stmt::Block(entries));
            }
            if self.eat("{") {
                let hidden = format!("$destructure_{}", self.at);
                let mut fields = Vec::new();
                self.separators();
                while !self.eat("}") {
                    let field_span = self.token().span;
                    let field = self.ident()?;
                    let name = if self.eat(":") { self.ident()? } else { field.clone() };
                    if fields.iter().any(|(_, existing, _)| existing == &name) {
                        return self.fail(format!("duplicate destructuring binding '{name}'"));
                    }
                    if constant { self.constants.push(field_span); }
                    fields.push((field, name, field_span));
                    self.separators();
                    if self.eat(",") { self.separators(); }
                    else if !self.is("}") { return self.fail("expected ',' or '}' in destructuring"); }
                }
                self.expect("=")?;
                let value = self.expr(0)?;
                self.end()?;
                let mut entries = vec![Stmt::Let { name: hidden.clone(), ty: None, value, span }];
                for (field, name, field_span) in fields {
                    entries.push(Stmt::Let {
                        name, ty: None, span: field_span,
                        value: Expr { kind: ExprKind::Field(Box::new(Expr {
                            kind: ExprKind::Name(vec![hidden.clone()]), span: field_span,
                        }), field), span: field_span },
                    });
                }
                return Ok(Stmt::Block(entries));
            }
            let name = self.ident()?;
            self.type_separator(false)?;
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
            let mut start = self.expr(0)?;
            let collection = if self.eat("..") { None } else { Some(start.clone()) };
            let collection_name = format!("$for_items_{}", self.at);
            let end = if collection.is_some() {
                start = Expr {kind: ExprKind::Number("0".into()), span};
                Expr {kind: ExprKind::Cast(Box::new(Expr {kind: ExprKind::Collection(Box::new(Expr {
                    kind: ExprKind::Name(vec![collection_name.clone()]), span }), "$iter_len".into(), vec![]), span}), Type::i64()), span}
            } else { self.expr(0)? };
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
                    ty: if collection.is_some() {None} else {Some(Type::i64())},
                    value: if collection.is_some() {Expr {kind: ExprKind::Index(Box::new(Expr {kind: ExprKind::Name(vec![collection_name.clone()]), span}), Box::new(variable.clone())), span}} else {variable.clone()},
                    span,
                },
            );
            body.push(increment);
            let mut lowered = vec![
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
            ];
            if let Some(value) = collection { lowered.insert(0, Stmt::Let {name: collection_name, ty: None, value, span}); }
            return Ok(Stmt::Block(lowered));
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
    fn expression_arguments(&mut self) -> Result<Vec<Expr>,String> {
        let saved = self.array_newline_end;
        self.array_newline_end = false;
        let mut args = vec![];
        while !self.eat(")") {
            args.push(self.expr(0)?);
            if !self.eat(",") { self.expect(")")?;break; }
        }
        self.array_newline_end = saved;
        Ok(args)
    }
    fn expr(&mut self, min: u8) -> Result<Expr, String> {
        let token = self.bump();
        let span = token.span;
        let mut left = match token.kind {
            Kind::Number(s) => Expr {
                kind: ExprKind::Number(s),
                span,
            },
            Kind::Template(parts) => {
                let mut arguments = Vec::new();
                for (expression, text, offset) in parts {
                    if !expression {
                        arguments.push(Expr {
                            kind: ExprKind::String(text),
                            span,
                        });
                        continue;
                    }
                    let mut tokens = lex(&text).map_err(|(mut p, msg)| {
                        p.line = span.line;
                        p.col += span.col + offset - 1;
                        error(&self.path, p, msg)
                    })?;
                    for token in &mut tokens {
                        token.span.line = span.line;
                        token.end_line = span.line;
                        token.span.col += span.col + offset - 1;
                    }
                    let mut inner = Parser {
                        tokens,
                        at: 0,
                        path: self.path.clone(),
                        constraints: vec![],
                        object_literals: false,
                        array_newline_end: false,
                        annotations: vec![],
                        constants: vec![],
                    };
                    arguments.push(inner.expr(0)?);
                    if inner.token().kind != Kind::Eof {
                        return inner.fail("expected end of interpolation expression");
                    }
                    self.object_literals |= inner.object_literals;
                    self.constants.extend(inner.constants);
                }
                Expr {
                    kind: ExprKind::Call(vec!["$interpolate".into()], arguments),
                    span,
                }
            }
            Kind::String(s) => Expr {
                kind: ExprKind::String(s),
                span,
            },
            Kind::Word(s) if s == "true" || s == "false" => Expr {
                kind: ExprKind::Bool(s == "true"),
                span,
            },
            Kind::Word(s) if s == "new" => {
                let mut names=vec![self.ident()?];
                while self.eat(".") {names.push(self.ident()?);}
                self.expect("(")?;
                let mut args=self.expression_arguments()?;
                let call=if names.as_slice()==["Object"] {vec!["Object".into()]} else {
                    args.insert(0,Expr{span,kind:ExprKind::Name(names)});
                    vec!["Object".into(),"construct".into()]
                };
                Expr{span,kind:ExprKind::Call(call,args)}
            }
            Kind::Word(s) if s == "fn" => {
                self.expect("(")?;
                let mut params = vec![];
                while !self.eat(")") {
                    let n = self.ident()?;
                    self.type_separator(false)?;
                    params.push((n, self.ty()?));
                    if !self.eat(",") {
                        self.expect(")")?;
                        break;
                    }
                }
                self.type_separator(true)?;
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
                let saved = self.array_newline_end; self.array_newline_end = false;
                let e = self.expr(0)?;
                self.expect(")")?; self.array_newline_end = saved;
                e
            }
            Kind::Symbol(s) if s == "{" => {
                self.object_literals = true;
                let saved = self.array_newline_end; self.array_newline_end = false;
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
                    if !self.eat(":") && !self.eat("=") { return self.fail("expected : or = after object key"); }
                    self.separators();
                    fields.push((key, self.expr(0)?));
                    let separated = matches!(self.token().kind, Kind::Newline) || self.is(";");
                    self.separators();
                    if self.eat(",") { self.separators(); }
                    else if self.eat("}") { break; }
                    else if !separated && self.token().span.line <= self.tokens[self.at-1].end_line {
                        return self.fail("expected comma, semicolon or newline between object fields");
                    }
                }
                self.array_newline_end = saved;
                Expr {
                    kind: ExprKind::Object(
                        Type::Named(vec!["$json".into(), "Value".into()], vec![]),
                        fields,
                    ),
                    span,
                }
            }
            Kind::Symbol(s) if s == "[" => {
                let saved = self.array_newline_end; self.array_newline_end = true;
                let mut values = Vec::new();
                let mut spread = Vec::new();
                let mut has_spread = false;
                let mut repeated = None;
                while !self.eat("]") {
                    let expand = self.eat("...");
                    has_spread |= expand;
                    let value = self.expr(0)?;
                    if !expand && values.is_empty() && self.is(";") && matches!(self.tokens.get(self.at+1).map(|t| &t.kind), Some(Kind::Number(_))) && matches!(self.tokens.get(self.at+2).map(|t| &t.kind), Some(Kind::Symbol(s)) if s == "]") {
                        self.bump();
                        let count_token = self.bump();
                        let Kind::Number(text) = count_token.kind else {return self.fail("repeat count must be an integer literal");};
                        let count = text.replace('_', "").parse::<usize>().map_err(|_| error(&self.path, count_token.span, "invalid repeat count"))?;
                        if count == 0 || count > 1_000_000 {return self.fail("repeat count must be 1..1000000");}
                        self.expect("]")?;
                        self.array_newline_end = saved;
                        repeated = Some((value, count));
                        break;
                    }
                    spread.push((expand, value.clone()));
                    values.push(value);
                    let previous_line = self.tokens[self.at-1].end_line;
                    if self.eat(",") || self.eat(";") { self.separators(); }
                    else if self.eat("]") { break; }
                    else if self.token().span.line <= previous_line { return self.fail("expected comma, semicolon or newline between array elements"); }
                }
                self.array_newline_end = saved;
                Expr {
                    kind: if let Some((value,count)) = repeated {ExprKind::Repeat(Box::new(value),count)} else if has_spread {ExprKind::Spread(spread)} else {ExprKind::Array(values)},
                    span,
                }
            }
            _ => return Err(error(&self.path, span, "expected an expression")),
        };
        loop {
            if self.array_newline_end && self.token().span.line > self.tokens[self.at-1].end_line { break; }
            if min <= 12
                && self.is("<")
                && matches!(left.kind, ExprKind::Name(_))
                && self.type_call_ahead()
            {
                let saved_at = self.at;
                let saved_tokens = self.tokens.clone();
                let saved_annotations = self.annotations.len();
                if let Ok(types) = self.type_arguments() {
                    let mut variant = None;
                    if self.eat(".") {
                        variant = Some(self.ident()?);
                    }
                    if self.eat("(") {
                        let args = self.expression_arguments()?;
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
                self.annotations.truncate(saved_annotations);
            }
            if min <= 12 && self.eat(".") {
                let field = self.ident()?;
                if self.eat("(") {
                    let args = self.expression_arguments()?;
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
                let args = self.expression_arguments()?;
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
                let saved=self.array_newline_end;self.array_newline_end=false;
                let index = self.expr(0)?;
                if self.eat("..") {
                    let end = self.expr(0)?;
                    self.expect("]")?;self.array_newline_end=saved;
                    left = Expr {kind: ExprKind::Collection(Box::new(left), "slice".into(), vec![index, end]), span};
                    continue;
                }
                self.expect("]")?;self.array_newline_end=saved;
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
            if min == 0 && self.eat("?") {
                let yes = self.expr(0)?;
                self.expect(":")?;
                let no = self.expr(0)?;
                left = Expr { kind: ExprKind::Conditional(Box::new(left), Box::new(yes), Box::new(no), None), span };
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
