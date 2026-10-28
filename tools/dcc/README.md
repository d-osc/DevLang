# dcc — C compiler จิ๋วเขียนด้วย DevLang ล้วน ๆ + backend แทน clang/GCC ของ devc

`dcc` ทำสองงาน:

1. **compiler สเตนด์อโลน** — รับไฟล์ `.c` (subset ของ C) แล้วสร้าง executable
   Windows x86-64 ได้เองโดยตรง ไม่ต้องมี clang/GCC/assembler/linker
   แม้แต่ตัวเดียว (lexer → parser → x64 codegen → PE writer ทั้งหมดอยู่ใน
   `main.dev` ไฟล์เดียวที่รันบน DevLang runtime)

2. **cc backend ของ `devc`** — ผ่าน wrapper `dcc-cc.cmd` ทำให้
   `d build FILE.dev --cc tools/dcc/dcc-cc.cmd` ทำงานได้ครบทั้ง pipeline
   โดยไม่ต้องติดตั้ง C compiler เลย: devc สร้าง C → dcc compile เป็น
   machine code → เขียน PE เอง → devc publish ตามปกติ (รวมถึงระบบ
   cache ของ devc ที่ใช้ได้เหมือนเดิม)

## ใช้แทน clang/GCC ใน `d build`

```sh
d build program.dev --cc tools/dcc/dcc-cc.cmd --output program.exe
./program.exe
```

หรือครอบทั้ง session ด้วย env:

```sh
DEV_CC=C:/path/to/tools/dcc/dcc-cc.cmd d build program.dev --output program.exe
```

devc เรียก backend สามแบบ และ dcc รองรับครบ:

| การเรียก | พฤติกรรมของ dcc |
| --- | --- |
| `cc --version` | ตอบ version string (เข้า cache identity ของ devc) |
| `cc <flags> -c file.c -o file.o` | preprocess + parse-check แล้วเก็บ text ของ TU ไว้เป็น "object" |
| `cc <flags> objs... -o program.exe` | compile ทุก TU รวมกัน (multi-TU merge) แล้ว emit PE |

Flags ของ cc (`-std=c11 -O0/-O3 -gdwarf-4 -Wall ...`) ถูกยอมรับและ
ละเว้น (dcc ไม่มีระดับ optimization — output เดียว)

## ตัวอย่างที่ผ่านการทดสอบจริง

- `print` int/str/bool, ฟังก์ชันเรียกซ้ำ (`fib(24)`), for/while/break/continue
- โปรแกรมหลายไฟล์ (`use` → เส้นทาง -c/link ของ devc)
- `examples/benchmark.dev` — loop 100 ล้านรอบ + bitwise `&` ได้คำตอบถูกต้อง
- checked runtime ของ devc: div/rem/shift ตรวจ divide-by-zero,
  `dev_checked_fail` เขียน stderr ผ่าน `fprintf(stderr, "%s:%zu:%zu...")`,
  compound assign ผ่าน pointer (`dev_checked_div_assign_i64(&v, ...)`)
- cache ของ devc ทำงานปกติ (`0 compiled, N cached`), `--release` ผ่าน

## C subset ที่รองรับ

- ชนิด: `int8_t..int64_t`/`uint8_t..uint64_t`, `intptr_t/uintptr_t`,
  `size_t`, `bool`, `char`, `void`, pointer ทุกชนิด, `typedef`, `struct`
  (ประกาศได้ — สร้าง instance ยังไม่ได้)
- ประโยค: `if/else`, `while`, `for`, `break/continue`, `return`, บล็อกซ้อน
- นิพจน์: เลขคณิต, เปรียบเทียบ, `&& || !` (short-circuit), bitwise
  `& | ^ ~ << >>` (shift แยก signed/unsigned ตามชนิด), เลขคณิต pointer
  (scale ตามขนาด element), `p[i]`, `*p`, `&v`, cast ทุกความกว้าง
  (truncate/sign-extend ถูกต้อง), ternary `?:`, assignment `= += -= *= /= %=`,
  `++/--` pre/post
- ฟังก์ชัน: สูงสุด 8 พารามิเตอร์ (ตัวที่ 5 ขึ้นไปส่งผ่าน stack ตาม
  Windows x64 ABI), prototype + definition ข้าม header, `static`
- preprocessor: `#include` (ไฟล์ local + system headers จำลอง),
  include guards, `#if defined(X)`/`#ifdef/#ifndef/#else/#endif`,
  macro แบบ object และ function (`UINT64_C(v)` ฯลฯ), `#line`/`#pragma`
  ถูกข้าม
- CRT ที่รู้จัก: `printf`/`fprintf(stderr|stdout, ...)` (format:
  `%d %i %u %ld %lld %llu %zu %s %c %x %p %%`), `puts`, `fputs`,
  `putchar`, `strlen`, `exit` — ทั้งหมด implement เป็น machine code
  helper ในตัว executable เอง
- dead-code elimination: emit เฉพาะฟังก์ชันที่ reachable จาก `main`
  (prelude ของ devc ที่ไม่ได้ใช้จะหายไปทั้งก้อน)

## ข้อจำกัด (fail พร้อม error message ที่ชัดเจน)

- **float ไม่รองรับ** — โปรแกรม Dev ที่มี `f64` จะได้ error ตอน build
  (`float values are not supported by the dcc backend`) เพราะการ
  format ตัวเลขทศนิยม (`%.17g`) ใน machine code เขียนเองยากมาก
- struct instance / record ของ Dev, enum payload, dynamic values
  (Vec/Map/object), การจอง heap (malloc), `--lib`/ar ยังไม่รองรับ —
  โปรแกรมที่ใช้สิ่งเหล่านี้ควรใช้ clang/GCC backend ตามเดิม
- debug info (DWARF) ไม่มี, optimization ไม่มี (output เดียว)

## สถาปัตยกรรม

```
.dev ──devc──> C11 ──dcc-cc.cmd──> d run main.dev -- cc <args>
                                         │
                                         ├─ preprocessor (includes/macros/#if)
                                         ├─ lexer + recursive-descent parser
                                         │   (precedence climbing ชั้นเดียว
                                         │    ให้อยู่ใน recursion limit ของ runtime)
                                         ├─ type registry + reachability
                                         ├─ x64 code generator (machine code ลง
                                         │   Vec<u8> + fixup records)
                                         └─ PE writer (headers, imports, IAT)
                                                │
                                                v
                                          program.exe (~2-4 KB)
```

- ไม่มี assembler/linker — เขียน PE file ตรง ทุก branch/call ใช้
  rel32 + patch หลัง layout นิ่ง, ทุก address เป็น RIP-relative
  จึงไม่ต้องมี relocation table
- Windows x64 ABI ครบ: args ใน `rcx/rdx/r8/r9` + shadow space,
  argument ตัวที่ 5-8 บน stack, stack 16-byte aligned ทุก call
- object file ของ dcc คือ preprocessed text ของ TU (self-contained
  ผ่าน cache ของ devc ได้) — ขั้น link คือ recompile ทุก TU รวมกัน

## การ debug

สร้างไฟล์เปล่าชื่อ `dccdbg` ใน working directory ก่อนรัน dcc
(หรือผ่าน dcc-cc.cmd) จะพิมพ์ตำแหน่ง fixup ทั้งหมด
(branch/call/import/string) กับ layout ของ section

## การทดสอบ

```sh
# C-subset โดยตรง
for t in hello fib loops logic; do
  d run tools/dcc/main.dev -- tools/dcc/tests/$t.c -o /tmp/$t.exe && /tmp/$t.exe
done

# ผ่าน d build แบบไม่มี C compiler
d build examples/benchmark.dev --cc tools/dcc/dcc-cc.cmd --output /tmp/bench.exe && /tmp/bench.exe
d build tools/dcc/tests/multi/main.dev --cc tools/dcc/dcc-cc.cmd --output /tmp/multi.exe && /tmp/multi.exe
```

- `tests/hello.c, fib.c, loops.c, logic.c` — C subset โดยตรง
- `tests/multi/` — โปรแกรม Dev สองไฟล์ (เส้นทาง -c/link)
