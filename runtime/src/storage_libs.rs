struct CoreDatabase {
    connection: rusqlite::Connection,
    timeout: std::time::Duration,
}
fn storage_charge(used: &mut usize, bytes: usize) -> Result<(), String> {
    *used = used.saturating_add(bytes).saturating_add(64);
    if *used > BASIC_LIMIT {
        Err("data exceeds 8 MiB result budget".into())
    } else {
        Ok(())
    }
}
fn config_budget(value: &serde_json::Value, depth: usize, used: &mut usize) -> Result<(), String> {
    use serde_json::Value as J;
    if depth > 128 {
        return Err("config nesting exceeds 128 levels".into());
    }
    storage_charge(
        used,
        match value {
            J::String(s) => s.len(),
            _ => 0,
        },
    )?;
    match value {
        J::Array(items) => {
            for item in items {
                config_budget(item, depth + 1, used)?;
            }
        }
        J::Object(items) => {
            for (key, item) in items {
                storage_charge(used, key.len())?;
                config_budget(item, depth + 1, used)?;
            }
        }
        _ => {}
    }
    Ok(())
}
fn toml_json(value: toml::Value, depth: usize) -> Result<serde_json::Value, String> {
    use serde_json::Value as J;
    if depth > 128 {
        return Err("config nesting exceeds 128 levels".into());
    }
    Ok(match value {
        toml::Value::String(value) => J::String(value),
        toml::Value::Integer(value) => J::Number(value.into()),
        toml::Value::Float(value) => J::Number(
            serde_json::Number::from_f64(value)
                .ok_or("TOML non-finite number is not JSON-compatible")?,
        ),
        toml::Value::Boolean(value) => J::Bool(value),
        toml::Value::Datetime(value) => J::String(value.to_string()),
        toml::Value::Array(items) => J::Array(
            items
                .into_iter()
                .map(|v| toml_json(v, depth + 1))
                .collect::<Result<_, _>>()?,
        ),
        toml::Value::Table(items) => J::Object(
            items
                .into_iter()
                .map(|(k, v)| Ok((k, toml_json(v, depth + 1)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
fn json_toml(value: &serde_json::Value) -> Result<toml::Value, String> {
    use serde_json::Value as J;
    Ok(match value {
        J::Null => return Err("TOML has no null value".into()),
        J::Bool(value) => toml::Value::Boolean(*value),
        J::String(value) => toml::Value::String(value.clone()),
        J::Number(value) => {
            if let Some(n) = value.as_i64() {
                toml::Value::Integer(n)
            } else if value.to_string().contains(['.', 'e', 'E']) {
                toml::Value::Float(
                    value
                        .as_f64()
                        .filter(|v| v.is_finite())
                        .ok_or("TOML number must be finite")?,
                )
            } else {
                return Err("TOML integer is outside i64 range".into());
            }
        }
        J::Array(items) => {
            toml::Value::Array(items.iter().map(json_toml).collect::<Result<_, _>>()?)
        }
        J::Object(items) => toml::Value::Table(
            items
                .iter()
                .map(|(k, v)| Ok((k.clone(), json_toml(v)?)))
                .collect::<Result<_, String>>()?,
        ),
    })
}
// Serialize JSON numbers as their actual scalar types even when serde_json's
// arbitrary_precision feature is enabled by the runtime.
struct ConfigJson<'a>(&'a serde_json::Value);
impl serde::Serialize for ConfigJson<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::{Error, SerializeMap, SerializeSeq};
        use serde_json::Value as J;
        match self.0 {
            J::Null => serializer.serialize_unit(),
            J::Bool(v) => serializer.serialize_bool(*v),
            J::String(v) => serializer.serialize_str(v),
            J::Number(v) => {
                if let Some(n) = v.as_i64() {
                    serializer.serialize_i64(n)
                } else if let Some(n) = v.as_u64() {
                    serializer.serialize_u64(n)
                } else if v.to_string().contains(['.', 'e', 'E']) {
                    serializer.serialize_f64(
                        v.as_f64()
                            .filter(|v| v.is_finite())
                            .ok_or_else(|| S::Error::custom("YAML number must be finite"))?,
                    )
                } else {
                    Err(S::Error::custom("YAML integer outside i64/u64 range"))
                }
            }
            J::Array(items) => {
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(&ConfigJson(item))?;
                }
                sequence.end()
            }
            J::Object(items) => {
                let mut map = serializer.serialize_map(Some(items.len()))?;
                for (key, item) in items {
                    map.serialize_entry(key, &ConfigJson(item))?;
                }
                map.end()
            }
        }
    }
}
#[derive(Default)]
struct ConfigText(String);
impl std::fmt::Write for ConfigText {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        data_append(&mut self.0, text).map_err(|_| std::fmt::Error)
    }
}
fn config_parse(module: &str, text: &str) -> Result<serde_json::Value, String> {
    if text.len() > BASIC_LIMIT {
        return Err("config input exceeds 8 MiB".into());
    }
    let value = if module == "std/toml" {
        toml_json(
            toml::from_str::<toml::Table>(text)
                .map(toml::Value::Table)
                .map_err(|e| format!("invalid TOML: {e}"))?,
            0,
        )?
    } else {
        let mut budget = serde_saphyr::Budget::default();
        budget.max_depth = 128;
        budget.flow_nesting_limit = 128;
        budget.max_documents = 1;
        budget.max_nodes = 65536;
        budget.max_events = 131072;
        budget.max_total_scalar_bytes = BASIC_LIMIT;
        budget.max_recorded_anchor_bytes = BASIC_LIMIT;
        budget.max_recorded_anchor_events = 65536;
        budget.max_aliases = 64;
        budget.max_anchors = 64;
        let mut options = serde_saphyr::Options::default();
        options.budget = Some(budget);
        options.emit_comments = false;
        options.duplicate_keys = serde_saphyr::options::DuplicateKeyPolicy::Error;
        options.merge_keys = serde_saphyr::options::MergeKeyPolicy::Error;
        options.strict_booleans = true;
        options.reject_unsupported_tags = true;
        options.reject_non_finite_typeless_float = true;
        options.alias_limits.max_total_replayed_events = 4096;
        options.alias_limits.max_alias_expansions_per_anchor = 16;
        options.alias_limits.max_replay_stack_depth = 32;
        serde_saphyr::from_str_with_options::<serde_json::Value>(text, options)
            .map_err(|e| format!("invalid YAML: {e}"))?
    };
    config_budget(&value, 0, &mut 0)?;
    Ok(value)
}
fn csv_delimiter(text: &str) -> Result<u8, String> {
    if text.len() != 1 || !text.is_ascii() || matches!(text.as_bytes()[0], 0 | b'"' | b'\r' | b'\n')
    {
        return Err("CSV delimiter must be one ASCII byte other than NUL, quote, CR or LF".into());
    }
    Ok(text.as_bytes()[0])
}
fn string_rows(items: Vec<Vec<String>>) -> Value {
    Value::Vector(
        Arc::new(items.into_iter().map(crate::platform::strings).collect()),
        Type::Vector(Box::new(Type::Vector(Box::new(Type::Str)))),
    )
}
fn sqlite_params(value: &Value) -> Result<Vec<rusqlite::types::Value>, String> {
    use rusqlite::types::Value as S;
    let Value::Vector(items, _) = value else {
        return Err("expected Vec<sqlite.Value>".into());
    };
    if items.len() > 999 {
        return Err("SQLite params exceed 999 items".into());
    }
    let mut used = 0;
    items
        .iter()
        .map(|value| {
            let Value::Enum(index, _, payload) = value else {
                return Err("expected sqlite.Value".into());
            };
            let result = match index {
                0 => S::Null,
                1 => S::Integer(Engine::basic_i64(&payload[0])?),
                2 => {
                    let value = Engine::basic_float(&payload[0])?;
                    if !value.is_finite() {
                        return Err("SQLite real must be finite".into());
                    }
                    S::Real(value)
                }
                3 => {
                    let text = payload[0].string()?;
                    storage_charge(&mut used, text.len())?;
                    S::Text(text.into())
                }
                4 => {
                    let data = crate::filesystem::bytes(&payload[0])?;
                    storage_charge(&mut used, data.len())?;
                    S::Blob(data)
                }
                _ => return Err("invalid SQLite value variant".into()),
            };
            if *index <= 2 {
                storage_charge(&mut used, 8)?;
            }
            Ok(result)
        })
        .collect()
}
impl Engine {
    fn config_call(
        &mut self,
        module: &str,
        name: &str,
        args: Vec<Value>,
        ret: Type,
    ) -> Result<Value, String> {
        let name = if name.starts_with("g_stringify_") {
            "stringify"
        } else {
            name
        };
        if ["parse", "valid", "toJSON"].contains(&name) {
            let value = config_parse(module, args[0].string()?);
            return match name {
                "valid" => Ok(Value::Bool(value.is_ok())),
                "parse" => Ok(Value::Json(Arc::new(value?), ret)),
                _ => {
                    let mut writer = DataWriter::default();
                    serde_json::to_writer(&mut writer, &value?).map_err(|e| e.to_string())?;
                    Ok(Value::Str(
                        String::from_utf8(writer.cursor.into_inner()).map_err(|e| e.to_string())?,
                    ))
                }
            };
        }
        let value = if name == "fromJSON" {
            let text = args[0].string()?;
            if text.len() > BASIC_LIMIT {
                return Err("config input exceeds 8 MiB".into());
            }
            serde_json::from_str(text).map_err(|e| format!("invalid JSON: {e}"))?
        } else if name == "stringify" {
            crate::json::encode(&args[0], 0)?
        } else {
            return Err("unknown config API".into());
        };
        config_budget(&value, 0, &mut 0)?;
        if module == "std/toml" {
            if !value.is_object() {
                return Err("TOML document root must be an object".into());
            }
            Self::basic_text(
                toml::to_string_pretty(&json_toml(&value)?).map_err(|e| e.to_string())?,
            )
        } else {
            let mut writer = ConfigText::default();
            serde_saphyr::to_fmt_writer(&mut writer, &ConfigJson(&value))
                .map_err(|e| e.to_string())?;
            Ok(Value::Str(writer.0))
        }
    }
    fn csv_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        let delimiter = csv_delimiter(args[1].string()?)?;
        if name == "parse" {
            let text = args[0].string()?;
            if text.contains('\0') {
                return Err("CSV text cannot contain NUL".into());
            }
            if text.len() > BASIC_LIMIT {
                return Err("CSV input exceeds 8 MiB".into());
            }
            let Value::Bool(has_headers) = args[2] else {
                unreachable!()
            };
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(has_headers)
                .delimiter(delimiter)
                .from_reader(text.as_bytes());
            let mut used = 0;
            let mut convert_record = |record: &csv::StringRecord| -> Result<Vec<String>, String> {
                if record.len() > 256 {
                    return Err("CSV exceeds 256 columns".into());
                }
                record
                    .iter()
                    .map(|cell| {
                        storage_charge(&mut used, cell.len())?;
                        Ok(cell.to_owned())
                    })
                    .collect()
            };
            let headers = if has_headers {
                convert_record(reader.headers().map_err(|e| e.to_string())?)?
            } else {
                Vec::new()
            };
            let mut rows = Vec::new();
            for record in reader.records() {
                if rows.len() >= 4096 {
                    return Err("CSV exceeds 4096 rows".into());
                }
                rows.push(convert_record(&record.map_err(|e| e.to_string())?)?);
            }
            return Ok(crate::platform::record(
                ret,
                vec![
                    ("headers", crate::platform::strings(headers)),
                    ("rows", string_rows(rows)),
                ],
            ));
        }
        if name != "stringify" {
            return Err("unknown CSV API".into());
        }
        let headers = cli_texts(crate::platform::field(&args[0], "headers")?)?;
        let Value::Vector(rows, _) = crate::platform::field(&args[0], "rows")? else {
            return Err("expected CSV rows".into());
        };
        if rows.len() > 4096 || headers.len() > 256 {
            return Err("CSV exceeds 4096 rows or 256 columns".into());
        }
        let mut writer = csv::WriterBuilder::new()
            .delimiter(delimiter)
            .from_writer(DataWriter::default());
        let mut width = (!headers.is_empty()).then_some(headers.len());
        if !headers.is_empty() {
            writer.write_record(&headers).map_err(|e| e.to_string())?;
        }
        let mut used = 0;
        for row in rows.iter() {
            let row = cli_texts(row)?;
            if row.is_empty() || row.len() > 256 {
                return Err("CSV rows must contain 1..256 columns".into());
            }
            if let Some(width) = width {
                if width != row.len() {
                    return Err("CSV rows have unequal lengths".into());
                }
            } else {
                width = Some(row.len());
            }
            for cell in &row {
                storage_charge(&mut used, cell.len())?;
            }
            writer.write_record(&row).map_err(|e| e.to_string())?;
        }
        let output = writer
            .into_inner()
            .map_err(|e| e.to_string())?
            .cursor
            .into_inner();
        Ok(Value::Str(
            String::from_utf8(output).map_err(|e| e.to_string())?,
        ))
    }
    fn sqlite_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use rusqlite::types::ValueRef;
        if name == "version" {
            return Ok(Value::Str(rusqlite::version().into()));
        }
        if name == "open" {
            self.core_capacity()?;
            if self.core.databases.len() >= 16 {
                return Err("at most 16 SQLite databases per interpreter".into());
            }
            let path = args[0].string()?;
            if path.is_empty() || path.len() > 32768 || path.contains('\0') {
                return Err("invalid SQLite path".into());
            }
            let connection = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
                    | rusqlite::OpenFlags::SQLITE_OPEN_CREATE
                    | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
            )
            .map_err(|e| e.to_string())?;
            use rusqlite::limits::Limit;
            for (limit, value) in [
                (Limit::SQLITE_LIMIT_LENGTH, BASIC_LIMIT as i32),
                (Limit::SQLITE_LIMIT_SQL_LENGTH, 65536),
                (Limit::SQLITE_LIMIT_COLUMN, 256),
                (Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 999),
                (Limit::SQLITE_LIMIT_TRIGGER_DEPTH, 32),
            ] {
                connection
                    .set_limit(limit, value)
                    .map_err(|e| e.to_string())?;
            }
            connection
                .busy_timeout(std::time::Duration::from_millis(5000))
                .map_err(|e| e.to_string())?;
            connection
                .execute_batch("PRAGMA foreign_keys=ON")
                .map_err(|e| e.to_string())?;
            let id = self.http_id()?;
            self.core.databases.insert(
                id,
                CoreDatabase {
                    connection,
                    timeout: std::time::Duration::from_millis(5000),
                },
            );
            return Ok(Self::core_handle(id, ret));
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_Database_close" {
            let database = self
                .core
                .databases
                .remove(&id)
                .ok_or("database is closed or belongs to another thread")?;
            database
                .connection
                .close()
                .map_err(|(_, e)| e.to_string())?;
            return Ok(Value::Void);
        }
        let enum_ty = self.modules["std/sqlite"]
            .module
            .concrete_types
            .iter()
            .find(|ty| matches!(ty, Type::Enum(..)))
            .cloned()
            .ok_or("SQLite value type unavailable")?;
        let database = self
            .core
            .databases
            .get_mut(&id)
            .ok_or("database is closed or belongs to another thread")?;
        match name {
            "method_Database_lastInsertRowId" => {
                return Ok(Value::Int(
                    database.connection.last_insert_rowid() as i128,
                    Type::i64(),
                ))
            }
            "method_Database_setTimeout" => {
                let ms = Self::core_size(&args[1], 300000)?;
                if ms == 0 {
                    return Err("SQLite timeout must be at least 1 ms".into());
                }
                database
                    .connection
                    .busy_timeout(std::time::Duration::from_millis(ms as u64))
                    .map_err(|e| e.to_string())?;
                database.timeout = std::time::Duration::from_millis(ms as u64);
                return Ok(Value::Void);
            }
            "method_Database_execute" | "method_Database_query" => {}
            _ => return Err("unknown SQLite API".into()),
        }
        let sql = args[1].string()?;
        if sql.is_empty() || sql.len() > 65536 || sql.contains('\0') {
            return Err("SQLite SQL must contain 1..65536 bytes without NUL".into());
        }
        let params = sqlite_params(&args[2])?;
        let start = Instant::now();
        let timeout = database.timeout;
        database
            .connection
            .progress_handler(1000, Some(move || start.elapsed() >= timeout))
            .map_err(|e| e.to_string())?;
        let result = (|| {
            let mut statement = database
                .connection
                .prepare(sql)
                .map_err(|e| e.to_string())?;
            if name == "method_Database_execute" {
                let affected = statement
                    .execute(rusqlite::params_from_iter(params.iter()))
                    .map_err(|e| e.to_string())?;
                return Ok(Value::Int(affected as i128, Type::i64()));
            }
            let count = statement.column_count();
            if count > 256 {
                return Err("SQLite exceeds 256 columns".into());
            }
            let columns = statement
                .column_names()
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>();
            let mut used = 0;
            for column in &columns {
                storage_charge(&mut used, column.len())?;
            }
            let mut rows = statement
                .query(rusqlite::params_from_iter(params.iter()))
                .map_err(|e| e.to_string())?;
            let mut output = Vec::new();
            let row_ty = Type::Vector(Box::new(Type::Nominal(enum_ty.name())));
            while let Some(row) = rows.next().map_err(|e| e.to_string())? {
                if output.len() >= 4096 {
                    return Err("SQLite exceeds 4096 rows".into());
                }
                let mut values = Vec::with_capacity(count);
                for column in 0..count {
                    let (index, payload) = match row.get_ref(column).map_err(|e| e.to_string())? {
                        ValueRef::Null => {
                            storage_charge(&mut used, 1)?;
                            (0, vec![])
                        }
                        ValueRef::Integer(n) => {
                            storage_charge(&mut used, 8)?;
                            (1, vec![Value::Int(n as i128, Type::i64())])
                        }
                        ValueRef::Real(n) => {
                            storage_charge(&mut used, 8)?;
                            if !n.is_finite() {
                                return Err("SQLite real must be finite".into());
                            }
                            (2, vec![Value::Float(n, Type::Float(64))])
                        }
                        ValueRef::Text(bytes) => {
                            storage_charge(&mut used, bytes.len())?;
                            (
                                3,
                                vec![Value::Str(
                                    std::str::from_utf8(bytes)
                                        .map_err(|_| "SQLite text is not UTF-8")?
                                        .into(),
                                )],
                            )
                        }
                        ValueRef::Blob(bytes) => {
                            storage_charge(&mut used, bytes.len())?;
                            (4, vec![crate::filesystem::bytes_value(bytes.to_vec())])
                        }
                    };
                    values.push(Value::Enum(index, enum_ty.clone(), payload));
                }
                output.push(Value::Vector(Arc::new(values), row_ty.clone()));
            }
            Ok(crate::platform::record(
                ret,
                vec![
                    ("columns", crate::platform::strings(columns)),
                    (
                        "rows",
                        Value::Vector(Arc::new(output), Type::Vector(Box::new(row_ty))),
                    ),
                ],
            ))
        })();
        database
            .connection
            .progress_handler(0, None::<fn() -> bool>)
            .map_err(|e| e.to_string())?;
        result
    }
}
