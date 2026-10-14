# Import C++, Rust และ Go เข้ามาใช้ใน DevLang

ทิศทางนี้คือ DevLang เรียก library ที่เขียนด้วยภาษาอื่นผ่าน C ABI ใช้ `extern fn` และ `unsafe` โดยมีไฟล์ DevLang เดียวกันสำหรับ C++, Rust และ Go ไม่ใช่ `use "library.rs"` หรือ import source ของภาษาอื่นโดยตรง

## ผลทดสอบจริง / Verified targets

| Platform | Shared libraries | DevLang modes |
| --- | --- | --- |
| Windows x86_64 | C++ (Clang 18.1.7/MSVC), Rust 1.91.0 | runtime auto, runtime AST, native |
| WSL Ubuntu Linux x86_64 | C++ (G++ 13.3.0), Rust 1.98.1, Go 1.22.2 | runtime auto, runtime AST, native |

การทดสอบ build library ด้วย toolchain จริง แล้วเรียก integer, floating-point และ boolean รวมค่าลบทั้งสองฝั่ง ไม่มีการจำลองผลลัพธ์ Go บน Windows ยังไม่อยู่ในชุดทดสอบนี้ เพราะต้องตั้งค่า cgo/C toolchain ที่เข้ากันต่างจาก MSVC fixture

Runtime ทดสอบด้วย `PATH` ว่างและ `CC`/`DEV_CC` ที่ไม่มีอยู่จริง: หลัง build foreign library แล้ว DevLang runtime ไม่ต้องใช้ C compiler ส่วน native build ยังต้องใช้ C backend

## DevLang caller

```dev
// Load exactly one implementation with --ffi-lib (runtime) or --link (native).
extern fn foreign_add(a: i32, b: i32): i32
extern fn foreign_scale(value: f64, factor: f64): f64
extern fn foreign_positive(value: i32): bool

fn main(): void {
    unsafe {
        print(foreign_add(20, 22))
        print(foreign_scale(2.5, 5.0))
        print(foreign_positive(7))
        print(foreign_positive(-7))
        print(foreign_add(-20, -22))
    }
}

main()
```

แต่ละ library เปิด symbol ชุดเดียวกัน จึงเลือกโหลดทีละ implementation โดยไม่แก้ไฟล์ `main.dev` ไม่ควรโหลดทั้งสาม library พร้อมกันเพราะชื่อ symbol ซ้ำกัน

## สร้าง shared library

รันคำสั่งจาก root repository และเตรียมโฟลเดอร์ `out/import-interop` ก่อน ต้องมี `d`/`devrun` และ foreign toolchain ที่เลือกบน PATH

Windows PowerShell:

```powershell
New-Item -ItemType Directory -Force out/import-interop
```

Linux shell:

```sh
mkdir -p out/import-interop
```

### C++

```cpp
#include <stdint.h>
#if defined(_WIN32)
#define API extern "C" __declspec(dllexport)
#else
#define API extern "C" __attribute__((visibility("default")))
#endif

API int32_t foreign_add(int32_t a, int32_t b) { return a + b; }
API double foreign_scale(double value, double factor) { return value * factor; }
API bool foreign_positive(int32_t value) { return value > 0; }
```

Windows:

```powershell
clang++ -shared examples/export-api/imports/cpp/library.cpp -o out/import-interop/foreign_cpp.dll -Wl,/implib:out/import-interop/foreign_cpp.lib
```

Linux:

```sh
g++ -shared -fPIC examples/export-api/imports/cpp/library.cpp -o out/import-interop/libforeign_cpp.so
```

### Rust

```rust
#[no_mangle]
pub extern "C" fn foreign_add(a: i32, b: i32) -> i32 { a + b }

#[no_mangle]
pub extern "C" fn foreign_scale(value: f64, factor: f64) -> f64 { value * factor }

#[no_mangle]
pub extern "C" fn foreign_positive(value: i32) -> bool { value > 0 }
```

Windows:

```powershell
rustc --edition 2021 --crate-type cdylib examples/export-api/imports/rust/library.rs -o out/import-interop/foreign_rust.dll
```

Linux:

```sh
rustc --edition 2021 --crate-type cdylib examples/export-api/imports/rust/library.rs -o out/import-interop/libforeign_rust.so
```

### Go

```go
package main

/*
#include <stdint.h>
#include <stdbool.h>
*/
import "C"

//export foreign_add
func foreign_add(a, b C.int32_t) C.int32_t { return a + b }

//export foreign_scale
func foreign_scale(value, factor C.double) C.double { return value * factor }

//export foreign_positive
func foreign_positive(value C.int32_t) C.bool { return C.bool(value > 0) }

func main() {}
```

Linux:

```sh
CGO_ENABLED=1 CC=gcc go build -buildmode=c-shared -o out/import-interop/libforeign_go.so examples/export-api/imports/go/library.go
```

C++ ใช้ `extern "C"` เพื่อไม่ให้ชื่อ symbol ถูก C++ name mangling และบน Windows เพิ่ม `__declspec(dllexport)` ส่วน Rust ใช้ `extern "C"` กับ `#[no_mangle]` (ตัวอย่าง build ด้วย edition 2021) Go ใช้ cgo กับ `//export` และมี `main()` ว่างตามรูปแบบ shared library

## รันผ่าน DevLang runtime

Windows เลือก C++ หรือ Rust:

```powershell
d examples/export-api/imports/main.dev --ffi-lib ./out/import-interop/foreign_cpp.dll
d examples/export-api/imports/main.dev --ffi-lib ./out/import-interop/foreign_rust.dll --engine ast
```

Linux เลือก implementation ที่ต้องการ:

```sh
d examples/export-api/imports/main.dev --ffi-lib ./out/import-interop/libforeign_cpp.so
d examples/export-api/imports/main.dev --ffi-lib ./out/import-interop/libforeign_rust.so
d examples/export-api/imports/main.dev --ffi-lib ./out/import-interop/libforeign_go.so
```

ไม่ต้อง `d build` ไฟล์ DevLang ก่อนรัน แต่ต้อง build DLL/.so ของภาษาฝั่งต้นทางให้เรียบร้อย

## Native executable

บน Windows link ด้วย import library ไม่ใช่ส่ง `.dll` ให้ linker โดยตรง เก็บ DLL ไว้ข้าง executable:

```powershell
d build examples/export-api/imports/main.dev --release --cc clang --link out/import-interop/foreign_cpp.lib -o out/import-interop/cpp-app.exe
./out/import-interop/cpp-app.exe

d build examples/export-api/imports/main.dev --release --cc clang --link out/import-interop/foreign_rust.dll.lib -o out/import-interop/rust-app.exe
./out/import-interop/rust-app.exe
```

บน Linux link shared library โดยตรง ตัวอย่างใช้ absolute path ของ library และต้องเก็บ library ไว้ที่ path นี้ตอนรัน:

```sh
d build examples/export-api/imports/main.dev --release --cc gcc --link "$(pwd)/out/import-interop/libforeign_cpp.so" -o out/import-interop/cpp-app
./out/import-interop/cpp-app

d build examples/export-api/imports/main.dev --release --cc gcc --link "$(pwd)/out/import-interop/libforeign_rust.so" -o out/import-interop/rust-app
./out/import-interop/rust-app

d build examples/export-api/imports/main.dev --release --cc gcc --link "$(pwd)/out/import-interop/libforeign_go.so" -o out/import-interop/go-app
./out/import-interop/go-app
```

Shared library ไม่ได้ถูกฝังไว้ใน executable ตอน deploy ต้องจัด loader search path/rpath และส่ง library ไปด้วย อย่าสลับไฟล์ Windows/Linux, x64/ARM64 หรือ toolchain/runtime ที่ไม่เข้ากัน

## Expected output

ทุก implementation และทุก DevLang mode ต้องได้:

```text
42
12.5
true
false
-42
```

## ขอบเขตและ ownership

`i32` ตรงกับ `int32_t`, `f64` ตรงกับ C `double`, `bool` ตรงกับ C `bool`/`_Bool` ต้องประกาศ signature ให้ตรงกัน การเรียก foreign function อยู่ใน `unsafe` เพราะ DevLang พิสูจน์ lifetime และ signature ของ binary ภายนอกให้ไม่ได้

ตัวอย่างใช้ scalar values เท่านั้น ไม่ส่ง C++ objects, Rust String/Vec หรือ Go slice/string ผ่าน ABI โดยตรง หากต้องใช้ข้อมูลเหล่านี้ต้องออกแบบ wrapper และ ownership ให้ชัดเจน ห้ามปล่อย C++ exception หรือ Rust panic ข้าม C ABI

## ทดสอบซ้ำ

```sh
# Linux: C++, Rust and Go, all three DevLang modes
python scripts/smoke_import_interop.py --d target/release/d --cc gcc --cxx g++

# Windows: C++ and Rust, all three DevLang modes
python scripts/smoke_import_interop.py --d target/release/d.exe --languages cpp,rust
```

ใช้ `--cc`, `--cxx` เพื่อเลือก toolchain และ `--report out/import-interop.json` เพื่อบันทึกเวอร์ชันและผลลัพธ์ Library/executable ถูก build ในโฟลเดอร์ชั่วคราว และ test จะล้มเหลวถ้า compile/link/run ไม่ผ่านหรือ output ไม่ตรงกัน

ดูทิศทางกลับกันที่ [Export ไป C++, Rust และ Go](export-interop.md) ดาวน์โหลดโค้ดครบสองทิศทางจากหน้า Examples → Export interoperability
