pub const MATH: &str = r#"
fn pi() f64 { return 0.0 }
fn e() f64 { return 0.0 }
fn tau() f64 { return 0.0 }
fn abs(x f64) f64 { return x }
fn sqrt(x f64) f64 { return x }
fn cbrt(x f64) f64 { return x }
fn pow(x f64, y f64) f64 { return x }
fn exp(x f64) f64 { return x }
fn log(x f64) f64 { return x }
fn log2(x f64) f64 { return x }
fn log10(x f64) f64 { return x }
fn sin(x f64) f64 { return x }
fn cos(x f64) f64 { return x }
fn tan(x f64) f64 { return x }
fn asin(x f64) f64 { return x }
fn acos(x f64) f64 { return x }
fn atan(x f64) f64 { return x }
fn atan2(y f64, x f64) f64 { return x }
fn floor(x f64) f64 { return x }
fn ceil(x f64) f64 { return x }
fn round(x f64) f64 { return x }
fn trunc(x f64) f64 { return x }
fn min(x f64, y f64) f64 { return x }
fn max(x f64, y f64) f64 { return x }
fn clamp(x f64, low f64, high f64) f64 { return x }
fn hypot(x f64, y f64) f64 { return x }
fn isFinite(x f64) bool { return true }
fn isNaN(x f64) bool { return false }
"#;
pub const RANDOM: &str = r#"
fn seed(value i64) { }
fn float() f64 { return 0.0 }
fn int(min i64, max i64) i64 { return 0 }
fn bool() bool { return false }
fn bytes(size i64) Vec<u8> { return Vec<u8>() }
"#;
pub const STRINGS: &str = r#"
fn len(text str) usize { return 0 }
fn charLength(text str) i64 { return 0 }
fn concat(a str, b str) str { return "" }
fn equal(a str, b str) bool { return false }
fn trim(text str) str { return "" }
fn trimStart(text str) str { return "" }
fn trimEnd(text str) str { return "" }
fn toLowerCase(text str) str { return "" }
fn toUpperCase(text str) str { return "" }
fn contains(text str, pattern str) bool { return false }
fn startsWith(text str, pattern str) bool { return false }
fn endsWith(text str, pattern str) bool { return false }
fn indexOf(text str, pattern str) i64 { return 0 }
fn lastIndexOf(text str, pattern str) i64 { return 0 }
fn split(text str, separator str) Vec<str> { return Vec<str>() }
fn join(parts Vec<str>, separator str) str { return "" }
fn replace(text str, pattern str, replacement str) str { return "" }
fn replaceAll(text str, pattern str, replacement str) str { return "" }
fn repeat(text str, count i64) str { return "" }
fn substring(text str, start i64, end i64) str { return "" }
fn charAt(text str, index i64) str { return "" }
"#;
pub const DATETIME: &str = r#"
struct Parts { year i64, month i64, day i64, hour i64, minute i64, second i64, millisecond i64, weekday i64, offsetMinutes i64 }
fn now() i64 { return 0 }
fn parse(text str) i64 { return 0 }
fn iso(timestamp i64) str { return "" }
fn format(timestamp i64, pattern str) str { return "" }
fn formatOffset(timestamp i64, offsetMinutes i64, pattern str) str { return "" }
fn parts(timestamp i64, offsetMinutes i64) Parts { return Parts(0,0,0,0,0,0,0,0,0) }
fn utc(year i64, month i64, day i64, hour i64, minute i64, second i64, millisecond i64) i64 { return 0 }
fn addMilliseconds(timestamp i64, duration i64) i64 { return 0 }
fn isLeapYear(year i64) bool { return false }
fn daysInMonth(year i64, month i64) i64 { return 0 }
"#;
pub const TEST: &str = r#"
struct Report { total i64, passed i64, failed i64, summary str }
fn expect(condition bool, message str) { }
fn equal<T>(actual T, expected T, message str) { }
fn near(actual f64, expected f64, tolerance f64, message str) { }
fn case(name str, body fn() void) { }
fn run() Report { return Report(0,0,0,"") }
"#;
pub const LOG: &str = r#"
fn setLevel(level str) { }
fn toFile(path str) { }
fn toStderr() { }
fn debug(message str) { }
fn info(message str) { }
fn warn(message str) { }
fn error(message str) { }
fn flush() { }
"#;
