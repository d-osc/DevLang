use crate::{
    engine::{MapStorage, Value},
    filesystem::{bytes, bytes_value, MAX_BYTES},
};
use dev_syntax::ast::Type;
use std::{sync::Arc, time::Duration};

pub(crate) fn call(
    name: &str,
    args: Vec<Value>,
    ty: Type,
    cached: &mut Option<ureq::Agent>,
) -> Result<Value, String> {
    let count = match name {
        "get" | "head" | "delete" => 1,
        "post" | "put" | "patch" => 2,
        "request" | "request_bytes" => 5,
        _ => return Err(format!("unknown intrinsic std/http.{name}")),
    };
    if args.len() != count {
        return Err(format!("http.{name} expects {count} arguments"));
    }
    let result: Result<Value, String> = (|| {
        let explicit = count == 5;
        let method = if explicit {
            args[0].string()?.to_ascii_uppercase()
        } else {
            name.to_ascii_uppercase()
        };
        if !["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"].contains(&method.as_str())
        {
            return Err("unsupported HTTP method".into());
        }
        let url = args[usize::from(explicit)].string()?;
        let uri: ureq::http::Uri = url.parse().map_err(|_| "invalid HTTP URL")?;
        if !matches!(uri.scheme_str(), Some("http" | "https")) || uri.host().is_none() {
            return Err("URL must use http:// or https:// and contain a host".into());
        }
        let mut request = ureq::http::Request::builder()
            .method(method.as_str())
            .uri(uri);
        let timeout = if explicit {
            match &args[4] {
                Value::Int(n, t) if *t == Type::i64() && (1..=300_000).contains(n) => *n as u64,
                _ => return Err("timeout_ms must be i64 between 1 and 300000".into()),
            }
        } else {
            10_000
        };
        if explicit {
            let Value::Map(headers, Type::Map(k, v)) = &args[2] else {
                return Err("headers must be Map<str,str>".into());
            };
            if **k != Type::Str || **v != Type::Str {
                return Err("headers must be Map<str,str>".into());
            }
            for (key, value) in headers.entries() {
                request = request.header(key.string()?, value.string()?);
            }
        }
        let body = if explicit {
            if name == "request_bytes" {
                bytes(&args[3])?
            } else {
                args[3].string()?.as_bytes().to_vec()
            }
        } else if count == 2 {
            args[1].string()?.as_bytes().to_vec()
        } else {
            vec![]
        };
        if body.len() > MAX_BYTES {
            return Err("request body exceeds 8 MiB".into());
        }
        let request = request
            .body(body)
            .map_err(|e| format!("invalid request/header: {e}"))?;
        let agent = cached.get_or_insert_with(|| {
            ureq::Agent::config_builder()
                .http_status_as_error(false)
                .max_redirects(5)
                .timeout_global(Some(Duration::from_secs(10)))
                .build()
                .new_agent()
        });
        let request = agent
            .configure_request(request)
            .timeout_global(Some(Duration::from_millis(timeout)))
            .build();
        let mut response = agent.run(request).map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let mut headers = MapStorage::default();
        // Preserve repeated values as newline-separated strings (including Set-Cookie).
        for key in response.headers().keys() {
            let value = response
                .headers()
                .get_all(key)
                .iter()
                .map(|v| {
                    v.to_str()
                        .map(str::to_owned)
                        .map_err(|_| "response header is not valid text".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?
                .join("\n");
            headers.set(Value::Str(key.as_str().into()), Value::Str(value))?;
        }
        let data = response
            .body_mut()
            .with_config()
            .limit(MAX_BYTES as u64)
            .read_to_vec()
            .map_err(|e| e.to_string())?;
        let body = String::from_utf8_lossy(&data).into_owned();
        Ok(Value::Record(
            vec![
                ("status".into(), Value::Int(status as i128, Type::i64())),
                ("body".into(), Value::Str(body)),
                ("bytes".into(), bytes_value(data)),
                (
                    "headers".into(),
                    Value::Map(
                        Arc::new(headers),
                        Type::Map(Box::new(Type::Str), Box::new(Type::Str)),
                    ),
                ),
                ("ok".into(), Value::Bool((200..300).contains(&status))),
            ],
            ty,
        ))
    })();
    result.map_err(|e| format!("http.{name}: {e}"))
}
