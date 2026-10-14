use std::ffi::{c_char, CStr};

unsafe extern "C" {
    fn devmath_add(a: i32, b: i32) -> i32;
    fn devmath_scale(value: f64, factor: f64) -> f64;
    fn devmath_positive(value: i32) -> bool;
    fn devmath_name() -> *const c_char;
}

fn main() {
    // Signatures match devmath.h; name points to a static NUL-terminated literal.
    unsafe {
        println!("{}", devmath_add(20, 22));
        println!("{}", devmath_scale(2.5, 5.0));
        println!("{}", devmath_positive(7));
        println!("{}", CStr::from_ptr(devmath_name()).to_str().unwrap());
    }
}
