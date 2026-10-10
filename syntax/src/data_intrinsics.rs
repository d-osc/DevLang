pub const REGEX: &str = r#"
struct Regex { id i64 }
struct Match { matched bool, text str, start i64, end i64 }
fn compile(pattern str, flags str) Regex { return Regex(0) }
fn escape(text str) str { return "" }
fn Regex.test(self Regex, text str) bool { return false }
fn Regex.find(self Regex, text str) Match { return Match(false,"",-1,-1) }
fn Regex.findAll(self Regex, text str) Vec<str> { return Vec<str>() }
fn Regex.captures(self Regex, text str) Vec<str> { return Vec<str>() }
fn Regex.split(self Regex, text str) Vec<str> { return Vec<str>() }
fn Regex.replace(self Regex, text str, replacement str) str { return "" }
fn Regex.replaceAll(self Regex, text str, replacement str) str { return "" }
fn Regex.close(self Regex) { }
"#;
pub const ENCODING: &str = r#"
fn encode(text str, charset str) Vec<u8> { return Vec<u8>() }
fn decode(data Vec<u8>, charset str) str { return "" }
fn transcode(data Vec<u8>, from str, to str) Vec<u8> { return Vec<u8>() }
fn supported(charset str) bool { return false }
"#;
pub const CRYPTO: &str = r#"
fn hash(algorithm str, data Vec<u8>) Vec<u8> { return Vec<u8>() }
fn hashText(algorithm str, text str) str { return "" }
fn hmac(algorithm str, key Vec<u8>, data Vec<u8>) Vec<u8> { return Vec<u8>() }
fn verifyHmac(algorithm str, key Vec<u8>, data Vec<u8>, tag Vec<u8>) bool { return false }
fn secureBytes(size i64) Vec<u8> { return Vec<u8>() }
fn timingSafeEqual(a Vec<u8>, b Vec<u8>) bool { return false }
fn encrypt(key Vec<u8>, data Vec<u8>, associatedData Vec<u8>) Vec<u8> { return Vec<u8>() }
fn decrypt(key Vec<u8>, packet Vec<u8>, associatedData Vec<u8>) Vec<u8> { return Vec<u8>() }
"#;
pub const COMPRESSION: &str = r#"
fn gzip(data Vec<u8>, level i64) Vec<u8> { return Vec<u8>() }
fn gunzip(data Vec<u8>) Vec<u8> { return Vec<u8>() }
fn zlib(data Vec<u8>, level i64) Vec<u8> { return Vec<u8>() }
fn unzlib(data Vec<u8>) Vec<u8> { return Vec<u8>() }
fn deflate(data Vec<u8>, level i64) Vec<u8> { return Vec<u8>() }
fn inflate(data Vec<u8>) Vec<u8> { return Vec<u8>() }
"#;
pub const ARCHIVE: &str = r#"
struct Entry { name str, data Vec<u8> }
fn writeZIP(entries Vec<Entry>) Vec<u8> { return Vec<u8>() }
fn listZIP(data Vec<u8>) Vec<str> { return Vec<str>() }
fn readZIP(data Vec<u8>, name str) Vec<u8> { return Vec<u8>() }
fn writeTAR(entries Vec<Entry>) Vec<u8> { return Vec<u8>() }
fn listTAR(data Vec<u8>) Vec<str> { return Vec<str>() }
fn readTAR(data Vec<u8>, name str) Vec<u8> { return Vec<u8>() }
"#;
pub const UUID: &str = r#"
fn v4() str { return "" }
fn parse(text str) str { return "" }
fn isValid(text str) bool { return false }
fn version(text str) i64 { return 0 }
fn nil() str { return "" }
"#;
