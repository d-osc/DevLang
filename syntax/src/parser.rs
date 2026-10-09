use crate::ast::*;
use crate::lexer::{lex, Kind, Token};
use std::path::PathBuf;

pub fn parse(path: PathBuf, source: &str) -> Result<Module, String> {
    let tokens = lex(source).map_err(|(span, msg)| error(&path, span, msg))?;
    Parser {
        tokens,
        at: 0,
        path,
    }
    .module()
}

struct Parser {
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
                    "extern", "export", "true", "false", "null", "and", "or", "not",
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
        let mut functions = Vec::new();
        self.separators();
        while self.token().kind != Kind::Eof {
            let span = self.token().span;
            if self.eat("use") {
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
            } else {
                let external = self.eat("extern");
                let exported = if !external { self.eat("export") } else { false };
                self.expect("fn")?;
                let name = self.ident()?;
                self.expect("(")?;
                let mut params = Vec::new();
                while !self.eat(")") {
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
                let ret = if self.is("{")
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
                let body = if external {
                    self.end()?;
                    None
                } else {
                    Some(self.block()?)
                };
                functions.push(Function {
                    name,
                    params,
                    ret,
                    body,
                    exported,
                    span,
                });
            }
            self.separators();
        }
        Ok(Module {
            path: self.path,
            imports,
            functions,
        })
    }
    fn ty(&mut self) -> Result<Type, String> {
        if self.eat("*") {
            return Ok(Type::Ptr(Box::new(self.ty()?)));
        }
        if self.eat("[") {
            let inner = self.ty()?;
            self.expect(";")?;
            let token = self.bump();
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
        let name = if let Kind::Word(s) = t.kind {
            s
        } else {
            return Err(error(&self.path, t.span, "expected a type"));
        };
        match name.as_str() {
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
            _ => Err(error(&self.path, t.span, format!("unknown type '{name}'"))),
        }
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
    fn stmt(&mut self) -> Result<Stmt, String> {
        let span = self.token().span;
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
            if min <= 12 && self.eat("(") {
                let names = if let ExprKind::Name(n) = left.kind {
                    n
                } else {
                    return self.fail("only named functions can be called");
                };
                let mut args = Vec::new();
                while !self.eat(")") {
                    args.push(self.expr(0)?);
                    if !self.eat(",") {
                        self.expect(")")?;
                        break;
                    }
                }
                left = Expr {
                    kind: ExprKind::Call(names, args),
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
