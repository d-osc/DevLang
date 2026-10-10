struct CoreTcp {
    stream: std::net::TcpStream,
    timeout: Option<std::time::Duration>,
    listeners: Vec<(String, Value)>,
}
struct CoreTcpServer {
    listener: Option<std::net::TcpListener>,
    handler: Value,
}
struct CoreUdp {
    socket: Option<std::net::UdpSocket>,
    ipv6: bool,
    handler: Option<Value>,
}
impl Engine {
    fn core_port(value: &Value) -> Result<u16, String> {
        u16::try_from(Self::core_int(value)?).map_err(|_| "port must be between 0 and 65535".into())
    }
    fn core_timeout(
        value: &Value,
        allow_zero: bool,
    ) -> Result<Option<std::time::Duration>, String> {
        let n = Self::core_size(value, 300_000)?;
        if n == 0 {
            if allow_zero {
                Ok(None)
            } else {
                Err("timeout_ms must be between 1 and 300000".into())
            }
        } else {
            Ok(Some(std::time::Duration::from_millis(n as u64)))
        }
    }
    fn core_address(address: std::net::SocketAddr, ret: Type) -> Value {
        crate::platform::record(
            ret,
            vec![
                ("address", Value::Str(address.ip().to_string())),
                ("port", Value::Int(address.port() as i128, Type::i64())),
                (
                    "family",
                    Value::Str(if address.is_ipv4() { "IPv4" } else { "IPv6" }.into()),
                ),
            ],
        )
    }
    fn add_tcp(&mut self, stream: std::net::TcpStream, ret: Type) -> Result<Value, String> {
        self.core_capacity()?;
        stream.set_nonblocking(true).map_err(|e| e.to_string())?;
        let id = self.http_id()?;
        self.core.tcp.insert(
            id,
            CoreTcp {
                stream,
                timeout: Some(std::time::Duration::from_secs(10)),
                listeners: vec![],
            },
        );
        Ok(Self::core_handle(id, ret))
    }
    fn tcp_write(socket: &mut CoreTcp, data: &[u8]) -> Result<(), String> {
        use std::io::Write;
        socket
            .stream
            .set_nonblocking(false)
            .map_err(|e| e.to_string())?;
        let result = (|| {
            socket.stream.set_write_timeout(socket.timeout)?;
            socket.stream.write_all(data)
        })();
        let reset = socket.stream.set_nonblocking(true);
        result.and(reset).map_err(|e| e.to_string())
    }
    fn tcp_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use std::io::Read;
        use std::net::{IpAddr, Shutdown, ToSocketAddrs};
        match name {
            "isIP" | "isIPv4" | "isIPv6" => {
                let ip = args[0].string()?.parse::<IpAddr>().ok();
                return Ok(match name {
                    "isIP" => Value::Int(
                        match ip {
                            Some(IpAddr::V4(_)) => 4,
                            Some(IpAddr::V6(_)) => 6,
                            None => 0,
                        },
                        Type::i64(),
                    ),
                    "isIPv4" => Value::Bool(matches!(ip, Some(IpAddr::V4(_)))),
                    _ => Value::Bool(matches!(ip, Some(IpAddr::V6(_)))),
                });
            }
            "connect" | "createConnection" => {
                self.core_capacity()?;
                let port = Self::core_port(&args[0])?;
                let timeout = Self::core_timeout(&args[2], false)?.unwrap();
                let addresses = (args[1].string()?, port)
                    .to_socket_addrs()
                    .map_err(|e| e.to_string())?;
                let start = Instant::now();
                let mut last = "host resolved to no addresses".to_string();
                for address in addresses {
                    let Some(left) = timeout
                        .checked_sub(start.elapsed())
                        .filter(|t| !t.is_zero())
                    else {
                        return Err("TCP connect timeout".into());
                    };
                    match std::net::TcpStream::connect_timeout(&address, left) {
                        Ok(stream) => return self.add_tcp(stream, ret),
                        Err(e) => last = e.to_string(),
                    }
                }
                return Err(last);
            }
            "createServer" => {
                self.core_capacity()?;
                let id = self.http_id()?;
                self.core.tcp_servers.insert(
                    id,
                    CoreTcpServer {
                        listener: None,
                        handler: args[0].clone(),
                    },
                );
                return Ok(Self::core_handle(id, ret));
            }
            _ => {}
        }
        let id = Self::handle_id(&args[0])?;
        if name.starts_with("method_Server_") {
            if name == "method_Server_close" {
                self.core
                    .tcp_servers
                    .remove(&id)
                    .ok_or("TCP server is closed")?;
                return Ok(Value::Void);
            }
            let server = self
                .core
                .tcp_servers
                .get_mut(&id)
                .ok_or("TCP server is closed or belongs to another thread")?;
            match name {
                "method_Server_on" => {
                    if args[1].string()? != "connection" {
                        return Err("TCP server.on supports only connection events".into());
                    }
                    server.handler = args[2].clone();
                    Ok(Value::Void)
                }
                "method_Server_listen" | "method_Server_listenOn" => {
                    if server.listener.is_some() {
                        return Err("TCP server is already listening".into());
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
                        .ok_or("TCP server is not listening")?
                        .local_addr()
                        .map_err(|e| e.to_string())?,
                    ret,
                )),
                _ => Err("unknown TCP server API".into()),
            }
        } else {
            if name == "method_Socket_destroy" {
                self.core.tcp.remove(&id).ok_or("TCP socket is closed")?;
                return Ok(Value::Void);
            }
            let socket = self
                .core
                .tcp
                .get_mut(&id)
                .ok_or("TCP socket is closed or belongs to another thread")?;
            match name {
                "method_Socket_read" => {
                    let size = Self::core_size(&args[1], crate::filesystem::MAX_BYTES)?;
                    if size == 0 {
                        return Ok(crate::filesystem::bytes_value(vec![]));
                    }
                    if socket.listeners.iter().any(|(event, _)| event == "data") {
                        return Err(
                            "cannot use blocking read while a data listener is registered".into(),
                        );
                    }
                    let mut data = vec![0; size];
                    socket
                        .stream
                        .set_nonblocking(false)
                        .map_err(|e| e.to_string())?;
                    let result = (|| {
                        socket.stream.set_read_timeout(socket.timeout)?;
                        socket.stream.read(&mut data)
                    })();
                    let reset = socket.stream.set_nonblocking(true);
                    let n = result.map_err(|e| e.to_string())?;
                    reset.map_err(|e| e.to_string())?;
                    data.truncate(n);
                    Ok(crate::filesystem::bytes_value(data))
                }
                "method_Socket_write" | "method_Socket_end" => {
                    let data = crate::filesystem::bytes(&args[1])?;
                    Self::tcp_write(socket, &data)?;
                    if name.ends_with("_end") {
                        socket
                            .stream
                            .shutdown(Shutdown::Write)
                            .map_err(|e| e.to_string())?;
                        Ok(Value::Void)
                    } else {
                        Ok(Value::Int(data.len() as i128, Type::i64()))
                    }
                }
                "method_Socket_setTimeout" => {
                    socket.timeout = Self::core_timeout(&args[1], true)?;
                    Ok(Value::Void)
                }
                "method_Socket_setNoDelay" => {
                    let Value::Bool(enable) = args[1] else {
                        unreachable!()
                    };
                    socket
                        .stream
                        .set_nodelay(enable)
                        .map_err(|e| e.to_string())?;
                    Ok(Value::Void)
                }
                "method_Socket_address" => Ok(Self::core_address(
                    socket.stream.local_addr().map_err(|e| e.to_string())?,
                    ret,
                )),
                "method_Socket_remoteAddress" => Ok(Self::core_address(
                    socket.stream.peer_addr().map_err(|e| e.to_string())?,
                    ret,
                )),
                "method_Socket_on" => {
                    let event = args[1].string()?;
                    if event != "data" && event != "end" {
                        return Err("TCP socket.on supports data and end events".into());
                    }
                    if socket.listeners.len() >= 1024 {
                        return Err("too many socket listeners".into());
                    }
                    socket.listeners.push((event.into(), args[2].clone()));
                    Ok(Value::Void)
                }
                _ => Err("unknown TCP socket API".into()),
            }
        }
    }
    fn bind_udp(socket: &mut CoreUdp, port: u16, host: &str) -> Result<(), String> {
        use std::net::ToSocketAddrs;
        if socket.socket.is_some() {
            return Err("UDP socket is already bound".into());
        }
        let address = (host, port)
            .to_socket_addrs()
            .map_err(|e| e.to_string())?
            .find(|a| a.is_ipv6() == socket.ipv6)
            .ok_or("no address matches UDP socket family")?;
        let bound = std::net::UdpSocket::bind(address).map_err(|e| e.to_string())?;
        bound.set_nonblocking(true).map_err(|e| e.to_string())?;
        socket.socket = Some(bound);
        Ok(())
    }
    fn udp_call(&mut self, name: &str, args: Vec<Value>, ret: Type) -> Result<Value, String> {
        use std::net::ToSocketAddrs;
        if name == "createSocket" {
            self.core_capacity()?;
            let ipv6 = match args[0].string()? {
                "udp4" => false,
                "udp6" => true,
                _ => return Err("kind must be udp4 or udp6".into()),
            };
            let id = self.http_id()?;
            self.core.udp.insert(
                id,
                CoreUdp {
                    socket: None,
                    ipv6,
                    handler: None,
                },
            );
            return Ok(Self::core_handle(id, ret));
        }
        let id = Self::handle_id(&args[0])?;
        if name == "method_Socket_close" {
            self.core.udp.remove(&id).ok_or("UDP socket is closed")?;
            return Ok(Value::Void);
        }
        let socket = self
            .core
            .udp
            .get_mut(&id)
            .ok_or("UDP socket is closed or belongs to another thread")?;
        match name {
            "method_Socket_bind" => {
                Self::bind_udp(socket, Self::core_port(&args[1])?, args[2].string()?)?;
                Ok(Value::Void)
            }
            "method_Socket_send" => {
                let data = crate::filesystem::bytes(&args[1])?;
                if data.len() > 65507 {
                    return Err("UDP datagram exceeds 65507 bytes".into());
                }
                let port = Self::core_port(&args[2])?;
                let address = (args[3].string()?, port)
                    .to_socket_addrs()
                    .map_err(|e| e.to_string())?
                    .find(|a| a.is_ipv6() == socket.ipv6)
                    .ok_or("no destination matches UDP family")?;
                if socket.socket.is_none() {
                    Self::bind_udp(socket, 0, if socket.ipv6 { "::" } else { "0.0.0.0" })?;
                }
                let n = socket
                    .socket
                    .as_ref()
                    .unwrap()
                    .send_to(&data, address)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Int(n as i128, Type::i64()))
            }
            "method_Socket_recv" => {
                if socket.handler.is_some() {
                    return Err("cannot use blocking recv with a message listener".into());
                }
                let size = Self::core_size(&args[1], 65535)?;
                if size == 0 {
                    return Err("receive size must be positive".into());
                }
                let timeout = Self::core_timeout(&args[2], false)?;
                let bound = socket.socket.as_ref().ok_or("UDP socket is not bound")?;
                let mut data = vec![0; 65536];
                bound.set_nonblocking(false).map_err(|e| e.to_string())?;
                let result = (|| {
                    bound.set_read_timeout(timeout)?;
                    bound.recv_from(&mut data)
                })();
                let reset = bound.set_nonblocking(true);
                let (n, address) = result.map_err(|e| e.to_string())?;
                reset.map_err(|e| e.to_string())?;
                if n > size {
                    return Err("received datagram exceeds requested size".into());
                }
                data.truncate(n);
                Ok(Self::udp_message(data, address, ret))
            }
            "method_Socket_address" => Ok(Self::core_address(
                socket
                    .socket
                    .as_ref()
                    .ok_or("UDP socket is not bound")?
                    .local_addr()
                    .map_err(|e| e.to_string())?,
                ret,
            )),
            "method_Socket_setBroadcast" => {
                let Value::Bool(enable) = args[1] else {
                    unreachable!()
                };
                socket
                    .socket
                    .as_ref()
                    .ok_or("UDP socket is not bound")?
                    .set_broadcast(enable)
                    .map_err(|e| e.to_string())?;
                Ok(Value::Void)
            }
            "method_Socket_on" => {
                if args[1].string()? != "message" {
                    return Err("UDP on supports only message events".into());
                }
                socket.handler = Some(args[2].clone());
                Ok(Value::Void)
            }
            _ => Err("unknown UDP socket API".into()),
        }
    }
    fn udp_message(data: Vec<u8>, address: std::net::SocketAddr, ret: Type) -> Value {
        crate::platform::record(
            ret,
            vec![
                ("data", crate::filesystem::bytes_value(data)),
                ("address", Value::Str(address.ip().to_string())),
                ("port", Value::Int(address.port() as i128, Type::i64())),
            ],
        )
    }
    fn core_network_active(&self) -> bool {
        self.core.tcp_servers.values().any(|s| s.listener.is_some())
            || self.core.tcp.values().any(|s| !s.listeners.is_empty())
            || self
                .core
                .udp
                .values()
                .any(|s| s.socket.is_some() && s.handler.is_some())
    }
    fn core_network_tick(&mut self) -> Result<bool, String> {
        use std::io::ErrorKind;
        use std::io::Read;
        let mut worked = false;
        self.core.scratch.resize(65536, 0);
        let ids = self.core.tcp_servers.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let Some(server) = self.core.tcp_servers.get(&id) else {
                continue;
            };
            let Some(listener) = &server.listener else {
                continue;
            };
            match listener.accept() {
                Ok((socket, _)) => {
                    let callback = server.handler.clone();
                    let Value::Callable(_, _, _, Type::Function(params, _)) = &callback else {
                        unreachable!()
                    };
                    let value = self.add_tcp(socket, params[0].clone())?;
                    self.invoke_core(callback, vec![value])?;
                    worked = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        let ids = self.core.tcp.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let Some(socket) = self.core.tcp.get_mut(&id) else {
                continue;
            };
            if socket.listeners.is_empty() {
                continue;
            }
            let read = socket.stream.read(&mut self.core.scratch);
            match read {
                Ok(n) => {
                    let event = if n == 0 { "end" } else { "data" };
                    let callbacks = socket
                        .listeners
                        .iter()
                        .filter(|(e, _)| e == event)
                        .map(|(_, v)| v.clone())
                        .collect::<Vec<_>>();
                    let data = crate::filesystem::bytes_value(self.core.scratch[..n].to_vec());
                    for callback in callbacks {
                        self.invoke_core(callback, vec![data.clone()])?;
                    }
                    if n == 0 {
                        self.core.tcp.remove(&id);
                    }
                    worked = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        let ids = self.core.udp.keys().copied().collect::<Vec<_>>();
        for id in ids {
            let Some(socket) = self.core.udp.get(&id) else {
                continue;
            };
            let (Some(bound), Some(callback)) = (&socket.socket, &socket.handler) else {
                continue;
            };
            match bound.recv_from(&mut self.core.scratch) {
                Ok((n, address)) => {
                    let callback = callback.clone();
                    let Value::Callable(_, _, _, Type::Function(params, _)) = &callback else {
                        unreachable!()
                    };
                    let message = Self::udp_message(
                        self.core.scratch[..n].to_vec(),
                        address,
                        params[0].clone(),
                    );
                    self.invoke_core(callback, vec![message])?;
                    worked = true;
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {}
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(worked)
    }
}
