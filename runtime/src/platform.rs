use crate::engine::Value;
use dev_syntax::ast::Type;
use std::path::{Component, Path, PathBuf};

pub(crate) fn text(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or("path is not valid Unicode".into())
}
pub(crate) fn normalize(input: &str) -> Result<String, String> {
    let mut result = PathBuf::new();
    let mut parts: Vec<std::ffi::OsString> = Vec::new();
    for component in Path::new(input).components() {
        match component {
            Component::Prefix(_) | Component::RootDir => result.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.last().is_some_and(|p| p != "..") {
                    parts.pop();
                } else if !result.has_root() {
                    parts.push("..".into());
                }
            }
            Component::Normal(p) => parts.push(p.to_owned()),
        }
    }
    for part in parts {
        result.push(part);
    }
    if result.as_os_str().is_empty() {
        return Ok(".".into());
    }
    let mut output = text(&result)?;
    if input.ends_with(std::path::MAIN_SEPARATOR) || (cfg!(windows) && input.ends_with('/')) {
        if !output.ends_with(std::path::MAIN_SEPARATOR) {
            output.push(std::path::MAIN_SEPARATOR);
        }
    }
    Ok(output)
}
fn separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}
fn basename(input: &str) -> String {
    input
        .trim_end_matches(separator)
        .rsplit(separator)
        .next()
        .unwrap_or("")
        .into()
}
fn dirname(input: &str) -> Result<String, String> {
    let path = Path::new(input);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => text(parent),
        _ if path.has_root() => text(path),
        _ => Ok(".".into()),
    }
}
fn extname(input: &str) -> String {
    let base = basename(input);
    if base == "." || base == ".." {
        return String::new();
    }
    base.rfind('.')
        .filter(|i| *i > 0)
        .map(|i| base[i..].into())
        .unwrap_or_default()
}
pub(crate) fn absolute(input: &str) -> Result<PathBuf, String> {
    let path = Path::new(input);
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(path)
    };
    Ok(PathBuf::from(normalize(&text(&path)?)?))
}
pub(crate) fn field<'a>(value: &'a Value, name: &str) -> Result<&'a Value, String> {
    let Value::Record(fields, _) = value else {
        return Err("expected a record".into());
    };
    fields
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v)
        .ok_or_else(|| format!("missing field {name}"))
}
pub(crate) fn record(ty: Type, fields: Vec<(&str, Value)>) -> Value {
    Value::Record(fields.into_iter().map(|(n, v)| (n.into(), v)).collect(), ty)
}
pub(crate) fn strings(items: Vec<String>) -> Value {
    Value::Vector(
        std::sync::Arc::new(items.into_iter().map(Value::Str).collect()),
        Type::Vector(Box::new(Type::Str)),
    )
}
pub(crate) fn path_call(name: &str, args: &[Value], ret: Type) -> Result<Value, String> {
    let s = |i: usize| args[i].string();
    let value = match name {
        "sep" => std::path::MAIN_SEPARATOR.to_string(),
        "delimiter" => if cfg!(windows) { ";" } else { ":" }.into(),
        "normalize" => normalize(s(0)?)?,
        "basename" => basename(s(0)?),
        "dirname" => dirname(s(0)?)?,
        "extname" => extname(s(0)?),
        "isAbsolute" => return Ok(Value::Bool(Path::new(s(0)?).is_absolute())),
        "join" => join(s(0)?, s(1)?)?,
        "joinMany" => {
            let Value::Vector(items, _) = &args[0] else {
                unreachable!()
            };
            let mut path = String::new();
            for item in items.iter() {
                path = join(&path, item.string()?)?;
            }
            normalize(&path)?
        }
        "resolve" => {
            let absolute = absolute(&text(&absolute(s(0)?)?.join(s(1)?))?)?;
            text(&absolute.components().collect::<PathBuf>())?
        }
        "relative" => {
            let from = absolute(s(0)?)?;
            let to = absolute(s(1)?)?;
            text(&pathdiff::diff_paths(&to, &from).unwrap_or(to))?
        }
        "parse" => {
            let input = s(0)?;
            let path = Path::new(input);
            let root: PathBuf = path
                .components()
                .take_while(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
                .collect();
            let base = basename(input);
            let ext = extname(input);
            let name = base.strip_suffix(&ext).unwrap_or(&base).to_owned();
            return Ok(record(
                ret,
                vec![
                    ("root", Value::Str(text(&root)?)),
                    (
                        "dir",
                        Value::Str(
                            if path.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
                                dirname(input)?
                            } else if path.has_root() {
                                text(&root)?
                            } else {
                                String::new()
                            },
                        ),
                    ),
                    ("base", Value::Str(base)),
                    ("ext", Value::Str(ext)),
                    ("name", Value::Str(name)),
                ],
            ));
        }
        "format" => {
            let get = |n| field(&args[0], n)?.string();
            let dir = get("dir")?;
            let dir = if dir.is_empty() { get("root")? } else { dir };
            let base = get("base")?;
            let base = if base.is_empty() {
                format!("{}{}", get("name")?, get("ext")?)
            } else {
                base.into()
            };
            if dir.is_empty() {
                base
            } else {
                text(&Path::new(dir).join(base))?
            }
        }
        _ => return Err("unknown path API".into()),
    };
    Ok(Value::Str(value))
}
fn join(base: &str, child: &str) -> Result<String, String> {
    if base.is_empty() {
        return normalize(child);
    }
    if child.is_empty() {
        return normalize(base);
    }
    normalize(&format!(
        "{}{}{}",
        base,
        std::path::MAIN_SEPARATOR,
        child.trim_start_matches(separator)
    ))
}
pub(crate) fn platform() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        std::env::consts::OS
    }
}
pub(crate) fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x64",
        "x86" => "ia32",
        "aarch64" => "arm64",
        arch => arch,
    }
}
pub(crate) fn os_call(name: &str) -> Result<Value, String> {
    let string = match name {
        "platform" => platform().into(),
        "arch" => arch().into(),
        "hostname" => sysinfo::System::host_name().ok_or("hostname unavailable")?,
        "release" => sysinfo::System::kernel_version().ok_or("kernel version unavailable")?,
        "type" => {
            if cfg!(windows) {
                "Windows_NT".into()
            } else {
                sysinfo::System::name().ok_or("OS name unavailable")?
            }
        }
        "homedir" => std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .map_err(|_| "home directory environment unavailable")?,
        "tmpdir" => text(&std::env::temp_dir())?,
        "endianness" => if cfg!(target_endian = "little") {
            "LE"
        } else {
            "BE"
        }
        .into(),
        "EOL" => if cfg!(windows) { "\r\n" } else { "\n" }.into(),
        "availableParallelism" => {
            return Ok(Value::Int(
                std::thread::available_parallelism()
                    .map_err(|e| e.to_string())?
                    .get() as i128,
                Type::i64(),
            ))
        }
        "uptime" => return Ok(Value::Int(sysinfo::System::uptime() as i128, Type::i64())),
        "totalmem" | "freemem" => {
            let mut system = sysinfo::System::new();
            system.refresh_memory();
            return Ok(Value::Int(
                if name == "totalmem" {
                    system.total_memory()
                } else {
                    system.free_memory()
                } as i128,
                Type::i64(),
            ));
        }
        _ => return Err("unknown os API".into()),
    };
    Ok(Value::Str(string))
}
pub(crate) fn url_call(name: &str, args: &[Value], ret: Type) -> Result<Value, String> {
    let parse = |s| url::Url::parse(s).map_err(|e| e.to_string());
    let s = |i: usize| args[i].string();
    let parsed = match name {
        "parse" => parse(s(0)?)?,
        "resolve" => parse(s(0)?)?.join(s(1)?).map_err(|e| e.to_string())?,
        "pathToFileURL" => {
            return Ok(Value::Str(
                url::Url::from_file_path(absolute(s(0)?)?)
                    .map_err(|_| "invalid file path")?
                    .to_string(),
            ))
        }
        "fileURLToPath" => {
            return Ok(Value::Str(text(
                &parse(s(0)?)?
                    .to_file_path()
                    .map_err(|_| "expected a local file URL")?,
            )?))
        }
        "domainToASCII" => {
            let domain = s(0)?;
            if domain.is_empty() || domain.contains(['/', ':', '?', '#', '@']) {
                return Err("invalid domain".into());
            }
            return Ok(Value::Str(
                parse(&format!("http://{domain}/"))?
                    .host_str()
                    .ok_or("missing domain")?
                    .into(),
            ));
        }
        "format" | "method_URL_toString" => {
            let get = |n| field(&args[0], n)?.string();
            if name == "method_URL_toString" {
                return Ok(Value::Str(get("href")?.into()));
            }
            let mut value = parse(get("href")?)?;
            value
                .set_scheme(get("protocol")?.trim_end_matches(':'))
                .map_err(|_| "invalid URL scheme")?;
            value
                .set_host(Some(get("hostname")?))
                .map_err(|e| e.to_string())?;
            let port = get("port")?;
            if !port.is_empty() || value.port().is_some() {
                value
                    .set_port(if port.is_empty() {
                        None
                    } else {
                        Some(port.parse::<u16>().map_err(|_| "invalid port")?)
                    })
                    .map_err(|_| "URL does not support a port")?;
            }
            if !get("username")?.is_empty() || !value.username().is_empty() {
                value
                    .set_username(get("username")?)
                    .map_err(|_| "URL does not support username")?;
            }
            if !get("password")?.is_empty() || value.password().is_some() {
                value
                    .set_password(if get("password")?.is_empty() {
                        None
                    } else {
                        Some(get("password")?)
                    })
                    .map_err(|_| "URL does not support password")?;
            }
            value.set_path(get("pathname")?);
            let query = get("search")?;
            value.set_query(if query.is_empty() {
                None
            } else {
                Some(query.trim_start_matches('?'))
            });
            let hash = get("hash")?;
            value.set_fragment(if hash.is_empty() {
                None
            } else {
                Some(hash.trim_start_matches('#'))
            });
            return Ok(Value::Str(value.to_string()));
        }
        _ => return Err("unknown URL API".into()),
    };
    let hostname = parsed.host_str().unwrap_or("").to_owned();
    let host = match parsed.port() {
        Some(p) => format!("{hostname}:{p}"),
        None => hostname.clone(),
    };
    Ok(record(
        ret,
        vec![
            ("href", Value::Str(parsed.to_string())),
            ("protocol", Value::Str(format!("{}:", parsed.scheme()))),
            ("host", Value::Str(host)),
            ("hostname", Value::Str(hostname)),
            (
                "port",
                Value::Str(parsed.port().map(|p| p.to_string()).unwrap_or_default()),
            ),
            ("pathname", Value::Str(parsed.path().into())),
            (
                "search",
                Value::Str(parsed.query().map(|q| format!("?{q}")).unwrap_or_default()),
            ),
            (
                "hash",
                Value::Str(
                    parsed
                        .fragment()
                        .map(|q| format!("#{q}"))
                        .unwrap_or_default(),
                ),
            ),
            ("origin", Value::Str(parsed.origin().ascii_serialization())),
            ("username", Value::Str(parsed.username().into())),
            (
                "password",
                Value::Str(parsed.password().unwrap_or("").into()),
            ),
        ],
    ))
}
