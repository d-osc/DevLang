# Module API

คู่มือแยกครบทุก builtin ของ source runtime และ native C stdlib จาก implementation ปัจจุบัน แต่ละหน้ามีวิธี import, signatures, types, ตัวอย่าง และ errors/ข้อจำกัด ใช้ source build ล่าสุด เพราะ installer v0.4.0 ยังไม่มี API ใหม่ทั้งหมด

## Source runtime — 42 modules

| Module | ใช้งาน |
| --- | --- |
| [std/archive](archive.md) | สร้างและอ่าน ZIP/TAR ในหน่วยความจำ |
| [std/args](args.md) | อ่าน arguments ของโปรแกรมที่ส่งหลัง -- |
| [std/buffer](buffer.md) | จัดการ binary buffer และอ่าน/เขียน integers |
| [std/child_process](child_process.md) | เริ่มและจัดการโปรแกรมภายนอกพร้อม timeout และ output |
| [std/cli](cli.md) | แยก command-line flags, values และ positionals |
| [std/compression](compression.md) | บีบอัด bytes ด้วย gzip, zlib และ deflate |
| [std/crypto](crypto.md) | hash, HMAC, secure random และ authenticated encryption |
| [std/csv](csv.md) | อ่านและเขียนตาราง CSV |
| [std/datetime](datetime.md) | แปลง Unix milliseconds, RFC3339 และปฏิทิน |
| [std/dgram](dgram.md) | UDP sockets และ datagrams |
| [std/dns](dns.md) | resolve host ด้วย DNS resolver ของระบบปฏิบัติการ |
| [std/don](don.md) | Dev Object Notation พร้อม references และ object access |
| [std/encoding](encoding.md) | แปลงข้อความและ bytes ตาม encoding |
| [std/events](events.md) | ส่ง event ด้วย callback และ listener IDs |
| [std/fs](fs.md) | อ่าน/เขียนไฟล์และจัดการ directories |
| [std/fs/promises](fs-promises.md) | filesystem operations บน worker threads ที่คืน Task<T> |
| [std/http](http.md) | HTTP/HTTPS client และ HTTP server callbacks |
| [std/io](io.md) | อ่าน/เขียนข้อความบน console และไฟล์ UTF-8 |
| [std/json](json.md) | JSON values, object literals และ typed accessors |
| [std/log](log.md) | เขียน log ตามระดับไปยัง stderr หรือไฟล์ |
| [std/math](math.md) | คำนวณ f64, ราก, logarithm, ตรีโกณมิติ และปัดเศษ |
| [std/module](module.md) | ค้นหา builtin และ resolve imports ของ DevLang |
| [std/net](net.md) | TCP clients, servers และ IP validation |
| [std/os](os.md) | ข้อมูล platform, memory และระบบปฏิบัติการ |
| [std/path](path.md) | จัดรูปแบบและประกอบ path แบบ lexical |
| [std/process](process.md) | ข้อมูล process, environment และ working directory |
| [std/random](random.md) | สุ่มแบบทำซ้ำได้ด้วย seed สำหรับ simulation และการทดสอบ |
| [std/regex](regex.md) | ค้นหาและแทนที่ข้อความด้วย regular expressions |
| [std/result](result.md) | จับ runtime errors และจัดการ Result<T> |
| [std/sqlite](sqlite.md) | ฐานข้อมูล SQLite พร้อม parameter binding และ transactions |
| [std/stream](stream.md) | อ่าน/เขียน file stream แบบ bytes |
| [std/strings](strings.md) | จัดการข้อความ Unicode, ค้นหา, แยก และแทนที่ข้อความ |
| [std/sync](sync.md) | channels, atomic mutex updates และ cooperative cancellation ข้าม worker |
| [std/test](test.md) | assertions และชุดทดสอบแบบ callback |
| [std/time](time.md) | วัดเวลาที่ผ่านไปด้วยนาฬิกา monotonic และพัก thread |
| [std/timers](timers.md) | ตั้งเวลา callback และควบคุม event pump |
| [std/tls](tls.md) | เชื่อมต่อและเปิด server TLS พร้อมตรวจ certificate |
| [std/toml](toml.md) | แปลง TOML กับ JSON value model |
| [std/url](url.md) | อ่าน/ประกอบ URL และ query parameters |
| [std/uuid](uuid.md) | สร้างและตรวจ UUID |
| [std/websocket](websocket.md) | ส่งข้อความ WS/WSS แบบ text และ binary |
| [std/yaml](yaml.md) | แปลง YAML subset กับ JSON value model |

## Native C stdlib — 4 bindings

| Module | ใช้งาน |
| --- | --- |
| [std/io](native-io.md) | Native C stdlib: File and console I/O |
| [std/memory](native-memory.md) | Native C stdlib: Memory |
| [std/strings](native-strings.md) | Native C stdlib: Strings |
| [std/time](native-time.md) | Native C stdlib: Time |

## เลือกโหมดให้ถูก

Runtime builtin ไม่ต้องระบุ --module-dir หรือ --link ส่วน native ใช้ `--module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a` หลัง build stdlib อย่าสลับ signature ของ std/io, std/strings และ std/time ระหว่างโหมด ใช้ std/memory เฉพาะ native หรือใช้ Ref/Vec/Map สำหรับ managed data ของภาษา

Runtime errors จับด้วย [std/result](result.md) ได้ในขอบเขตที่ระบุ การ catch ไม่ย้อน side effects และ await บล็อก thread; filesystem/network/process APIs ไม่ใช่ Node.js compatibility หรือ coroutine async I/O

## ดาวน์โหลด

[Module API Markdown ZIP](https://d-osc.github.io/DevLang/downloads/devlang-module-api.zip) รวมหน้ารวมและคู่มือแยก module ส่วน [ตัวอย่างพร้อมผลลัพธ์และ ZIP](https://d-osc.github.io/DevLang/#/examples?category=Module%20API) อยู่ในเว็บไซต์ ใช้ repository checkout เมื่อต้องเปิด source และ aggregate reference links ของ Markdown แบบ local
