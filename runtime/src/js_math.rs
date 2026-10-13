// JavaScript-style Math namespace; the strict std/math API stays independent.
fn math_f16(x: f64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let a = x.abs();
    if a >= 65520.0 {
        return f64::INFINITY.copysign(x);
    }
    let step = if a < 2f64.powi(-14) {
        2f64.powi(-24)
    } else {
        let e = ((a.to_bits() >> 52) & 2047) as i32 - 1023;
        2f64.powi(e - 10)
    };
    let q = a / step;
    let n = q.floor();
    let f = q - n;
    let rounded = if f > 0.5 || (f == 0.5 && (n as u64) & 1 != 0) {
        n + 1.0
    } else {
        n
    };
    (rounded * step).copysign(x)
}
impl Engine {
    fn js_math(&mut self, name: &str, args: Vec<Value>) -> Result<Value, String> {
        let nums = args
            .iter()
            .map(|v| match v {
                Value::Float(x, _) => Ok(*x),
                Value::Int(x, _) => Ok(*x as f64),
                _ => Err("Math arguments require numeric values".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let x = nums.first().copied().unwrap_or(0.0);
        let y = nums.get(1).copied().unwrap_or(0.0);
        let n = match name {
            "abs" => x.abs(),
            "acos" => x.acos(),
            "acosh" => x.acosh(),
            "asin" => x.asin(),
            "asinh" => x.asinh(),
            "atan" => x.atan(),
            "atanh" => x.atanh(),
            "atan2" => x.atan2(y),
            "cbrt" => x.cbrt(),
            "ceil" => x.ceil(),
            "cos" => x.cos(),
            "cosh" => x.cosh(),
            "exp" => x.exp(),
            "expm1" => x.exp_m1(),
            "floor" => x.floor(),
            "fround" => x as f32 as f64,
            "f16round" => math_f16(x),
            "log" => x.ln(),
            "log10" => x.log10(),
            "log1p" => x.ln_1p(),
            "log2" => x.log2(),
            "sin" => x.sin(),
            "sinh" => x.sinh(),
            "sqrt" => x.sqrt(),
            "tan" => x.tan(),
            "tanh" => x.tanh(),
            "trunc" => x.trunc(),
            "clz32" => {
                let mut n = if x.is_finite() {
                    x.trunc() % 4294967296.0
                } else {
                    0.0
                };
                if n < 0.0 {
                    n += 4294967296.0;
                }
                (n as u32).leading_zeros() as f64
            }
            "pow" => {
                if y.is_nan() {
                    f64::NAN
                } else if y == 0.0 {
                    1.0
                } else if x.is_nan() {
                    f64::NAN
                } else if x.abs() == 1.0 && y.is_infinite() {
                    f64::NAN
                } else {
                    x.powf(y)
                }
            }
            "round" => {
                if !x.is_finite() || x == 0.0 {
                    x
                } else if x >= -0.5 && x < 0.0 {
                    -0.0
                } else {
                    let n = x.floor();
                    if x - n < 0.5 {
                        n
                    } else {
                        n + 1.0
                    }
                }
            }
            "sign" => {
                if x.is_nan() || x == 0.0 {
                    x
                } else if x > 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            "random" => (self.random_next()? >> 11) as f64 / 9007199254740992.0,
            "min" | "max" => {
                let min = name == "min";
                let mut n = if min {
                    f64::INFINITY
                } else {
                    f64::NEG_INFINITY
                };
                for x in nums {
                    if x.is_nan() {
                        n = f64::NAN;
                        break;
                    }
                    if (min && x < n)
                        || (!min && x > n)
                        || (x == 0.0 && n == 0.0 && x.is_sign_negative() == min)
                    {
                        n = x;
                    }
                }
                n
            }
            _ => return Err("unknown Math method".into()),
        };
        Ok(Value::Float(n, Type::Float(64)))
    }
}

#[cfg(test)]
mod tests {
    use super::math_f16;
    fn half(bits: u16) -> f64 {
        let exponent = (bits >> 10) & 31;
        let fraction = bits & 1023;
        if exponent == 0 {
            fraction as f64 * 2f64.powi(-24)
        } else {
            (1.0 + fraction as f64 / 1024.0) * 2f64.powi(exponent as i32 - 15)
        }
    }
    #[test]
    fn binary16_values_and_all_finite_midpoints() {
        for bits in 0..0x7c00u16 {
            let x = half(bits);
            assert_eq!(math_f16(x).to_bits(), x.to_bits());
            assert_eq!(math_f16(-x).to_bits(), (-x).to_bits());
        }
        for bits in 0..0x7bffu16 {
            let a = half(bits);
            let b = half(bits + 1);
            let midpoint = (a + b) * 0.5;
            let even = if bits & 1 == 0 { a } else { b };
            assert_eq!(math_f16(midpoint), even);
            assert_eq!(math_f16(-midpoint), -even);
            assert_eq!(math_f16(f64::from_bits(midpoint.to_bits() - 1)), a);
            assert_eq!(math_f16(f64::from_bits(midpoint.to_bits() + 1)), b);
        }
        assert_eq!(math_f16(65520.0), f64::INFINITY);
        assert_eq!(math_f16(-65520.0), f64::NEG_INFINITY);
    }
}
