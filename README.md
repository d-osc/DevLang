# Dev Lang

ใช้คำสั่ง `d` ได้จาก bundle ใน `dist/d/<platform>/` หรือ `target/release/` หลัง `cargo build --release`:

```sh
d examples/modules/main.dev
d run examples/modules/main.dev
d --run examples/modules/main.dev
d -r examples/modules/main.dev

d build examples/modules/main.dev
d --build examples/modules/main.dev
d -b examples/modules/main.dev
d compiler examples/modules/main.dev
d --compiler examples/modules/main.dev
d -c examples/modules/main.dev
```

กลุ่มแรกใช้ runtime กลุ่มหลังใช้ compiler โดย `d` เรียก `devrun` หรือ `devc` ที่อยู่ข้างกัน ทั้งสองยังเป็น executable แยกกัน ใส่โฟลเดอร์ bundle ใน PATH เพื่อใช้คำสั่งสั้น ๆ ดู [คู่มือ CLI](cli/README.md)

Option พื้นฐาน:

```sh
d --help
d --version
d --cwd examples/modules main.dev --timings
d --eval 'print(40 + 2)'
d --check examples/modules/main.dev
d build examples/modules/main.dev --release --output out/app
d build examples/modules/main.dev --debug
d emit examples/modules/main.dev --output out/generated
d examples/runtime/main.dev -- hello
```

`-h` คือ help, `-v`/`-V` คือ version, `-C` คือ working directory และ `-o` คือ output ส่วน `--timings` แสดงเวลาทาง stderr โดยไม่ปนกับผลลัพธ์โปรแกรม Options สำหรับ build ใส่หลังชื่อไฟล์

Dev Lang ใช้ syntax สั้น ๆ มี compiler `devc` สำหรับ native code และ runtime `devrun` สำหรับรัน source โดยตรง ทั้งสองเขียนด้วย Rust และใช้ parser จาก `syntax/` ร่วมกัน Compiler แปลง Dev → C11 → native executable หรือ static library ผ่าน Clang/GCC หรือ TinyCC ส่วน C library แยกอยู่ใน `stdlib/`

```dev
fn main() {
    let answer = 40 + 2
    print("Hello Dev")
    print(answer)
}

main()
```

ไม่ต้องใส่ semicolon, ไม่ต้องเขียนชนิดของตัวแปรทุกตัว และไม่ต้องใส่ `:` หรือ `->` ในฟังก์ชัน ตัวแปร `let` แก้ค่าได้

`compiler/` builds native programs (`devc`); `runtime/` executes source (`devrun`);
`syntax/` provides shared parsing; `stdlib/` contains the independent native C library.

## เริ่มใช้งาน

มี compiler ที่ build แล้วใน `dist/compiler/windows-x86_64/devc.exe` และ `dist/compiler/linux-x86_64/devc` พร้อม `SHA256SUMS` ใช้งานเดี่ยว ๆ ได้ ส่วน runtime เป็นอีกแพ็กเกจใน `dist/runtime/<platform>/`:

```powershell
.\dist\compiler\windows-x86_64\devc.exe run examples/hello.dev
```

ตัว binary Linux นี้เป็น x86-64 แบบ dynamic และต้องใช้ glibc 2.39 ขึ้นไป หากต้องใช้ระบบเก่ากว่าให้ build จาก source บนระบบนั้น

การ build compiler จาก source ต้องมี Rust ส่วนการ compile โปรแกรม Dev ต้องมี Clang/GCC หรือ TinyCC ถ้าใช้ Clang บน Windows ต้องมี Windows SDK / C runtime toolchain ด้วย

```powershell
cargo build --release
.\target\release\devc.exe run examples/hello.dev
.\target\release\devc.exe run examples/modules/main.dev
.\target\release\devc.exe run examples/memory.dev
.\target\release\devc.exe run examples/ffi/main.dev --link examples/ffi/device.c
```

บน Linux ใช้คำสั่งเดียวกันโดยเปลี่ยนชื่อ executable:

```sh
cargo build --release
./target/release/devc run examples/hello.dev
./target/release/devc build examples/benchmark.dev --release --timings
```

Linux binary ใน `dist` มี TinyCC backend รวมไว้แล้ว ใช้โหมด build เร็วระหว่างพัฒนาได้ทันที:

```sh
./dist/compiler/linux-x86_64/devc run examples/hello.dev --fast
./dist/compiler/linux-x86_64/devc run examples/modules/main.dev --fast
./dist/compiler/linux-x86_64/devc build main.dev --release --native
```

`--fast` เน้นเวลา build และสร้าง code ที่ไม่ optimize ส่วน `--release` ใช้ `-O3` กับ Clang/GCC เพื่อความเร็วขณะรัน `--native` เพิ่มคำสั่งตาม CPU ปัจจุบันและอาจรันบนเครื่องอื่นไม่ได้ ผล benchmark พบว่าบางงานเร็วขึ้นและบางงานช้าลง จึงเปิดเป็นตัวเลือก โหมด `--fast` ใช้กับ executable บน OS; static library และ hardware ใช้ Clang/GCC ตามเดิม

ถ้า build compiler จาก source ให้ติดตั้ง TinyCC หรือกำหนด `DEV_TCC` / `--cc /path/to/tcc` Windows ไม่มี TinyCC binary รวมไว้และโหมดนี้ยังไม่ได้ตรวจด้วย TinyCC บน Windows สร้าง backend Linux ซ้ำได้ด้วย `bash scripts/build-fast-backend.sh`

หากต้องการติดตั้งคำสั่ง `devc` ไว้ใน PATH ใช้ `cargo install --path compiler` ชื่อ compiler คือ `devc` เพื่อใช้ร่วมกับคำสั่ง `dev` ของ Dev OS Package Manager ได้

## สิ่งที่รองรับ

| ความต้องการ | การทำงานใน v0.4 |
| --- | --- |
| Syntax น้อย | `fn`, `let`, `if`, `while`, `for`, `return`, `use`; newline จบ statement |
| เขียนง่าย | อนุมานชนิดตัวแปรและ literal; ตรวจชนิดข้อมูลก่อนเรียก backend |
| Native code | ไม่มี VM หรือ GC; `--release` ใช้ `-O3`; `--native` เปิดคำสั่งตาม CPU |
| Hardware / C | C ABI, `extern fn`, `export fn`, pointer, fixed-width integers, volatile access |
| Compile เร็ว | `--fast` ใช้ TinyCC; ไฟล์เดียว compile/link ครั้งเดียว; object/executable cache |
| หลายไฟล์ | import แบบ relative, alias, forward declaration, import เป็นวงจรได้ และ compile object ขนาน |
| Runtime | `devrun` อ่านและรัน source โดยตรง ไม่ต้องมี C compiler |

default ใช้ `-O0`, release ใช้ `-O3` และ `--fast` เลือก TinyCC ความเร็วขณะรันและเวลาคอมไพล์ขึ้นกับโปรแกรมและ backend ดูผลก่อน/หลัง ตัวเลือกที่ตรวจแล้ว และข้อแลกเปลี่ยนใน [docs/optimization.md](docs/optimization.md)

มี benchmark เปรียบเทียบ **Dev Lang / C / Rust / Go** บนโจทย์เดียวกัน 3 แบบ โดยรับ input ขณะรัน ตรวจ checksum กับ Python reference และเก็บค่ากลางหลายรอบ ดู [ผล v0.2 แบบ portable](docs/optimization-portable.md), [แบบ CPU ปัจจุบัน](docs/optimization-native.md) และ [ผล v0.1 เดิม](docs/comparison-linux.md)

## Runtime: execute source directly

```powershell
.\dist\runtime\windows-x86_64\devrun.exe examples/hello.dev
.\dist\runtime\windows-x86_64\devrun.exe examples/modules/main.dev
.\dist\runtime\windows-x86_64\devrun.exe examples/runtime/main.dev -- hello
.\dist\runtime\windows-x86_64\devrun.exe -e 'print(40 + 2)'
```

Linux: `./dist/runtime/linux-x86_64/devrun`. Interprets Dev using in-memory numeric plans and AST fallback.
Pure Dev execution needs no compiler or build artifacts.
File-level statements start execution; call `main()` explicitly. Built-in I/O, strings, time and arguments use
`std/*` imports. Values are managed automatically. The default engine uses typed numeric plans for compute loops, with AST fallback.
It has no JIT and does not claim native performance. See [runtime measurements](docs/runtime-compute.md). Native C FFI uses `extern fn` and `--ffi-lib PATH`. Direct hardware/volatile access uses `devc` or a native library.
See [runtime](runtime/README.md) and [API](docs/runtime.md).

Function declarations do not run automatically. Add `main()` at file level to start your program, or use `return main()` to propagate its exit code. Both the runtime and compiler execute entry-file statements.

Runtime FFI example:

```powershell
.\target\release\d.exe examples/ffi/main.dev
```

ไม่ต้อง build เอง: runtime หา library ข้างโมดูลก่อน ถ้ายังไม่พบ symbol จะให้ `devc` ที่อยู่ข้าง `devrun` เตรียม C dependency ให้อัตโนมัติ ครั้งแรกต้องมี Clang/GCC หรือ `DEV_CC` แล้วเก็บแคชใน `.dev-cache/native/` ครั้งถัดไปใช้แคชได้โดยไม่ต้องมี C backend; แก้ C/header แล้วจะเตรียมใหม่ ไฟล์ Dev ยังคงรันผ่าน interpreter โดยตรง ใช้ `--ffi-lib PATH` เพื่อเลือก library เอง (prebuilt library มีลำดับก่อน source อัตโนมัติ). See [FFI example](examples/ffi/README.md).

## Native stdlib

The previous C library is now in `stdlib/`, with its own build and package:

```sh
python stdlib/build.py
devc run examples/stdlib/main.dev --module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a
```

See [native API](docs/stdlib.md). Its C ABI and explicit ownership differ from
the interpreter's intrinsic API.

## หลายไฟล์

`math.dev`:

```dev
fn add(a i64, b i64) i64 {
    return a + b
}
```

`main.dev`:

```dev
use math

fn main() {
    print(math.add(20, 22))
}

main()
```

`use math` โหลด `math.dev` จากโฟลเดอร์เดียวกับไฟล์ที่ import ส่วน path แบบกำหนดเองใช้ `use "lib/math.dev" as math` ฟังก์ชันของโมดูลเรียกผ่าน `math.add(...)`

การแก้ body ของหนึ่งโมดูลจะ compile ใหม่เฉพาะ object ของโมดูลนั้น แล้ว link ใหม่ การเปลี่ยน signature จะ compile โมดูลที่ import มันโดยตรงด้วย

## Pointer, C และ hardware

```dev
extern fn puts(message str) i32

fn main() {
    puts("C ABI works")
    let register u32 = 0
    let port = &register
    volatile_store(port, 0x20)
    print(volatile_load(port))
}

main()
```

`&x` ให้ address, `*p` อ่านหรือเขียนหน่วยความจำ, `address as *u32` แปลง address เป็น pointer และ `volatile_load/store` ใช้ volatile access ของ C มีตัวอย่าง MMIO library ใน `examples/hardware/registers.dev`

```sh
devc build examples/hardware/registers.dev --lib --freestanding -o out/libregisters.a
```

สร้าง ARM Cortex-M4 library ด้วย Clang:

```sh
devc build examples/hardware/registers.dev --lib --freestanding --cc clang --ar llvm-ar --cflag --target=arm-none-eabi --cflag -mcpu=cortex-m4 --cflag -mthumb -o out/libregisters-arm.a
```

ตรวจแล้วว่า archive ที่ได้มี ARM 32-bit ELF object แต่ยังไม่ได้รันบนบอร์ดจริง ต้อง link กับ startup code, linker script และ driver ของบอร์ด ส่วนโปรแกรมบน OS ต้องใช้ address ที่ OS อนุญาตหรือ map ไว้ `volatile` ไม่ใช่ memory barrier หรือ atomic operation

## คำสั่ง

```sh
devc check main.dev                         # ตรวจ syntax/type ทุกโมดูล
devc run main.dev                           # build และรัน
devc build main.dev -o out/app              # build แบบเร็ว
devc run main.dev --fast                    # TinyCC development build
devc build main.dev --release --timings     # native optimized build
devc build main.dev --release --native      # optimize ตาม CPU ปัจจุบัน
devc run main.dev --cc clang                # เลือก C compiler
devc run main.dev --link driver.c           # link กับ C source
devc build library.dev --lib -o out/lib.a   # export fn เพื่อให้ C เรียกได้
devc emit main.dev -o out/generated         # ส่งออก .c/.h เพื่อตรวจหรือใช้ toolchain อื่น
devc --help
```

`--cflag` และ `--ldflag` ใช้ซ้ำได้ โดยแต่ละ option รับหนึ่ง argument ถ้าใช้ flag ที่อ้าง header/library ภายนอก compiler จะเลือก rebuild/relink เพื่อหลีกเลี่ยงแคชที่ล้าสมัย ตั้ง `DEV_CC` หรือ `--cc` เพื่อเลือก backend และ `--jobs N` เพื่อกำหนดจำนวนงาน compile ขนาน

## ทดสอบ

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
python scripts/smoke.py --compiler target/release/devc.exe
python scripts/smoke_fast.py --compiler target/release/devc.exe
python scripts/smoke_stdlib.py --compiler target/release/devc.exe
python stdlib/tests/test_native.py --cc clang
python scripts/benchmark.py --compiler target/release/devc.exe --output docs/benchmarks-local.json
python scripts/test_benchmark_compare.py
```

บน Linux เปลี่ยน compiler เป็น `target/release/devc` จาก Windows ที่มี Ubuntu WSL สามารถใช้ `wsl -d Ubuntu -- bash -l /mnt/c/Users/ondev/Projects/dev-lang/scripts/validate-linux.sh` เพื่อ stage บน Linux filesystem แล้วทดสอบและวัดความเร็วได้

เปรียบเทียบทั้ง 4 ภาษาเมื่อมี C, Rust และ Go compiler:

```sh
python3 scripts/benchmark_compare.py --compiler dist/compiler/linux-x86_64/devc --runs 11 --build-runs 5 --output docs/comparison-local.json --markdown docs/comparison-local.md
```

เพิ่ม `--native` เพื่อเทียบ CPU-specific flags ทั้ง 4 ภาษา และ `--fast-tcc /path/to/tcc` เพื่อวัด development build ของ Dev ด้วย TinyCC optimized runtime ยังคงใช้ Clang/GCC ตัวเดียวกับ C

ตัว script stage compiler ของ Dev, source, executable และแคชไว้ใน temporary filesystem ของระบบนั้น ตาราง build วัดการ compile โปรแกรมใหม่โดย dependency ของ toolchain อุ่นแล้ว แยกเวลาที่ Go สร้าง standard library ครั้งแรกออกต่างหาก

## ขอบเขต v0.4

นี่คือ compiler รุ่นเริ่มต้นที่รัน native program ได้จริง รองรับฟังก์ชัน, scalar types, array หนึ่งมิติ, raw pointer, module และ C ABI แบบ scalar/pointer รองรับ `for` แบบ range, value struct, enum ที่มี payload และ `match` และ explicit generics ของ function/struct/enum แล้ว รองรับ variadic FFI, scalar callbacks, C struct ABI, collections, closures, traits, const generics และ thread-backed async แล้ว ยังไม่มี inline assembly, package manager, debugger integration หรือ language server

มี `unsafe` ที่บังคับใช้ และตรวจ bounds, integer division และ shift ขณะรัน; raw pointer ที่หมดอายุและ bounds ของ allocation ภายนอกยังเป็นสัญญาที่ผู้เขียนต้องรักษา การจัดการหน่วยความจำและลำดับ side effect ใน expression ใช้แนวทาง C แยกการเรียกฟังก์ชันที่แก้ state เป็นคนละ statement เมื่อจำเป็น อ่านรายละเอียดใน [docs/language.md](docs/language.md)

Compiler ใช้ MIT ส่วน TinyCC ที่รวมใน Linux distribution ใช้ LGPL-2.1 มี license, revision และ source archive อยู่ใน `dist/compiler/linux-x86_64/backend/`

## Types and generics examples

```powershell
.\target\release\d.exe examples/features/main.dev
.\target\release\d.exe build examples/features/main.dev
```

Both runtime and native compilation support range `for`, structs, payload enums with exhaustive `match` and inferred/explicit function/struct/enum generics. See [language syntax and limits](docs/language.md).


Payload enum and recursive reference example:

```powershell
target\release\d.exe examples/features/payload.dev
target\release\d.exe build examples/features/payload.dev -o out/payload.exe
```

`Ref<T>` is immutable managed storage reclaimed by reference counting in both modes. Foreign-memory operations require `unsafe`; raw-pointer lifetimes remain the caller's responsibility. See
[recursive types and reference lifetimes](docs/language.md#recursive-types-and-immutable-references).


See [advanced features and current boundaries](docs/advanced.md) and
`examples/features/advanced.dev` for collections, closures, traits, const generics,
match guards, OS threads, async functions and C callbacks/varargs. Both source
runtime and native compiler share these language features.

Map now uses a COW hash table in both modes. `ready(task)` polls without waiting;
dropping a task detaches unfinished work. Capturing C callbacks retain their
environments (native: 64 unique pairs per signature per module). Try
`examples/features/tasks_callbacks.dev` and see [Map measurements](docs/map-performance.md).

`callback_context(f)` supports C APIs with userdata and managed environment cleanup,
without the native 64-instance limit. `then(task, next)` chains work without waiting
in the caller. Try `examples/features/continuations.dev`; tasks still use OS threads.
