use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Callback(Vec<Type>, Box<Type>),
    Task(Box<Type>),
    Const(usize),
    ArrayConst(Box<Type>, String),
    Function(Vec<Type>, Box<Type>),
    Void,
    Bool,
    Int { bits: u8, signed: bool },
    Size { signed: bool },
    Float(u8),
    Str,
    Ptr(Box<Type>),
    Ref(Box<Type>),
    Vector(Box<Type>),
    Slice(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Array(Box<Type>, usize),
    Named(Vec<String>, Vec<Type>),
    Nominal(String),
    Record(String, Vec<(String, Type)>),
    Enum(String, Vec<(String, Vec<Type>)>),
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
        self.numeric()
            || matches!(
                self,
                Self::Callback(..) | Self::Bool | Self::Ptr(_) | Self::Str
            )
    }
    pub fn name(&self) -> String {
        match self {
            Self::Callback(ps, r) => format!(
                "callback({}) {}",
                ps.iter().map(Type::name).collect::<Vec<_>>().join(","),
                r.name()
            ),
            Self::Task(t) => format!("Task<{}>", t.name()),
            Self::Const(n) => n.to_string(),
            Self::ArrayConst(t, n) => format!("[{};{n}]", t.name()),
            Self::Function(ps, r) => format!(
                "fn({}) {}",
                ps.iter().map(Type::name).collect::<Vec<_>>().join(","),
                r.name()
            ),
            Self::Void => "void".into(),
            Self::Bool => "bool".into(),
            Self::Str => "str".into(),
            Self::Int { bits, signed } => format!("{}{bits}", if *signed { "i" } else { "u" }),
            Self::Size { signed } => if *signed { "isize" } else { "usize" }.into(),
            Self::Float(bits) => format!("f{bits}"),
            Self::Map(k, v) => format!("Map<{},{}>", k.name(), v.name()),
            Self::Vector(t) => format!("Vec<{}>", t.name()),
            Self::Slice(t) => format!("Slice<{}>", t.name()),
            Self::Ref(t) => format!("Ref<{}>", t.name()),
            Self::Ptr(t) => format!("*{}", t.name()),
            Self::Array(t, n) => format!("[{}; {n}]", t.name()),
            Self::Named(n, args) => {
                if args.is_empty() {
                    n.join(".")
                } else {
                    format!(
                        "{}<{}>",
                        n.join("."),
                        args.iter().map(Type::name).collect::<Vec<_>>().join(",")
                    )
                }
            }
            Self::Nominal(n) | Self::Record(n, _) | Self::Enum(n, _) => n.clone(),
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
    SizeOf(Type),
    Closure(Vec<(String, Type)>, Type, Vec<Stmt>),
    Callable(Vec<String>, Option<Box<Expr>>, Type),
    Invoke(Box<Expr>, Vec<Expr>),
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
    GenericCall(Vec<String>, Vec<Type>, Vec<Expr>),
    Record(Type, Vec<Expr>),
    Enum(Type, usize, Vec<Expr>),
    Field(Box<Expr>, String),
    Reference(Box<Expr>),
    Dereference(Box<Expr>),
    Vector(Type, Vec<Expr>),
    Map(Type),
    Collection(Box<Expr>, String, Vec<Expr>),
}

#[derive(Clone, Debug)]
pub enum Pattern {
    Bind(String),
    Variant(String, Vec<Pattern>),
}

#[derive(Clone, Debug)]
pub struct MatchArm {
    pub variant: String,
    pub bindings: Vec<Pattern>,
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub enum Stmt {
    Unsafe(Vec<Stmt>),
    Match {
        value: Expr,
        arms: Vec<MatchArm>,
    },
    Block(Vec<Stmt>),
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
    pub constraints: Vec<(String, Vec<String>)>,
    pub variadic: bool,
    pub name: String,
    pub generics: Vec<String>,
    pub main_default: bool,
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
    pub traits: Vec<TraitDefinition>,
    pub path: PathBuf,
    pub imports: Vec<Import>,
    pub functions: Vec<Function>,
    pub statements: Vec<Stmt>,
    pub definitions: Vec<TypeDefinition>,
    pub concrete_types: Vec<Type>,
}

#[derive(Clone, Debug)]
pub struct TypeDefinition {
    pub constraints: Vec<(String, Vec<String>)>,
    pub name: String,
    pub generics: Vec<String>,
    pub fields: Vec<(String, Type)>,
    pub variants: Vec<(String, Vec<Type>)>,
    pub enumeration: bool,
    pub span: Span,
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

/// Shared validation: every enum variant occurs once, with exact payload arity.
pub fn pattern_validate(
    p: &Pattern,
    t: &Type,
    names: &mut std::collections::HashSet<String>,
) -> Result<bool, String> {
    match p {
        Pattern::Bind(n) => {
            if let Type::Enum(_, vs) = t {
                if let Some((_, ts)) = vs.iter().find(|(v, _)| v == n) {
                    if !ts.is_empty() {
                        return Err("nested variant requires payload patterns".into());
                    }
                    return Ok(vs.len() == 1);
                }
            }
            if n != "_" && !names.insert(n.clone()) {
                return Err("duplicate match binding".into());
            }
            Ok(true)
        }
        Pattern::Variant(n, ps) => {
            let Type::Enum(_, vs) = t else {
                return Err("nested variant pattern requires an enum".into());
            };
            let index = vs
                .iter()
                .position(|(v, _)| v == n)
                .ok_or_else(|| format!("unknown enum variant '{n}'"))?;
            if ps.len() != vs[index].1.len() {
                return Err("wrong match payload binding count".into());
            }
            let mut irrefutable = vs.len() == 1;
            for (p, t) in ps.iter().zip(&vs[index].1) {
                irrefutable &= pattern_validate(p, t, names)?;
            }
            Ok(irrefutable)
        }
    }
}
pub fn match_variants(ty: &Type, arms: &[MatchArm]) -> Result<Vec<usize>, String> {
    let Type::Enum(_, variants) = ty else {
        return Err("match requires an enum".into());
    };
    let mut covered = std::collections::HashSet::new();
    let mut wildcard = false;
    let mut result = vec![];
    for arm in arms {
        if wildcard {
            return Err("unreachable arm after wildcard".into());
        }
        if arm.variant == "_" {
            if !arm.bindings.is_empty() {
                return Err("wildcard arm cannot bind payload".into());
            }
            wildcard = arm.guard.is_none();
            result.push(usize::MAX);
            continue;
        }
        let index = variants
            .iter()
            .position(|(name, _)| name == &arm.variant)
            .ok_or_else(|| format!("unknown enum variant '{}'", arm.variant))?;
        if covered.contains(&index) {
            return Err("duplicate match variant or unreachable arm".into());
        }
        if arm.bindings.len() != variants[index].1.len() {
            return Err("wrong match payload binding count".into());
        }
        let mut names = std::collections::HashSet::new();
        let mut irrefutable = true;
        for (p, t) in arm.bindings.iter().zip(&variants[index].1) {
            irrefutable &= pattern_validate(p, t, &mut names)?;
        }
        if arm.guard.is_none() && irrefutable {
            covered.insert(index);
        }
        result.push(index);
    }
    if !wildcard && covered.len() != variants.len() {
        return Err("non-exhaustive match; handle every enum variant".into());
    }
    Ok(result)
}

/// Managed storage is internal to Dev and cannot cross the C ABI by value.
pub fn contains_managed(ty: &Type) -> bool {
    match ty {
        Type::Task(_)
        | Type::Function(..)
        | Type::Map(..)
        | Type::Vector(_)
        | Type::Slice(_)
        | Type::Ref(_) => true,
        Type::Record(_, fields) => fields.iter().any(|(_, t)| contains_managed(t)),
        Type::Enum(_, variants) => variants
            .iter()
            .any(|(_, ts)| ts.iter().any(contains_managed)),
        Type::Array(t, _) => contains_managed(t),
        _ => false,
    }
}

pub fn map_entry(ty: &Type) -> Type {
    let Type::Map(k, v) = ty else { unreachable!() };
    let hash = format!("{ty:?}")
        .bytes()
        .fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ b as u64).wrapping_mul(0x100000001b3)
        });
    Type::Record(
        format!("dev_map_entry_{hash:016x}"),
        vec![
            ("key".into(), (**k).clone()),
            ("value".into(), (**v).clone()),
        ],
    )
}

/// C default argument promotions; aggregates and managed values are not varargs.
pub fn variadic_type(t: &Type) -> Result<Type, String> {
    Ok(match t {
        Type::Bool | Type::Int { bits: 8 | 16, .. } => Type::i32(),
        Type::Float(32) => Type::Float(64),
        t if t.scalar() => t.clone(),
        _ => return Err("variadic arguments must be C scalar values".into()),
    })
}

#[derive(Clone, Debug)]
pub struct TraitDefinition {
    pub name: String,
    pub methods: Vec<Function>,
    pub span: Span,
}

/// Supported C ABI values shared by the compiler and source runtime.
pub fn foreign_value(t: &Type, parameter: bool) -> Result<(), String> {
    match t {
        Type::Void if !parameter => Ok(()),
        Type::Record(_, fs) => {
            for (_, t) in fs {
                foreign_field(t)?
            }
            Ok(())
        }
        Type::Enum(_, vs) if vs.iter().all(|(_, ps)| ps.is_empty()) => Ok(()),
        t if t.scalar() => Ok(()),
        _ => Err("unsupported C ABI value; use a C scalar/struct wrapper or raw pointer".into()),
    }
}
fn foreign_field(t: &Type) -> Result<(), String> {
    match t {
        Type::Array(t, n) if *n <= 1024 => foreign_field(t),
        _ => foreign_value(t, true),
    }
}

/// A managed owner plus the ordinary C function/userdata pair (userdata last).
pub fn callback_context_type(t: &Type) -> Option<Type> {
    let Type::Function(ps, r) = t else {
        return None;
    };
    let name = format!(
        "dev_context_callback_{:016x}",
        format!("{t:?}")
            .bytes()
            .fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64)
                .wrapping_mul(0x100000001b3))
    );
    let pointer = Type::Ptr(Box::new(Type::Void));
    let mut args = ps.clone();
    args.push(pointer.clone());
    Some(Type::Record(
        name,
        vec![
            ("call".into(), Type::Callback(args, r.clone())),
            ("data".into(), pointer),
            ("owner".into(), Type::Ref(Box::new(t.clone()))),
        ],
    ))
}
