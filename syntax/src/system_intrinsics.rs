pub const DNS: &str = r#"
struct Address { address str, family i64 }
fn lookup(host str, family i64) Vec<str> { return Vec<str>() }
fn lookupOne(host str, family i64) Address { return Address("",0) }
fn isIP(address str) i64 { return 0 }
fn isIPv4(address str) bool { return false }
fn isIPv6(address str) bool { return false }
"#;
pub const CLI: &str = r#"
struct Option { name str, short str, takesValue bool }
struct Parsed { values Map<str,str>, flags Map<str,bool>, positionals Vec<str> }
fn args() Vec<str> { return Vec<str>() }
fn parse(args Vec<str>, options Vec<Option>) Parsed { return Parsed(Map<str,str>(),Map<str,bool>(),Vec<str>()) }
"#;
pub const RESULT: &str = r#"
enum Result<T> { Ok(T), Err(str) }
fn attempt<T>(body fn() T) Result<T> { return Result<T>.Err("") }
fn run(body fn() void) Result<bool> { return Result<bool>.Err("") }
fn isOk<T>(value Result<T>) bool { return false }
fn isErr<T>(value Result<T>) bool { return false }
fn unwrap<T>(value Result<T>) T { return unwrap(value) }
fn unwrapOr<T>(value Result<T>, fallback T) T { return fallback }
fn message<T>(value Result<T>) str { return "" }
fn raise(message str) { }
"#;
pub const TIMERS: &str = r#"
struct Timer { id i64 }
fn setTimeout(callback fn(Timer) void, delayMs i64) Timer { return Timer(0) }
fn setInterval(callback fn(Timer) void, delayMs i64) Timer { return Timer(0) }
fn setImmediate(callback fn(Timer) void) Timer { return Timer(0) }
fn clearTimeout(timer Timer) bool { return false }
fn clearInterval(timer Timer) bool { return false }
fn Timer.cancel(self Timer) bool { return false }
fn Timer.ticks(self Timer) i64 { return 0 }
fn Timer.ref(self Timer) { }
fn Timer.unref(self Timer) { }
fn Timer.hasRef(self Timer) bool { return false }
fn run() { }
fn sleep(delayMs i64) { }
"#;
pub const CHILD_PROCESS: &str = r#"
struct Options { cwd str, timeoutMs i64, maxBuffer i64, input Vec<u8> }
struct Output { pid i64, code i64, success bool, timedOut bool, killed bool, stdout Vec<u8>, stderr Vec<u8> }
struct Child { id i64 }
fn options() Options { return Options("",30000,1048576,Vec<u8>()) }
fn spawn(file str, args Vec<str>, options Options) Child { return Child(0) }
fn execFileSync(file str, args Vec<str>, options Options) Output { return Output(0,0,false,false,false,Vec<u8>(),Vec<u8>()) }
fn Child.pid(self Child) i64 { return 0 }
fn Child.ready(self Child) bool { return false }
fn Child.kill(self Child) bool { return false }
fn Child.wait(self Child) Output { return Output(0,0,false,false,false,Vec<u8>(),Vec<u8>()) }
"#;
pub const SQLITE: &str = r#"
enum Value { Null, Integer(i64), Real(f64), Text(str), Blob(Vec<u8>) }
struct Database { id i64 }
struct Rows { columns Vec<str>, rows Vec<Vec<Value>> }
fn open(path str) Database { return Database(0) }
fn version() str { return "" }
fn Database.execute(self Database, sql str, params Vec<Value>) i64 { return 0 }
fn Database.query(self Database, sql str, params Vec<Value>) Rows { return Rows(Vec<str>(),Vec<Vec<Value>>()) }
fn Database.lastInsertRowId(self Database) i64 { return 0 }
fn Database.setTimeout(self Database, timeoutMs i64) { }
fn Database.close(self Database) { }
"#;
pub const CSV: &str = r#"
struct Table { headers Vec<str>, rows Vec<Vec<str>> }
fn parse(text str, delimiter str, hasHeaders bool) Table { return Table(Vec<str>(),Vec<Vec<str>>()) }
fn stringify(table Table, delimiter str) str { return "" }
"#;
pub const TOML: &str = r#"
use "std/json"
fn parse(text str) json.Value { return json.parse("{}") }
fn valid(text str) bool { return false }
fn stringify<T>(value T) str { return "" }
fn toJSON(text str) str { return "" }
fn fromJSON(text str) str { return "" }
"#;
pub const YAML: &str = r#"
use "std/json"
fn parse(text str) json.Value { return json.parse("null") }
fn valid(text str) bool { return false }
fn stringify<T>(value T) str { return "" }
fn toJSON(text str) str { return "" }
fn fromJSON(text str) str { return "" }
"#;
pub const TLS: &str = r#"
struct Options { serverName str, caFile str, timeoutMs i64 }
struct Socket { id i64 }
struct Server { id i64 }
struct Address { address str, port i64, family str }
fn options() Options { return Options("","",10000) }
fn connect(port i64, host str, options Options) Socket { return Socket(0) }
fn createServer(certFile str, keyFile str, callback fn(Socket) void) Server { return Server(0) }
fn Socket.read(self Socket, size i64) Vec<u8> { return Vec<u8>() }
fn Socket.write(self Socket, data Vec<u8>) i64 { return 0 }
fn Socket.setTimeout(self Socket, timeoutMs i64) { }
fn Socket.close(self Socket) { }
fn Server.listen(self Server, port i64) { }
fn Server.listenOn(self Server, port i64, host str) { }
fn Server.address(self Server) Address { return Address("",0,"") }
fn Server.on(self Server, event str, callback fn(Socket) void) { }
fn Server.close(self Server) { }
"#;
pub const WEBSOCKET: &str = r#"
struct Socket { id i64 }
struct Server { id i64 }
struct Address { address str, port i64, family str }
struct Message { kind str, text str, data Vec<u8>, code i64 }
fn connect(url str, timeoutMs i64, caFile str) Socket { return Socket(0) }
fn createServer(callback fn(Socket) void) Server { return Server(0) }
fn createSecureServer(certFile str, keyFile str, callback fn(Socket) void) Server { return Server(0) }
fn Socket.sendText(self Socket, text str) { }
fn Socket.sendBytes(self Socket, data Vec<u8>) { }
fn Socket.receive(self Socket) Message { return Message("","",Vec<u8>(),0) }
fn Socket.setTimeout(self Socket, timeoutMs i64) { }
fn Socket.path(self Socket) str { return "" }
fn Socket.origin(self Socket) str { return "" }
fn Socket.close(self Socket) { }
fn Server.listen(self Server, port i64) { }
fn Server.listenOn(self Server, port i64, host str) { }
fn Server.address(self Server) Address { return Address("",0,"") }
fn Server.on(self Server, event str, callback fn(Socket) void) { }
fn Server.close(self Server) { }
"#;
