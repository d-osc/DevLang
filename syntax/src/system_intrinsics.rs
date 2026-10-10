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
