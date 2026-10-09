//! In-memory numeric execution plan. No native backend, disk cache or JIT.
//! Unsupported or ill-typed plans fall back to the ordinary AST interpreter.
use crate::engine::{convert, wrap, Value};
use dev_syntax::ast::*;
use std::{cell::RefCell, collections::HashMap};

type Signature<'a> = dyn Fn(&[String]) -> Option<(Vec<Type>, Type)> + 'a;
type Call<'a> = dyn FnMut(&[String], Vec<Value>, Span) -> Result<Value, String> + 'a;

#[derive(Clone, Copy, Debug)]
enum Number {
    Int(i128),
    Float(f64),
    Bool(bool),
    Void,
}
impl Number {
    fn from_value(value: &Value) -> Option<Self> {
        Some(match value {
            Value::Int(n, _) => Self::Int(*n),
            Value::Float(n, _) => Self::Float(*n),
            Value::Bool(n) => Self::Bool(*n),
            Value::Void => Self::Void,
            _ => return None,
        })
    }
    fn value(self, ty: &Type) -> Value {
        match self {
            Self::Int(n) => Value::Int(n, ty.clone()),
            Self::Float(n) => Value::Float(n, ty.clone()),
            Self::Bool(n) => Value::Bool(n),
            Self::Void => Value::Void,
        }
    }
    fn integer(self) -> i128 {
        match self {
            Self::Int(n) => n,
            _ => unreachable!("verified numeric plan"),
        }
    }
    fn float(self) -> f64 {
        match self {
            Self::Float(n) => n,
            _ => unreachable!("verified numeric plan"),
        }
    }
    fn boolean(self) -> bool {
        match self {
            Self::Bool(n) => n,
            _ => unreachable!("verified numeric plan"),
        }
    }
}

#[derive(Clone, Copy)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
impl Op {
    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "+" => Self::Add,
            "-" => Self::Sub,
            "*" => Self::Mul,
            "/" => Self::Div,
            "%" => Self::Rem,
            "&" => Self::And,
            "|" => Self::Or,
            "^" => Self::Xor,
            "<<" => Self::Shl,
            ">>" => Self::Shr,
            "==" => Self::Eq,
            "!=" => Self::Ne,
            "<" => Self::Lt,
            "<=" => Self::Le,
            ">" => Self::Gt,
            ">=" => Self::Ge,
            _ => return None,
        })
    }
    fn comparison(self) -> bool {
        matches!(
            self,
            Self::Eq | Self::Ne | Self::Lt | Self::Le | Self::Gt | Self::Ge
        )
    }
}
#[derive(Clone, Copy)]
enum Kind {
    Int { mask: i128, sign: i128, bits: u32 },
    Float { narrow: bool },
    Bool,
}
impl Kind {
    fn of(t: &Type) -> Option<Self> {
        Some(match t {
            Type::Int { bits, signed } => Self::integer(*bits as u32, *signed),
            Type::Size { signed } => Self::integer(usize::BITS, *signed),
            Type::Float(bits) => Self::Float {
                narrow: *bits == 32,
            },
            Type::Bool => Self::Bool,
            _ => return None,
        })
    }
    fn integer(bits: u32, signed: bool) -> Self {
        Self::Int {
            mask: (1i128 << bits) - 1,
            sign: if signed { 1i128 << (bits - 1) } else { 0 },
            bits,
        }
    }
    fn normalize(self, n: i128) -> i128 {
        let Self::Int { mask, sign, .. } = self else {
            unreachable!()
        };
        let n = n & mask;
        if n & sign != 0 {
            n - (mask + 1)
        } else {
            n
        }
    }
    fn unary(self, op: u8, n: Number) -> Number {
        match (self, op) {
            (Self::Int { .. }, b'-') => Number::Int(self.normalize(-n.integer())),
            (Self::Int { .. }, b'~') => Number::Int(self.normalize(!n.integer())),
            (Self::Float { .. }, b'-') => Number::Float(-n.float()),
            (Self::Bool, b'!') => Number::Bool(!n.boolean()),
            _ => unreachable!("verified unary operator"),
        }
    }
    fn binary(self, op: Op, a: Number, b: Number) -> Result<Number, &'static str> {
        use Op::*;
        Ok(match self {
            Self::Int { bits, .. } => {
                let (a, b) = (a.integer(), b.integer());
                let n = match op {
                    Eq => return Ok(Number::Bool(a == b)),
                    Ne => return Ok(Number::Bool(a != b)),
                    Lt => return Ok(Number::Bool(a < b)),
                    Le => return Ok(Number::Bool(a <= b)),
                    Gt => return Ok(Number::Bool(a > b)),
                    Ge => return Ok(Number::Bool(a >= b)),
                    Add => a.wrapping_add(b),
                    Sub => a.wrapping_sub(b),
                    Mul => a.wrapping_mul(b),
                    Div | Rem if b == 0 => return Err("division by zero"),
                    Div => a / b,
                    Rem => a % b,
                    And => a & b,
                    Or => a | b,
                    Xor => a ^ b,
                    Shl | Shr if b < 0 || b >= bits as i128 => {
                        return Err("shift count out of range")
                    }
                    Shl => a << b,
                    Shr => a >> b,
                };
                Number::Int(self.normalize(n))
            }
            Self::Float { narrow } => {
                let (a, b) = (a.float(), b.float());
                let n = match op {
                    Eq => return Ok(Number::Bool(a == b)),
                    Ne => return Ok(Number::Bool(a != b)),
                    Lt => return Ok(Number::Bool(a < b)),
                    Le => return Ok(Number::Bool(a <= b)),
                    Gt => return Ok(Number::Bool(a > b)),
                    Ge => return Ok(Number::Bool(a >= b)),
                    Add => a + b,
                    Sub => a - b,
                    Mul => a * b,
                    Div => a / b,
                    Rem => a % b,
                    _ => unreachable!(),
                };
                Number::Float(if narrow { n as f32 as f64 } else { n })
            }
            Self::Bool => Number::Bool(match op {
                Eq => a.boolean() == b.boolean(),
                Ne => a.boolean() != b.boolean(),
                _ => unreachable!(),
            }),
        })
    }
}
enum Instruction {
    Copy {
        dst: usize,
        src: usize,
        array: bool,
    },
    Unary {
        dst: usize,
        src: usize,
        kind: Kind,
        op: u8,
    },
    Binary {
        dst: usize,
        a: usize,
        b: usize,
        kind: Kind,
        op: Op,
    },
    Cast {
        dst: usize,
        src: usize,
        from: Kind,
        to: Kind,
    },
    Array {
        dst: usize,
        items: Vec<usize>,
    },
    Index {
        dst: usize,
        array: usize,
        index: usize,
    },
    Store {
        array: usize,
        index: usize,
        src: usize,
    },
    Jump(usize),
    Branch {
        cond: usize,
        target: usize,
        when: bool,
    },
    Call {
        dst: usize,
        name: Vec<String>,
        args: Vec<usize>,
    },
    Return(usize),
    Fail(&'static str),
}
pub struct Program {
    instructions: Vec<(Instruction, Span)>,
    initial: Vec<Number>,
    types: Vec<Type>,
    params: Vec<usize>,
    workspace: RefCell<Option<Workspace>>,
}
struct Workspace {
    numbers: Vec<Number>,
    arrays: Vec<Vec<Number>>,
}
fn supported(t: &Type) -> bool {
    Kind::of(t).is_some()
        || *t == Type::Void
        || matches!(t,Type::Array(inner,_) if Kind::of(inner).is_some())
}
struct Loop {
    start: usize,
    breaks: Vec<usize>,
}
struct Builder<'a> {
    program: Program,
    scopes: Vec<HashMap<String, usize>>,
    loops: Vec<Loop>,
    ret: Type,
    signature: &'a Signature<'a>,
}
impl Program {
    pub fn lower(f: &Function, signature: &Signature<'_>) -> Option<Self> {
        if !supported(&f.ret) {
            return None;
        }
        let mut b = Builder {
            program: Self {
                instructions: vec![],
                initial: vec![],
                types: vec![],
                params: vec![],
                workspace: RefCell::new(None),
            },
            scopes: vec![HashMap::new()],
            loops: vec![],
            ret: f.ret.clone(),
            signature,
        };
        for (name, ty) in &f.params {
            if !supported(ty) || *ty == Type::Void || b.scopes[0].contains_key(name) {
                return None;
            }
            let slot = b.slot(ty.clone(), Number::Void);
            b.scopes[0].insert(name.clone(), slot);
            b.program.params.push(slot);
        }
        b.block(f.body.as_ref()?)?;
        if f.ret == Type::Void {
            let slot = b.slot(Type::Void, Number::Void);
            b.emit(Instruction::Return(slot), f.span);
        } else if f.main_default && f.ret == Type::i32() {
            let slot = b.slot(Type::i32(), Number::Int(0));
            b.emit(Instruction::Return(slot), f.span);
        } else {
            b.emit(Instruction::Fail("missing return value"), f.span);
        }
        Some(b.program)
    }
    pub fn run(&self, args: Vec<Value>, call: &mut Call<'_>) -> Result<Value, (Span, String)> {
        // Take, rather than borrow, the workspace so recursive calls remain valid.
        let Workspace {
            mut numbers,
            mut arrays,
        } = self
            .workspace
            .borrow_mut()
            .take()
            .unwrap_or_else(|| Workspace {
                numbers: self.initial.clone(),
                arrays: vec![vec![]; self.initial.len()],
            });
        numbers.clone_from(&self.initial);
        for (slot, value) in self.params.iter().zip(args) {
            Self::put(&mut numbers, &mut arrays, *slot, value);
        }
        let mut pc = 0;
        let result = loop {
            let (instruction, span) = &self.instructions[pc];
            let step = (|| -> Result<Option<Value>, String> {
                match instruction {
                    Instruction::Copy { dst, src, array } => {
                        if *array {
                            arrays[*dst] = arrays[*src].clone();
                        } else {
                            numbers[*dst] = numbers[*src];
                        }
                    }
                    Instruction::Unary { dst, src, kind, op } => {
                        numbers[*dst] = kind.unary(*op, numbers[*src])
                    }
                    Instruction::Binary {
                        dst,
                        a,
                        b,
                        kind,
                        op,
                    } => {
                        numbers[*dst] = kind
                            .binary(*op, numbers[*a], numbers[*b])
                            .map_err(str::to_owned)?
                    }
                    Instruction::Cast { dst, src, from, to } => {
                        let value = match from {
                            Kind::Int { .. } => numbers[*src].integer() as f64,
                            Kind::Float { .. } => numbers[*src].float(),
                            Kind::Bool => unreachable!(),
                        };
                        numbers[*dst] = match to {
                            Kind::Int { .. } => Number::Int(to.normalize(match from {
                                Kind::Int { .. } => numbers[*src].integer(),
                                _ => value as i128,
                            })),
                            Kind::Float { narrow } => Number::Float(if *narrow {
                                match from {
                                    Kind::Int { .. } => numbers[*src].integer() as f32 as f64,
                                    _ => value as f32 as f64,
                                }
                            } else {
                                value
                            }),
                            Kind::Bool => unreachable!(),
                        };
                    }
                    Instruction::Array { dst, items } => {
                        arrays[*dst].clear();
                        arrays[*dst].extend(items.iter().map(|i| numbers[*i]));
                    }
                    Instruction::Index { dst, array, index } => {
                        let index = usize::try_from(numbers[*index].integer())
                            .map_err(|_| "negative or excessive index")?;
                        numbers[*dst] = *arrays[*array]
                            .get(index)
                            .ok_or("array index out of bounds")?;
                    }
                    Instruction::Store { array, index, src } => {
                        let index = usize::try_from(numbers[*index].integer())
                            .map_err(|_| "invalid index")?;
                        *arrays[*array]
                            .get_mut(index)
                            .ok_or("invalid array assignment or index out of bounds")? =
                            numbers[*src];
                    }
                    Instruction::Jump(target) => {
                        pc = *target;
                        return Ok(None);
                    }
                    Instruction::Branch { cond, target, when }
                        if numbers[*cond].boolean() == *when =>
                    {
                        pc = *target;
                        return Ok(None);
                    }
                    Instruction::Branch { .. } => {}
                    Instruction::Call { dst, name, args } => {
                        let args = args
                            .iter()
                            .map(|i| self.value(&numbers, &arrays, *i))
                            .collect();
                        let value = call(name, args, *span)?;
                        let value = convert(value, &self.types[*dst], false)?;
                        Self::put(&mut numbers, &mut arrays, *dst, value);
                    }
                    Instruction::Return(slot) => {
                        return Ok(Some(self.value(&numbers, &arrays, *slot)))
                    }
                    Instruction::Fail(msg) => return Err((*msg).into()),
                }
                pc += 1;
                Ok(None)
            })();
            match step {
                Ok(Some(value)) => break Ok(value),
                Ok(None) => {}
                Err(msg) => break Err((*span, msg)),
            }
        };
        // Retain at most one frame per function, including after recursive calls.
        let mut cached = self.workspace.borrow_mut();
        if cached.is_none() {
            *cached = Some(Workspace { numbers, arrays });
        }
        result
    }
    fn put(numbers: &mut [Number], arrays: &mut [Vec<Number>], slot: usize, value: Value) {
        if let Value::Array(items, _) = value {
            arrays[slot] = items
                .iter()
                .map(|v| Number::from_value(v).expect("checked array type"))
                .collect();
        } else {
            numbers[slot] = Number::from_value(&value).expect("checked scalar type");
        }
    }
    fn value(&self, numbers: &[Number], arrays: &[Vec<Number>], slot: usize) -> Value {
        let ty = &self.types[slot];
        if let Type::Array(element, _) = ty {
            Value::Array(
                arrays[slot].iter().map(|n| n.value(element)).collect(),
                ty.clone(),
            )
        } else {
            numbers[slot].value(ty)
        }
    }
}
impl Builder<'_> {
    fn slot(&mut self, t: Type, n: Number) -> usize {
        let slot = self.program.types.len();
        self.program.types.push(t);
        self.program.initial.push(n);
        slot
    }
    fn emit(&mut self, i: Instruction, s: Span) -> usize {
        let at = self.program.instructions.len();
        self.program.instructions.push((i, s));
        at
    }
    fn patch(&mut self, at: usize, target: usize) {
        match &mut self.program.instructions[at].0 {
            Instruction::Jump(t) | Instruction::Branch { target: t, .. } => *t = target,
            _ => unreachable!(),
        }
    }
    fn name(&self, n: &[String]) -> Option<usize> {
        if n.len() != 1 {
            return None;
        }
        self.scopes.iter().rev().find_map(|s| s.get(&n[0]).copied())
    }
    fn copy(&mut self, dst: usize, src: usize, s: Span) {
        self.emit(
            Instruction::Copy {
                dst,
                src,
                array: matches!(self.program.types[dst], Type::Array(..)),
            },
            s,
        );
    }
    fn block(&mut self, stmts: &[Stmt]) -> Option<()> {
        self.scopes.push(HashMap::new());
        for s in stmts {
            self.statement(s)?;
        }
        self.scopes.pop();
        Some(())
    }
    fn statement(&mut self, s: &Stmt) -> Option<()> {
        match s {
            Stmt::Match { .. } => return None,
            Stmt::Unsafe(body) | Stmt::Block(body) => self.block(body)?,
            Stmt::Let {
                name,
                ty,
                value,
                span,
            } => {
                if self.scopes.last()?.contains_key(name) {
                    return None;
                }
                let src = self.expr(value, ty.as_ref())?;
                let dst = self.slot(self.program.types[src].clone(), Number::Void);
                self.copy(dst, src, *span);
                self.scopes.last_mut()?.insert(name.clone(), dst);
            }
            Stmt::Expr(e) => {
                self.expr(e, None)?;
            }
            Stmt::Return(e, span) => {
                let ret = self.ret.clone();
                let slot = if let Some(e) = e {
                    self.expr(e, Some(&ret))?
                } else {
                    if ret != Type::Void {
                        return None;
                    }
                    self.slot(Type::Void, Number::Void)
                };
                self.emit(Instruction::Return(slot), *span);
            }
            Stmt::Assign { target, op, value } => {
                let old = self.expr(target, None)?;
                let ty = self.program.types[old].clone();
                let rhs = self.expr(value, Some(&ty))?;
                let src = if op == "=" {
                    rhs
                } else {
                    self.binary(op.trim_end_matches('='), old, rhs, target.span)?
                };
                match &target.kind {
                    ExprKind::Name(n) => {
                        let dst = self.name(n)?;
                        self.copy(dst, src, target.span);
                    }
                    ExprKind::Index(base, index) => {
                        let ExprKind::Name(n) = &base.kind else {
                            return None;
                        };
                        let array = self.name(n)?;
                        let index = self.expr(index, None)?;
                        self.emit(Instruction::Store { array, index, src }, target.span);
                    }
                    _ => return None,
                }
            }
            Stmt::If { cond, yes, no } => {
                let c = self.expr(cond, None)?;
                if self.program.types[c] != Type::Bool {
                    return None;
                }
                let branch = self.emit(
                    Instruction::Branch {
                        cond: c,
                        target: 0,
                        when: false,
                    },
                    cond.span,
                );
                self.block(yes)?;
                let end = self.emit(Instruction::Jump(0), cond.span);
                self.patch(branch, self.program.instructions.len());
                self.block(no)?;
                self.patch(end, self.program.instructions.len());
            }
            Stmt::While { cond, body } => {
                let start = self.program.instructions.len();
                let c = self.expr(cond, None)?;
                if self.program.types[c] != Type::Bool {
                    return None;
                }
                let branch = self.emit(
                    Instruction::Branch {
                        cond: c,
                        target: 0,
                        when: false,
                    },
                    cond.span,
                );
                self.loops.push(Loop {
                    start,
                    breaks: vec![],
                });
                self.block(body)?;
                self.emit(Instruction::Jump(start), cond.span);
                let end = self.program.instructions.len();
                self.patch(branch, end);
                for at in self.loops.pop()?.breaks {
                    self.patch(at, end);
                }
            }
            Stmt::Break(span) => {
                self.loops.last()?;
                let at = self.emit(Instruction::Jump(0), *span);
                self.loops.last_mut()?.breaks.push(at);
            }
            Stmt::Continue(span) => {
                let start = self.loops.last()?.start;
                self.emit(Instruction::Jump(start), *span);
            }
        }
        Some(())
    }
    fn binary(&mut self, op: &str, a: usize, b: usize, span: Span) -> Option<usize> {
        let ty = self.program.types[a].clone();
        if ty != self.program.types[b] {
            return None;
        }
        let kind = Kind::of(&ty)?;
        let op = Op::parse(op)?;
        match kind {
            Kind::Float { .. } if matches!(op, Op::And | Op::Or | Op::Xor | Op::Shl | Op::Shr) => {
                return None
            }
            Kind::Bool if !matches!(op, Op::Eq | Op::Ne) => return None,
            _ => {}
        }
        let dst = self.slot(if op.comparison() { Type::Bool } else { ty }, Number::Void);
        self.emit(
            Instruction::Binary {
                dst,
                a,
                b,
                kind,
                op,
            },
            span,
        );
        Some(dst)
    }
    fn expr(&mut self, e: &Expr, hint: Option<&Type>) -> Option<usize> {
        let slot = match &e.kind {
            ExprKind::Number(s) => {
                let ty = hint.filter(|t| t.numeric()).cloned().unwrap_or_else(|| {
                    if !s.trim_start_matches('-').starts_with("0x")
                        && !s.trim_start_matches('-').starts_with("0X")
                        && (s.contains('.') || s.contains('e') || s.contains('E'))
                    {
                        Type::Float(64)
                    } else {
                        Type::i64()
                    }
                });
                let n = if let Type::Float(bits) = ty {
                    let n = s.parse::<f64>().ok()?;
                    Number::Float(if bits == 32 { n as f32 as f64 } else { n })
                } else {
                    let text = s.replace('_', "");
                    let n = if let Some(hex) =
                        text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"))
                    {
                        i128::from_str_radix(hex, 16).ok()?
                    } else {
                        text.parse::<i128>().ok()?
                    };
                    if wrap(n, &ty) != n {
                        return None;
                    }
                    Number::Int(n)
                };
                self.slot(ty, n)
            }
            ExprKind::Bool(b) => self.slot(Type::Bool, Number::Bool(*b)),
            ExprKind::Name(n) => self.name(n)?,
            ExprKind::Unary(op, a) => {
                if op == "-" {
                    if let ExprKind::Number(s) = &a.kind {
                        let text = if let Some(hex) =
                            s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
                        {
                            format!("-{}", i128::from_str_radix(hex, 16).ok()?)
                        } else {
                            format!("-{s}")
                        };
                        return self.expr(
                            &Expr {
                                kind: ExprKind::Number(text),
                                span: e.span,
                            },
                            hint,
                        );
                    }
                }
                let src = self.expr(a, hint)?;
                let ty = self.program.types[src].clone();
                let kind = Kind::of(&ty)?;
                let op = match (op.as_str(), kind) {
                    ("-", Kind::Int { .. } | Kind::Float { .. }) => b'-',
                    ("~", Kind::Int { .. }) => b'~',
                    ("!", Kind::Bool) => b'!',
                    _ => return None,
                };
                let dst = self.slot(ty, Number::Void);
                self.emit(Instruction::Unary { dst, src, kind, op }, e.span);
                dst
            }
            ExprKind::Binary(op, a, b) => {
                let a = self.expr(a, hint.filter(|t| t.numeric()))?;
                let ty = self.program.types[a].clone();
                if op == "&&" || op == "||" {
                    if ty != Type::Bool {
                        return None;
                    }
                    let dst = self.slot(Type::Bool, Number::Void);
                    self.copy(dst, a, e.span);
                    let branch = self.emit(
                        Instruction::Branch {
                            cond: dst,
                            target: 0,
                            when: op == "||",
                        },
                        e.span,
                    );
                    let b = self.expr(b, Some(&ty))?;
                    self.copy(dst, b, e.span);
                    self.patch(branch, self.program.instructions.len());
                    dst
                } else {
                    let b = self.expr(b, Some(&ty))?;
                    self.binary(op, a, b, e.span)?
                }
            }
            ExprKind::Cast(a, to) => {
                let src = self.expr(a, None)?;
                let from = self.program.types[src].clone();
                if from == *to {
                    src
                } else {
                    if !from.numeric() || !to.numeric() {
                        return None;
                    }
                    let dst = self.slot(to.clone(), Number::Void);
                    self.emit(
                        Instruction::Cast {
                            dst,
                            src,
                            from: Kind::of(&from)?,
                            to: Kind::of(to)?,
                        },
                        e.span,
                    );
                    dst
                }
            }
            ExprKind::Call(name, args) => {
                let (params, ret) = if name == &["print"] {
                    (vec![], Type::Void)
                } else {
                    (self.signature)(name)?
                };
                if !supported(&ret) || (name != &["print"] && params.len() != args.len()) {
                    return None;
                }
                let args = args
                    .iter()
                    .enumerate()
                    .map(|(i, a)| self.expr(a, params.get(i)))
                    .collect::<Option<Vec<_>>>()?;
                let dst = self.slot(ret, Number::Void);
                self.emit(
                    Instruction::Call {
                        dst,
                        name: name.clone(),
                        args,
                    },
                    e.span,
                );
                dst
            }
            ExprKind::Array(items) => {
                let element = if let Some(Type::Array(t, _)) = hint {
                    Some(t.as_ref().clone())
                } else {
                    None
                };
                let mut slots = vec![];
                let mut element = element;
                for item in items {
                    let slot = self.expr(item, element.as_ref())?;
                    element = Some(self.program.types[slot].clone());
                    slots.push(slot);
                }
                let element = element?;
                Kind::of(&element)?;
                let dst = self.slot(Type::Array(Box::new(element), slots.len()), Number::Void);
                self.emit(Instruction::Array { dst, items: slots }, e.span);
                dst
            }
            ExprKind::Index(a, index) => {
                let array = self.expr(a, None)?;
                let Type::Array(element, _) = self.program.types[array].clone() else {
                    return None;
                };
                let index = self.expr(index, None)?;
                if !self.program.types[index].integer() {
                    return None;
                }
                let dst = self.slot(*element, Number::Void);
                self.emit(Instruction::Index { dst, array, index }, e.span);
                dst
            }
            _ => return None,
        };
        if hint.is_some_and(|ty| *ty != self.program.types[slot]) {
            return None;
        }
        Some(slot)
    }
}
