//! Math namespace metadata shared by syntax and backends.
pub fn arity(name: &str) -> Option<Option<usize>> {
    match name {
        "abs" | "acos" | "acosh" | "asin" | "asinh" | "atan" | "atanh" | "cbrt" | "ceil"
        | "clz32" | "cos" | "cosh" | "exp" | "expm1" | "f16round" | "floor" | "fround" | "log"
        | "log10" | "log1p" | "log2" | "round" | "sign" | "sin" | "sinh" | "sqrt" | "tan"
        | "tanh" | "trunc" => Some(Some(1)),
        "atan2" | "pow" => Some(Some(2)),
        "random" => Some(Some(0)),
        "min" | "max" => Some(None),
        _ => None,
    }
}
pub fn constant(name: &str) -> Option<&'static str> {
    match name {
        "E" => Some("2.718281828459045"),
        "LN2" => Some("0.6931471805599453"),
        "LN10" => Some("2.302585092994046"),
        "LOG10E" => Some("0.4342944819032518"),
        "LOG2E" => Some("1.4426950408889634"),
        "PI" => Some("3.141592653589793"),
        "SQRT1_2" => Some("0.7071067811865476"),
        "SQRT2" => Some("1.4142135623730951"),
        _ => None,
    }
}
