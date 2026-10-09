//! Native layout and marshaling; only called for explicitly unsafe foreign memory.
use crate::engine::Value;
use dev_syntax::ast::Type;
use std::ffi::{CStr, CString};
pub fn layout(t: &Type) -> Result<(usize, usize), String> {
    Ok(match t {
        Type::Bool => (1, 1),
        Type::Int { bits, .. } | Type::Float(bits) => {
            let n = *bits as usize / 8;
            (n, n)
        }
        Type::Callback(..) | Type::Size { .. } | Type::Ptr(_) | Type::Str => {
            (std::mem::size_of::<usize>(), std::mem::align_of::<usize>())
        }
        Type::Enum(_, vs) if vs.iter().all(|(_, ts)| ts.is_empty()) => (4, 4),
        Type::Array(t, n) => {
            let (s, a) = layout(t)?;
            (s.checked_mul(*n).ok_or("native layout overflow")?, a)
        }
        Type::Record(_, fs) => {
            let (mut size, mut align) = (0, 1);
            for (_, t) in fs {
                let (s, a) = layout(t)?;
                size = round(size, a)
                    .checked_add(s)
                    .ok_or("native layout overflow")?;
                align = align.max(a);
            }
            (round(size.max(1), align), align)
        }
        _ => return Err("unsupported native memory layout".into()),
    })
}
fn round(n: usize, a: usize) -> usize {
    (n + a - 1) & !(a - 1)
}
pub struct Buffer {
    pub words: Vec<u64>,
    strings: Vec<CString>,
}
impl Buffer {
    pub fn new(t: &Type) -> Result<Self, String> {
        let (size, align) = layout(t)?;
        if size > 1024 * 1024 || align > 8 {
            return Err("native ABI storage exceeds supported layout".into());
        }
        Ok(Self {
            words: vec![0; size.div_ceil(8).max(1)],
            strings: vec![],
        })
    }
    pub fn from_value(v: &Value) -> Result<Self, String> {
        let mut b = Self::new(&v.ty())?;
        // The buffer is aligned, sized by the checked type and survives the native call.
        unsafe {
            write(b.words.as_mut_ptr() as usize, v, &mut b.strings)?;
        }
        Ok(b)
    }
}
fn address(p: usize, t: &Type) -> Result<(), String> {
    let (size, align) = layout(t)?;
    if p == 0 {
        return Err("null pointer dereference".into());
    }
    if !p.is_multiple_of(align) {
        return Err("misaligned native pointer".into());
    }
    p.checked_add(size).ok_or("native address overflow")?;
    Ok(())
}
/// Caller must guarantee readable allocation and live pointed-to strings.
pub unsafe fn read(p: usize, t: &Type) -> Result<Value, String> {
    unsafe { read_mode(p, t, false) }
}
pub unsafe fn volatile_read(p: usize, t: &Type) -> Result<Value, String> {
    if !t.numeric() && *t != Type::Bool {
        return Err("volatile access requires numeric/bool pointer".into());
    }
    unsafe { read_mode(p, t, true) }
}
unsafe fn read_mode(p: usize, t: &Type, volatile: bool) -> Result<Value, String> {
    address(p, t)?;
    macro_rules! load {
        ($ty:ty) => {
            unsafe {
                if volatile {
                    std::ptr::read_volatile(p as *const $ty)
                } else {
                    std::ptr::read(p as *const $ty)
                }
            }
        };
    }
    Ok(match t {
        Type::Bool => Value::Bool(load!(u8) != 0),
        Type::Int { bits, signed } => {
            let n = match (bits, signed) {
                (8, true) => load!(i8) as i128,
                (8, false) => load!(u8) as i128,
                (16, true) => load!(i16) as i128,
                (16, false) => load!(u16) as i128,
                (32, true) => load!(i32) as i128,
                (32, false) => load!(u32) as i128,
                (64, true) => load!(i64) as i128,
                (64, false) => load!(u64) as i128,
                _ => return Err("unsupported integer layout".into()),
            };
            Value::Int(n, t.clone())
        }
        Type::Size { signed } => Value::Int(
            if *signed {
                load!(isize) as i128
            } else {
                load!(usize) as i128
            },
            t.clone(),
        ),
        Type::Float(32) => Value::Float(load!(f32) as f64, t.clone()),
        Type::Float(64) => Value::Float(load!(f64), t.clone()),
        Type::Callback(..) | Type::Ptr(_) => Value::Ptr(load!(usize), t.clone()),
        Type::Str => {
            let s = load!(usize);
            if s == 0 {
                return Err("null native str".into());
            }
            Value::Str(
                unsafe { CStr::from_ptr(s as *const i8) }
                    .to_str()
                    .map_err(|_| "invalid UTF-8 native str")?
                    .into(),
            )
        }
        Type::Enum(_, vs) if vs.iter().all(|(_, ts)| ts.is_empty()) => {
            let n = load!(i32);
            if n < 0 || n as usize >= vs.len() {
                return Err("invalid native enum discriminant".into());
            }
            Value::Enum(n as usize, t.clone(), vec![])
        }
        Type::Record(_, fs) => {
            let mut at = 0;
            let mut fields = vec![];
            for (n, t) in fs {
                let (s, a) = layout(t)?;
                at = round(at, a);
                fields.push((n.clone(), unsafe { read(p + at, t) }?));
                at += s;
            }
            Value::Record(fields, t.clone())
        }
        Type::Array(inner, n) => {
            let (s, _) = layout(inner)?;
            let mut values = vec![];
            for i in 0..*n {
                values.push(unsafe { read(p + i * s, inner) }?)
            }
            Value::Array(values, t.clone())
        }
        _ => return Err("unsupported native read type".into()),
    })
}
/// Caller must guarantee writable allocation; strings live as long as `strings`.
pub unsafe fn write(p: usize, v: &Value, strings: &mut Vec<CString>) -> Result<(), String> {
    unsafe { write_mode(p, v, strings, false) }
}
pub unsafe fn volatile_write(p: usize, v: &Value) -> Result<(), String> {
    if !v.ty().numeric() && v.ty() != Type::Bool {
        return Err("volatile access requires numeric/bool pointer".into());
    }
    unsafe { write_mode(p, v, &mut vec![], true) }
}
unsafe fn write_mode(
    p: usize,
    v: &Value,
    strings: &mut Vec<CString>,
    volatile: bool,
) -> Result<(), String> {
    address(p, &v.ty())?;
    macro_rules! store {
        ($ty:ty,$value:expr) => {
            unsafe {
                if volatile {
                    std::ptr::write_volatile(p as *mut $ty, $value as $ty)
                } else {
                    std::ptr::write(p as *mut $ty, $value as $ty)
                }
            }
        };
    }
    match v {
        Value::Bool(b) => store!(u8, *b as u8),
        Value::Int(n, t) => match t {
            Type::Int { bits: 8, .. } => store!(u8, *n),
            Type::Int { bits: 16, .. } => store!(u16, *n),
            Type::Int { bits: 32, .. } => store!(u32, *n),
            Type::Int { bits: 64, .. } => store!(u64, *n),
            Type::Size { .. } => store!(usize, *n),
            _ => return Err("unsupported native integer write".into()),
        },
        Value::Float(n, Type::Float(32)) => store!(f32, *n),
        Value::Float(n, _) => store!(f64, *n),
        Value::Ptr(n, _) => store!(usize, *n),
        Value::Str(s) => {
            let s = CString::new(s.as_bytes())
                .map_err(|_| "native str arguments cannot contain NUL bytes")?;
            store!(usize, s.as_ptr() as usize);
            strings.push(s)
        }
        Value::Enum(n, _, payload) if payload.is_empty() => store!(i32, *n),
        Value::Record(fs, _) => {
            let mut at = 0;
            for (_, v) in fs {
                let (s, a) = layout(&v.ty())?;
                at = round(at, a);
                unsafe { write(p + at, v, strings) }?;
                at += s;
            }
        }
        Value::Array(xs, Type::Array(t, _)) => {
            let (s, _) = layout(t)?;
            for (i, v) in xs.iter().enumerate() {
                unsafe { write(p + i * s, v, strings) }?;
            }
        }
        _ => return Err("unsupported native memory write".into()),
    }
    Ok(())
}

pub fn field_offset(t: &Type, name: &str) -> Result<(usize, Type), String> {
    let Type::Record(_, fs) = t else {
        return Err("field access requires native struct".into());
    };
    let mut at = 0;
    for (n, t) in fs {
        let (s, a) = layout(t)?;
        at = round(at, a);
        if n == name {
            return Ok((at, t.clone()));
        }
        at += s;
    }
    Err("unknown native struct field".into())
}

/// Dev's C value representation, including managed handles (not their allocation).
pub fn object_layout(t: &Type) -> Result<(usize, usize), String> {
    let pointer = std::mem::size_of::<usize>();
    Ok(match t {
        Type::Ref(_) | Type::Task(_) => (pointer, pointer),
        Type::Function(..) => (2 * pointer, pointer),
        Type::Vector(_) | Type::Slice(_) => (3 * pointer, pointer),
        Type::Map(..) => (6 * pointer, pointer),
        Type::Record(_, fs) => {
            let (mut size, mut align) = (0, 1);
            for (_, t) in fs {
                let (s, a) = object_layout(t)?;
                size = round(size, a) + s;
                align = align.max(a)
            }
            (round(size.max(1), align), align)
        }
        Type::Array(t, n) => {
            let (s, a) = object_layout(t)?;
            (s.checked_mul(*n).ok_or("layout overflow")?, a)
        }
        Type::Enum(_, vs) if vs.iter().any(|(_, ts)| !ts.is_empty()) => {
            let (mut size, mut align) = (0, 1);
            for (_, ts) in vs {
                let (mut field, mut a) = (0, 1);
                for t in ts {
                    let (s, b) = object_layout(t)?;
                    field = round(field, b) + s;
                    a = a.max(b)
                }
                size = size.max(round(field, a));
                align = align.max(a)
            }
            (round(round(4, align) + size, align.max(4)), align.max(4))
        }
        _ => layout(t)?,
    })
}
