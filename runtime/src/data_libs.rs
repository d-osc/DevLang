fn data_append(output: &mut String, text: &str) -> Result<(), String> {
    if output.len().saturating_add(text.len()) > BASIC_LIMIT {
        return Err("text output exceeds 8 MiB".into());
    }
    output.push_str(text);
    Ok(())
}
fn data_expand(
    caps: &regex::Captures<'_>,
    replacement: &str,
    out: &mut String,
) -> Result<(), String> {
    let mut rest = replacement;
    while let Some(index) = rest.find('$') {
        data_append(out, &rest[..index])?;
        rest = &rest[index + 1..];
        if rest.starts_with('$') {
            data_append(out, "$")?;
            rest = &rest[1..];
            continue;
        }
        let (name, consumed) = if rest.starts_with('{') {
            match rest.find('}') {
                Some(end) => (&rest[1..end], end + 1),
                None => {
                    data_append(out, "$")?;
                    continue;
                }
            }
        } else {
            let end = rest
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            (&rest[..end], end)
        };
        if consumed == 0 {
            data_append(out, "$")?;
            continue;
        }
        let capture = if let Ok(index) = name.parse::<usize>() {
            caps.get(index)
        } else {
            caps.name(name)
        };
        if let Some(value) = capture {
            data_append(out, value.as_str())?;
        }
        rest = &rest[consumed..];
    }
    data_append(out, rest)
}
fn data_charset(label: &str) -> Result<&'static encoding_rs::Encoding, String> {
    encoding_rs::Encoding::for_label(label.as_bytes())
        .ok_or_else(|| format!("unsupported encoding '{label}'"))
}
fn data_encode(text: &str, label: &str) -> Result<Vec<u8>, String> {
    if text.len() > BASIC_LIMIT {
        return Err("encoding input exceeds 8 MiB".into());
    }
    let charset = data_charset(label)?;
    let bytes = if charset == encoding_rs::UTF_16LE || charset == encoding_rs::UTF_16BE {
        text.encode_utf16()
            .flat_map(|n| {
                if charset == encoding_rs::UTF_16LE {
                    n.to_le_bytes()
                } else {
                    n.to_be_bytes()
                }
            })
            .collect()
    } else {
        let (bytes, _, errors) = charset.encode(text);
        if errors {
            return Err("text is not representable in requested encoding".into());
        }
        bytes.into_owned()
    };
    if bytes.len() > BASIC_LIMIT {
        return Err("encoding output exceeds 8 MiB".into());
    }
    Ok(bytes)
}
fn data_decode(bytes: &[u8], label: &str) -> Result<String, String> {
    let charset = data_charset(label)?;
    let text = if charset == encoding_rs::UTF_16LE || charset == encoding_rs::UTF_16BE {
        if bytes.len() % 2 != 0 {
            return Err("UTF-16 requires an even byte length".into());
        }
        let units = bytes
            .chunks_exact(2)
            .map(|b| {
                if charset == encoding_rs::UTF_16LE {
                    u16::from_le_bytes([b[0], b[1]])
                } else {
                    u16::from_be_bytes([b[0], b[1]])
                }
            })
            .collect::<Vec<_>>();
        String::from_utf16(&units).map_err(|_| "invalid UTF-16 surrogate sequence")?
    } else {
        charset
            .decode_without_bom_handling_and_without_replacement(bytes)
            .ok_or("invalid byte sequence for encoding")?
            .into_owned()
    };
    if text.len() > BASIC_LIMIT {
        return Err("decoded text exceeds 8 MiB".into());
    }
    Ok(text)
}
fn data_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn data_strings<'a>(items: impl Iterator<Item = &'a str>) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    let mut size = 0usize;
    for item in items {
        size = size
            .checked_add(item.len())
            .ok_or("regex results size overflow")?;
        if result.len() >= 4096 || size > BASIC_LIMIT {
            return Err("regex results exceed 4096 items or 8 MiB".into());
        }
        result.push(item.to_string());
    }
    Ok(result)
}
#[derive(Default)]
struct DataWriter {
    cursor: std::io::Cursor<Vec<u8>>,
}
impl std::io::Write for DataWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self
            .cursor
            .position()
            .checked_add(bytes.len() as u64)
            .is_none_or(|n| n > BASIC_LIMIT as u64)
        {
            return Err(std::io::Error::other("output exceeds 8 MiB"));
        }
        std::io::Write::write(&mut self.cursor, bytes)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::io::Seek for DataWriter {
    fn seek(&mut self, from: std::io::SeekFrom) -> std::io::Result<u64> {
        let pos = match from {
            std::io::SeekFrom::Start(n) => n as i128,
            std::io::SeekFrom::End(n) => self.cursor.get_ref().len() as i128 + n as i128,
            std::io::SeekFrom::Current(n) => self.cursor.position() as i128 + n as i128,
        };
        if pos < 0 || pos > BASIC_LIMIT as i128 {
            return Err(std::io::Error::other("output seek exceeds bounds"));
        }
        self.cursor.set_position(pos as u64);
        Ok(pos as u64)
    }
}
fn data_archive_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 1024
        || name.starts_with('/')
        || name.contains(['\\', ':', '\0'])
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err("archive name must be a relative slash-separated path without '..'".into());
    }
    Ok(())
}
fn data_read(mut reader: impl std::io::Read) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    std::io::Read::take(&mut reader, (BASIC_LIMIT + 1) as u64)
        .read_to_end(&mut output)
        .map_err(|e| e.to_string())?;
    if output.len() > BASIC_LIMIT {
        return Err("decompressed data exceeds 8 MiB".into());
    }
    Ok(output)
}
use std::io::Read as _;
impl Engine {
    fn regex_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        if name == "escape" {
            let text = args[0].string()?;
            if text.len() > 65536 {
                return Err("regex text exceeds 64 KiB".into());
            }
            return Self::basic_text(regex::escape(text));
        }
        if name == "compile" {
            self.core_capacity()?;
            if self.core.regexes.len() >= 32 {
                return Err("at most 32 compiled regex handles; close unused regexes".into());
            }
            let pattern = args[0].string()?;
            if pattern.len() > 65536 {
                return Err("regex pattern exceeds 64 KiB".into());
            }
            let mut builder = regex::RegexBuilder::new(pattern);
            builder
                .size_limit(2 * 1024 * 1024)
                .dfa_size_limit(2 * 1024 * 1024);
            let mut seen = std::collections::HashSet::new();
            for flag in args[1].string()?.chars() {
                if !seen.insert(flag) {
                    return Err("duplicate regex flag".into());
                }
                match flag {
                    'i' => {
                        builder.case_insensitive(true);
                    }
                    'm' => {
                        builder.multi_line(true);
                    }
                    's' => {
                        builder.dot_matches_new_line(true);
                    }
                    'U' => {
                        builder.swap_greed(true);
                    }
                    _ => return Err("regex flags support only i/m/s/U".into()),
                }
            }
            let compiled = builder.build().map_err(|e| format!("invalid regex: {e}"))?;
            let id = self.http_id()?;
            self.core.regexes.insert(id, compiled);
            return Ok(Self::core_handle(id, ret));
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_Regex_close" {
            self.core
                .regexes
                .remove(&id)
                .ok_or("regex is closed or belongs to another thread")?;
            return Ok(Value::Void);
        }
        let regex = self
            .core
            .regexes
            .get(&id)
            .ok_or("regex is closed or belongs to another thread")?;
        let text = args[1].string()?;
        if text.len() > BASIC_LIMIT {
            return Err("regex input exceeds 8 MiB".into());
        }
        match name {
            "method_Regex_test" => Ok(Value::Bool(regex.is_match(text))),
            "method_Regex_find" => {
                let found = regex.find(text);
                let integer = |n: i128| Value::Int(n, Type::i64());
                Ok(crate::platform::record(
                    ret,
                    vec![
                        ("matched", Value::Bool(found.is_some())),
                        ("text", Value::Str(found.map_or("", |m| m.as_str()).into())),
                        ("start", integer(found.map_or(-1, |m| m.start() as i128))),
                        ("end", integer(found.map_or(-1, |m| m.end() as i128))),
                    ],
                ))
            }
            "method_Regex_findAll" | "method_Regex_split" | "method_Regex_captures" => {
                let items = match name {
                    "method_Regex_findAll" => {
                        data_strings(regex.find_iter(text).map(|m| m.as_str()))?
                    }
                    "method_Regex_split" => data_strings(regex.split(text))?,
                    _ => match regex.captures(text) {
                        Some(c) => data_strings(c.iter().map(|m| m.map_or("", |m| m.as_str())))?,
                        None => Vec::new(),
                    },
                };
                if items.len() > 4096 || items.iter().map(String::len).sum::<usize>() > BASIC_LIMIT
                {
                    return Err("regex results exceed 4096 items or 8 MiB".into());
                }
                Ok(crate::platform::strings(items))
            }
            "method_Regex_replace" | "method_Regex_replaceAll" => {
                let replacement = args[2].string()?;
                if replacement.len() > 65536 {
                    return Err("regex replacement exceeds 64 KiB".into());
                }
                let limit = if name == "method_Regex_replace" {
                    1
                } else {
                    usize::MAX
                };
                let mut output = String::new();
                let mut end = 0;
                for captures in regex.captures_iter(text).take(limit) {
                    let whole = captures.get(0).ok_or("regex match unavailable")?;
                    data_append(&mut output, &text[end..whole.start()])?;
                    data_expand(&captures, replacement, &mut output)?;
                    end = whole.end();
                }
                data_append(&mut output, &text[end..])?;
                Ok(Value::Str(output))
            }
            _ => Err("unknown regex API".into()),
        }
    }
    fn encoding_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        match name {
            "supported" => Ok(Value::Bool(data_charset(args[0].string()?).is_ok())),
            "encode" => Ok(crate::filesystem::bytes_value(data_encode(
                args[0].string()?,
                args[1].string()?,
            )?)),
            "decode" => Ok(Value::Str(data_decode(
                &crate::filesystem::bytes(&args[0])?,
                args[1].string()?,
            )?)),
            "transcode" => {
                let text = data_decode(&crate::filesystem::bytes(&args[0])?, args[1].string()?)?;
                Ok(crate::filesystem::bytes_value(data_encode(
                    &text,
                    args[2].string()?,
                )?))
            }
            _ => Err("unknown encoding API".into()),
        }
    }
    fn crypto_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        use aes_gcm::{
            aead::{Aead, KeyInit, Payload},
            Aes256Gcm, Nonce,
        };
        use hmac::{Hmac, Mac};
        use sha2::{Digest, Sha256, Sha512};
        use subtle::ConstantTimeEq;
        if name == "secureBytes" {
            let count = Self::core_size(&args[0], BASIC_LIMIT)?;
            let mut bytes = vec![0u8; count];
            getrandom::getrandom(&mut bytes).map_err(|e| format!("OS randomness failed: {e}"))?;
            return Ok(crate::filesystem::bytes_value(bytes));
        }
        if name == "timingSafeEqual" {
            let a = crate::filesystem::bytes(&args[0])?;
            let b = crate::filesystem::bytes(&args[1])?;
            return Ok(Value::Bool(a.len() == b.len() && bool::from(a.ct_eq(&b))));
        }
        if matches!(name, "encrypt" | "decrypt") {
            let key = crate::filesystem::bytes(&args[0])?;
            let cipher = Aes256Gcm::new_from_slice(&key)
                .map_err(|_| "AES-256-GCM key must contain 32 bytes")?;
            let data = crate::filesystem::bytes(&args[1])?;
            let aad = crate::filesystem::bytes(&args[2])?;
            let bytes = if name == "encrypt" {
                if data.len() > BASIC_LIMIT - 28 {
                    return Err("encrypted packet exceeds 8 MiB".into());
                }
                let mut nonce = [0u8; 12];
                getrandom::getrandom(&mut nonce)
                    .map_err(|e| format!("OS randomness failed: {e}"))?;
                let encrypted = cipher
                    .encrypt(
                        Nonce::from_slice(&nonce),
                        Payload {
                            msg: &data,
                            aad: &aad,
                        },
                    )
                    .map_err(|_| "encryption failed")?;
                let mut packet = nonce.to_vec();
                packet.extend_from_slice(&encrypted);
                packet
            } else {
                if data.len() < 28 {
                    return Err("AES-GCM packet requires nonce and authentication tag".into());
                }
                cipher
                    .decrypt(
                        Nonce::from_slice(&data[..12]),
                        Payload {
                            msg: &data[12..],
                            aad: &aad,
                        },
                    )
                    .map_err(|_| "authentication failed")?
            };
            return Ok(crate::filesystem::bytes_value(bytes));
        }
        let algorithm = args[0].string()?;
        if !matches!(algorithm, "sha256" | "sha512") {
            return Err("hash algorithm must be sha256 or sha512".into());
        }
        if matches!(name, "hash" | "hashText") {
            let data = if name == "hashText" {
                let text = args[1].string()?;
                if text.len() > BASIC_LIMIT {
                    return Err("hash text exceeds 8 MiB".into());
                }
                text.as_bytes().to_vec()
            } else {
                crate::filesystem::bytes(&args[1])?
            };
            let bytes = if algorithm == "sha256" {
                Sha256::digest(data).to_vec()
            } else {
                Sha512::digest(data).to_vec()
            };
            return Ok(if name == "hashText" {
                Value::Str(data_hex(&bytes))
            } else {
                crate::filesystem::bytes_value(bytes)
            });
        }
        if matches!(name, "hmac" | "verifyHmac") {
            let key = crate::filesystem::bytes(&args[1])?;
            let data = crate::filesystem::bytes(&args[2])?;
            macro_rules! mac {
                ($hash:ty) => {{
                    let mut mac = <Hmac<$hash> as Mac>::new_from_slice(&key)
                        .map_err(|_| "invalid HMAC key")?;
                    mac.update(&data);
                    if name == "verifyHmac" {
                        let tag = crate::filesystem::bytes(&args[3])?;
                        Ok(Value::Bool(mac.verify_slice(&tag).is_ok()))
                    } else {
                        Ok(crate::filesystem::bytes_value(
                            mac.finalize().into_bytes().to_vec(),
                        ))
                    }
                }};
            }
            return if algorithm == "sha256" {
                mac!(Sha256)
            } else {
                mac!(Sha512)
            };
        }
        Err("unknown crypto API".into())
    }
    fn compression_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        use std::io::Write;
        let input = crate::filesystem::bytes(&args[0])?;
        let bytes = if matches!(name, "gzip" | "zlib" | "deflate") {
            let level = Self::basic_i64(&args[1])?;
            if !(0..=9).contains(&level) {
                return Err("compression level must be 0..9".into());
            }
            let level = flate2::Compression::new(level as u32);
            macro_rules! compress {
                ($kind:ident) => {{
                    let mut encoder = flate2::write::$kind::new(DataWriter::default(), level);
                    encoder.write_all(&input).map_err(|e| e.to_string())?;
                    encoder
                        .finish()
                        .map_err(|e| e.to_string())?
                        .cursor
                        .into_inner()
                }};
            }
            match name {
                "gzip" => compress!(GzEncoder),
                "zlib" => compress!(ZlibEncoder),
                _ => compress!(DeflateEncoder),
            }
        } else if name == "gunzip" {
            data_read(flate2::read::MultiGzDecoder::new(input.as_slice()))?
        } else if matches!(name, "unzlib" | "inflate") {
            let mut decoder = flate2::Decompress::new(name == "unzlib");
            let mut output = Vec::new();
            let mut chunk = [0u8; 65536];
            loop {
                let start_in = decoder.total_in();
                let start_out = decoder.total_out();
                let status = decoder
                    .decompress(
                        &input[start_in as usize..],
                        &mut chunk,
                        flate2::FlushDecompress::Finish,
                    )
                    .map_err(|e| format!("invalid compressed data: {e}"))?;
                let produced = (decoder.total_out() - start_out) as usize;
                if output.len() + produced > BASIC_LIMIT {
                    return Err("decompressed data exceeds 8 MiB".into());
                }
                output.extend_from_slice(&chunk[..produced]);
                if status == flate2::Status::StreamEnd {
                    if decoder.total_in() != input.len() as u64 {
                        return Err("trailing compressed data".into());
                    }
                    break;
                }
                if decoder.total_in() == start_in && produced == 0 {
                    return Err("truncated compressed data".into());
                }
            }
            output
        } else {
            return Err("unknown compression API".into());
        };
        Ok(crate::filesystem::bytes_value(bytes))
    }
    fn archive_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        use std::collections::HashSet;
        use std::io::{Cursor, Write};
        if matches!(name, "writeZIP" | "writeTAR") {
            let Value::Vector(entries, _) = &args[0] else {
                return Err("expected Vec<archive.Entry>".into());
            };
            if entries.len() > 4096 {
                return Err("archive exceeds 4096 entries".into());
            }
            let mut seen = HashSet::new();
            let mut total = 0usize;
            let mut files = Vec::new();
            for entry in entries.iter() {
                let path = crate::platform::field(entry, "name")?.string()?.to_string();
                data_archive_name(&path)?;
                if !seen.insert(path.clone()) {
                    return Err("duplicate archive name".into());
                }
                let bytes = crate::filesystem::bytes(crate::platform::field(entry, "data")?)?;
                total += bytes.len();
                if total > BASIC_LIMIT {
                    return Err("archive file data exceeds 8 MiB".into());
                }
                files.push((path, bytes));
            }
            let writer = if name == "writeZIP" {
                let mut archive = zip::ZipWriter::new(DataWriter::default());
                for (path, bytes) in files {
                    archive
                        .start_file(
                            path,
                            zip::write::SimpleFileOptions::default()
                                .compression_method(zip::CompressionMethod::Deflated)
                                .unix_permissions(0o644),
                        )
                        .map_err(|e| e.to_string())?;
                    archive.write_all(&bytes).map_err(|e| e.to_string())?;
                }
                archive.finish().map_err(|e| e.to_string())?
            } else {
                let mut archive = tar::Builder::new(DataWriter::default());
                for (path, bytes) in files {
                    let mut header = tar::Header::new_gnu();
                    header.set_size(bytes.len() as u64);
                    header.set_mode(0o644);
                    header.set_uid(0);
                    header.set_gid(0);
                    header.set_mtime(0);
                    header.set_cksum();
                    archive
                        .append_data(&mut header, path, bytes.as_slice())
                        .map_err(|e| e.to_string())?;
                }
                archive.into_inner().map_err(|e| e.to_string())?
            };
            return Ok(crate::filesystem::bytes_value(writer.cursor.into_inner()));
        }
        let input = crate::filesystem::bytes(&args[0])?;
        let target = if name.starts_with("read") {
            let path = args[1].string()?;
            data_archive_name(path)?;
            Some(path)
        } else {
            None
        };
        let mut seen = HashSet::new();
        let mut names = Vec::new();
        let mut total = 0u64;
        let mut result = None;
        if matches!(name, "listZIP" | "readZIP") {
            let mut archive = zip::ZipArchive::new(Cursor::new(input))
                .map_err(|e| format!("invalid ZIP: {e}"))?;
            if archive.len() > 4096 {
                return Err("archive exceeds 4096 entries".into());
            }
            for i in 0..archive.len() {
                let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
                let path = file.name().map_err(|e| e.to_string())?.into_owned();
                if file.is_dir() {
                    data_archive_name(path.trim_end_matches('/'))?;
                    continue;
                }
                data_archive_name(&path)?;
                if file.is_symlink() || file.encrypted() {
                    return Err("archive links/encrypted entries are unsupported".into());
                }
                if !seen.insert(path.clone()) {
                    return Err("duplicate archive name".into());
                }
                total = total
                    .checked_add(file.size())
                    .ok_or("archive size overflow")?;
                if total > BASIC_LIMIT as u64 {
                    return Err("archive expanded data exceeds 8 MiB".into());
                }
                if target == Some(path.as_str()) {
                    result = Some(data_read(&mut file)?);
                }
                names.push(path);
            }
        } else if matches!(name, "listTAR" | "readTAR") {
            let input_len = input.len() as u64;
            let mut count = 0usize;
            let mut archive = tar::Archive::new(Cursor::new(input));
            for item in archive.entries().map_err(|e| e.to_string())? {
                count += 1;
                if count > 4096 {
                    return Err("archive exceeds 4096 entries".into());
                }
                let mut entry = item.map_err(|e| format!("invalid TAR: {e}"))?;
                if entry
                    .raw_file_position()
                    .checked_add(entry.size())
                    .is_none_or(|end| end > input_len)
                {
                    return Err("truncated TAR data".into());
                }
                let path = entry
                    .path()
                    .map_err(|e| e.to_string())?
                    .to_str()
                    .ok_or("archive path is not UTF-8")?
                    .to_string();
                if entry.header().entry_type().is_dir() {
                    data_archive_name(path.trim_end_matches('/'))?;
                    continue;
                }
                if !entry.header().entry_type().is_file() {
                    return Err("archive links/special entries are unsupported".into());
                }
                data_archive_name(&path)?;
                if !seen.insert(path.clone()) {
                    return Err("duplicate archive name".into());
                }
                total = total
                    .checked_add(entry.size())
                    .ok_or("archive size overflow")?;
                if names.len() >= 4096 || total > BASIC_LIMIT as u64 {
                    return Err("archive exceeds 4096 entries or 8 MiB expanded data".into());
                }
                if target == Some(path.as_str()) {
                    result = Some(data_read(&mut entry)?);
                }
                names.push(path);
            }
        } else {
            return Err("unknown archive API".into());
        }
        if target.is_some() {
            Ok(crate::filesystem::bytes_value(
                result.ok_or("archive member not found")?,
            ))
        } else {
            Ok(crate::platform::strings(names))
        }
    }
    fn uuid_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        use uuid::{Builder, Uuid};
        if name == "v4" {
            let mut bytes = [0u8; 16];
            getrandom::getrandom(&mut bytes).map_err(|e| format!("OS randomness failed: {e}"))?;
            return Ok(Value::Str(
                Builder::from_random_bytes(bytes).into_uuid().to_string(),
            ));
        }
        if name == "nil" {
            return Ok(Value::Str(Uuid::nil().to_string()));
        }
        let parsed = Uuid::parse_str(args[0].string()?);
        if name == "isValid" {
            return Ok(Value::Bool(parsed.is_ok()));
        }
        let id = parsed.map_err(|e| format!("invalid UUID: {e}"))?;
        match name {
            "parse" => Ok(Value::Str(id.to_string())),
            "version" => Ok(Value::Int(id.get_version_num() as i128, Type::i64())),
            _ => Err("unknown UUID API".into()),
        }
    }
}
