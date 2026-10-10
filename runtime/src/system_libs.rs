struct CliOption {
    name: String,
    short: Option<char>,
    takes_value: bool,
}
fn cli_texts(value: &Value) -> Result<Vec<String>, String> {
    let Value::Vector(items, _) = value else {
        return Err("expected Vec<str>".into());
    };
    if items.len() > 4096 {
        return Err("CLI arguments exceed 4096 items".into());
    }
    let mut bytes = 0usize;
    let mut output = Vec::with_capacity(items.len());
    for item in items.iter() {
        let text = item.string()?;
        bytes = bytes.saturating_add(text.len());
        if bytes > BASIC_LIMIT {
            return Err("CLI arguments exceed 8 MiB".into());
        }
        if text.contains('\0') {
            return Err("CLI arguments cannot contain NUL".into());
        }
        output.push(text.to_owned());
    }
    Ok(output)
}
fn cli_store(
    option: &CliOption,
    value: Option<&str>,
    seen: &mut std::collections::HashSet<String>,
    values: &mut MapStorage,
    flags: &mut MapStorage,
) -> Result<(), String> {
    if !seen.insert(option.name.clone()) {
        return Err(format!("duplicate option --{}", option.name));
    }
    if option.takes_value {
        values.set(
            Value::Str(option.name.clone()),
            Value::Str(
                value
                    .ok_or_else(|| format!("option --{} requires a value", option.name))?
                    .to_owned(),
            ),
        )?;
    } else {
        if value.is_some() {
            return Err(format!("flag --{} does not accept a value", option.name));
        }
        flags.set(Value::Str(option.name.clone()), Value::Bool(true))?;
    }
    Ok(())
}
impl Engine {
    fn dns_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use std::net::{IpAddr, ToSocketAddrs};
        let host = args[0].string()?;
        let literal = host.parse::<IpAddr>().ok();
        let family = literal.map_or(0, |ip| if ip.is_ipv4() { 4 } else { 6 });
        match name {
            "isIP" => return Ok(Value::Int(family, Type::i64())),
            "isIPv4" => return Ok(Value::Bool(family == 4)),
            "isIPv6" => return Ok(Value::Bool(family == 6)),
            _ => {}
        }
        let requested = Self::basic_i64(&args[1])?;
        if ![0, 4, 6].contains(&requested) {
            return Err("DNS family must be 0, 4 or 6".into());
        }
        // Numeric literals avoid the system resolver entirely. Hostnames use the
        // OS resolver (including its hosts file and policy), not a custom DNS client.
        if literal.is_none() {
            if host.is_empty() || host.len() > 253 {
                return Err("DNS hostname must contain 1..253 bytes".into());
            }
            let labels = host.strip_suffix('.').unwrap_or(host).split('.');
            if !labels.clone().all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            }) {
                return Err("invalid DNS hostname; use ASCII hostname or unbracketed IP".into());
            }
        }
        let addresses = if let Some(ip) = literal {
            Ok(vec![std::net::SocketAddr::new(ip, 0)].into_iter())
        } else {
            (host, 0u16).to_socket_addrs()
        }
        .map_err(|e| format!("DNS lookup failed: {e}"))?;
        let mut seen = std::collections::HashSet::new();
        let mut items = Vec::new();
        for address in addresses {
            let ip = address.ip();
            let family = if ip.is_ipv4() { 4 } else { 6 };
            if (requested == 0 || requested == family) && seen.insert(ip) {
                if items.len() >= 256 {
                    return Err("DNS results exceed 256 addresses".into());
                }
                items.push(ip);
            }
        }
        match name {
            "lookup" => Ok(crate::platform::strings(
                items.iter().map(ToString::to_string).collect(),
            )),
            "lookupOne" => {
                let ip = items
                    .first()
                    .ok_or("DNS lookup returned no matching address")?;
                Ok(crate::platform::record(
                    ret,
                    vec![
                        ("address", Value::Str(ip.to_string())),
                        (
                            "family",
                            Value::Int(if ip.is_ipv4() { 4 } else { 6 }, Type::i64()),
                        ),
                    ],
                ))
            }
            _ => Err("unknown DNS API".into()),
        }
    }
    fn cli_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        if name == "args" {
            let value = crate::platform::strings(self.args.clone());
            cli_texts(&value)?;
            return Ok(value);
        }
        if name != "parse" {
            return Err("unknown CLI API".into());
        }
        let input = cli_texts(&args[0])?;
        let Value::Vector(definitions, _) = &args[1] else {
            return Err("expected Vec<cli.Option>".into());
        };
        if definitions.len() > 128 {
            return Err("CLI schema exceeds 128 options".into());
        }
        let mut options = Vec::new();
        let mut long = HashMap::new();
        let mut short = HashMap::new();
        for definition in definitions.iter() {
            let name = crate::platform::field(definition, "name")?.string()?;
            if name.is_empty()
                || name.len() > 64
                || !name.as_bytes()[0].is_ascii_alphabetic()
                || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                return Err("CLI option name must start with a letter and contain at most 64 ASCII letters, digits or hyphens".into());
            }
            let alias = crate::platform::field(definition, "short")?.string()?;
            let alias = if alias.is_empty() {
                None
            } else {
                if alias.len() != 1 || !alias.as_bytes()[0].is_ascii_alphabetic() {
                    return Err("CLI short option must be empty or one ASCII letter".into());
                }
                Some(alias.as_bytes()[0] as char)
            };
            let Value::Bool(takes_value) = crate::platform::field(definition, "takesValue")? else {
                return Err("CLI takesValue must be bool".into());
            };
            if long.insert(name.to_owned(), options.len()).is_some() {
                return Err(format!("duplicate CLI definition --{name}"));
            }
            if let Some(alias) = alias {
                if short.insert(alias, options.len()).is_some() {
                    return Err(format!("duplicate CLI short option -{alias}"));
                }
            }
            options.push(CliOption {
                name: name.into(),
                short: alias,
                takes_value: *takes_value,
            });
        }
        let mut values = MapStorage::default();
        let mut flags = MapStorage::default();
        let mut seen = std::collections::HashSet::new();
        let mut positionals = Vec::new();
        let mut index = 0usize;
        while index < input.len() {
            let token = &input[index];
            if token == "--" {
                positionals.extend(input[index + 1..].iter().cloned());
                break;
            } else if let Some(text) = token.strip_prefix("--") {
                let (key, attached) = text
                    .split_once('=')
                    .map_or((text, None), |(k, v)| (k, Some(v)));
                let option = &options[*long
                    .get(key)
                    .ok_or_else(|| format!("unknown option --{key}"))?];
                let value = if option.takes_value && attached.is_none() {
                    index += 1;
                    let value = input
                        .get(index)
                        .filter(|s| *s != "--")
                        .ok_or_else(|| format!("option --{key} requires a value"))?;
                    Some(value.as_str())
                } else {
                    attached
                };
                cli_store(option, value, &mut seen, &mut values, &mut flags)?;
            } else if token.starts_with('-') && token != "-" {
                let text = &token[1..];
                for (offset, alias) in text.char_indices() {
                    let option = &options[*short
                        .get(&alias)
                        .ok_or_else(|| format!("unknown option -{alias}"))?];
                    debug_assert_eq!(option.short, Some(alias));
                    if option.takes_value {
                        let tail = &text[offset + alias.len_utf8()..];
                        let value = if tail.is_empty() {
                            index += 1;
                            input
                                .get(index)
                                .filter(|s| *s != "--")
                                .ok_or_else(|| {
                                    format!("option --{} requires a value", option.name)
                                })?
                                .as_str()
                        } else {
                            tail.strip_prefix('=').unwrap_or(tail)
                        };
                        cli_store(option, Some(value), &mut seen, &mut values, &mut flags)?;
                        break;
                    } else {
                        cli_store(option, None, &mut seen, &mut values, &mut flags)?;
                    }
                }
            } else {
                positionals.push(token.clone());
            }
            index += 1;
        }
        Ok(crate::platform::record(
            ret,
            vec![
                (
                    "values",
                    Value::Map(
                        Arc::new(values),
                        Type::Map(Box::new(Type::Str), Box::new(Type::Str)),
                    ),
                ),
                (
                    "flags",
                    Value::Map(
                        Arc::new(flags),
                        Type::Map(Box::new(Type::Str), Box::new(Type::Bool)),
                    ),
                ),
                ("positionals", crate::platform::strings(positionals)),
            ],
        ))
    }
}
