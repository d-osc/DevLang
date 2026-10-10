# ตัวอย่าง package.don แบบครบ

ตัวอย่างนี้รันได้โดยไม่ใช้อินเทอร์เน็ต แสดง workspace dependency, local path dependency, version requirements และการ import module มาตรฐาน ใช้ `d` ที่ build จาก source ล่าสุด

## โครงสร้าง

```text
package-don/
  package.don                  # workspace root
  src/main.dev                 # โปรแกรมของ root
  apps/demo/
    package.don                # demo_app
    src/main.dev
  libs/math/
    package.don                # workspace member ชื่อ math
    src/lib.dev
  vendor/greetings/
    package.don                # path dependency; ไม่ใช่ workspace member
    src/lib.dev
```

## รันบน Windows จาก root ของ repository

```powershell
.\target\release\d.exe -C examples/package-don pkg workspace
.\target\release\d.exe -C examples/package-don pkg install --workspace
.\target\release\d.exe -C examples/package-don run --package demo_app
.\target\release\d.exe -C examples/package-don run --package demo_app --engine ast
.\target\release\d.exe -C examples/package-don pkg install --workspace --locked
.\target\release\d.exe -C examples/package-don --package demo_app pkg list
```

ผลลัพธ์ของโปรแกรม:

```text
Hello from package.don
42
```

เมื่อ `d` อยู่ใน PATH ใช้ `d` แทน `.\target\release\d.exe` ได้ `-C` เปลี่ยน working directory; `--package` เลือกจาก `package.name` ของ member คำสั่ง `run` ที่ไม่ระบุไฟล์ใช้ `package.entry` โดยอัตโนมัติ ต้อง install dependencies ก่อน run

## แต่ละ field ใช้ทำอะไร

| Field | ความหมาย |
|---|---|
| `version` | เวอร์ชันของ package อยู่ระดับ root; รับ `1.0.0` หรือ `v1.0.0` |
| `package.name` | ชื่อ package และชื่อที่ใช้เลือก workspace member |
| `package.entry` | ไฟล์เริ่มต้น ค่าเริ่มต้น `src/main.dev` |
| `package.modules` | โฟลเดอร์สำหรับ resolve imports จากผู้ใช้ package ค่าเริ่มต้น `src` |
| `dependencies` | ชื่อ namespace ที่ใช้ใน `use` และแหล่ง package |
| `dependencies.NAME.path` | พาธ relative จากโฟลเดอร์ที่มี manifest ของผู้ใช้ dependency; อนุญาต `..` |
| `dependencies.NAME.git` | URL Git repository |
| `dependencies.NAME.tag` | Git tag, branch หรือ commit; ใช้ร่วมกับ dependency `version` ไม่ได้ |
| `dependencies.NAME.branch` | ชื่อ remote branch เช่น `main` หรือ `feature/new-api`; ใช้พร้อม `tag`/`version` ไม่ได้ |
| `dependencies.NAME.version` | SemVer requirement สำหรับตรวจ local/workspace หรือเลือก Git tag |
| `dependencies.NAME.workspace` | `true` เพื่อใช้ member ที่ชื่อ package ตรงกัน |
| `workspace.members` | รายการโฟลเดอร์ member แบบ explicit relative paths; ไม่รับ `..` หรือ glob |

เลือกแหล่ง dependency เพียงหนึ่งแบบ: `path`, `git` หรือ `workspace: true` สมาชิกทุกตัวและ path/Git package ต้องมี manifest ของตนเอง

ชื่อ package/dependency ต้องเริ่มด้วยตัวอักษรภาษาอังกฤษหรือ `_` ตามด้วยตัวอักษร ตัวเลข หรือ `_` และห้ามใช้ชื่อ `std`; ใช้ `demo_app` แทน `demo-app`

`use "math/lib"` หมายถึง dependency namespace `math` ตามด้วยไฟล์ `lib.dev` ภายใน `package.modules` ของ math ส่วน `use "std/io"` เป็น runtime builtin ไม่ต้องใส่ใน dependencies

## Git dependency และ DON reference

ตัวอย่างต่อไปนี้เป็นแม่แบบ ต้องเปลี่ยน URL เป็น repository จริงก่อน install ไม่ได้เปิดใช้งานในตัวอย่างที่รันข้างต้น

```don
version: 'v1.0.0'
package: { name: 'my_app', entry: 'src/main.dev', modules: 'src' }
dependencies: {
    // เลือก tag ที่ตรงช่วงเวอร์ชัน เช่น v1.2.3
    math: {
        git: 'https://github.com/your-org/math.git'
        version: '^1.2'
    }
    // ใช้ค่า version ระดับ root เป็นชื่อ Git ref
    utils: {
        git: 'https://github.com/your-org/utils.git'
        tag: @version
    }
}
```

`@version` คือ DON reference; `tag: version` เป็น string `version` ไม่ใช่การอ่านตัวแปร Git ref ที่ระบุต้องมีอยู่จริง ถ้าใช้ dependency `version` ระบบเลือก tag SemVer สูงสุดที่ตรงเงื่อนไข และ version ของ manifest ใน tag ต้องตรงกัน

| Requirement | ตัวอย่างความหมาย |
|---|---|
| `=1.2.3` | เวอร์ชันเดียว |
| `^1.2` | ตั้งแต่ 1.2.0 จนก่อน 2.0.0 |
| `~1.2` | ตั้งแต่ 1.2.0 จนก่อน 1.3.0 |
| `>=1.2, <2` | ต้องผ่านทั้งสองเงื่อนไข |
| `*` | เวอร์ชัน stable ใดก็ได้ |

## เพิ่ม ลบ อัปเดต และ lock

จาก root ของ repository (คำสั่ง add ด้านล่างเพิ่ม dependency ที่มีอยู่แล้วในตัวอย่าง):

```powershell
.\target\release\d.exe -C examples/package-don --package demo_app pkg add math --workspace --version '^1.2'
.\target\release\d.exe -C examples/package-don --package demo_app pkg remove math
.\target\release\d.exe -C examples/package-don --package demo_app pkg add math --workspace --version '^1.2'
.\target\release\d.exe -C examples/package-don pkg update --workspace
```

`pkg install` สร้าง `package-lock.don` และ cache `.dev/packages` ของแต่ละ package ควร commit `package-lock.don` และไม่ commit cache `install` ปกติคง Git commit ที่ lock ไว้; `update` เลือกใหม่; `install --locked` ต้องมี lock เดิมและตรวจเนื้อหา การเปลี่ยน source ของ path package ต้อง install ปกติอีกครั้งเพื่ออัปเดต hash

`pkg add`/`remove` อาจเขียน manifest ใหม่และไม่รักษา comments คำสั่งจัดรูปแบบที่รักษา comments และ references คือ:

```powershell
.\target\release\d.exe don fmt-source examples/package-don/apps/demo/package.don
```

คำสั่งนี้ส่งผลลัพธ์ไป stdout; ใน VS Code ใช้ Format Document เพื่อจัดรูปแบบไฟล์ได้

## ข้อจำกัดปัจจุบัน

ยังไม่มี public registry, publish, `scripts`, `description`, `license` หรือ custom metadata ใน manifest; field ที่ไม่รู้จักถูกปฏิเสธ การ install ทั้ง workspace ไม่เป็น transaction เดียว และยังไม่มี resolver ที่ backtrack หรือโหลดหลาย version ใน namespace เดียว

ตัวอย่างนี้ใช้ runtime builtin `std/io`; native build ต้องใช้ native stdlib bindings/link settings ที่เหมาะสมกับ native API อย่านำ runtime API ไปสมมติว่าเป็น native API เดียวกัน

อ่านเพิ่มเติม: [package manager](../../docs/packages.md), [DON](../../docs/don.md)
