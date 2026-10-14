#[no_mangle]
pub extern "C" fn foreign_add(a: i32, b: i32) -> i32 { a + b }

#[no_mangle]
pub extern "C" fn foreign_scale(value: f64, factor: f64) -> f64 { value * factor }

#[no_mangle]
pub extern "C" fn foreign_positive(value: i32) -> bool { value > 0 }
