enum SecureTransport {
    Plain(std::net::TcpStream),
    Client(Box<rustls::StreamOwned<rustls::ClientConnection, std::net::TcpStream>>),
    Server(Box<rustls::StreamOwned<rustls::ServerConnection, std::net::TcpStream>>),
}
impl SecureTransport {
    fn tcp(&self) -> &std::net::TcpStream {
        match self {
            Self::Plain(s) => s,
            Self::Client(s) => &s.sock,
            Self::Server(s) => &s.sock,
        }
    }
    fn timeout(&self, duration: std::time::Duration) -> Result<(), String> {
        self.tcp()
            .set_read_timeout(Some(duration))
            .map_err(|e| e.to_string())?;
        self.tcp()
            .set_write_timeout(Some(duration))
            .map_err(|e| e.to_string())
    }
}
impl std::io::Read for SecureTransport {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.read(output),
            Self::Client(s) => s.read(output),
            Self::Server(s) => s.read(output),
        }
    }
}
impl std::io::Write for SecureTransport {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(s) => s.write(input),
            Self::Client(s) => s.write(input),
            Self::Server(s) => s.write(input),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(s) => s.flush(),
            Self::Client(s) => s.flush(),
            Self::Server(s) => s.flush(),
        }
    }
}
struct SecureServer {
    config: Option<Arc<rustls::ServerConfig>>,
    websocket: bool,
    listener: Option<std::net::TcpListener>,
    callback: Value,
}
struct CoreWebSocket {
    socket: tungstenite::WebSocket<SecureTransport>,
    path: String,
    origin: String,
    closed: bool,
}
fn secure_pem(path: &str) -> Result<Vec<u8>, String> {
    use std::io::Read;
    if path.is_empty() || path.len() > 32768 || path.contains('\0') {
        return Err("invalid PEM path".into());
    }
    let mut data = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(65537)
        .read_to_end(&mut data)
        .map_err(|e| e.to_string())?;
    if data.len() > 65536 {
        return Err("PEM file exceeds 64 KiB".into());
    }
    Ok(data)
}
fn secure_client_config(ca_file: &str) -> Result<Arc<rustls::ClientConfig>, String> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    if !ca_file.is_empty() {
        let data = secure_pem(ca_file)?;
        let certs = rustls_pemfile::certs(&mut data.as_slice())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        if certs.is_empty() {
            return Err("CA file contains no PEM certificates".into());
        }
        for cert in certs {
            roots.add(cert).map_err(|e| e.to_string())?;
        }
    }
    Ok(Arc::new(
        rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_root_certificates(roots)
        .with_no_client_auth(),
    ))
}
fn secure_server_config(
    cert_file: &str,
    key_file: &str,
) -> Result<Arc<rustls::ServerConfig>, String> {
    let certs = secure_pem(cert_file)?;
    let key = secure_pem(key_file)?;
    let certs = rustls_pemfile::certs(&mut certs.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let key = rustls_pemfile::private_key(&mut key.as_slice())
        .map_err(|e| e.to_string())?
        .ok_or("key file contains no PEM private key")?;
    Ok(Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| e.to_string())?,
    ))
}
fn secure_tcp(
    host: &str,
    port: u16,
    timeout: std::time::Duration,
) -> Result<std::net::TcpStream, String> {
    use std::net::ToSocketAddrs;
    if host.is_empty() || host.len() > 253 || host.contains('\0') {
        return Err("invalid connection host".into());
    }
    let start = Instant::now();
    let mut last = "host resolved to no addresses".to_string();
    let addresses: Vec<_> = (host, port)
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .collect();
    for (index, address) in addresses.iter().enumerate() {
        let remaining = timeout
            .checked_sub(start.elapsed())
            .ok_or("connect timeout")?;
        let attempt = remaining / ((addresses.len() - index) as u32).max(1);
        match std::net::TcpStream::connect_timeout(
            address,
            attempt.max(std::time::Duration::from_nanos(1)),
        ) {
            Ok(socket) => {
                socket
                    .set_read_timeout(Some(timeout))
                    .map_err(|e| e.to_string())?;
                socket
                    .set_write_timeout(Some(timeout))
                    .map_err(|e| e.to_string())?;
                return Ok(socket);
            }
            Err(error) => last = error.to_string(),
        }
    }
    Err(last)
}
fn secure_client(
    socket: std::net::TcpStream,
    server_name: &str,
    ca_file: &str,
) -> Result<SecureTransport, String> {
    let name = rustls::pki_types::ServerName::try_from(server_name.to_owned())
        .map_err(|_| "invalid TLS server name")?;
    let mut connection = rustls::ClientConnection::new(secure_client_config(ca_file)?, name)
        .map_err(|e| e.to_string())?;
    connection.set_buffer_limit(Some(BASIC_LIMIT));
    let mut stream = rustls::StreamOwned::new(connection, socket);
    while stream.conn.is_handshaking() {
        stream
            .conn
            .complete_io(&mut stream.sock)
            .map_err(|e| format!("TLS handshake failed: {e}"))?;
    }
    Ok(SecureTransport::Client(Box::new(stream)))
}
fn secure_server(
    socket: std::net::TcpStream,
    config: Arc<rustls::ServerConfig>,
) -> Result<SecureTransport, String> {
    let mut connection = rustls::ServerConnection::new(config).map_err(|e| e.to_string())?;
    connection.set_buffer_limit(Some(BASIC_LIMIT));
    let mut stream = rustls::StreamOwned::new(connection, socket);
    while stream.conn.is_handshaking() {
        stream
            .conn
            .complete_io(&mut stream.sock)
            .map_err(|e| format!("TLS handshake failed: {e}"))?;
    }
    Ok(SecureTransport::Server(Box::new(stream)))
}
fn websocket_config() -> tungstenite::protocol::WebSocketConfig {
    let mut config = tungstenite::protocol::WebSocketConfig::default();
    config.max_message_size = Some(BASIC_LIMIT);
    config.max_frame_size = Some(BASIC_LIMIT);
    config.max_write_buffer_size = BASIC_LIMIT + 65536;
    config
}
impl Engine {
    fn secure_active(&self) -> bool {
        self.core
            .secure_servers
            .values()
            .any(|server| server.listener.is_some())
    }
    fn secure_create(
        &mut self,
        websocket: bool,
        config: Option<Arc<rustls::ServerConfig>>,
        callback: Value,
        ret: Type,
    ) -> Result<Value, String> {
        self.core_capacity()?;
        if self.core.secure_servers.len() >= 16 {
            return Err("at most 16 TLS/WebSocket servers per interpreter".into());
        }
        let id = self.http_id()?;
        self.core.secure_servers.insert(
            id,
            SecureServer {
                config,
                websocket,
                listener: None,
                callback,
            },
        );
        Ok(Self::core_handle(id, ret))
    }
    fn secure_server_call(
        &mut self,
        name: &str,
        args: Vec<Value>,
        ret: Type,
    ) -> Result<Value, String> {
        let id = Self::handle_id(&args[0])?;
        if name == "method_Server_close" {
            self.core
                .secure_servers
                .remove(&id)
                .ok_or("TLS/WebSocket server is closed")?;
            return Ok(Value::Void);
        }
        let server = self
            .core
            .secure_servers
            .get_mut(&id)
            .ok_or("server is closed or belongs to another thread")?;
        match name {
            "method_Server_listen" | "method_Server_listenOn" => {
                if server.listener.is_some() {
                    return Err("server is already listening".into());
                }
                let port = Self::core_port(&args[1])?;
                let host = if name.ends_with("On") {
                    args[2].string()?
                } else {
                    "0.0.0.0"
                };
                let listener =
                    std::net::TcpListener::bind((host, port)).map_err(|e| e.to_string())?;
                listener.set_nonblocking(true).map_err(|e| e.to_string())?;
                server.listener = Some(listener);
                Ok(Value::Void)
            }
            "method_Server_address" => Ok(Self::core_address(
                server
                    .listener
                    .as_ref()
                    .ok_or("server is not listening")?
                    .local_addr()
                    .map_err(|e| e.to_string())?,
                ret,
            )),
            "method_Server_on" => {
                if args[1].string()? != "connection" {
                    return Err("server.on supports only connection".into());
                }
                server.callback = args[2].clone();
                Ok(Value::Void)
            }
            _ => Err("unknown TLS/WebSocket server API".into()),
        }
    }
    fn tls_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use std::io::{Read, Write};
        if name.starts_with("method_Server_") {
            return self.secure_server_call(name, args, ret);
        }
        match name {
            "options" => {
                return Ok(crate::platform::record(
                    ret,
                    vec![
                        ("serverName", Value::Str(String::new())),
                        ("caFile", Value::Str(String::new())),
                        ("timeoutMs", Value::Int(10000, Type::i64())),
                    ],
                ))
            }
            "createServer" => {
                return self.secure_create(
                    false,
                    Some(secure_server_config(args[0].string()?, args[1].string()?)?),
                    args[2].clone(),
                    ret,
                )
            }
            "connect" => {
                self.core_capacity()?;
                if self.core.secure_sockets.len() >= 32 {
                    return Err("at most 32 TLS sockets per interpreter".into());
                }
                let port = Self::core_port(&args[0])?;
                let host = args[1].string()?;
                let name = crate::platform::field(&args[2], "serverName")?.string()?;
                let ca = crate::platform::field(&args[2], "caFile")?.string()?;
                let timeout =
                    Self::core_timeout(crate::platform::field(&args[2], "timeoutMs")?, false)?
                        .unwrap();
                let socket = secure_client(
                    secure_tcp(host, port, timeout)?,
                    if name.is_empty() { host } else { name },
                    ca,
                )?;
                let id = self.http_id()?;
                self.core.secure_sockets.insert(id, socket);
                return Ok(Self::core_handle(id, ret));
            }
            _ => {}
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_Socket_close" {
            let mut socket = self
                .core
                .secure_sockets
                .remove(&id)
                .ok_or("TLS socket is closed")?;
            match &mut socket {
                SecureTransport::Client(s) => s.conn.send_close_notify(),
                SecureTransport::Server(s) => s.conn.send_close_notify(),
                _ => {}
            }
            let result = socket.flush().map_err(|e| e.to_string());
            let _ = socket.tcp().shutdown(std::net::Shutdown::Both);
            result?;
            return Ok(Value::Void);
        }
        let socket = self
            .core
            .secure_sockets
            .get_mut(&id)
            .ok_or("TLS socket is closed or belongs to another thread")?;
        match name {
            "method_Socket_read" => {
                let size = Self::core_size(&args[1], BASIC_LIMIT)?;
                let mut output = vec![0; size];
                let n = socket.read(&mut output).map_err(|e| e.to_string())?;
                output.truncate(n);
                Ok(crate::filesystem::bytes_value(output))
            }
            "method_Socket_write" => {
                let bytes = crate::filesystem::bytes(&args[1])?;
                if bytes.len() > BASIC_LIMIT {
                    return Err("TLS write exceeds 8 MiB".into());
                }
                socket.write_all(&bytes).map_err(|e| e.to_string())?;
                socket.flush().map_err(|e| e.to_string())?;
                Ok(Value::Int(bytes.len() as i128, Type::i64()))
            }
            "method_Socket_setTimeout" => {
                socket.timeout(Self::core_timeout(&args[1], false)?.unwrap())?;
                Ok(Value::Void)
            }
            _ => Err("unknown TLS socket API".into()),
        }
    }
    fn websocket_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use tungstenite::Message;
        if name.starts_with("method_Server_") {
            return self.secure_server_call(name, args, ret);
        }
        match name {
            "createServer" => return self.secure_create(true, None, args[0].clone(), ret),
            "createSecureServer" => {
                return self.secure_create(
                    true,
                    Some(secure_server_config(args[0].string()?, args[1].string()?)?),
                    args[2].clone(),
                    ret,
                )
            }
            "connect" => {
                self.core_capacity()?;
                if self.core.websockets.len() >= 32 {
                    return Err("at most 32 WebSocket sockets per interpreter".into());
                }
                let text = args[0].string()?;
                if text.len() > 8192 {
                    return Err("WebSocket URL exceeds 8 KiB".into());
                }
                let url = url::Url::parse(text).map_err(|e| e.to_string())?;
                if !["ws", "wss"].contains(&url.scheme())
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.fragment().is_some()
                {
                    return Err("WebSocket URL must use ws/wss without userinfo or fragment".into());
                }
                let host = url
                    .host_str()
                    .ok_or("WebSocket URL needs a host")?
                    .trim_matches(['[', ']']);
                let timeout = Self::core_timeout(&args[1], false)?.unwrap();
                let tcp = secure_tcp(
                    host,
                    url.port_or_known_default()
                        .ok_or("WebSocket URL needs a port")?,
                    timeout,
                )?;
                let transport = if url.scheme() == "wss" {
                    secure_client(tcp, host, args[2].string()?)?
                } else {
                    SecureTransport::Plain(tcp)
                };
                let (socket, _) = tungstenite::client::client_with_config(
                    text,
                    transport,
                    Some(websocket_config()),
                )
                .map_err(|e| e.to_string())?;
                let id = self.http_id()?;
                self.core.websockets.insert(
                    id,
                    CoreWebSocket {
                        socket,
                        path: format!(
                            "{}{}",
                            url.path(),
                            url.query().map_or(String::new(), |q| format!("?{q}"))
                        ),
                        origin: String::new(),
                        closed: false,
                    },
                );
                return Ok(Self::core_handle(id, ret));
            }
            _ => {}
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_Socket_close" {
            let mut socket = self
                .core
                .websockets
                .remove(&id)
                .ok_or("WebSocket is closed")?;
            if !socket.closed {
                match socket.socket.close(None) {
                    Ok(())
                    | Err(
                        tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed,
                    ) => {}
                    Err(error) => return Err(error.to_string()),
                }
                let _ = socket.socket.flush();
            }
            let _ = socket
                .socket
                .get_ref()
                .tcp()
                .shutdown(std::net::Shutdown::Both);
            return Ok(Value::Void);
        }
        let socket = self
            .core
            .websockets
            .get_mut(&id)
            .ok_or("WebSocket is closed or belongs to another thread")?;
        match name {
            "method_Socket_path" => return Ok(Value::Str(socket.path.clone())),
            "method_Socket_origin" => return Ok(Value::Str(socket.origin.clone())),
            "method_Socket_setTimeout" => {
                socket
                    .socket
                    .get_ref()
                    .timeout(Self::core_timeout(&args[1], false)?.unwrap())?;
                return Ok(Value::Void);
            }
            _ => {}
        }
        if socket.closed {
            return Err("WebSocket close frame was received".into());
        }
        match name {
            "method_Socket_sendText" => {
                let text = args[1].string()?;
                if text.len() > BASIC_LIMIT {
                    return Err("WebSocket text exceeds 8 MiB".into());
                }
                socket
                    .socket
                    .send(Message::Text(text.into()))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Void)
            }
            "method_Socket_sendBytes" => {
                let bytes = crate::filesystem::bytes(&args[1])?;
                if bytes.len() > BASIC_LIMIT {
                    return Err("WebSocket bytes exceed 8 MiB".into());
                }
                socket
                    .socket
                    .send(Message::Binary(bytes.into()))
                    .map_err(|e| e.to_string())?;
                Ok(Value::Void)
            }
            "method_Socket_receive" => loop {
                let message = socket.socket.read().map_err(|e| e.to_string())?;
                let (kind, text, data, code) = match message {
                    Message::Text(text) => ("text", text.to_string(), Vec::new(), 0),
                    Message::Binary(bytes) => ("binary", String::new(), bytes.to_vec(), 0),
                    Message::Close(frame) => {
                        socket.closed = true;
                        let _ = socket.socket.flush();
                        (
                            "close",
                            frame
                                .as_ref()
                                .map_or(String::new(), |f| f.reason.to_string()),
                            Vec::new(),
                            frame.map_or(0, |f| u16::from(f.code) as i64),
                        )
                    }
                    Message::Ping(_) => {
                        socket.socket.flush().map_err(|e| e.to_string())?;
                        continue;
                    }
                    Message::Pong(_) => continue,
                    Message::Frame(_) => return Err("unexpected raw WebSocket frame".into()),
                };
                return Ok(crate::platform::record(
                    ret,
                    vec![
                        ("kind", Value::Str(kind.into())),
                        ("text", Value::Str(text)),
                        ("data", crate::filesystem::bytes_value(data)),
                        ("code", Value::Int(code as i128, Type::i64())),
                    ],
                ));
            },
            _ => Err("unknown WebSocket API".into()),
        }
    }
    fn secure_tick(&mut self) -> Result<bool, String> {
        let ids = self.core.secure_servers.keys().copied().collect::<Vec<_>>();
        let mut worked = false;
        for id in ids {
            let Some(server) = self.core.secure_servers.get(&id) else {
                continue;
            };
            let Some(listener) = &server.listener else {
                continue;
            };
            let tcp = match listener.accept() {
                Ok((socket, _)) => socket,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(error) => return Err(error.to_string()),
            };
            worked = true;
            // Windows accepted sockets inherit the listener's nonblocking mode.
            tcp.set_nonblocking(false).map_err(|e| e.to_string())?;
            let websocket = server.websocket;
            let callback = server.callback.clone();
            let config = server.config.clone();
            self.core_capacity()?;
            if (websocket && self.core.websockets.len() >= 32)
                || (!websocket && self.core.secure_sockets.len() >= 32)
            {
                continue;
            }
            tcp.set_read_timeout(Some(std::time::Duration::from_secs(10)))
                .map_err(|e| e.to_string())?;
            tcp.set_write_timeout(Some(std::time::Duration::from_secs(10)))
                .map_err(|e| e.to_string())?;
            let transport = if let Some(config) = config {
                match secure_server(tcp, config) {
                    Ok(stream) => stream,
                    Err(_) => continue,
                }
            } else {
                SecureTransport::Plain(tcp)
            };
            let Value::Callable(_, _, _, Type::Function(params, _)) = &callback else {
                return Err("invalid connection callback".into());
            };
            let socket_id = self.http_id()?;
            if websocket {
                let mut path = String::new();
                let mut origin = String::new();
                let socket = tungstenite::accept_hdr_with_config(
                    transport,
                    |request: &tungstenite::handshake::server::Request, response| {
                        path = request
                            .uri()
                            .path_and_query()
                            .map_or("/", |p| p.as_str())
                            .to_owned();
                        origin = request
                            .headers()
                            .get("origin")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_owned();
                        Ok(response)
                    },
                    Some(websocket_config()),
                );
                let Ok(socket) = socket else {
                    continue;
                };
                self.core.websockets.insert(
                    socket_id,
                    CoreWebSocket {
                        socket,
                        path,
                        origin,
                        closed: false,
                    },
                );
            } else {
                self.core.secure_sockets.insert(socket_id, transport);
            }
            let outcome = self.invoke_core(
                callback.clone(),
                vec![Self::core_handle(socket_id, params[0].clone())],
            );
            if outcome.is_err() {
                self.core.websockets.remove(&socket_id);
                self.core.secure_sockets.remove(&socket_id);
            }
            outcome?;
        }
        Ok(worked)
    }
}
