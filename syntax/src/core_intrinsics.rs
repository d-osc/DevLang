pub const BUILTINS: &[&str] = &[
    "std/io",
    "std/strings",
    "std/time",
    "std/args",
    "std/json",
    "std/don",
    "std/math",
    "std/random",
    "std/datetime",
    "std/test",
    "std/log",
    "std/regex",
    "std/encoding",
    "std/crypto",
    "std/compression",
    "std/archive",
    "std/uuid",
    "std/fs",
    "std/http",
    "std/fs/promises",
    "std/net",
    "std/path",
    "std/os",
    "std/stream",
    "std/url",
    "std/module",
    "std/process",
    "std/events",
    "std/buffer",
    "std/dgram",
];
pub const PATH: &str = r#"
struct Parts { root str, dir str, base str, ext str, name str }
fn join(base str, child str) str { return "" }
fn joinMany(parts Vec<str>) str { return "" }
fn resolve(base str, child str) str { return "" }
fn normalize(path str) str { return "" }
fn dirname(path str) str { return "" }
fn basename(path str) str { return "" }
fn extname(path str) str { return "" }
fn isAbsolute(path str) bool { return false }
fn relative(from str, to str) str { return "" }
fn parse(path str) Parts { return Parts("","","","","") }
fn format(parts Parts) str { return "" }
fn sep() str { return "" }
fn delimiter() str { return "" }
"#;
pub const OS: &str = r#"
fn platform() str { return "" }
fn arch() str { return "" }
fn hostname() str { return "" }
fn homedir() str { return "" }
fn tmpdir() str { return "" }
fn release() str { return "" }
fn type() str { return "" }
fn endianness() str { return "" }
fn EOL() str { return "" }
fn availableParallelism() i64 { return 0 }
fn uptime() i64 { return 0 }
fn totalmem() i64 { return 0 }
fn freemem() i64 { return 0 }
"#;
pub const PROCESS: &str = r#"
fn cwd() str { return "" }
fn chdir(path str) bool { return true }
fn pid() i64 { return 0 }
fn argv() Vec<str> { return Vec<str>() }
fn execPath() str { return "" }
fn env() Map<str,str> { return Map<str,str>() }
fn getenv(name str) str { return "" }
fn hasEnv(name str) bool { return false }
fn platform() str { return "" }
fn arch() str { return "" }
fn uptime() f64 { return 0.0 }
fn hrtime() i64 { return 0 }
"#;
pub const MODULE: &str = r#"
fn builtinModules() Vec<str> { return Vec<str>() }
fn isBuiltin(name str) bool { return false }
fn resolve(specifier str, from_file str) str { return "" }
fn loaded() Vec<str> { return Vec<str>() }
fn entry() str { return "" }
"#;
pub const URL: &str = r#"
struct URL { href str, protocol str, host str, hostname str, port str, pathname str, search str, hash str, origin str, username str, password str }
struct URLSearchParams { id i64 }
fn parse(input str) URL { return URL("","","","","","","","","","","") }
fn resolve(base str, relative str) URL { return URL("","","","","","","","","","","") }
fn format(value URL) str { return "" }
fn URL.toString(self URL) str { return "" }
fn pathToFileURL(path str) str { return "" }
fn fileURLToPath(input str) str { return "" }
fn domainToASCII(domain str) str { return "" }
fn searchParams(query str) URLSearchParams { return URLSearchParams(0) }
fn URLSearchParams.append(self URLSearchParams, name str, value str) { }
fn URLSearchParams.set(self URLSearchParams, name str, value str) { }
fn URLSearchParams.get(self URLSearchParams, name str) str { return "" }
fn URLSearchParams.getAll(self URLSearchParams, name str) Vec<str> { return Vec<str>() }
fn URLSearchParams.has(self URLSearchParams, name str) bool { return false }
fn URLSearchParams.delete(self URLSearchParams, name str) { }
fn URLSearchParams.toString(self URLSearchParams) str { return "" }
fn URLSearchParams.close(self URLSearchParams) { }
"#;
pub const BUFFER: &str = r#"
struct Buffer { id i64, length i64 }
fn alloc(size i64, fill u8) Buffer { return Buffer(0,0) }
fn from(text str, encoding str) Buffer { return Buffer(0,0) }
fn fromBytes(bytes Vec<u8>) Buffer { return Buffer(0,0) }
fn concat(buffers Vec<Buffer>) Buffer { return Buffer(0,0) }
fn byteLength(text str, encoding str) i64 { return 0 }
fn Buffer.toString(self Buffer, encoding str) str { return "" }
fn Buffer.toBytes(self Buffer) Vec<u8> { return Vec<u8>() }
fn Buffer.readUInt8(self Buffer, offset i64) u8 { return 0 as u8 }
fn Buffer.writeUInt8(self Buffer, value u8, offset i64) bool { return true }
fn Buffer.readUInt16LE(self Buffer, offset i64) u16 { return 0 as u16 }
fn Buffer.readUInt16BE(self Buffer, offset i64) u16 { return 0 as u16 }
fn Buffer.readUInt32LE(self Buffer, offset i64) u32 { return 0 as u32 }
fn Buffer.readUInt32BE(self Buffer, offset i64) u32 { return 0 as u32 }
fn Buffer.writeUInt16LE(self Buffer, value u16, offset i64) bool { return true }
fn Buffer.writeUInt16BE(self Buffer, value u16, offset i64) bool { return true }
fn Buffer.writeUInt32LE(self Buffer, value u32, offset i64) bool { return true }
fn Buffer.writeUInt32BE(self Buffer, value u32, offset i64) bool { return true }
fn Buffer.slice(self Buffer, start i64, end i64) Buffer { return Buffer(0,0) }
fn Buffer.equals(self Buffer, other Buffer) bool { return false }
fn Buffer.fill(self Buffer, value u8) bool { return true }
fn Buffer.copy(self Buffer, target Buffer, targetStart i64, sourceStart i64, sourceEnd i64) i64 { return 0 }
fn Buffer.close(self Buffer) { }
"#;
pub const EVENTS: &str = r#"
struct EventEmitter { id i64 }
fn createEmitter() EventEmitter { return EventEmitter(0) }
fn EventEmitter.on(self EventEmitter, event str, listener fn(str) void) i64 { return 0 }
fn EventEmitter.once(self EventEmitter, event str, listener fn(str) void) i64 { return 0 }
fn EventEmitter.off(self EventEmitter, event str, listener_id i64) bool { return false }
fn EventEmitter.emit(self EventEmitter, event str, payload str) bool { return false }
fn EventEmitter.listenerCount(self EventEmitter, event str) i64 { return 0 }
fn EventEmitter.removeAllListeners(self EventEmitter, event str) i64 { return 0 }
fn EventEmitter.eventNames(self EventEmitter) Vec<str> { return Vec<str>() }
fn EventEmitter.close(self EventEmitter) { }
"#;
pub const STREAM: &str = r#"
struct Readable { id i64 }
struct Writable { id i64 }
fn createReadStream(path str) Readable { return Readable(0) }
fn createWriteStream(path str, append bool) Writable { return Writable(0) }
fn Readable.read(self Readable, size i64) Vec<u8> { return Vec<u8>() }
fn Readable.pipe(self Readable, destination Writable) i64 { return 0 }
fn Readable.close(self Readable) { }
fn Writable.write(self Writable, bytes Vec<u8>) i64 { return 0 }
fn Writable.end(self Writable) { }
fn Writable.close(self Writable) { }
"#;
pub const NET: &str = r#"
struct Socket { id i64 }
struct Server { id i64 }
struct Address { address str, port i64, family str }
fn isIP(input str) i64 { return 0 }
fn isIPv4(input str) bool { return false }
fn isIPv6(input str) bool { return false }
fn connect(port i64, host str, timeout_ms i64) Socket { return Socket(0) }
fn createConnection(port i64, host str, timeout_ms i64) Socket { return Socket(0) }
fn createServer(handler fn(Socket) void) Server { return Server(0) }
fn Server.listen(self Server, port i64) { }
fn Server.listenOn(self Server, port i64, host str) { }
fn Server.address(self Server) Address { return Address("",0,"") }
fn Server.close(self Server) { }
fn Server.on(self Server, event str, handler fn(Socket) void) { }
fn Socket.on(self Socket, event str, handler fn(Vec<u8>) void) { }
fn Socket.read(self Socket, size i64) Vec<u8> { return Vec<u8>() }
fn Socket.write(self Socket, bytes Vec<u8>) i64 { return 0 }
fn Socket.end(self Socket, bytes Vec<u8>) { }
fn Socket.destroy(self Socket) { }
fn Socket.setTimeout(self Socket, timeout_ms i64) { }
fn Socket.setNoDelay(self Socket, enable bool) { }
fn Socket.address(self Socket) Address { return Address("",0,"") }
fn Socket.remoteAddress(self Socket) Address { return Address("",0,"") }
"#;
pub const DGRAM: &str = r#"
struct Socket { id i64 }
struct Address { address str, port i64, family str }
struct Message { data Vec<u8>, address str, port i64 }
fn createSocket(kind str) Socket { return Socket(0) }
fn Socket.bind(self Socket, port i64, host str) { }
fn Socket.send(self Socket, bytes Vec<u8>, port i64, host str) i64 { return 0 }
fn Socket.recv(self Socket, size i64, timeout_ms i64) Message { return Message(Vec<u8>(),"",0) }
fn Socket.on(self Socket, event str, handler fn(Message) void) { }
fn Socket.address(self Socket) Address { return Address("",0,"") }
fn Socket.setBroadcast(self Socket, enable bool) { }
fn Socket.close(self Socket) { }
"#;
