// Node-style HTTP server handles remain owned by their interpreter thread.
struct DevServer {
    handler: Value,
    listener: Option<Arc<tiny_http::Server>>,
}
struct DevResponse {
    request: Option<tiny_http::Request>,
    status: u16,
    seen_status_code: i128,
    headers: Vec<tiny_http::Header>,
    body: Vec<u8>,
    ended: bool,
}
impl Engine {
    fn http_id(&mut self) -> Result<i64, String> {
        use std::sync::atomic::{AtomicI64, Ordering};
        static NEXT: AtomicI64 = AtomicI64::new(1);
        NEXT.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map_err(|_| "HTTP handle limit reached".into())
    }
    fn handle_id(value: &Value) -> Result<i64, String> {
        let Value::Record(fields, _) = value else {
            return Err("invalid HTTP handle".into());
        };
        match fields.iter().find(|(n, _)| n == "id").map(|(_, v)| v) {
            Some(Value::Int(id, _)) => i64::try_from(*id).map_err(|_| "invalid HTTP handle".into()),
            _ => Err("invalid HTTP handle".into()),
        }
    }
    fn server_call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let signature = self.modules["std/http"]
            .functions
            .get(name)
            .ok_or("unknown HTTP server API")?
            .clone();
        let normalized = name
            .replace("method_ServerResponse_", "ServerResponse.")
            .replace("method_Server_", "Server.");
        let name = normalized.as_str();
        if args.len() != signature.params.len() {
            return Err(format!("http.{name}: wrong argument count"));
        }
        let args = args
            .into_iter()
            .zip(&signature.params)
            .map(|(v, (_, t))| convert(v, t, false))
            .collect::<Result<Vec<_>, _>>()?;
        if name == "createServer" {
            if self.servers.len() >= 64 {
                return Err("at most 64 HTTP server handles per interpreter".into());
            }
            let id = self.http_id()?;
            self.servers.insert(
                id,
                DevServer {
                    handler: args[0].clone(),
                    listener: None,
                },
            );
            return Ok(Value::Record(
                vec![("id".into(), Value::Int(id as i128, Type::i64()))],
                signature.ret.clone(),
            ));
        }
        let id = Self::handle_id(&args[0])?;
        if name.starts_with("Server.") {
            let server = self
                .servers
                .get_mut(&id)
                .ok_or("unknown or closed HTTP server handle")?;
            if name == "Server.close" {
                self.servers.remove(&id);
                return Ok(Value::Void);
            }
            if name == "Server.on" {
                if args[1].string()? != "request" {
                    return Err("server.on currently supports only the request event".into());
                }
                server.handler = args[2].clone();
                return Ok(Value::Void);
            }
            if server.listener.is_some() {
                return Err("server is already listening".into());
            }
            let Value::Int(port, _) = args[1] else {
                unreachable!()
            };
            let port = u16::try_from(port).map_err(|_| "port must be between 0 and 65535")?;
            let host = if name == "Server.listenOn" {
                args[2].string()?
            } else {
                "0.0.0.0"
            };
            server.listener = Some(Arc::new(
                tiny_http::Server::http((host, port)).map_err(|e| format!("http.listen: {e}"))?,
            ));
            return Ok(Value::Void);
        }
        let response = self
            .responses
            .get_mut(&id)
            .ok_or("response is no longer active")?;
        if response.ended {
            return Err("response already ended".into());
        }
        if let Value::Record(fields, _) = &args[0] {
            if let Some((_, Value::Int(code, _))) = fields.iter().find(|(k, _)| k == "statusCode") {
                if *code != response.seen_status_code {
                    let code = u16::try_from(*code).map_err(|_| "invalid HTTP status")?;
                    if !(200..=599).contains(&code) {
                        return Err("final HTTP status must be between 200 and 599".into());
                    }
                    response.status = code;
                    response.seen_status_code = i128::from(code);
                }
            }
        }
        match name {
            "ServerResponse.writeHead" => {
                let Value::Int(status, _) = args[1] else {
                    unreachable!()
                };
                let status = u16::try_from(status).map_err(|_| "invalid HTTP status")?;
                if !(200..=599).contains(&status) {
                    return Err("final HTTP status must be between 200 and 599".into());
                }
                let Value::Map(headers, _) = &args[2] else {
                    unreachable!()
                };
                let parsed = headers
                    .entries()
                    .iter()
                    .map(|(k, v)| Self::response_header(k.string()?, v.string()?))
                    .collect::<Result<Vec<_>, _>>()?;
                response.status = status;
                for header in parsed {
                    response.headers.retain(|h| {
                        !h.field
                            .as_str()
                            .as_str()
                            .eq_ignore_ascii_case(header.field.as_str().as_str())
                    });
                    response.headers.push(header);
                }
            }
            "ServerResponse.setHeader" => {
                let name = args[1].string()?;
                let header = Self::response_header(name, args[2].string()?)?;
                response
                    .headers
                    .retain(|h| !h.field.as_str().as_str().eq_ignore_ascii_case(name));
                response.headers.push(header);
            }
            "ServerResponse.write" | "ServerResponse.end" => {
                let text = args[1].string()?;
                if response.body.len().saturating_add(text.len()) > crate::filesystem::MAX_BYTES {
                    return Err("response body exceeds 8 MiB".into());
                }
                response.body.extend_from_slice(text.as_bytes());
                if name == "ServerResponse.end" {
                    let request = response.request.take().ok_or("response already ended")?;
                    let mut reply =
                        tiny_http::Response::from_data(std::mem::take(&mut response.body))
                            .with_status_code(response.status);
                    for header in response.headers.drain(..) {
                        reply.add_header(header);
                    }
                    response.ended = true;
                    request
                        .respond(reply)
                        .map_err(|e| format!("http response: {e}"))?;
                }
            }
            _ => return Err("unknown server response API".into()),
        }
        Ok(Value::Void)
    }
    fn response_header(name: &str, value: &str) -> Result<tiny_http::Header, String> {
        // Let the HTTP library own framing; user-supplied lengths must not split responses.
        if name.eq_ignore_ascii_case("content-length")
            || name.eq_ignore_ascii_case("transfer-encoding")
        {
            return Err("response framing headers are managed by the runtime".into());
        }
        let name = ureq::http::HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| "invalid response header name")?;
        ureq::http::HeaderValue::from_str(value).map_err(|_| "invalid response header value")?;
        tiny_http::Header::from_bytes(name.as_str(), value)
            .map_err(|_| "response header requires ASCII text".into())
    }
    fn serve_http(&mut self) -> Result<(), String> {
        use std::io::Read;
        use std::time::Duration;
        loop {
            let active: Vec<_> = self
                .servers
                .iter()
                .filter_map(|(id, s)| {
                    s.listener
                        .as_ref()
                        .map(|l| (*id, l.clone(), s.handler.clone()))
                })
                .collect();
            if active.is_empty() {
                return Ok(());
            }
            let mut handled = false;
            for (server_id, listener, handler) in active {
                if !self.servers.contains_key(&server_id) {
                    continue;
                }
                let Some(mut request) = listener.try_recv().map_err(|e| e.to_string())? else {
                    continue;
                };
                handled = true;
                if request
                    .body_length()
                    .is_some_and(|n| n > crate::filesystem::MAX_BYTES)
                {
                    let _ = request.respond(
                        tiny_http::Response::from_string("Request body too large")
                            .with_status_code(413),
                    );
                    continue;
                }
                let mut data = Vec::new();
                let read = request
                    .as_reader()
                    .take((crate::filesystem::MAX_BYTES + 1) as u64)
                    .read_to_end(&mut data);
                if read.is_err() || data.len() > crate::filesystem::MAX_BYTES {
                    let _ = request.respond(
                        tiny_http::Response::from_string("Request body too large or unreadable")
                            .with_status_code(413),
                    );
                    continue;
                }
                let mut headers = MapStorage::default();
                for header in request.headers() {
                    let key = Value::Str(header.field.as_str().as_str().to_ascii_lowercase());
                    let text = header.value.as_str().to_owned();
                    // Repeated incoming headers retain their values.
                    let text = if let Some((_, Value::Str(old))) = headers
                        .entries()
                        .iter()
                        .find(|(k, _)| MapStorage::equal(k, &key))
                    {
                        format!("{old}\n{text}")
                    } else {
                        text
                    };
                    headers.set(key, Value::Str(text))?;
                }
                let Value::Callable(module, function, context, Type::Function(params, _)) = handler
                else {
                    return Err("invalid server callback".into());
                };
                let incoming = Value::Record(
                    vec![
                        (
                            "method".into(),
                            Value::Str(request.method().as_str().into()),
                        ),
                        ("url".into(), Value::Str(request.url().into())),
                        (
                            "headers".into(),
                            Value::Map(
                                Arc::new(headers),
                                Type::Map(Box::new(Type::Str), Box::new(Type::Str)),
                            ),
                        ),
                        (
                            "body".into(),
                            Value::Str(String::from_utf8_lossy(&data).into_owned()),
                        ),
                        ("bytes".into(), crate::filesystem::bytes_value(data)),
                    ],
                    params[0].clone(),
                );
                let id = self.http_id()?;
                let outgoing = Value::Record(
                    vec![
                        ("id".into(), Value::Int(id as i128, Type::i64())),
                        ("statusCode".into(), Value::Int(200, Type::i64())),
                    ],
                    params[1].clone(),
                );
                self.responses.insert(
                    id,
                    DevResponse {
                        request: Some(request),
                        status: 200,
                        seen_status_code: 200,
                        headers: vec![],
                        body: vec![],
                        ended: false,
                    },
                );
                let mut values = vec![];
                if let Some(v) = context {
                    let inner = match v.ty() {
                        Type::Record(n, _) | Type::Enum(n, _) => Type::Nominal(n),
                        t => t,
                    };
                    values.push(Value::Ref(v, Type::Ref(Box::new(inner))));
                }
                values.extend([incoming, outgoing]);
                let result = self.call(&module, &[function], values, Span { line: 1, col: 1 });
                let mut response = self.responses.remove(&id).unwrap();
                if let Some(request) = response.request.take() {
                    let _ = request.respond(
                        tiny_http::Response::from_string("Handler must call res.end(text)")
                            .with_status_code(500),
                    );
                }
                result?;
            }
            if !handled {
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
}
