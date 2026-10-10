# Dependencies จาก URL ของ ZIP/TAR

รองรับ `.zip`, `.tar`, `.tar.gz`, `.tgz` ผ่าน `https://`, `http://` และ `file://` ใช้ HTTPS สำหรับไฟล์เผยแพร่

```don
package: { name: 'archive_app' }
dependencies: {
    math: {
        url: 'https://YOUR_HOST/dev-math-1.2.0.zip'
        sha256: 'YOUR_64_CHARACTER_SHA256'
        version: '=1.2.0'
    }
}
```

เปลี่ยน URL และ checksum จริงก่อนใช้ `version` เป็น optional requirement สำหรับตรวจ manifest ภายในไฟล์; ไม่ได้ค้นหา archive เวอร์ชันอื่น `url` ใช้ร่วมกับ path/git/workspace/tag/branch ไม่ได้

ใน archive ต้องมี `package.don` และ `src/lib.dev` จากโฟลเดอร์ `library` ที่ root หรืออยู่ใน wrapper directory เดียว เช่น `math-1.2.0/package.don` ไม่ต้องมี Git repository ภายใน archive

## เตรียม archive และคำนวณ hash

จาก root ของ DevLang repository:

```powershell
Compress-Archive -Path examples/archive-url/library/* -DestinationPath out/dev-math-1.2.0.zip -Force
tar -cf out/dev-math-1.2.0.tar -C examples/archive-url/library package.don src
tar -czf out/dev-math-1.2.0.tar.gz -C examples/archive-url/library package.don src
(Get-FileHash out/dev-math-1.2.0.zip -Algorithm SHA256).Hash.ToLower()
```

คัดลอก hash ของ **archive ที่จะดาวน์โหลด** ลง `sha256` จากนั้นนำไฟล์ไปไว้บนเว็บหรือ GitHub Release asset URL เช่น `https://github.com/YOUR_USER/dev-math/releases/download/v1.2.0/dev-math-1.2.0.zip` เป็นแม่แบบ URL ต้องเผยแพร่ไฟล์จริงก่อน

เปลี่ยน URL เป็น `.tar` หรือ `.tar.gz` ได้ แต่ต้องคำนวณ checksum ของไฟล์นั้นใหม่

## Install และ run

แก้ `app/package.don` แล้วรัน:

```powershell
.\target\release\d.exe -C examples/archive-url/app pkg install
.\target\release\d.exe -C examples/archive-url/app run
.\target\release\d.exe -C examples/archive-url/app pkg install --locked
```

ผลลัพธ์ `42` แอป import ด้วย `use "math/lib" as math`

CLI เพิ่ม dependency ได้ด้วย:

```powershell
d pkg add math --url https://YOUR_HOST/dev-math-1.2.0.zip --sha256 YOUR_64_CHARACTER_SHA256 --version '=1.2.0'
```

`sha256` จำเป็นเสมอ ระบบตรวจ bytes ของ archive ก่อนแตกไฟล์ บันทึก source และ hash ของไฟล์ package ใน `dev.lock`; cache ที่มีอยู่จะถูกตรวจเนื้อหา และเมื่อ cache หายสามารถดาวน์โหลดกลับด้วย locked install ได้ ถ้า archive บน server เปลี่ยน checksum จะไม่ตรง ต้องแก้ URL/hash แล้ว install หรือ update ใหม่

## ตัวอย่างรันได้ทันทีโดยไม่ต้องมีเว็บภายนอก

```powershell
python examples/archive-url/verify.py target/release/d.exe
```

สคริปต์สร้าง ZIP/TAR/TAR.GZ จาก library เปิด HTTP server บน localhost แล้วทดสอบ install/run จริงในโฟลเดอร์ชั่วคราว ไม่แก้ manifest ของตัวอย่าง ผลลัพธ์เป็น PASS เมื่อทุกโหมดทำงานถูกต้อง

ขนาดดาวน์โหลดสูงสุด 64 MiB, แตกไฟล์รวมสูงสุด 128 MiB, ต่อไฟล์สูงสุด 16 MiB และ 8192 entries ไม่รับ symlink/hardlink, special files, duplicate paths หรือ path ที่ออกนอกโฟลเดอร์ แตกไฟล์ที่ล้มเหลวเก็บไว้ใน cache เพื่อให้ตรวจได้ ชื่อชนกันถือเป็น duplicate โดยไม่แยกตัวพิมพ์ใหญ่/เล็ก
