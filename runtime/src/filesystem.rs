use crate::engine::Value;
use dev_syntax::ast::Type;
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::Arc,
};

pub(crate) const MAX_BYTES: usize = 8 * 1024 * 1024;
pub(crate) fn byte_type() -> Type {
    Type::Int {
        bits: 8,
        signed: false,
    }
}
pub(crate) fn bytes_value(bytes: Vec<u8>) -> Value {
    let t = byte_type();
    Value::Vector(
        Arc::new(
            bytes
                .into_iter()
                .map(|b| Value::Int(b as i128, t.clone()))
                .collect(),
        ),
        Type::Vector(Box::new(t)),
    )
}
pub(crate) fn bytes(value: &Value) -> Result<Vec<u8>, String> {
    let Value::Vector(values, Type::Vector(t)) = value else {
        return Err("expected Vec<u8>".into());
    };
    if **t != byte_type() {
        return Err("expected Vec<u8>".into());
    }
    if values.len() > MAX_BYTES {
        return Err("byte buffer exceeds 8 MiB".into());
    }
    values
        .iter()
        .map(|v| match v {
            Value::Int(n, _) => u8::try_from(*n).map_err(|_| "byte out of range".into()),
            _ => Err("expected u8 element".into()),
        })
        .collect()
}
fn read(path: &str) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES as u64 {
        return Err("file exceeds 8 MiB; streaming reads are not yet supported".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES {
        return Err("file exceeds 8 MiB".into());
    }
    Ok(bytes)
}
fn path_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "path is not valid Unicode".into())
}
pub(crate) fn call(name: &str, args: Vec<Value>) -> Result<Value, String> {
    let count = match name {
        "current_dir" => 0,
        "write_text" | "append_text" | "write_bytes" | "copy" | "rename" | "join" => 2,
        "read_text" | "read_bytes" | "exists" | "is_file" | "is_dir" | "create_dir"
        | "create_dirs" | "remove_file" | "remove_dir" | "read_dir" | "size" => 1,
        _ => return Err(format!("unknown intrinsic std/fs.{name}")),
    };
    if args.len() != count {
        return Err(format!("fs.{name} expects {count} arguments"));
    }
    let result = (|| {
        let p = if count > 0 { args[0].string()? } else { "" };
        match name {
            "read_text" => Ok(Value::Str(
                String::from_utf8(read(p)?).map_err(|_| "file is not valid UTF-8")?,
            )),
            "read_bytes" => Ok(bytes_value(read(p)?)),
            "write_text" | "append_text" => {
                let text = args[1].string()?;
                if text.len() > MAX_BYTES {
                    return Err("text exceeds 8 MiB".into());
                }
                if name == "write_text" {
                    fs::write(p, text).map_err(|e| e.to_string())?;
                } else {
                    let mut file = fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(p)
                        .map_err(|e| e.to_string())?;
                    file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
                }
                Ok(Value::Bool(true))
            }
            "write_bytes" => {
                fs::write(p, bytes(&args[1])?).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "exists" => Ok(Value::Bool(
                Path::new(p).try_exists().map_err(|e| e.to_string())?,
            )),
            "is_file" | "is_dir" => match fs::metadata(p) {
                Ok(m) => Ok(Value::Bool(if name == "is_file" {
                    m.is_file()
                } else {
                    m.is_dir()
                })),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::Bool(false)),
                Err(e) => Err(e.to_string()),
            },
            "create_dir" | "create_dirs" => {
                if name == "create_dir" {
                    fs::create_dir(p)
                } else {
                    fs::create_dir_all(p)
                }
                .map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "remove_file" => {
                fs::remove_file(p).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "remove_dir" => {
                fs::remove_dir(p).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "copy" => {
                fs::copy(p, args[1].string()?).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "rename" => {
                fs::rename(p, args[1].string()?).map_err(|e| e.to_string())?;
                Ok(Value::Bool(true))
            }
            "size" => {
                let size = i64::try_from(fs::metadata(p).map_err(|e| e.to_string())?.len())
                    .map_err(|_| "file size exceeds i64")?;
                Ok(Value::Int(size as i128, Type::i64()))
            }
            "current_dir" => Ok(Value::Str(path_string(
                &std::env::current_dir().map_err(|e| e.to_string())?,
            )?)),
            "join" => Ok(Value::Str(path_string(
                &Path::new(p).join(args[1].string()?),
            )?)),
            "read_dir" => {
                let mut entries = fs::read_dir(p)
                    .map_err(|e| e.to_string())?
                    .map(|e| {
                        path_string(&std::path::PathBuf::from(
                            e.map_err(|e| e.to_string())?.file_name(),
                        ))
                    })
                    .collect::<Result<Vec<String>, String>>()?;
                entries.sort();
                Ok(Value::Vector(
                    Arc::new(entries.into_iter().map(Value::Str).collect()),
                    Type::Vector(Box::new(Type::Str)),
                ))
            }
            _ => unreachable!(),
        }
    })();
    result.map_err(|e| format!("fs.{name}: {e}"))
}
