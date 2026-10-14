# Python, C#/.NET และ Node.js: JavaScript/TypeScript

ตัวอย่างนี้เพิ่มการเชื่อมต่อสองทิศทาง: Python/.NET/Node เรียก native DevLang library และ DevLang runtime เรียกโค้ด Python/.NET/Node ผ่าน JSON ทาง stdin/stdout ต้อง build native adapter ก่อนใช้ฝั่งแรก และติดตั้ง runtime ของภาษาฝั่ง worker สำหรับฝั่งหลัง

## ช่องทางและผลทดสอบ

| Direction | Mechanism | Tested locally |
| --- | --- | --- |
| Python → DevLang | ctypes + DLL/shared library | Windows x64 Python 3.14, WSL Linux Python 3.12.3 |
| C#/.NET → DevLang | P/Invoke + DLL/shared library | Windows x64 .NET SDK 9.0.306 |
| JavaScript → DevLang | Node-API addon (.node) | Windows Node 24.18.0, WSL Linux Node 18.19.1 |
| TypeScript → DevLang | strict typecheck → JS → Node-API | Windows/WSL TypeScript 5.9.3 |
| DevLang → Python/.NET/JS/TS | JSON request/response in a child process | auto + AST; .NET locally on Windows, others on Windows/WSL |

CI รันทั้ง Python, .NET, JavaScript และ TypeScript บน Windows/Linux เพิ่มเติม ผลที่ตรวจมีเลขจำนวนเต็ม, double, bool true/false, string, JSON ภาษาไทย, ข้อมูลผิดชนิด, worker nonzero exit และ paths ที่มีช่องว่าง ไม่มีการจำลอง native library

## โครงสร้างและข้อกำหนด

```text
examples/export-api/
  api.dev                   # DevLang exports เดิม
  devmath.h
  managed/
    build.py                # Static library + shared adapter + optional Node addon
    prepare_node_sdk.py      # Official Node headers/import library with SHA256 checks
    bridge.c                # C ABI adapter for ctypes/PInvoke
    bridge.dev              # DevLang JSON worker caller
    python/client.py
    python/worker.py
    dotnet/ManagedInterop.csproj
    dotnet/Program.cs
    node/addon.c
    node/client.js
    node/client.ts
    node/worker.js
    node/worker.ts
```

ต้องมี Python 3.12+, `d`/`devc`, C compiler และ archiver สำหรับ build native artifacts; .NET example ใช้ .NET 9 SDK และ Node example ใช้ Node.js กับ TypeScript (`tsc` บน PATH) คำสั่งทั้งหมดด้านล่างรันจาก root repository

สำหรับ Node ใช้ Node 24 และ TypeScript เวอร์ชันที่ทดสอบ:

```sh
npm install --global typescript@5.9.3
python examples/export-api/managed/prepare_node_sdk.py --output out/node-sdk
```

`prepare_node_sdk.py` ดาวน์โหลด headers ที่ตรงกับ `node` ปัจจุบันจาก nodejs.org; Windows ดาวน์โหลด `node.lib` ของ architecture เดียวกันด้วย ตรวจไฟล์กับ SHA256SUMS จากแหล่งเดียวกันก่อนแตกไฟล์ ไม่มีการติดตั้ง Node runtime ใหม่

## Build native adapter

Windows (Clang/MSVC + llvm-ar บน PATH):

```powershell
python examples/export-api/managed/build.py --d target/release/d.exe --node-sdk out/node-sdk --output out/managed-interop
```

Linux (GCC + GNU ar):

```sh
python examples/export-api/managed/build.py --d target/release/d --cc gcc --ar ar --node-sdk out/node-sdk --output out/managed-interop
```

หากใช้เฉพาะ Python/.NET ไม่ต้องส่ง `--node-sdk` และไม่ต้องเตรียม Node SDK script จะสร้างเฉพาะ shared adapter `devbridge.dll` บน Windows หรือ `libdevbridge.so` บน Linux ส่วน Node addon ชื่อ `devmath.node` จะมีเมื่อส่ง SDK

C adapter เรียกฟังก์ชันจาก `api.dev` ชุดเดิม; Node addon link static library ชุดเดียวกันและเปิด `add`, `scale`, `positive`, `name` ผ่าน Node-API ไม่ได้สร้าง binding ให้ export ใด ๆ อัตโนมัติ

## Python → DevLang

ใช้ ctypes โดยระบุชนิด parameter/return ให้ตรง C ABI:

```python
"""Call the DevLang C ABI through a shared adapter library."""
import ctypes as C
from pathlib import Path
import sys

lib = C.CDLL(str(Path(sys.argv[1]).resolve()))
lib.bridge_add.argtypes = [C.c_int32, C.c_int32]
lib.bridge_add.restype = C.c_int32
lib.bridge_scale.argtypes = [C.c_double, C.c_double]
lib.bridge_scale.restype = C.c_double
lib.bridge_positive.argtypes = [C.c_int32]
lib.bridge_positive.restype = C.c_bool
lib.bridge_name.argtypes = []
lib.bridge_name.restype = C.c_char_p
print(lib.bridge_add(20, 22))
print(lib.bridge_scale(2.5, 5.0))
print(str(lib.bridge_positive(7)).lower())
print(str(lib.bridge_positive(-7)).lower())
# c_char_p yields bytes from a borrowed static string; do not free it.
print(lib.bridge_name().decode('utf-8'))
```

```powershell
python examples/export-api/managed/python/client.py out/managed-interop/devbridge.dll
```

บน Linux เปลี่ยน path เป็น `out/managed-interop/libdevbridge.so`

## C#/.NET → DevLang

P/Invoke ใช้ calling convention แบบ C และกำหนด bool ขนาดหนึ่ง byte:

```csharp
[DllImport("devbridge", CallingConvention = CallingConvention.Cdecl)]
private static extern int bridge_add(int a, int b);

[DllImport("devbridge", CallingConvention = CallingConvention.Cdecl)]
[return: MarshalAs(UnmanagedType.I1)]
private static extern bool bridge_positive(int value);
```

`Program.cs` ตั้ง `NativeLibrary.SetDllImportResolver` เพื่อโหลด path ที่ส่งให้จริง และใช้ `Marshal.PtrToStringUTF8` สำหรับ string ที่ยืมจาก native library มี source เต็มใน [Program.cs](../examples/export-api/managed/dotnet/Program.cs)

```powershell
dotnet build examples/export-api/managed/dotnet/ManagedInterop.csproj -o out/managed-interop/dotnet
dotnet out/managed-interop/dotnet/ManagedInterop.dll out/managed-interop/devbridge.dll
```

บน Linux ใช้ `libdevbridge.so` แทน DLL ตัวอย่างนี้เป็น P/Invoke บน .NET runtime ไม่ได้ต้องใช้ Native AOT

## JavaScript → DevLang

Node โหลด native addon ที่สร้างจาก [addon.c](../examples/export-api/managed/node/addon.c) ซึ่งใช้ Node-API version 8:

```javascript
const math = require(require('node:path').resolve(process.argv[2]));
console.log(math.add(20, 22));
console.log(math.scale(2.5, 5));
console.log(math.positive(7));
console.log(math.positive(-7));
console.log(math.name());
```

```sh
node examples/export-api/managed/node/client.js out/managed-interop/devmath.node
```

Addon ตรวจชนิด/range ของ i32 และปฏิเสธผลรวมที่ overflow ก่อนเรียก DevLang; ชุดทดสอบตรวจ string แทน number, NaN และ overflow ด้วย ไม่ต้องใช้ third-party FFI npm package

## TypeScript → DevLang

TypeScript ประกาศ interface ของ native methods แล้วตรวจด้วย `tsc --strict` ก่อนรัน:

```typescript
interface DevMath {
  add(a: number, b: number): number;
  scale(value: number, factor: number): number;
  positive(value: number): boolean;
  name(): string;
}
declare function require(name: "node:path"): { resolve(path: string): string };
declare function require(name: string): DevMath;
declare const process: { argv: string[] };
const math: DevMath = require(require('node:path').resolve(process.argv[2]));
console.log(math.add(20, 22));
console.log(math.scale(2.5, 5));
console.log(math.positive(7));
console.log(math.positive(-7));
console.log(math.name());
```

```sh
tsc --strict --noEmitOnError --module commonjs --target ES2020 --moduleDetection force --outDir out/managed-interop/ts examples/export-api/managed/node/client.ts examples/export-api/managed/node/worker.ts
node out/managed-interop/ts/client.js out/managed-interop/devmath.node
```

ผลที่คาดหวังของ native consumers ทั้งสี่แบบ:

```text
42
12.5
true
false
DevLang Math
```

## DevLang → Python/.NET/JavaScript/TypeScript

ทิศทางกลับใช้ `std/child_process.execFileSync` ส่ง JSON request ไปยัง runtime ของภาษานั้น และอ่าน JSON response โค้ด DevLang เดียวกันรับ executable กับ arguments ผ่าน `--`:

```dev-runtime
use "std/args"
use "std/child_process"
use "std/encoding"
use "std/json"
use "std/result"

fn main(): void {
    if args.len() == 0 as usize {
        result.raise("Pass the worker executable followed by its arguments")
    }
    let arguments = Vec<str>()
    for i in 1..(args.len() as i64) {
        arguments.push(args.get(i as usize))
    }
    let options = child_process.options()
    options.timeoutMs = 5000
    options.maxBuffer = 65536
    options.input = encoding.encode(json.stringify({a: 20, b: 22, name: "โลก"}), "utf8")
    let output = child_process.execFileSync(args.get(0), arguments, options)
    if !output.success {
        result.raise("Worker failed: " + encoding.decode(output.stderr, "utf8"))
    }
    let response = json.parse(encoding.decode(output.stdout, "utf8"))
    print(json.int(json.get(response, "sum")))
    print(json.string(json.get(response, "message")))
}

main()
```

```sh
d examples/export-api/managed/bridge.dev -- python examples/export-api/managed/python/worker.py
d examples/export-api/managed/bridge.dev -- node examples/export-api/managed/node/worker.js
d examples/export-api/managed/bridge.dev -- node out/managed-interop/ts/worker.js
d examples/export-api/managed/bridge.dev -- dotnet out/managed-interop/dotnet/ManagedInterop.dll worker
```

บน Linux อาจใช้ `python3` แทน `python` ทุก worker รับ `a`, `b` เป็น integer ระหว่าง -1000000 ถึง 1000000 และ `name` เป็น string ส่งกลับ `sum` กับ `message` พิมพ์เฉพาะ JSON ไป stdout และส่ง errors ไป stderr; invalid requests ออกด้วย code 2

ผลที่คาดหวัง:

```text
42
Hello โลก
```

ไฟล์ Python worker ตั้ง stdin/stdout เป็น UTF-8 ชัดเจน และ C# ใช้ UTF-8 console encoding เพื่อให้ข้อความภาษาไทยผ่าน pipe ได้

Bridge ส่ง arguments แยกโดยไม่ผ่าน shell ตั้ง timeout 5 วินาทีและ stdout/stderr รวมไม่เกิน 64 KiB แล้วตรวจ `success` ก่อน parse JSON หาก worker ล้มเหลว error จะส่งกลับไปให้ DevLang ด้วย

การเรียกนี้เป็น process request/response ต่อครั้ง มีค่า startup และ serialization เหมาะส่งงานเป็นชุด โค้ด worker แก้ฟังก์ชัน sum/greeting ได้เอง ไม่ใช่การ import Python/npm/.NET object เข้า DevLang โดยตรง และไม่มี shared-memory object หรือ callback อัตโนมัติ

`std/child_process` ใช้ใน source runtime auto/AST; ยังไม่รองรับ native build ของ `bridge.dev` ส่วน native library ฝั่ง consumers ใช้ DevLang compiler ตามปกติ

## Ownership และ deployment

- Build native artifacts ให้ตรง OS/architecture ของ Python/.NET/Node ที่โหลด
- String จาก `bridge_name` เป็น borrowed static literal ห้าม free; Python/Node/.NET แปลงเป็น string ของ runtime ตัวเอง
- C ABI ไม่รับ managed Vec/Map/Ref หรือ object ของ Python/.NET/JS โดยตรง
- ส่ง DLL/.so/.node ไปกับ application และติดตั้ง runtime ที่ต้องใช้; TypeScript ต้อง compile ก่อนรัน JS
- ตัวอย่างไม่ใช่ universal binding generator และไม่ได้ติดตั้ง package จาก PyPI/NuGet/npm ให้อัตโนมัติ

## Regression checks

```powershell
python scripts/smoke_managed_interop.py --d target/release/d.exe --node-sdk out/node-sdk
```

```sh
python scripts/smoke_managed_interop.py --d target/release/d --cc gcc --ar ar --node-sdk out/node-sdk
```

ใช้ `--languages python,dotnet` เพื่อทดสอบเฉพาะ Python/.NET โดยไม่ต้องมี Node SDK หรือ `--languages python,node` หากไม่มี .NET SDK; `--report out/managed-interop.json` เก็บผลและ toolchain versions

ดาวน์โหลด source ทั้งหมดจากหน้า [Export interoperability](export-interop.md) และดูการโหลด C++/Rust/Go เข้า DevLang โดยตรงผ่าน C ABI ที่ [Import interoperability](import-interop.md)

## Official references

- [Python ctypes](https://docs.python.org/3/library/ctypes.html)
- [.NET P/Invoke](https://learn.microsoft.com/en-us/dotnet/standard/native-interop/pinvoke)
- [Node-API](https://nodejs.org/api/n-api.html)
- [Node TypeScript support](https://nodejs.org/api/typescript.html)
