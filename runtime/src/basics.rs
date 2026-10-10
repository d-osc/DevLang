const BASIC_LIMIT: usize = 8 * 1024 * 1024;
impl Engine {
    fn basic_float(v: &Value) -> Result<f64, String> {
        if let Value::Float(n, _) = v {
            Ok(*n)
        } else {
            Err("expected f64".into())
        }
    }
    fn basic_i64(v: &Value) -> Result<i64, String> {
        i64::try_from(Self::core_int(v)?).map_err(|_| "integer out of i64 range".into())
    }
    fn basic_text(s: String) -> Result<Value, String> {
        if s.len() > BASIC_LIMIT {
            Err("text output exceeds 8 MiB".into())
        } else {
            Ok(Value::Str(s))
        }
    }
    fn math_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        use std::f64::consts;
        let values = args
            .iter()
            .map(Self::basic_float)
            .collect::<Result<Vec<_>, _>>()?;
        let x = values.first().copied().unwrap_or(0.0);
        if name == "isFinite" {
            return Ok(Value::Bool(x.is_finite()));
        }
        if name == "isNaN" {
            return Ok(Value::Bool(x.is_nan()));
        }
        if values.iter().any(|v| !v.is_finite()) {
            return Err("math input must be finite".into());
        }
        let result = match name {
            "pi" => consts::PI,
            "e" => consts::E,
            "tau" => consts::TAU,
            "abs" => x.abs(),
            "sqrt" => x.sqrt(),
            "cbrt" => x.cbrt(),
            "pow" => x.powf(values[1]),
            "exp" => x.exp(),
            "log" => x.ln(),
            "log2" => x.log2(),
            "log10" => x.log10(),
            "sin" => x.sin(),
            "cos" => x.cos(),
            "tan" => x.tan(),
            "asin" => x.asin(),
            "acos" => x.acos(),
            "atan" => x.atan(),
            "atan2" => x.atan2(values[1]),
            "hypot" => x.hypot(values[1]),
            "floor" => x.floor(),
            "ceil" => x.ceil(),
            "round" => x.round(),
            "trunc" => x.trunc(),
            "min" => x.min(values[1]),
            "max" => x.max(values[1]),
            "clamp" => {
                if values[1] > values[2] {
                    return Err("clamp minimum exceeds maximum".into());
                }
                x.clamp(values[1], values[2])
            }
            _ => return Err("unknown math API".into()),
        };
        if !result.is_finite() {
            return Err("math domain error or floating-point overflow".into());
        }
        Ok(Value::Float(result, Type::Float(64)))
    }
    fn random_next(&mut self) -> Result<u64, String> {
        if self.core.random.is_none() {
            let mut seed = [0u8; 8];
            getrandom::getrandom(&mut seed).map_err(|e| format!("cannot seed random: {e}"))?;
            self.core.random = Some(u64::from_le_bytes(seed));
        }
        let state = self.core.random.as_mut().unwrap();
        *state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = *state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        Ok(z ^ (z >> 31))
    }
    fn random_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "seed" => {
                self.core.random = Some(Self::basic_i64(&args[0])? as u64);
                Ok(Value::Void)
            }
            "float" => Ok(Value::Float(
                (self.random_next()? >> 11) as f64 / 9007199254740992.0,
                Type::Float(64),
            )),
            "bool" => Ok(Value::Bool(self.random_next()? & 1 != 0)),
            "int" => {
                let min = Self::basic_i64(&args[0])?;
                let max = Self::basic_i64(&args[1])?;
                if min >= max {
                    return Err("random.int requires min < max (exclusive)".into());
                }
                let range = (max as i128 - min as i128) as u64;
                let threshold = range.wrapping_neg() % range;
                let n = loop {
                    let n = self.random_next()?;
                    if n >= threshold {
                        break n;
                    }
                };
                Ok(Value::Int(min as i128 + (n % range) as i128, Type::i64()))
            }
            "bytes" => {
                let size = Self::core_size(&args[0], BASIC_LIMIT)?;
                let mut bytes = Vec::with_capacity(size);
                while bytes.len() < size {
                    let chunk = self.random_next()?.to_le_bytes();
                    let n = (size - bytes.len()).min(8);
                    bytes.extend_from_slice(&chunk[..n]);
                }
                Ok(crate::filesystem::bytes_value(bytes))
            }
            _ => Err("unknown random API".into()),
        }
    }
    fn strings_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        if name == "join" {
            let Value::Vector(parts, _) = &args[0] else {
                return Err("join expects Vec<str>".into());
            };
            let separator = args[1].string()?;
            let mut size = separator
                .len()
                .checked_mul(parts.len().saturating_sub(1))
                .ok_or("text size overflow")?;
            let mut values = Vec::new();
            for part in parts.iter() {
                let text = part.string()?;
                size = size.checked_add(text.len()).ok_or("text size overflow")?;
                values.push(text);
            }
            if size > BASIC_LIMIT {
                return Err("text output exceeds 8 MiB".into());
            }
            return Self::basic_text(values.join(separator));
        }
        let text = args[0].string()?;
        let pattern = || args[1].string();
        let index = |n: Option<usize>| Value::Int(n.map_or(-1, |n| n as i128), Type::i64());
        let result = match name {
            "len" => return Ok(Value::Int(text.len() as i128, Type::Size { signed: false })),
            "charLength" => return Ok(Value::Int(text.chars().count() as i128, Type::i64())),
            "equal" => return Ok(Value::Bool(text == pattern()?)),
            "contains" => return Ok(Value::Bool(text.contains(pattern()?))),
            "startsWith" => return Ok(Value::Bool(text.starts_with(pattern()?))),
            "endsWith" => return Ok(Value::Bool(text.ends_with(pattern()?))),
            "indexOf" => return Ok(index(text.find(pattern()?))),
            "lastIndexOf" => return Ok(index(text.rfind(pattern()?))),
            "trim" => text.trim().to_string(),
            "trimStart" => text.trim_start().to_string(),
            "trimEnd" => text.trim_end().to_string(),
            "toLowerCase" => text.to_lowercase(),
            "toUpperCase" => text.to_uppercase(),
            "concat" => {
                if text.len().saturating_add(pattern()?.len()) > BASIC_LIMIT {
                    return Err("text output exceeds 8 MiB".into());
                }
                text.to_string() + pattern()?
            }
            "split" => {
                let values: Vec<String> = if pattern()?.is_empty() {
                    text.chars().take(65537).map(|c| c.to_string()).collect()
                } else {
                    text.split(pattern()?)
                        .take(65537)
                        .map(str::to_string)
                        .collect()
                };
                if values.len() > 65536
                    || values.iter().map(String::len).sum::<usize>() > BASIC_LIMIT
                {
                    return Err("split exceeds 65536 parts or 8 MiB".into());
                }
                return Ok(crate::platform::strings(values));
            }
            "repeat" => {
                let count = Self::core_size(&args[1], BASIC_LIMIT)?;
                if text
                    .len()
                    .checked_mul(count)
                    .is_none_or(|n| n > BASIC_LIMIT)
                {
                    return Err("text output exceeds 8 MiB".into());
                }
                text.repeat(count)
            }
            "replace" | "replaceAll" => {
                let replacement = args[2].string()?;
                let pattern = pattern()?;
                let count = if name == "replace" {
                    usize::from(text.contains(pattern))
                } else {
                    text.matches(pattern).count()
                };
                let size = text
                    .len()
                    .checked_sub(count * pattern.len())
                    .and_then(|n| {
                        replacement
                            .len()
                            .checked_mul(count)
                            .and_then(|m| n.checked_add(m))
                    })
                    .ok_or("text size overflow")?;
                if size > BASIC_LIMIT {
                    return Err("text output exceeds 8 MiB".into());
                }
                if name == "replace" {
                    text.replacen(pattern, replacement, 1)
                } else {
                    text.replace(pattern, replacement)
                }
            }
            "substring" | "charAt" => {
                let start = Self::core_size(&args[1], usize::MAX)?;
                let end = if name == "charAt" {
                    start.checked_add(1).ok_or("character index overflow")?
                } else {
                    Self::core_size(&args[2], usize::MAX)?
                };
                let length = text.chars().count();
                if start > end || end > length {
                    return Err("character range out of bounds".into());
                }
                text.chars().skip(start).take(end - start).collect()
            }
            _ => return Err("unknown strings API".into()),
        };
        Self::basic_text(result)
    }
    fn datetime_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use chrono::{DateTime, Datelike, FixedOffset, NaiveDate, SecondsFormat, Timelike, Utc};
        let integer = |n: i64| Value::Int(n as i128, Type::i64());
        let arg = |i| Self::basic_i64(&args[i]);
        let timestamp = |n| {
            DateTime::<Utc>::from_timestamp_millis(n)
                .ok_or("timestamp outside supported calendar".to_string())
        };
        let offset = |n: i64| {
            n.checked_mul(60)
                .and_then(|n| i32::try_from(n).ok())
                .and_then(FixedOffset::east_opt)
                .ok_or("offset must be within -1439..1439 minutes".to_string())
        };
        if name == "now" {
            return Ok(integer(Utc::now().timestamp_millis()));
        }
        if name == "parse" {
            let date = DateTime::parse_from_rfc3339(args[0].string()?)
                .map_err(|e| format!("invalid RFC3339 datetime: {e}"))?;
            if date.timestamp_subsec_millis() >= 1000 {
                return Err("leap seconds are unsupported".into());
            }
            return Ok(integer(date.timestamp_millis()));
        }
        if name == "utc" {
            let year = i32::try_from(arg(0)?).map_err(|_| "year out of range")?;
            let nums = (1..7)
                .map(|i| {
                    u32::try_from(arg(i)?).map_err(|_| "negative/excessive date field".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let date = NaiveDate::from_ymd_opt(year, nums[0], nums[1])
                .and_then(|d| d.and_hms_milli_opt(nums[2], nums[3], nums[4], nums[5]))
                .ok_or("invalid UTC calendar fields")?;
            if nums[4] > 59 || nums[5] > 999 {
                return Err("leap seconds are unsupported".into());
            }
            return Ok(integer(date.and_utc().timestamp_millis()));
        }
        if matches!(name, "isLeapYear" | "daysInMonth") {
            let year = i32::try_from(arg(0)?).map_err(|_| "year out of range")?;
            if NaiveDate::from_ymd_opt(year, 1, 1).is_none() {
                return Err("year outside supported calendar".into());
            }
            let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
            if name == "isLeapYear" {
                return Ok(Value::Bool(leap));
            }
            let days = match arg(1)? {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 if leap => 29,
                2 => 28,
                _ => return Err("month must be 1..12".into()),
            };
            return Ok(integer(days));
        }
        let date = timestamp(arg(0)?)?;
        if name == "iso" {
            return Ok(Value::Str(
                date.to_rfc3339_opts(SecondsFormat::Millis, true),
            ));
        }
        if name == "addMilliseconds" {
            let n = arg(0)?
                .checked_add(arg(1)?)
                .ok_or("datetime addition overflow")?;
            timestamp(n)?;
            return Ok(integer(n));
        }
        let minutes = if matches!(name, "parts" | "formatOffset") {
            arg(1)?
        } else {
            0
        };
        let zone = offset(minutes)?;
        date.naive_utc().checked_add_signed(chrono::TimeDelta::minutes(minutes)).ok_or("offset date outside supported calendar")?;
        let date = date.with_timezone(&zone);
        if name == "parts" {
            return Ok(crate::platform::record(
                ret,
                vec![
                    ("year", integer(date.year() as i64)),
                    ("month", integer(date.month() as i64)),
                    ("day", integer(date.day() as i64)),
                    ("hour", integer(date.hour() as i64)),
                    ("minute", integer(date.minute() as i64)),
                    ("second", integer(date.second() as i64)),
                    (
                        "millisecond",
                        integer(date.timestamp_subsec_millis() as i64),
                    ),
                    (
                        "weekday",
                        integer(date.weekday().number_from_monday() as i64),
                    ),
                    ("offsetMinutes", integer(minutes)),
                ],
            ));
        }
        if matches!(name, "format" | "formatOffset") {
            let pattern = args[if name == "format" { 1 } else { 2 }].string()?;
            if pattern.len() > 65536 {
                return Err("datetime pattern exceeds 64 KiB".into());
            }
            if chrono::format::StrftimeItems::new(pattern)
                .any(|i| matches!(i, chrono::format::Item::Error))
            {
                return Err("invalid datetime format directive".into());
            }
            let mut text = String::new();
            use std::fmt::Write;
            write!(&mut text, "{}", date.format(pattern)).map_err(|_| "unsupported datetime formatting directive")?;
            return Self::basic_text(text);
        }
        Err("unknown datetime API".into())
    }
    fn test_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        let name = if name.starts_with("g_equal_") {
            "equal"
        } else {
            name
        };
        match name {
            "expect" => {
                let Value::Bool(ok) = args[0] else {
                    return Err("expect needs bool".into());
                };
                if !ok {
                    return Err(format!("assertion failed: {}", args[1].string()?));
                }
                Ok(Value::Void)
            }
            "equal" => {
                if crate::json::encode(&args[0], 0)? != crate::json::encode(&args[1], 0)? {
                    return Err(format!("assertion failed: {}", args[2].string()?));
                }
                Ok(Value::Void)
            }
            "near" => {
                let a = Self::basic_float(&args[0])?;
                let b = Self::basic_float(&args[1])?;
                let tolerance = Self::basic_float(&args[2])?;
                if !a.is_finite() || !b.is_finite() || !tolerance.is_finite() || tolerance < 0.0 {
                    return Err("near requires finite values and nonnegative tolerance".into());
                }
                if (a - b).abs() > tolerance {
                    return Err(format!("assertion failed: {}", args[3].string()?));
                }
                Ok(Value::Void)
            }
            "case" => {
                if self.core.testing {
                    return Err("cannot register cases during test.run".into());
                }
                let name = args[0].string()?.to_string();
                if self.core.tests.len() >= 1024 || self.core.tests.iter().any(|(n, _)| n == &name)
                {
                    return Err("duplicate test name or more than 1024 cases".into());
                }
                self.core.tests.push((name, args[1].clone()));
                Ok(Value::Void)
            }
            "run" => {
                if self.core.testing {
                    return Err("test.run cannot be nested".into());
                }
                self.core.testing = true;
                let tests = std::mem::take(&mut self.core.tests);
                let total = tests.len() as i64;
                let mut failed = 0i64;
                let mut rows = Vec::new();
                for (name, body) in tests {
                    let mut row = match self.invoke_core(body, vec![]) {
                        Ok(_) => format!("PASS {name}"),
                        Err(e) => {
                            failed += 1;
                            format!("FAIL {name}: {e}")
                        }
                    };
                    if row.len() > 4096 {
                        let mut end = 4096;
                        while !row.is_char_boundary(end) {
                            end -= 1;
                        }
                        row.truncate(end);
                        row.push_str("...");
                    }
                    rows.push(row);
                }
                self.core.testing = false;
                let int = |n: i64| Value::Int(n as i128, Type::i64());
                Ok(crate::platform::record(
                    ret,
                    vec![
                        ("total", int(total)),
                        ("passed", int(total - failed)),
                        ("failed", int(failed)),
                        ("summary", Value::Str(rows.join("\n"))),
                    ],
                ))
            }
            _ => Err("unknown test API".into()),
        }
    }
    fn log_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        use std::io::Write;
        let level = |s: &str| match s {
            "debug" => Ok(0),
            "info" => Ok(1),
            "warn" => Ok(2),
            "error" => Ok(3),
            "off" => Ok(4),
            _ => Err("log level must be debug/info/warn/error/off".to_string()),
        };
        match name {
            "setLevel" => {
                self.core.log_level = Some(level(args[0].string()?)?);
            }
            "toFile" => {
                self.core.log_file = Some(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(args[0].string()?)
                        .map_err(|e| e.to_string())?,
                );
            }
            "toStderr" => {
                if let Some(mut file) = self.core.log_file.take() {
                    file.flush().map_err(|e| e.to_string())?;
                }
            }
            "flush" => {
                if let Some(file) = self.core.log_file.as_mut() {
                    file.flush().map_err(|e| e.to_string())?;
                } else {
                    std::io::stderr().flush().map_err(|e| e.to_string())?;
                }
            }
            _ => {
                if level(name)? < self.core.log_level.unwrap_or(1) {
                    return Ok(Value::Void);
                }
                let text = args[0].string()?;
                if text.len() > BASIC_LIMIT {
                    return Err("log message exceeds 8 MiB".into());
                }
                let stamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let line = format!(
                    "{stamp} [{}] {}\n",
                    name.to_uppercase(),
                    text.replace('\r', "\\r").replace('\n', "\\n")
                );
                if let Some(file) = self.core.log_file.as_mut() {
                    file.write_all(line.as_bytes())
                        .and_then(|_| file.flush())
                        .map_err(|e| e.to_string())?;
                } else {
                    let mut out = std::io::stderr().lock();
                    out.write_all(line.as_bytes())
                        .and_then(|_| out.flush())
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        Ok(Value::Void)
    }
}
