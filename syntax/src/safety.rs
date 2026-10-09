//! Lexical boundary for inherently unsafe source operations, shared by both engines.
use crate::ast::*;
use std::collections::HashMap;
pub fn validate(modules: &[Module], aliases: &[HashMap<String, usize>]) -> Result<(), String> {
    for (id, m) in modules.iter().enumerate() {
        walk(modules, aliases, id, &m.statements, false)?;
        for f in &m.functions {
            if let Some(body) = &f.body {
                walk(modules, aliases, id, body, false)?;
            }
        }
    }
    Ok(())
}
fn expression(
    ms: &[Module],
    aliases: &[HashMap<String, usize>],
    id: usize,
    e: &Expr,
    permitted: bool,
) -> Result<(), String> {
    let require = || {
        if permitted {
            Ok(())
        } else {
            Err(error(
                &ms[id].path,
                e.span,
                "operation requires an unsafe block",
            ))
        }
    };
    match &e.kind {
        ExprKind::Callable(_, Some(e), _) => expression(ms, aliases, id, e, permitted)?,
        ExprKind::Invoke(a, args) => {
            expression(ms, aliases, id, a, permitted)?;
            for e in args {
                expression(ms, aliases, id, e, permitted)?
            }
        }
        ExprKind::Unary(op, a) => {
            if op == "&" || op == "*" {
                require()?;
            }
            expression(ms, aliases, id, a, permitted)?;
        }
        ExprKind::Cast(a, t) => {
            if matches!(t, Type::Ptr(_)) {
                require()?;
            }
            expression(ms, aliases, id, a, permitted)?;
        }
        ExprKind::Call(names, args) | ExprKind::GenericCall(names, _, args) => {
            let target = match names.as_slice() {
                [n] => Some((id, n)),
                [alias, n] => aliases[id].get(alias).map(|i| (*i, n)),
                _ => None,
            };
            if let Some((owner, n)) = target {
                if ms[owner]
                    .functions
                    .iter()
                    .any(|f| &f.name == n && f.body.is_none())
                    || [
                        "callback",
                        "callback_context",
                        "volatile_load",
                        "volatile_store",
                    ]
                    .contains(&n.as_str())
                {
                    require()?;
                }
            }
            for a in args {
                expression(ms, aliases, id, a, permitted)?;
            }
        }
        ExprKind::Binary(_, a, b) | ExprKind::Index(a, b) => {
            expression(ms, aliases, id, a, permitted)?;
            expression(ms, aliases, id, b, permitted)?;
        }
        ExprKind::Field(a, _) | ExprKind::Reference(a) | ExprKind::Dereference(a) => {
            expression(ms, aliases, id, a, permitted)?
        }
        ExprKind::Vector(_, xs)
        | ExprKind::Record(_, xs)
        | ExprKind::Enum(_, _, xs)
        | ExprKind::Array(xs) => {
            for a in xs {
                expression(ms, aliases, id, a, permitted)?;
            }
        }
        ExprKind::Collection(a, _, args) => {
            expression(ms, aliases, id, a, permitted)?;
            for e in args {
                expression(ms, aliases, id, e, permitted)?;
            }
        }
        ExprKind::Object(_, fields) => {
            for (_, value) in fields {
                expression(ms, aliases, id, value, permitted)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn walk(
    ms: &[Module],
    aliases: &[HashMap<String, usize>],
    id: usize,
    stmts: &[Stmt],
    permitted: bool,
) -> Result<(), String> {
    for s in stmts {
        match s {
            Stmt::Unsafe(body) => walk(ms, aliases, id, body, true)?,
            Stmt::Block(body) => walk(ms, aliases, id, body, permitted)?,
            Stmt::Let { value, .. } | Stmt::Expr(value) | Stmt::Return(Some(value), _) => {
                expression(ms, aliases, id, value, permitted)?
            }
            Stmt::Assign { target, value, .. } => {
                expression(ms, aliases, id, target, permitted)?;
                expression(ms, aliases, id, value, permitted)?;
            }
            Stmt::If { cond, yes, no } => {
                expression(ms, aliases, id, cond, permitted)?;
                walk(ms, aliases, id, yes, permitted)?;
                walk(ms, aliases, id, no, permitted)?;
            }
            Stmt::While { cond, body } => {
                expression(ms, aliases, id, cond, permitted)?;
                walk(ms, aliases, id, body, permitted)?;
            }
            Stmt::Match { value, arms } => {
                expression(ms, aliases, id, value, permitted)?;
                for a in arms {
                    if let Some(g) = &a.guard {
                        expression(ms, aliases, id, g, permitted)?;
                    }
                    walk(ms, aliases, id, &a.body, permitted)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
