#[derive(Default)]
struct CoreState {
    regexes: HashMap<i64, regex::Regex>,
    random: Option<u64>,
    tests: Vec<(String, Value)>,
    testing: bool,
    log_level: Option<u8>,
    log_file: Option<std::fs::File>,
    scratch: Vec<u8>,
    buffers: HashMap<i64, Vec<u8>>,
    events: HashMap<i64, Vec<CoreListener>>,
    queries: HashMap<i64, Vec<(String, String)>>,
    readers: HashMap<i64, std::fs::File>,
    writers: HashMap<i64, std::fs::File>,
    tcp: HashMap<i64, CoreTcp>,
    tcp_servers: HashMap<i64, CoreTcpServer>,
    udp: HashMap<i64, CoreUdp>,
}
#[derive(Clone)]
struct CoreListener {
    id: i64,
    event: String,
    once: bool,
    callback: Value,
}
impl Engine {
    fn core_capacity(&self) -> Result<(), String> {
        let c = &self.core;
        if c.buffers.len()
            + c.events.len()
            + c.queries.len()
            + c.readers.len()
            + c.writers.len()
            + c.tcp.len()
            + c.tcp_servers.len()
            + c.udp.len()
            + c.regexes.len()
            >= 256
        {
            Err("at most 256 core resource handles per interpreter; close unused resources".into())
        } else {
            Ok(())
        }
    }
    fn core_handle(id: i64, ret: Type) -> Value {
        crate::platform::record(ret, vec![("id", Value::Int(id as i128, Type::i64()))])
    }
    fn core_int(v: &Value) -> Result<i128, String> {
        if let Value::Int(n, _) = v {
            Ok(*n)
        } else {
            Err("expected integer".into())
        }
    }
    fn core_size(v: &Value, max: usize) -> Result<usize, String> {
        let n = usize::try_from(Self::core_int(v)?).map_err(|_| "negative or excessive size")?;
        if n > max {
            Err(format!("size exceeds {max}"))
        } else {
            Ok(n)
        }
    }
    fn invoke_core(&mut self, callback: Value, args: Vec<Value>) -> Result<Value, String> {
        let Value::Callable(module, name, context, Type::Function(params, _)) = callback else {
            return Err("expected callback".into());
        };
        if params.len() != args.len() {
            return Err("wrong callback argument count".into());
        }
        let mut values = vec![];
        if let Some(v) = context {
            let inner = match v.ty() {
                Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                t => t,
            };
            values.push(Value::Ref(v, Type::Ref(Box::new(inner))));
        }
        values.extend(args);
        self.call(&module, &[name], values, Span { line: 1, col: 1 })
    }
    fn core_call(&mut self, module: &str, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let function = self.modules[module]
            .functions
            .get(name)
            .ok_or("unknown core API")?
            .clone();
        if args.len() != function.params.len() {
            return Err(format!("expects {} arguments", function.params.len()));
        }
        let args = args
            .into_iter()
            .zip(&function.params)
            .map(|(v, (_, t))| convert(v, t, false))
            .collect::<Result<Vec<_>, _>>()?;
        let ret = function.ret.clone();
        match module {
            "std/regex" => self.regex_call(name, args, ret),
            "std/encoding" => self.encoding_call(name, args),
            "std/crypto" => self.crypto_call(name, args),
            "std/compression" => self.compression_call(name, args),
            "std/archive" => self.archive_call(name, args),
            "std/uuid" => self.uuid_call(name, args),
            "std/math" => self.math_call(name, args),
            "std/random" => self.random_call(name, args),
            "std/strings" => self.strings_call(name, args),
            "std/datetime" => self.datetime_call(name, args, ret),
            "std/test" => self.test_call(name, args, ret),
            "std/log" => self.log_call(name, args),
            "std/path" => crate::platform::path_call(name, &args, ret),
            "std/os" => crate::platform::os_call(name),
            "std/url" if name == "searchParams" || name.starts_with("method_URLSearchParams_") => {
                self.query_call(name, args, ret)
            }
            "std/url" => crate::platform::url_call(name, &args, ret),
            "std/buffer" => self.buffer_call(name, args, ret),
            "std/events" => self.events_call(name, args, ret),
            "std/stream" => self.stream_call(name, args, ret),
            "std/net" => self.tcp_call(name, args, ret),
            "std/dgram" => self.udp_call(name, args, ret),
            "std/module" => {
                use dev_syntax::intrinsics::BUILTINS;
                let canonical = |s: &str| {
                    if s.starts_with("std/") {
                        s.to_owned()
                    } else {
                        format!("std/{s}")
                    }
                };
                match name {
                    "builtinModules" => Ok(crate::platform::strings(
                        BUILTINS.iter().map(|s| s.to_string()).collect(),
                    )),
                    "isBuiltin" => Ok(Value::Bool(
                        BUILTINS.contains(&canonical(args[0].string()?).as_str()),
                    )),
                    "entry" => Ok(Value::Str(self.entry.clone())),
                    "loaded" => {
                        let mut names = self.modules.keys().cloned().collect::<Vec<_>>();
                        names.sort();
                        Ok(crate::platform::strings(names))
                    }
                    "resolve" => {
                        let spec = args[0].string()?;
                        if BUILTINS.contains(&spec) {
                            return Ok(Value::Str(spec.into()));
                        }
                        let resolved = dev_syntax::modules::resolve(
                            Path::new(args[1].string()?),
                            spec,
                            &self.module_dirs,
                        )?;
                        Ok(Value::Str(crate::platform::text(
                            &resolved.canonicalize().map_err(|e| e.to_string())?,
                        )?))
                    }
                    _ => Err("unknown module API".into()),
                }
            }
            "std/process" => match name {
                "cwd" => Ok(Value::Str(crate::platform::text(
                    &std::env::current_dir().map_err(|e| e.to_string())?,
                )?)),
                "chdir" => {
                    std::env::set_current_dir(args[0].string()?).map_err(|e| e.to_string())?;
                    Ok(Value::Bool(true))
                }
                "pid" => Ok(Value::Int(std::process::id() as i128, Type::i64())),
                "execPath" => Ok(Value::Str(crate::platform::text(
                    &std::env::current_exe().map_err(|e| e.to_string())?,
                )?)),
                "argv" => {
                    let mut argv = vec![
                        crate::platform::text(
                            &std::env::current_exe().map_err(|e| e.to_string())?,
                        )?,
                        self.entry.clone(),
                    ];
                    argv.extend(self.args.clone());
                    Ok(crate::platform::strings(argv))
                }
                "env" => {
                    let mut map = MapStorage::default();
                    for (key, val) in std::env::vars_os() {
                        if let (Ok(key), Ok(val)) = (key.into_string(), val.into_string()) {
                            map.set(Value::Str(key), Value::Str(val))?;
                        }
                    }
                    Ok(Value::Map(Arc::new(map), ret))
                }
                "getenv" => Ok(Value::Str(
                    std::env::var(args[0].string()?).unwrap_or_default(),
                )),
                "hasEnv" => Ok(Value::Bool(std::env::var_os(args[0].string()?).is_some())),
                "platform" | "arch" => crate::platform::os_call(name),
                "uptime" => Ok(Value::Float(
                    self.start.elapsed().as_secs_f64(),
                    Type::Float(64),
                )),
                "hrtime" => Ok(Value::Int(
                    i64::try_from(self.start.elapsed().as_nanos())
                        .map_err(|_| "hrtime exceeds i64")? as i128,
                    Type::i64(),
                )),
                _ => Err("unknown process API".into()),
            },
            _ => Err("unknown module".into()),
        }
    }
    fn decode_buffer(text: &str, encoding: &str) -> Result<Vec<u8>, String> {
        use base64::Engine as _;
        if text.len() > crate::filesystem::MAX_BYTES * 2 {
            return Err("encoded buffer exceeds limit".into());
        }
        let bytes = match encoding {
            "utf8" | "utf-8" => text.as_bytes().to_vec(),
            "hex" => {
                if text.len() % 2 != 0 || !text.is_ascii() {
                    return Err("hex requires complete ASCII byte pairs".into());
                }
                (0..text.len())
                    .step_by(2)
                    .map(|i| {
                        u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| "invalid hex".into())
                    })
                    .collect::<Result<Vec<_>, String>>()?
            }
            "base64" => base64::engine::general_purpose::STANDARD
                .decode(text)
                .map_err(|e| e.to_string())?,
            "base64url" => base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(text.trim_end_matches('='))
                .map_err(|e| e.to_string())?,
            _ => return Err("encoding must be utf8, hex, base64 or base64url".into()),
        };
        if bytes.len() > crate::filesystem::MAX_BYTES {
            return Err("buffer exceeds 8 MiB".into());
        }
        Ok(bytes)
    }
    fn alloc_buffer(&mut self, data: Vec<u8>, ret: Type) -> Result<Value, String> {
        self.core_capacity()?;
        if self
            .core
            .buffers
            .values()
            .map(Vec::len)
            .sum::<usize>()
            .saturating_add(data.len())
            > crate::filesystem::MAX_BYTES
        {
            return Err("live Buffers exceed 8 MiB; close unused buffers".into());
        }
        let id = self.http_id()?;
        let len = data.len();
        self.core.buffers.insert(id, data);
        Ok(crate::platform::record(
            ret,
            vec![
                ("id", Value::Int(id as i128, Type::i64())),
                ("length", Value::Int(len as i128, Type::i64())),
            ],
        ))
    }
    fn buffer_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use base64::Engine as _;
        let max = crate::filesystem::MAX_BYTES;
        match name {
            "alloc" => {
                return self.alloc_buffer(
                    vec![Self::core_int(&args[1])? as u8; Self::core_size(&args[0], max)?],
                    ret,
                )
            }
            "from" => {
                return self.alloc_buffer(
                    Self::decode_buffer(args[0].string()?, args[1].string()?)?,
                    ret,
                )
            }
            "fromBytes" => return self.alloc_buffer(crate::filesystem::bytes(&args[0])?, ret),
            "byteLength" => {
                return Ok(Value::Int(
                    Self::decode_buffer(args[0].string()?, args[1].string()?)?.len() as i128,
                    Type::i64(),
                ))
            }
            "concat" => {
                let Value::Vector(buffers, _) = &args[0] else {
                    unreachable!()
                };
                let mut data = Vec::new();
                for buffer in buffers.iter() {
                    let item = self
                        .core
                        .buffers
                        .get(&Self::handle_id(buffer)?)
                        .ok_or("buffer is closed or belongs to another thread")?;
                    if data.len() + item.len() > max {
                        return Err("concatenated buffer exceeds 8 MiB".into());
                    }
                    data.extend_from_slice(item);
                }
                return self.alloc_buffer(data, ret);
            }
            _ => {}
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_Buffer_close" {
            self.core
                .buffers
                .remove(&id)
                .ok_or("buffer is already closed")?;
            return Ok(Value::Void);
        }
        let data = self
            .core
            .buffers
            .get(&id)
            .ok_or("buffer is closed or belongs to another thread")?;
        match name {
            "method_Buffer_toBytes" => Ok(crate::filesystem::bytes_value(data.clone())),
            "method_Buffer_toString" => Ok(Value::Str(match args[1].string()? {
                "utf8" | "utf-8" => String::from_utf8_lossy(data).into_owned(),
                "hex" => data.iter().map(|b| format!("{b:02x}")).collect(),
                "base64" => base64::engine::general_purpose::STANDARD.encode(data),
                "base64url" => base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data),
                _ => return Err("unknown encoding".into()),
            })),
            "method_Buffer_equals" => Ok(Value::Bool(
                data == self
                    .core
                    .buffers
                    .get(&Self::handle_id(&args[1])?)
                    .ok_or("other buffer is closed")?,
            )),
            "method_Buffer_fill" => {
                let fill = Self::core_int(&args[1])? as u8;
                self.core.buffers.get_mut(&id).unwrap().fill(fill);
                Ok(Value::Bool(true))
            }
            "method_Buffer_slice" => {
                let len = data.len() as i128;
                let bound = |v: &Value| {
                    Self::core_int(v).map(|n|if n<0{(len+n).clamp(0,len)}else{n.min(len)} as usize)
                };
                let start = bound(&args[1])?;
                let end = bound(&args[2])?.max(start);
                let copy = data[start..end].to_vec();
                self.alloc_buffer(copy, ret)
            }
            "method_Buffer_copy" => {
                let target = Self::handle_id(&args[1])?;
                let start = Self::core_size(&args[3], data.len())?;
                let end = Self::core_size(&args[4], data.len())?;
                if end < start {
                    return Err("sourceEnd precedes sourceStart".into());
                }
                let copy = data[start..end].to_vec();
                let target = self
                    .core
                    .buffers
                    .get_mut(&target)
                    .ok_or("target buffer is closed")?;
                let offset = Self::core_size(&args[2], target.len())?;
                let size = copy.len().min(target.len() - offset);
                target[offset..offset + size].copy_from_slice(&copy[..size]);
                Ok(Value::Int(size as i128, Type::i64()))
            }
            _ if name.starts_with("method_Buffer_readUInt")
                || name.starts_with("method_Buffer_writeUInt") =>
            {
                let write = name.starts_with("method_Buffer_write");
                let size = if name.contains("32") {
                    4
                } else if name.contains("16") {
                    2
                } else {
                    1
                };
                let offset = Self::core_size(&args[if write { 2 } else { 1 }], data.len())?;
                if offset.checked_add(size).is_none_or(|end| end > data.len()) {
                    return Err("buffer offset out of bounds".into());
                }
                let little = !name.ends_with("BE");
                if write {
                    let n = Self::core_int(&args[1])? as u64;
                    let data = self.core.buffers.get_mut(&id).unwrap();
                    for i in 0..size {
                        data[offset + i] = (n >> (8 * if little { i } else { size - 1 - i })) as u8;
                    }
                    Ok(Value::Bool(true))
                } else {
                    let mut n = 0u64;
                    for i in 0..size {
                        n |= (data[offset + i] as u64)
                            << (8 * if little { i } else { size - 1 - i });
                    }
                    Ok(Value::Int(n as i128, ret))
                }
            }
            _ => Err("unknown Buffer API".into()),
        }
    }
    fn events_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        if name == "createEmitter" {
            self.core_capacity()?;
            let id = self.http_id()?;
            self.core.events.insert(id, vec![]);
            return Ok(Self::core_handle(id, ret));
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_EventEmitter_close" {
            self.core.events.remove(&id).ok_or("emitter is closed")?;
            return Ok(Value::Void);
        }
        if !self.core.events.contains_key(&id) {
            return Err("emitter is closed or belongs to another thread".into());
        }
        if name == "method_EventEmitter_emit" {
            let event = args[1].string()?;
            let callbacks = self.core.events[&id]
                .iter()
                .filter(|l| l.event == event)
                .cloned()
                .collect::<Vec<_>>();
            if callbacks.is_empty() && event == "error" {
                return Err(format!("unhandled error event: {}", args[2].string()?));
            }
            let once = callbacks
                .iter()
                .filter(|l| l.once)
                .map(|l| l.id)
                .collect::<Vec<_>>();
            self.core
                .events
                .get_mut(&id)
                .unwrap()
                .retain(|l| !once.contains(&l.id));
            for listener in &callbacks {
                self.invoke_core(listener.callback.clone(), vec![args[2].clone()])?;
            }
            return Ok(Value::Bool(!callbacks.is_empty()));
        }
        if name == "method_EventEmitter_on" || name == "method_EventEmitter_once" {
            if self.core.events[&id].len() >= 1024 {
                return Err("at most 1024 listeners per emitter".into());
            }
            let token = self.http_id()?;
            self.core.events.get_mut(&id).unwrap().push(CoreListener {
                id: token,
                event: args[1].string()?.into(),
                once: name.ends_with("_once"),
                callback: args[2].clone(),
            });
            return Ok(Value::Int(token as i128, Type::i64()));
        }
        let listeners = self.core.events.get_mut(&id).unwrap();
        match name {
            "method_EventEmitter_listenerCount" => Ok(Value::Int(
                listeners
                    .iter()
                    .filter(|l| l.event == args[1].string().unwrap())
                    .count() as i128,
                Type::i64(),
            )),
            "method_EventEmitter_off" => {
                let token = Self::core_int(&args[2])?;
                let event = args[1].string()?;
                let old = listeners.len();
                listeners.retain(|l| !(l.id as i128 == token && l.event == event));
                Ok(Value::Bool(old != listeners.len()))
            }
            "method_EventEmitter_removeAllListeners" => {
                let old = listeners.len();
                let event = args[1].string()?;
                listeners.retain(|l| l.event != event);
                Ok(Value::Int((old - listeners.len()) as i128, Type::i64()))
            }
            "method_EventEmitter_eventNames" => {
                let mut names = vec![];
                for listener in listeners {
                    if !names.contains(&listener.event) {
                        names.push(listener.event.clone());
                    }
                }
                Ok(crate::platform::strings(names))
            }
            _ => Err("unknown EventEmitter API".into()),
        }
    }
    fn query_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        if name == "searchParams" {
            self.core_capacity()?;
            let text = args[0].string()?.trim_start_matches('?');
            if text.len() > crate::filesystem::MAX_BYTES {
                return Err("query exceeds 8 MiB".into());
            }
            let pairs = url::form_urlencoded::parse(text.as_bytes())
                .take(4097)
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect::<Vec<_>>();
            if pairs.len() > 4096 {
                return Err("at most 4096 query pairs".into());
            }
            let id = self.http_id()?;
            self.core.queries.insert(id, pairs);
            return Ok(Self::core_handle(id, ret));
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_URLSearchParams_close" {
            self.core.queries.remove(&id).ok_or("query is closed")?;
            return Ok(Value::Void);
        }
        let pairs = self
            .core
            .queries
            .get_mut(&id)
            .ok_or("query is closed or belongs to another thread")?;
        match name {
            "method_URLSearchParams_toString" => {
                let mut writer = url::form_urlencoded::Serializer::new(String::new());
                writer.extend_pairs(pairs.iter().map(|(k, v)| (k, v)));
                Ok(Value::Str(writer.finish()))
            }
            "method_URLSearchParams_get" => Ok(Value::Str(
                pairs
                    .iter()
                    .find(|(k, _)| k == args[1].string().unwrap())
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default(),
            )),
            "method_URLSearchParams_getAll" => Ok(crate::platform::strings(
                pairs
                    .iter()
                    .filter(|(k, _)| k == args[1].string().unwrap())
                    .map(|(_, v)| v.clone())
                    .collect(),
            )),
            "method_URLSearchParams_has" => Ok(Value::Bool(
                pairs.iter().any(|(k, _)| k == args[1].string().unwrap()),
            )),
            "method_URLSearchParams_delete" => {
                pairs.retain(|(k, _)| k != args[1].string().unwrap());
                Ok(Value::Void)
            }
            "method_URLSearchParams_append" | "method_URLSearchParams_set" => {
                let key = args[1].string()?;
                let value = args[2].string()?;
                let bytes = pairs.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>();
                let replacing = name.ends_with("_set");
                let removed = pairs
                    .iter()
                    .filter(|(k, _)| replacing && k == key)
                    .collect::<Vec<_>>();
                let removed_bytes = removed
                    .iter()
                    .map(|(k, v)| k.len() + v.len())
                    .sum::<usize>();
                if pairs.len() - removed.len() + 1 > 4096
                    || (bytes - removed_bytes)
                        .saturating_add(key.len())
                        .saturating_add(value.len())
                        > crate::filesystem::MAX_BYTES
                {
                    return Err("query pair/size limit exceeded".into());
                }
                if replacing {
                    if let Some(index) = pairs.iter().position(|(k, _)| k == key) {
                        pairs[index].1 = value.into();
                        let mut n = 0;
                        pairs.retain(|(k, _)| {
                            if k == key {
                                n += 1;
                                n == 1
                            } else {
                                true
                            }
                        });
                        return Ok(Value::Void);
                    }
                }
                pairs.push((key.into(), value.into()));
                Ok(Value::Void)
            }
            _ => Err("unknown URLSearchParams API".into()),
        }
    }
    fn stream_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use std::io::{Read, Write};
        match name {
            "createReadStream" => {
                self.core_capacity()?;
                let file = std::fs::File::open(args[0].string()?).map_err(|e| e.to_string())?;
                let id = self.http_id()?;
                self.core.readers.insert(id, file);
                return Ok(Self::core_handle(id, ret));
            }
            "createWriteStream" => {
                self.core_capacity()?;
                let Value::Bool(append) = args[1] else {
                    unreachable!()
                };
                let file = std::fs::OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(append)
                    .truncate(!append)
                    .open(args[0].string()?)
                    .map_err(|e| e.to_string())?;
                let id = self.http_id()?;
                self.core.writers.insert(id, file);
                return Ok(Self::core_handle(id, ret));
            }
            _ => {}
        }
        let id = Self::handle_id(&args[0])?;
        match name {
            "method_Readable_read" => {
                let size = Self::core_size(&args[1], crate::filesystem::MAX_BYTES)?;
                let file = self
                    .core
                    .readers
                    .get_mut(&id)
                    .ok_or("reader is closed or belongs to another thread")?;
                let mut data = vec![0; size];
                let n = file.read(&mut data).map_err(|e| e.to_string())?;
                data.truncate(n);
                Ok(crate::filesystem::bytes_value(data))
            }
            "method_Readable_close" => {
                self.core.readers.remove(&id).ok_or("reader is closed")?;
                Ok(Value::Void)
            }
            "method_Writable_write" => {
                let data = crate::filesystem::bytes(&args[1])?;
                self.core
                    .writers
                    .get_mut(&id)
                    .ok_or("writer is closed or belongs to another thread")?
                    .write_all(&data)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Int(data.len() as i128, Type::i64()))
            }
            "method_Writable_close" | "method_Writable_end" => {
                let mut file = self.core.writers.remove(&id).ok_or("writer is closed")?;
                file.flush().map_err(|e| e.to_string())?;
                Ok(Value::Void)
            }
            "method_Readable_pipe" => {
                let destination = Self::handle_id(&args[1])?;
                if !self.core.writers.contains_key(&destination)
                    || !self.core.readers.contains_key(&id)
                {
                    return Err("pipe requires live reader/writer handles".into());
                }
                let mut reader = self.core.readers.remove(&id).unwrap();
                let mut writer = self.core.writers.remove(&destination).unwrap();
                let n = std::io::copy(&mut reader, &mut writer).map_err(|e| e.to_string())?;
                writer.flush().map_err(|e| e.to_string())?;
                Ok(Value::Int(
                    i64::try_from(n).map_err(|_| "stream count exceeds i64")? as i128,
                    Type::i64(),
                ))
            }
            _ => Err("unknown stream API".into()),
        }
    }
}
