use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "devc-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }
    fn file(&self, name: &str, code: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, code).unwrap();
        path
    }
    fn generate(&self, source: &str) -> Result<Vec<codegen::Generated>, String> {
        let entry = self.file("main.dev", source);
        let p = program::Program::load(&entry)?;
        codegen::generate(&p, true)
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn lexer_tracks_lines_and_comments() {
    let tokens = lexer::lex("# comment\nlet x = 0x20 // hi\nprint(\n x\n)\n").unwrap();
    assert_eq!(tokens[1].span.line, 2);
    assert_eq!(
        tokens
            .iter()
            .filter(|t| t.kind == lexer::Kind::Newline)
            .count(),
        3
    );
}
#[test]
fn lexer_reports_unterminated_strings_and_bad_exponents() {
    assert!(lexer::lex("\"hello").is_err());
    assert!(lexer::lex("1e+").is_err());
    assert!(lexer::lex("0x").is_err());
}
#[test]
fn inference_and_typed_literals() {
    let generated = Project::new()
        .generate("fn main() { let x u32 = 42; let y = x + 1; print(y) }")
        .unwrap();
    assert!(generated[0].source.contains("uint32_t dev_v_1"));
}
#[test]
fn signed_minimum_and_unsigned_maximum() {
    Project::new().generate("fn main() { let x i64 = -9223372036854775808; let y u64 = 18446744073709551615; print(x); print(y) }").unwrap();
    let minimum_magnitude = 1u128 << (usize::BITS - 1);
    Project::new()
        .generate(&format!(
            "fn main() {{ let x isize = -{minimum_magnitude}; print(x) }}"
        ))
        .unwrap();
}
#[test]
fn casts_bind_more_tightly_than_multiplication() {
    Project::new()
        .generate("fn main() { let x i32 = 2; let y = x * 3 as i32; print(y) }")
        .unwrap();
}
#[test]
fn forward_calls_and_mutual_modules() {
    let p = Project::new();
    p.file(
        "a.dev",
        "use b\nfn a(n i64) i64 { if n == 0 { return 0 }; return b.b(n - 1) }",
    );
    p.file(
        "b.dev",
        "use a\nfn b(n i64) i64 { if n == 0 { return 0 }; return a.a(n - 1) }",
    );
    assert_eq!(
        p.generate("use a\nfn main() { print(a.a(4)) }")
            .unwrap()
            .len(),
        3
    );
}
#[test]
fn missing_import_reports_import_location() {
    let error = Project::new()
        .generate("use missing\nfn main() {}")
        .err()
        .unwrap();
    assert!(error.contains("main.dev:1:1: cannot import"), "{error}");
}
#[test]
fn arrays_pointer_access_and_volatile() {
    Project::new().generate("fn main() { unsafe { let a [u8; 2] = [1, 2]; let p = &a[0]; volatile_store(p, 7); print(volatile_load(p)); *p = 3 } }").unwrap();
}
#[test]
fn string_is_escaped_as_utf8() {
    assert_eq!(codegen::c_string("\"\nก"), "\"\\\"\\012\\340\\270\\201\"");
}
#[test]
fn reject_invalid_programs_without_codegen_panics() {
    let cases = [
        ("fn main() { let x u8 = 256 }", "out of range"),
        ("fn main() { let x i8 = -129 }", "out of range"),
        ("fn main() { let x i32 = 1.5 }", "floating literal"),
        ("fn main() { let x = unknown }", "unknown variable"),
        ("fn main() { break }", "while loop"),
        ("fn main() { continue }", "while loop"),
        ("fn main() { let x = 1; let x = 2 }", "duplicate variable"),
        ("fn f(a i32, a i32) {}", "duplicate parameter"),
        ("fn f() {}\nfn f() {}", "duplicate function"),
        ("fn f() i32 { let x = 1 }", "every path"),
        ("fn f() i32 { if true { return 1 } }", "every path"),
        ("fn f() { return 1 }", "void function"),
        ("fn main() { if 1 { print(1) } }", "expected bool"),
        ("fn main() { let a = [1, 2]; print(a[2]) }", "out of bounds"),
        ("fn main() { let a [i32; 2] = [1] }", "length"),
        ("fn main() { let a = []; print(a) }", "empty"),
        ("fn main() { let a = [1]; let b = a }", "array literal"),
        ("fn main() { print([1][0]) }", "bind the array"),
        (
            "fn main() { unsafe { let a = [1]; let p = &a } }",
            "invalid unary",
        ),
        ("fn main() { let x void = 1 }", "void"),
        ("fn main() { let a [void; 2] = [1, 2] }", "void"),
        (
            "fn main() { unsafe { let p = null; print(*p) } }",
            "cast *void",
        ),
        (
            "fn main() { unsafe { volatile_load(1) } }",
            "requires a pointer",
        ),
        ("fn main() { let s = \"hi\"; s[0] = 1 }", "writable"),
        (
            "fn main() { let s = \"hi\"; unsafe { let p = s as *u8 } }",
            "read-only",
        ),
        ("fn main() { print(1.5 & 2.0) }", "integers"),
        ("fn main() { let x = 1.5; x %= 2 }", "integers"),
        ("fn main() { print(true + false) }", "numeric"),
        ("fn main() { print(\"a\" == \"b\") }", "string contents"),
        ("fn f(x i32) {}\nfn main() { f() }", "arguments"),
        ("fn main() { other.f() }", "unknown module"),
        ("fn main() { let x = 1; x = true }", "expected i64"),
        (
            "fn main() { unsafe { let x = 1; let p = &x; let n = p as f64 } }",
            "pointer casts",
        ),
        (
            "fn main() { unsafe { let x = 1; let p = &x; let n = p * 2 } }",
            "expected",
        ),
        ("fn f(a [i32; 2]) {}", "array parameters"),
        ("fn f() [i32; 2] { return [1, 2] }", "return an array"),
        (
            "fn main() { let a [[i32; 2]; 2] = [[1, 2], [3, 4]] }",
            "nested arrays",
        ),
    ];
    for (source, expected) in cases {
        let error = Project::new()
            .generate(source)
            .err()
            .unwrap_or_else(|| panic!("accepted invalid program: {source}"));
        assert!(error.contains(expected), "expected {expected}, got {error}");
    }
}
#[test]
fn mismatched_extern_signatures_are_rejected() {
    let p = Project::new();
    p.file("a.dev", "extern fn puts(s str) i32");
    let e = p
        .generate("use a\nextern fn puts(s i64) i32")
        .err()
        .unwrap();
    assert!(e.contains("conflicting C symbol"));
}
#[test]
fn freestanding_rejects_hosted_print() {
    let p = Project::new();
    let path = p.file("main.dev", "export fn task() { print(1) }");
    let program = program::Program::load(&path).unwrap();
    assert!(codegen::generate(&program, false)
        .err()
        .unwrap()
        .contains("freestanding"));
}
#[test]
fn function_returns_through_both_if_branches() {
    Project::new()
        .generate("fn f(n i32) i32 { if n > 0 { return 1 } else { return 2 } }")
        .unwrap();
}
#[test]
fn sizeof_uses_str_type_instead_of_literal_length() {
    let generated = Project::new()
        .generate("fn main() { print(sizeof(\"abc\")) }")
        .unwrap();
    assert!(generated[0].source.contains("sizeof(const char *)"));
}
