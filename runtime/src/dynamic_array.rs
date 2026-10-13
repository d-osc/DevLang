fn is_dynamic_array_element(t: &Type) -> bool {
    matches!(t,Type::Record(n,_)|Type::Nominal(n) if n=="dev_dynamic_value"||n=="dev_dynamic_don_value")
}
fn dynamic_number_text(x: f64) -> String {
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0".into()
        } else {
            "0".into()
        };
    }
    let text = format!("{:.16e}", x);
    let Some((mantissa, e)) = text.split_once('e') else {
        return text;
    };
    let exponent = e.parse::<i32>().unwrap();
    let negative = mantissa.starts_with('-');
    let mut digits = mantissa.trim_start_matches('-').replace('.', "");
    while digits.ends_with('0') {
        digits.pop();
    }
    let body = if !(-4..17).contains(&exponent) {
        let mut m = digits[..1].to_string();
        if digits.len() > 1 {
            m.push('.');
            m.push_str(&digits[1..]);
        }
        format!("{m}e{exponent:+03}")
    } else {
        let point = exponent + 1;
        if point <= 0 {
            format!("0.{}{}", "0".repeat((-point) as usize), digits)
        } else if point as usize >= digits.len() {
            format!("{}{}", digits, "0".repeat(point as usize - digits.len()))
        } else {
            format!(
                "{}.{}",
                &digits[..point as usize],
                &digits[point as usize..]
            )
        }
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
}
impl Engine {
    fn dynamic_array_text(
        &mut self,
        value: &Value,
        depth: usize,
        span: Span,
    ) -> Result<String, String> {
        if depth > 128 {
            return Err("array text nesting limit exceeded".into());
        }
        match value {
            Value::Json(node, _) => match node.as_ref() {
                serde_json::Value::Null => Ok(String::new()),
                serde_json::Value::String(s) => Ok(s.clone()),
                serde_json::Value::Bool(b) => Ok(b.to_string()),
                serde_json::Value::Number(n) => Ok(if let Some(i) = n.as_i64() {
                    i.to_string()
                } else if let Some(i) = n.as_u64() {
                    i.to_string()
                } else {
                    dynamic_number_text(n.as_f64().ok_or("number out of f64 range")?)
                }),
                serde_json::Value::Object(_) => Ok("[object Object]".into()),
                serde_json::Value::Array(xs) => {
                    let mut texts = Vec::new();
                    for x in xs {
                        texts.push(self.dynamic_array_text(
                            &Value::Json(Arc::new(x.clone()), object_type()),
                            depth + 1,
                            span,
                        )?);
                    }
                    Ok(texts.join(","))
                }
            },
            Value::PrototypeObject(..) => {
                let out = self.object_method(value.clone(), "toString", vec![], span)?;
                if matches!(out, Value::PrototypeObject(..) | Value::PrototypeArray(..)) {
                    return Err("object toString must return a scalar".into());
                }
                if matches!(&out,Value::Json(n,_) if n.is_null()) {
                    return Ok("null".into());
                }
                self.dynamic_array_text(&out, depth + 1, span)
            }
            Value::PrototypeArray(xs, _) | Value::Vector(xs, _) => {
                let mut texts = Vec::new();
                for x in xs.iter() {
                    texts.push(self.dynamic_array_text(x, depth + 1, span)?);
                }
                Ok(texts.join(","))
            }
            Value::PrototypeFunction(..) | Value::Callable(..) => Ok("[Function]".into()),
            Value::Float(x, _) => Ok(dynamic_number_text(*x)),
            _ => Ok(value.display()),
        }
    }
    fn dynamic_array_sort_text(&mut self, value: &Value, span: Span) -> Result<String, String> {
        if matches!(value,Value::Json(n,_) if n.is_null()) {
            Ok("null".into())
        } else {
            self.dynamic_array_text(value, 0, span)
        }
    }
    fn dynamic_array_equal(&self, a: &Value, b: &Value, nan_equal: bool) -> bool {
        if let (Value::Json(a, _), Value::Json(b, _)) = (a, b) {
            if a.is_null() || b.is_null() {
                return a.is_null() && b.is_null();
            }
            if a.is_number() && b.is_number() {
                if let (Some(x), Some(y)) = (a.as_i64(), b.as_i64()) {
                    return x == y;
                }
                if let (Some(x), Some(y)) = (a.as_u64(), b.as_u64()) {
                    return x == y;
                }
                let integer_float = |n: &serde_json::Value, f: f64| {
                    if !f.is_finite() || f.fract() != 0.0 {
                        return false;
                    }
                    if let Some(n) = n.as_i64() {
                        f >= -9223372036854775808.0 && f < 9223372036854775808.0 && f as i64 == n
                    } else if let Some(n) = n.as_u64() {
                        f >= 0.0 && f < 18446744073709551616.0 && f as u64 == n
                    } else {
                        false
                    }
                };
                if !a.is_f64() {
                    return integer_float(a, b.as_f64().unwrap());
                }
                if !b.is_f64() {
                    return integer_float(b, a.as_f64().unwrap());
                }
                let (x, y) = (a.as_f64().unwrap(), b.as_f64().unwrap());
                return x == y || (nan_equal && x.is_nan() && y.is_nan());
            }
            if let (Some(x), Some(y)) = (a.as_str(), b.as_str()) {
                return x == y;
            }
            if let (Some(x), Some(y)) = (a.as_bool(), b.as_bool()) {
                return x == y;
            }
            return Arc::ptr_eq(a, b);
        }
        object_same(a, b)
    }
    fn dynamic_array_flatten(
        &mut self,
        value: &Value,
        depth: usize,
        out: &mut Vec<Value>,
    ) -> Result<(), String> {
        if depth > 0
            && (matches!(
                value,
                Value::Array(..) | Value::Vector(..) | Value::Slice(..) | Value::PrototypeArray(..)
            ) || matches!(value,Value::Json(n,_) if n.is_array()))
        {
            for item in object_sequence(value)? {
                self.dynamic_array_flatten(&item, depth - 1, out)?;
            }
            return Ok(());
        }
        if out.len() == 1000000 {
            return Err("flattened array exceeds 1000000 elements".into());
        }
        out.push(self.object_box(value.clone())?);
        Ok(())
    }
}
