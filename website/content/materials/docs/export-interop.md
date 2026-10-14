# Export interoperability: C++, Rust และ Go

ตัวอย่างนี้ใช้ `api.dev` ไฟล์เดียว แล้วสร้าง static library ให้ C, C++, Rust และ Go เรียกผ่าน C ABI โดยไม่เปลี่ยนตัวฟังก์ชัน DevLang ตามภาษาที่เรียก ฝั่งผู้เรียกยังต้องประกาศ signature และตั้งค่า linker ให้ตรงกัน

## สิ่งที่ทดสอบจริง / Verified targets

| Platform | Consumers | Toolchain |
| --- | --- | --- |
| Windows x86_64 | C, C++, Rust | Clang 18.1.7, Rust 1.91.0 MSVC |
| WSL Ubuntu Linux x86_64 | C, C++, Rust, Go | GCC/G++ 13.3.0, Rust 1.98.1, Go 1.22.2, CGO_ENABLED=1 |

ผลจากการ compile/link/run โปรแกรมจริง ไม่ใช่เพียงตรวจ syntax; ไม่ได้หมายความว่า Go บน Windows หรือทุก ABI ถูกทดสอบแล้ว ระบบ CI ทดสอบ C/C++/Rust บน Windows และ C/C++/Rust/Go บน Linux เพิ่มเติม

## โครงสร้างไฟล์

```text
examples/export-api/
  api.dev       # DevLang library
  devmath.h     # Public C/C++ header
  main.dev      # DevLang consumer
  main.c
  main.cpp
  main.rs
  go/main.go
```

## DevLang library

`export fn` เก็บชื่อ C symbol ตามที่ประกาศ ส่วน `fn twice` ธรรมดาก็เรียกจาก DevLang ผ่าน `use` ได้ แต่ชื่อ native symbol จะเป็นชื่อเฉพาะ module

### api.dev

```dev
// Ordinary functions are callable by direct DevLang importers too.
fn twice(value: i32): i32 {
    return value * 2
}

// Export keeps these exact names as native C ABI symbols.
export fn devmath_add(a: i32, b: i32): i32 {
    return a + b
}

export fn devmath_scale(value: f64, factor: f64): f64 {
    return value * factor
}

export fn devmath_positive(value: i32): bool {
    return value > 0
}

// The returned literal has static storage in native builds.
export fn devmath_name(): str {
    return "DevLang Math"
}
```

### devmath.h

```c
#ifndef DEVMATH_H
#define DEVMATH_H
#include <stdint.h>
#include <stdbool.h>
#ifdef __cplusplus
extern "C" {
#endif
int32_t devmath_add(int32_t a, int32_t b);
double devmath_scale(double value, double factor);
bool devmath_positive(int32_t value);
const char *devmath_name(void);
#ifdef __cplusplus
}
#endif
#endif
```

## Build static library

รันคำสั่งจาก root repository ต้องติดตั้ง `d`, C compiler และ archiver ที่เข้ากันก่อน ค่า `-o` ด้านล่างเป็นตำแหน่งที่ Go example ใช้หา library ด้วย

Windows (Clang/MSVC target + llvm-ar):

```powershell
d build examples/export-api/api.dev --lib --release --cc clang --ar llvm-ar -o out/export-api/libdevmath.a
```

Linux (GCC + GNU ar):

```sh
d build examples/export-api/api.dev --lib --release --cc gcc --ar ar -o out/export-api/libdevmath.a
```

อย่าใช้ archive ที่สร้างบน Windows ไป link Linux หรือกลับกัน และอย่าผสม MSVC กับ MinGW โดยถือว่า ABI/runtime เหมือนกันทั้งหมด ต้อง rebuild ด้วย target และ toolchain ที่ตรงกับผู้เรียก

## C++

```cpp
#include <stdio.h>
#include "devmath.h"

int main() {
    const auto answer = devmath_add(20, 22);
    const auto scaled = devmath_scale(2.5, 5.0);
    const auto positive = devmath_positive(7);
    printf("%d\n%.1f\n%s\n%s\n", (int)answer, scaled,
           positive ? "true" : "false", devmath_name());
}
```

```sh
clang++ examples/export-api/main.cpp out/export-api/libdevmath.a -o out/export-api/cpp-app.exe
./out/export-api/cpp-app.exe
```

บน Linux เปลี่ยน `clang++` เป็น `g++` และใช้ชื่อ output โดยไม่เติม `.exe`; header ใช้ `extern "C"` เพื่อปิด C++ name mangling

## Rust

```rust
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
```

```sh
rustc --edition 2021 examples/export-api/main.rs -L native=out/export-api -l static=devmath -o out/export-api/rust-app.exe
./out/export-api/rust-app.exe
```

บน Linux ใช้ชื่อ output โดยไม่เติม `.exe` และใช้ library ที่ build บน Linux; `unsafe extern "C"` ประกาศ ABI และการเรียกอยู่ใน `unsafe` เพราะ Rust พิสูจน์ signature/lifetime ของ foreign code ให้ไม่ได้

## Go (Linux cgo)

```go
package main

/*
#cgo CFLAGS: -I${SRCDIR}/..
#cgo LDFLAGS: -L${SRCDIR}/../../../out/export-api -ldevmath
#include "devmath.h"
*/
import "C"
import "fmt"

func main() {
    fmt.Println(int32(C.devmath_add(20, 22)))
    fmt.Println(float64(C.devmath_scale(2.5, 5.0)))
    fmt.Println(bool(C.devmath_positive(7)))
    // GoString copies the borrowed C string; do not free the DevLang literal.
    fmt.Println(C.GoString(C.devmath_name()))
}
```

```sh
CGO_ENABLED=1 CC=gcc go build -o out/export-api/go-app examples/export-api/go/main.go
./out/export-api/go-app
```

Go ต้องมี cgo และ C compiler; `#cgo` อ้าง path จากไฟล์ `main.go` ไม่ใช่ working directory และ `C.GoString` คัดลอก string ที่ยืมมา ตัวอย่างนี้ไม่ใช้ dependency ภายนอก

## Expected output

ทั้ง C++, Rust และ Go ต้องได้ผลลัพธ์เดียวกัน:

```text
42
12.5
true
DevLang Math
```

## ขอบเขตของ ABI

| DevLang | C/C++ | Rust | Go cgo |
| --- | --- | --- | --- |
| i32 | int32_t | i32 | C.int32_t |
| f64 | double | f64 | C.double |
| bool | bool | bool | C.bool |
| str | const char * | *const c_char | pointer + C.GoString |

String ในตัวอย่างเป็น literal ที่มี lifetime ตลอดโปรแกรม ห้าม free จากผู้เรียก `export` ไม่ได้คัดลอกหรือจัด ownership ของ memory ข้ามภาษาให้เอง Managed values เช่น Vec/Map/Ref และ generic functions ต้องทำ wrapper ด้วยชนิดที่รองรับ C ABI ก่อน

ตัวอย่างนี้เป็น native static library ไม่ใช่การ import `.dev` โดยตรงใน Rust/Go/C++ และยังไม่ได้สร้าง DLL/shared library ให้ Python หรือ C#

## Reproduce regression checks

```sh
# Linux: all four consumers; requires rustc, go, clang and llvm-ar
python scripts/smoke_export_interop.py --d target/release/d

# Windows: C, C++ and Rust
python scripts/smoke_export_interop.py --d target/release/d.exe --languages c,cpp,rust
```

ใช้ `--cc`, `--cxx`, `--ar` เพื่อเลือก toolchain และ `--report out/export-interop.json` เพื่อเก็บ compiler versions กับผลลัพธ์ การไม่มี compiler ที่ระบุหรือ output ไม่ตรงจะทำให้ test ล้มเหลว ไม่มีการ skip ภาษาเงียบ ๆ

ดาวน์โหลดโค้ดครบทุกภาษาได้จาก [ตัวอย่าง export](../examples/export-api/README.md) หรือหน้า Examples → Export interoperability

## Reverse direction

ดู [นำ C++, Rust และ Go เข้ามาเรียกใน DevLang](import-interop.md) สำหรับ DLL/shared libraries, `extern fn`, `--ffi-lib` และ native linking ที่ทดสอบจริง
