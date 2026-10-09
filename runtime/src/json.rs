use crate::engine::Value;
use dev_syntax::ast::Type;
use serde_json::{Number, Value as Json};
use std::sync::Arc;

// Signatures let the shared expander infer values captured by closures.
// Calls are dispatched to the runtime intrinsics, not these placeholder bodies.
pub(crate) const MODULE: &str = r#"
struct Value {}
fn parse(text str) Value { return Value() }
fn get(value Value, key str) Value { return Value() }
fn at(value Value, index i64) Value { return Value() }
fn object() Value { return Value() }
fn array() Value { return Value() }
fn null_value() Value { return Value() }
fn remove(value Value, key str) Value { return Value() }
"#;

pub(crate) fn check_depth(value: &Json, initial: usize) -> Result<(), String> {
    let mut pending = vec![(value, initial)];
    while let Some((value, depth)) = pending.pop() {
        if depth > 128 {
            return Err("JSON nesting limit exceeded".into());
        }
        match value {
            Json::Array(values) => pending.extend(values.iter().map(|v| (v, depth + 1))),
            Json::Object(values) => pending.extend(values.values().map(|v| (v, depth + 1))),
            _ => {}
        }
    }
    Ok(())
}

pub(crate) fn encode(value: &Value, depth: usize) -> Result<Json, String> {
    if depth > 128 {
        return Err("JSON nesting limit exceeded".into());
    }
    Ok(match value {
        Value::Json(value, _) => {
            check_depth(value, depth)?;
            (**value).clone()
        }
        Value::Bool(value) => Json::Bool(*value),
        Value::Str(value) => Json::String(value.clone()),
        Value::Int(value, _) => Json::Number(if *value < 0 {
            Number::from(i64::try_from(*value).map_err(|_| "JSON integer out of range")?)
        } else {
            Number::from(u64::try_from(*value).map_err(|_| "JSON integer out of range")?)
        }),
        Value::Float(value, _) => {
            Json::Number(Number::from_f64(*value).ok_or("JSON numbers must be finite")?)
        }
        Value::Ptr(0, _) => Json::Null,
        Value::Ref(value, _) => encode(value, depth + 1)?,
        Value::Array(values, _) => Json::Array(
            values
                .iter()
                .map(|v| encode(v, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        Value::Vector(values, _) => Json::Array(
            values
                .iter()
                .map(|v| encode(v, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        Value::Slice(values, offset, len, _) => Json::Array(
            values[*offset..*offset + *len]
                .iter()
                .map(|v| encode(v, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        Value::Record(fields, _) => Json::Object(
            fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), encode(v, depth + 1)?)))
                .collect::<Result<_, String>>()?,
        ),
        Value::Map(values, _) => Json::Object(
            values
                .entries()
                .iter()
                .map(|(k, v)| Ok((text(k)?.into(), encode(v, depth + 1)?)))
                .collect::<Result<_, String>>()?,
        ),
        _ => return Err(
            "unsupported Dev value for JSON; use JSON constructors or scalar/array/struct values"
                .into(),
        ),
    })
}
pub(crate) enum Access {
    Key(String),
    Index(usize),
}
pub(crate) fn access(value: Value) -> Result<Access, String> {
    match value {
        Value::Str(key) => Ok(Access::Key(key)),
        Value::Int(index, _) => Ok(Access::Index(
            usize::try_from(index).map_err(|_| "invalid JSON index")?,
        )),
        _ => Err("JSON index must be str or integer".into()),
    }
}
pub(crate) fn read<'a>(value: &'a Json, path: &[Access]) -> Result<&'a Json, String> {
    let mut value = value;
    for step in path {
        value = match step {
            Access::Key(key) => value
                .as_object()
                .ok_or("JSON field access requires object")?
                .get(key)
                .ok_or_else(|| format!("missing JSON key '{key}'"))?,
            Access::Index(index) => value
                .as_array()
                .ok_or("JSON integer index requires array")?
                .get(*index)
                .ok_or("JSON index out of bounds")?,
        };
    }
    Ok(value)
}
pub(crate) fn write(value: &mut Json, path: &[Access], replacement: Json) -> Result<(), String> {
    let (last, parents) = path.split_last().ok_or("empty JSON assignment path")?;
    let mut dest = &mut *value;
    for step in parents {
        dest = match step {
            Access::Key(key) => dest
                .as_object_mut()
                .ok_or("JSON field access requires object")?
                .get_mut(key)
                .ok_or_else(|| format!("missing JSON key '{key}'"))?,
            Access::Index(index) => dest
                .as_array_mut()
                .ok_or("JSON integer index requires array")?
                .get_mut(*index)
                .ok_or("JSON index out of bounds")?,
        };
    }
    match last {
        Access::Key(key) => {
            dest.as_object_mut()
                .ok_or("JSON field access requires object")?
                .insert(key.clone(), replacement);
        }
        Access::Index(index) => {
            *dest
                .as_array_mut()
                .ok_or("JSON integer index requires array")?
                .get_mut(*index)
                .ok_or("JSON index out of bounds")? = replacement;
        }
    }
    check_depth(value, 0)
}
pub(crate) fn unwrap(value: Json, ty: Type) -> Value {
    match value {
        Json::Null => Value::Ptr(0, Type::Ptr(Box::new(Type::Void))),
        Json::Bool(v) => Value::Bool(v),
        Json::String(v) => Value::Str(v),
        Json::Number(ref n) if n.as_i64().is_some() => {
            Value::Int(n.as_i64().unwrap() as i128, Type::i64())
        }
        Json::Number(ref n) if n.as_u64().is_some() => Value::Int(
            n.as_u64().unwrap() as i128,
            Type::Int {
                bits: 64,
                signed: false,
            },
        ),
        Json::Number(ref n)
            if n.to_string().contains(['.', 'e', 'E'])
                && n.as_f64().is_some_and(|v| v.is_finite()) =>
        {
            Value::Float(n.as_f64().unwrap(), Type::Float(64))
        }
        value => Value::Json(Arc::new(value), ty),
    }
}
fn node(value: &Value) -> Result<&Json, String> {
    if let Value::Json(value, _) = value {
        Ok(value)
    } else {
        Err("expected json.Value".into())
    }
}
fn text(value: &Value) -> Result<&str, String> {
    if let Value::Str(value) = value {
        Ok(value)
    } else {
        Err("expected str".into())
    }
}
pub(crate) fn call(name: &str, args: Vec<Value>, ty: Type) -> Result<Value, String> {
    let count = match name {
        "object" | "array" | "null_value" => 0,
        "get" | "has" | "at" | "push" | "remove" => 2,
        "set" => 3,
        "parse" | "valid" | "stringify" | "pretty" | "value" | "kind" | "len" | "keys"
        | "string" | "int" | "uint" | "float" | "bool" | "is_null" => 1,
        _ => return Err(format!("unknown intrinsic std/json.{name}")),
    };
    if args.len() != count {
        return Err(format!("json.{name} expects {count} arguments"));
    }
    let wrap = |value| Value::Json(Arc::new(value), ty.clone());
    match name {
        "parse" => Ok(wrap(
            serde_json::from_str(text(&args[0])?).map_err(|e| format!("invalid JSON: {e}"))?,
        )),
        "valid" => Ok(Value::Bool(
            serde_json::from_str::<Json>(text(&args[0])?).is_ok(),
        )),
        "value" => Ok(wrap(encode(&args[0], 0)?)),
        "stringify" | "pretty" => {
            let value = encode(&args[0], 0)?;
            Ok(Value::Str(
                if name == "pretty" {
                    serde_json::to_string_pretty(&value)
                } else {
                    serde_json::to_string(&value)
                }
                .map_err(|e| e.to_string())?,
            ))
        }
        "object" => Ok(wrap(Json::Object(Default::default()))),
        "array" => Ok(wrap(Json::Array(vec![]))),
        "null_value" => Ok(wrap(Json::Null)),
        "kind" => Ok(Value::Str(
            match node(&args[0])? {
                Json::Null => "null",
                Json::Bool(_) => "bool",
                Json::Number(_) => "number",
                Json::String(_) => "string",
                Json::Array(_) => "array",
                Json::Object(_) => "object",
            }
            .into(),
        )),
        "is_null" => Ok(Value::Bool(node(&args[0])?.is_null())),
        "len" => Ok(Value::Int(
            match node(&args[0])? {
                Json::Array(v) => v.len(),
                Json::Object(v) => v.len(),
                _ => return Err("JSON len requires object or array".into()),
            } as i128,
            Type::Size { signed: false },
        )),
        "keys" => {
            let keys = node(&args[0])?
                .as_object()
                .ok_or("JSON keys requires object")?
                .keys()
                .map(|k| Value::Str(k.clone()))
                .collect();
            Ok(Value::Vector(
                Arc::new(keys),
                Type::Vector(Box::new(Type::Str)),
            ))
        }
        "get" | "has" => {
            let object = node(&args[0])?
                .as_object()
                .ok_or("JSON get/has requires object")?;
            let key = text(&args[1])?;
            if name == "has" {
                Ok(Value::Bool(object.contains_key(key)))
            } else {
                Ok(wrap(
                    object
                        .get(key)
                        .ok_or_else(|| format!("missing JSON key '{key}'"))?
                        .clone(),
                ))
            }
        }
        "at" => {
            let Value::Int(index, _) = args[1] else {
                return Err("JSON index must be integer".into());
            };
            let index = usize::try_from(index).map_err(|_| "invalid JSON index")?;
            Ok(wrap(
                node(&args[0])?
                    .as_array()
                    .ok_or("JSON at requires array")?
                    .get(index)
                    .ok_or("JSON index out of bounds")?
                    .clone(),
            ))
        }
        "set" | "push" | "remove" => {
            let mut value = node(&args[0])?.clone();
            match name {
                "set" => {
                    value
                        .as_object_mut()
                        .ok_or("JSON set requires object")?
                        .insert(text(&args[1])?.into(), encode(&args[2], 0)?);
                }
                "remove" => {
                    value
                        .as_object_mut()
                        .ok_or("JSON remove requires object")?
                        .remove(text(&args[1])?);
                }
                _ => {
                    value
                        .as_array_mut()
                        .ok_or("JSON push requires array")?
                        .push(encode(&args[1], 0)?);
                }
            }
            check_depth(&value, 0)?;
            Ok(wrap(value))
        }
        "string" => Ok(Value::Str(
            node(&args[0])?
                .as_str()
                .ok_or("expected JSON string")?
                .into(),
        )),
        "bool" => Ok(Value::Bool(
            node(&args[0])?.as_bool().ok_or("expected JSON bool")?,
        )),
        "int" => Ok(Value::Int(
            node(&args[0])?
                .as_i64()
                .ok_or("expected JSON i64 integer")? as i128,
            Type::i64(),
        )),
        "uint" => Ok(Value::Int(
            node(&args[0])?
                .as_u64()
                .ok_or("expected JSON u64 integer")? as i128,
            Type::Int {
                bits: 64,
                signed: false,
            },
        )),
        "float" => Ok(Value::Float(
            node(&args[0])?
                .as_f64()
                .filter(|n| n.is_finite())
                .ok_or("expected finite JSON number representable as f64")?,
            Type::Float(64),
        )),
        _ => unreachable!(),
    }
}
