//! Source-runtime signatures shared by runtime loading and editor checking.
pub fn module(name: &str) -> Option<&'static str> {
    match name {
        "std/math" => Some(MATH),
        "std/random" => Some(RANDOM),
        "std/strings" => Some(STRINGS),
        "std/datetime" => Some(DATETIME),
        "std/test" => Some(TEST),
        "std/log" => Some(LOG),
        "std/don" => Some(DON),
        "std/fs" => Some(FS),
        "std/http" => Some(HTTP),
        "std/fs/promises" => Some(FS_PROMISES),
        "std/net" => Some(NET),
        "std/path" => Some(PATH),
        "std/os" => Some(OS),
        "std/stream" => Some(STREAM),
        "std/url" => Some(URL),
        "std/module" => Some(MODULE),
        "std/process" => Some(PROCESS),
        "std/events" => Some(EVENTS),
        "std/buffer" => Some(BUFFER),
        "std/dgram" => Some(DGRAM),
        _ => None,
    }
}
include!("core_intrinsics.rs");
include!("basic_intrinsics.rs");
pub const DON: &str = r#"
struct Value {}
fn parse(text str) Value { return Value() }
fn valid(text str) bool { return false }
fn stringify<T>(value T) str { return "" }
fn toJSON<T>(value T) str { return "" }
fn fromJSON(text str) Value { return Value() }
fn get(value Value, key str) Value { return Value() }
fn at(value Value, index i64) Value { return Value() }
fn has(value Value, key str) bool { return false }
fn string(value Value) str { return "" }
fn int(value Value) i64 { return 0 }
fn bool(value Value) bool { return false }
"#;
pub const FS: &str = r#"
fn readFileSync(path str, encoding str) str { return "" }
fn writeFileSync(path str, data str) bool { return true }
fn appendFileSync(path str, data str) bool { return true }
fn existsSync(path str) bool { return false }
fn mkdirSync(path str, recursive bool) bool { return true }
fn readdirSync(path str) Vec<str> { return Vec<str>() }
fn unlinkSync(path str) bool { return true }
fn rmdirSync(path str) bool { return true }
fn copyFileSync(from str, to str) bool { return true }
fn renameSync(from str, to str) bool { return true }
fn read_text(path str) str { return "" }
fn write_text(path str, text str) bool { return true }
fn append_text(path str, text str) bool { return true }
fn read_bytes(path str) Vec<u8> { return Vec<u8>() }
fn write_bytes(path str, bytes Vec<u8>) bool { return true }
fn exists(path str) bool { return false }
fn is_file(path str) bool { return false }
fn is_dir(path str) bool { return false }
fn create_dir(path str) bool { return true }
fn create_dirs(path str) bool { return true }
fn remove_file(path str) bool { return true }
fn remove_dir(path str) bool { return true }
fn read_dir(path str) Vec<str> { return Vec<str>() }
fn copy(from str, to str) bool { return true }
fn rename(from str, to str) bool { return true }
fn size(path str) i64 { return 0 }
fn current_dir() str { return "" }
fn join(base str, child str) str { return "" }
"#;
pub const HTTP: &str = r#"
struct IncomingMessage { method str, url str, headers Map<str,str>, body str, bytes Vec<u8> }
struct ServerResponse { id i64, statusCode i64 }
struct Server { id i64 }
fn createServer(handler fn(IncomingMessage,ServerResponse) void) Server { return Server(0) }
fn Server.listen(self Server, port i64) { }
fn Server.listenOn(self Server, port i64, host str) { }
fn Server.close(self Server) { }
fn Server.on(self Server, event str, handler fn(IncomingMessage,ServerResponse) void) { }
fn ServerResponse.writeHead(self ServerResponse, status i64, headers Map<str,str>) { }
fn ServerResponse.setHeader(self ServerResponse, name str, value str) { }
fn ServerResponse.write(self ServerResponse, text str) { }
fn ServerResponse.end(self ServerResponse, text str) { }
struct Response { status i64, body str, bytes Vec<u8>, headers Map<str,str>, ok bool }
fn get(url str) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
fn head(url str) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
fn post(url str, body str) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
fn put(url str, body str) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
fn patch(url str, body str) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
fn delete(url str) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
fn request(method str, url str, headers Map<str,str>, body str, timeout_ms i64) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
fn request_bytes(method str, url str, headers Map<str,str>, body Vec<u8>, timeout_ms i64) Response { return Response(0,"",Vec<u8>(),Map<str,str>(),false) }
"#;
pub const FS_PROMISES: &str = r#"
use "std/fs"
fn readFile(path str, encoding str) Task<str> {
    return spawn(fn() str { return fs.readFileSync(path, encoding) })
}
fn writeFile(path str, data str) Task<bool> {
    return spawn(fn() bool { return fs.writeFileSync(path, data) })
}
fn appendFile(path str, data str) Task<bool> {
    return spawn(fn() bool { return fs.appendFileSync(path, data) })
}
fn mkdir(path str, recursive bool) Task<bool> {
    return spawn(fn() bool { return fs.mkdirSync(path, recursive) })
}
fn readdir(path str) Task<Vec<str>> {
    return spawn(fn() Vec<str> { return fs.readdirSync(path) })
}
fn unlink(path str) Task<bool> {
    return spawn(fn() bool { return fs.unlinkSync(path) })
}
fn rename(from str, to str) Task<bool> {
    return spawn(fn() bool { return fs.renameSync(from, to) })
}
"#;
