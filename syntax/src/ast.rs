use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Void,
    Bool,
    Int { bits: u8, signed: bool },
    Size { signed: bool },
    Float(u8),
    Str,
    Ptr(Box<Type>),
    Array(Box<Type>, usize),
}

impl Type {
    pub fn i64() -> Self {
        Self::Int {
            bits: 64,
            signed: true,
        }
    }
    pub fn i32() -> Self {
        Self::Int {
            bits: 32,
            signed: true,
        }
    }
    pub fn integer(&self) -> bool {
        matches!(self, Self::Int { .. } | Self::Size { .. })
    }
    pub fn numeric(&self) -> bool {
        self.integer() || matches!(self, Self::Float(_))
    }
    pub fn scalar(&self) -> bool {
        self.numeric() || matches!(self, Self::Bool | Self::Ptr(_) | Self::Str)
    }
    pub fn name(&self) -> String {
        match self {
            Self::Void => "void".into(),
            Self::Bool => "bool".into(),
            Self::Str => "str".into(),
            Self::Int { bits, signed } => format!("{}{bits}", if *signed { "i" } else { "u" }),
            Self::Size { signed } => if *signed { "isize" } else { "usize" }.into(),
            Self::Float(bits) => format!("f{bits}"),
            Self::Ptr(t) => format!("*{}", t.name()),
            Self::Array(t, n) => format!("[{}; {n}]", t.name()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Number(String),
    String(String),
    Bool(bool),
    Name(Vec<String>),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
    Call(Vec<String>, Vec<Expr>),
    Cast(Box<Expr>, Type),
    Index(Box<Expr>, Box<Expr>),
    Array(Vec<Expr>),
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Let {
        name: String,
        ty: Option<Type>,
        value: Expr,
        span: Span,
    },
    Assign {
        target: Expr,
        op: String,
        value: Expr,
    },
    Expr(Expr),
    Return(Option<Expr>, Span),
    If {
        cond: Expr,
        yes: Vec<Stmt>,
        no: Vec<Stmt>,
    },
    While {
        cond: Expr,
        body: Vec<Stmt>,
    },
    Break(Span),
    Continue(Span),
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub ret: Type,
    pub body: Option<Vec<Stmt>>,
    pub exported: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Import {
    pub path: String,
    pub alias: String,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Module {
    pub path: PathBuf,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
}

pub fn error(path: &std::path::Path, span: Span, message: impl AsRef<str>) -> String {
    format!(
        "{}:{}:{}: {}",
        path.display(),
        span.line,
        span.col,
        message.as_ref()
    )
}
