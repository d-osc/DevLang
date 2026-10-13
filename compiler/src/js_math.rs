use crate::ast::*;
use crate::codegen::{c_string, Emitter, Value};
pub(crate) const HELPERS: &str = include_str!("js_math.c");
impl Emitter<'_> {
    pub(crate) fn js_math_call(
        &mut self,
        name: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<Value, String> {
        if !self.hosted {
            return self.fail(span, "Math methods require hosted mode");
        }
        self.uses_math = true;
        let mut codes = Vec::new();
        for arg in args {
            let v = self.expr(arg, None)?;
            if !v.ty.numeric() {
                return self.fail(arg.span, "Math arguments require numeric values");
            }
            let v = self.sequence_save(v);
            codes.push(format!("(double)({})", v.code));
        }
        let values = if codes.is_empty() {
            "NULL".into()
        } else {
            format!("(double[]){{{}}}", codes.join(","))
        };
        Ok(Value {
            code: format!(
                "dev_math_call({}, {}, {values})",
                c_string(name),
                codes.len()
            ),
            ty: Type::Float(64),
            lvalue: false,
        })
    }
}
