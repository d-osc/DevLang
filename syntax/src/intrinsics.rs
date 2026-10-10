//! Source-runtime signatures shared by runtime loading and editor checking.
pub fn module(name: &str) -> Option<&'static str> {
    match name {
        "std/fs" => Some(FS),
        "std/http" => Some(HTTP),
        _ => None,
    }
}
pub const FS: &str = r#"
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
