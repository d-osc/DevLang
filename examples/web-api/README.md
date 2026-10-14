# DevLang Web API

This directory is a runnable `package.don` project. With the current CLI/runtime:

```powershell
cd examples/web-api
d start
# Equivalent:
d run start
# Run the configured package entry directly:
d run -- serve
```

`d format` formats `main.dev`; `d format-check` checks formatting without changing it.
`src/` is reserved for local modules. HTTP/JSON are built-in runtime modules, so
`dependencies` is empty and no package install is required.

Or run from the repository root:

```powershell
d examples/web-api/main.dev -- serve
```

Listens on `http://127.0.0.1:3000`. Stop with Ctrl+C. Running without `-- serve`
prints usage and exits, so automated example validation does not start a listener.

| Method | Route | Response |
| --- | --- | --- |
| GET | `/healthz` | `{"status":"ok"}` |
| GET | `/api/users` | Two fixed sample users |
| POST | `/api/greet` | Accepts `{"name":"Dev"}`, returns `{"message":"Hello Dev"}` |

```powershell
Invoke-RestMethod http://127.0.0.1:3000/healthz
Invoke-RestMethod http://127.0.0.1:3000/api/users
Invoke-RestMethod http://127.0.0.1:3000/api/greet -Method Post -ContentType 'application/json' -Body '{"name":"Dev"}'
```

```sh
curl http://127.0.0.1:3000/api/users
curl -X POST http://127.0.0.1:3000/api/greet -H 'Content-Type: application/json' -d '{"name":"Dev"}'
```

Invalid JSON/missing or invalid name returns 400; unknown routes return 404;
wrong methods return 405 with `Allow`. Names must contain 1–100 UTF-8 bytes.
Responses use JSON and `Cache-Control: no-store`. Routes match exact URLs;
query parameters are not handled in this example. Users are fixed sample data;
the example does not create accounts or persist data.

This uses the source runtime. For public deployment, use the supervised
Windows/HTTPS ingress profile in [deployment instructions](../../deploy/http/README.md)
and add application authentication, storage and browser CORS policy as needed.

Validate real HTTP behavior:

```sh
python scripts/smoke_web_api.py
```

ภาษาไทย: ตัวอย่างนี้แสดง routing, ส่ง JSON, รับ POST body, ตรวจข้อมูล และ
HTTP status codes ใช้ข้อมูล users แบบคงที่ ไม่มีการบันทึกฐานข้อมูล
