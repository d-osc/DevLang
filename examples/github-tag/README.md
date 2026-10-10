# Dependency จาก GitHub tag

ตัวอย่างมีสองโปรเจกต์: `library` สำหรับ repository `dev-math` และ `app` สำหรับผู้ใช้ library URL `YOUR_USER` เป็น placeholder ต้องเปลี่ยนก่อน install ไม่ใช่ repository ที่เผยแพร่ไว้แล้ว

## 1. สร้าง library repository

นำ **เนื้อหาภายใน** `library/` ไปไว้ที่ root ของ GitHub repository ใหม่ชื่อ `dev-math`:

```text
dev-math/
  package.don
  src/lib.dev
```

Manifest ของ library:

```don
version: '1.2.0'
package: {
    name: 'math'
    entry: 'src/lib.dev'
    modules: 'src'
}
```

จากโฟลเดอร์ library ที่คัดลอกแล้ว และหลังสร้าง repository ว่างบน GitHub:

```powershell
git init
git add package.don src/lib.dev
git commit -m "Release math 1.2.0"
git branch -M main
git remote add origin https://github.com/YOUR_USER/dev-math.git
git push -u origin main
git tag v1.2.0
git push origin v1.2.0
```

ต้องมี `package.don` ที่ root **ของ commit ที่ tag ชี้อยู่** การมี manifest เฉพาะใน subfolder หรือเพิ่มหลังสร้าง tag ยังไม่เพียงพอ ถ้าเป็น private repository ต้องให้ Git ในเครื่องเข้าถึง repository ได้ก่อน อย่าใส่ token ลง manifest

## 2. ใช้ tag ตรง ๆ

แก้ `app/package.don` ให้ URL ตรงกับ repository ที่สร้าง:

```don
package: { name: 'github_tag_app' }
dependencies: {
    math: {
        git: 'https://github.com/YOUR_USER/dev-math.git'
        rev: 'v1.2.0'
    }
}
```

จาก root ของ DevLang repository:

```powershell
.\target\release\d.exe -C examples/github-tag/app pkg install
.\target\release\d.exe -C examples/github-tag/app run
.\target\release\d.exe -C examples/github-tag/app pkg install --locked
```

ผลลัพธ์ `42` โปรแกรมใช้ `use "math/lib" as math`; namespace มาจาก key `math` ใน dependencies และ `lib` คือ `src/lib.dev` ตาม `package.modules` ของ library

## 3. เลือก tag ด้วย SemVer

แทนที่ `rev` ด้วย `version` หากต้องการให้ resolver เลือก tag ที่ตรงช่วง:

```don
dependencies: {
    math: {
        git: 'https://github.com/YOUR_USER/dev-math.git'
        version: '^1.2'
    }
}
```

`^1.2` เลือก tag SemVer สูงสุดตั้งแต่ 1.2.0 จนก่อน 2.0.0; ใช้ `=1.2.0` หากต้องการ exact version tag รับทั้ง `1.2.0` และ `v1.2.0` แต่ไม่ควรสร้างทั้งคู่สำหรับ version เดียว เพราะจะกำกวม manifest version ใน tag ต้องตรงกับ tag

ห้ามใส่ `rev` และ dependency `version` พร้อมกัน `rev` เลือก Git ref โดยตรง ส่วน `version` เลือกจาก SemVer tags

## 4. อัปเดตและ lock

เมื่อ library มี tag ใหม่ เช่น `v1.3.0` และแอปใช้ `version: '^1.2'`:

```powershell
.\target\release\d.exe -C examples/github-tag/app pkg update
.\target\release\d.exe -C examples/github-tag/app run
```

ถ้าใช้ `rev: 'v1.2.0'` จะยังเลือก tag นั้น ต้องแก้เป็น `v1.3.0` ก่อน update `pkg install` ปกติคง commit ที่ lock ไว้; `pkg update` resolve ใหม่; `pkg install --locked` ยืนยัน commit/hash ตาม `dev.lock` เดิม ควร commit `dev.lock` และไม่ commit `.dev/packages` หลีกเลี่ยงการย้าย tag ที่เผยแพร่แล้ว

เพิ่ม dependency ด้วย CLI ได้เช่นกัน:

```powershell
.\target\release\d.exe -C examples/github-tag/app pkg add math --git https://github.com/YOUR_USER/dev-math.git --rev v1.2.0
```

หรือใช้ `--version '^1.2'` แทน `--rev v1.2.0`

## 5. ใช้ DON reference

หากแอปและ library ใช้ชื่อ tag เดียวกัน:

```don
version: 'v1.2.0'
package: { name: 'github_tag_app' }
dependencies: {
    math: {
        git: 'https://github.com/YOUR_USER/dev-math.git'
        rev: @version
    }
}
```

`@version` อ่านค่า root version ของ manifest แอป และส่ง `v1.2.0` ให้ Git; `rev: version` เป็น string ชื่อ `version`

ตัวอย่างนี้ทดสอบการ install tag, run ทั้งสอง source engines และ locked install ด้วย Git repository ในเครื่อง การเข้าถึง GitHub จริงต้องเปลี่ยน URL และเผยแพร่ library/tag ตามขั้นตอนข้างต้น
