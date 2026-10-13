# Bounded HTTP deployment profile

This template is a starting profile for a **source-runtime** HTTP app behind a
trusted TLS edge. It does not certify the application's production readiness.

1. Build and keep `d`, `devrun` and `devc` together. Have the app use
   `server.listenOn(3000, "127.0.0.1")`. Keep port 3000 private.
2. Install NGINX and create `logs/`, `temp/client/`, `temp/proxy/` in this directory.
3. Validate with `nginx -t -p /absolute/path/to/deploy/http/ -c nginx.conf`.
4. Start with `nginx -p /absolute/path/to/deploy/http/ -c nginx.conf`.
5. Send trusted-edge traffic to loopback port 8080; configure TLS at that edge.
   Keep this internal port inaccessible to public clients too.
6. Supervise the runtime, rotate stderr/access logs, monitor RSS/error rates and
   check an application health endpoint through the proxy. Restart stalled
   workers; app state and writes must tolerate restart. The proxy timeout cannot
   interrupt a handler stuck in an infinite loop.

The profile uses one NGINX worker, 512 worker connections, at most 16 active
requests to this virtual server, a conservative 64 KiB body limit, full request buffering and
timeouts. Responses and requests close backend connections. Adjust limits from
measured workloads rather than removing them. The body/send/proxy read timeouts
are inactivity limits, not total execution deadlines.

Run application qualification on the deployment machine:

```sh
python scripts/soak_http_server.py --bin-dir target/release --requests 20000 --duration-seconds 3600 --report out/http-hour.json
```

This bundled test exercises a generated local app, not your business routes.
Also load-test the actual application with representative payloads, authentication,
database failures, client disconnects, process restarts and recovery. Native
compilation does not support this HTTP server; public exposure of the interpreter's
initial transport without an ingress proxy remains outside the qualified scope.

References: [request buffering](https://nginx.org/en/docs/http/ngx_http_proxy_module.html),
[core limits](https://nginx.org/en/docs/http/ngx_http_core_module.html),
[connection limits](https://nginx.org/en/docs/http/ngx_http_limit_conn_module.html).

ภาษาไทย: ใช้เป็น backend ที่ bind loopback ผ่าน reverse proxy และ TLS edge
ต้องมี supervisor/health check และทดสอบระบบจริงก่อนเปิด production ไม่ควร
เปิด port ของ runtime ให้ผู้ใช้ภายนอกเชื่อมต่อโดยตรง

## Live ingress verification

```sh
python scripts/smoke_http_ingress.py --bin-dir target/release --nginx nginx
```

The test starts the shipped profile on temporary loopback ports against an actual
DevLang interpreter, checks handler-error recovery, rejects bodies above 64 KiB
and oversized headers, checks slow body/header deadlines, and fills all 16 active
request slots to verify 503 load shedding. It then releases those slots and checks
recovery. Denied and partially uploaded requests must never appear in the backend
request log. Each test owns its processes and shuts down its own NGINX instance.
CI runs this on Linux; local Windows validation also uses the actual NGINX binary.

A timeout may close the connection without an HTTP error page. Body timeouts are
inactivity deadlines: continuously trickling data can keep a request alive.
This test does not qualify a TLS edge, enforce a total upload duration or test all
request-smuggling variants. Keep public ingress protection and application
qualification separate from these local transport regressions.

ภาษาไทย: มีการทดสอบ NGINX กับ DevLang จริงแล้ว รวม body/header เกินขนาด,
การส่งค้าง, เพดาน active requests และการกลับมารับงานหลัง overload แต่ยังต้อง
ทดสอบ TLS edge และ app จริงแยกต่างหาก timeout ของ body เป็น inactivity timeout
ไม่ใช่เพดานเวลารวมของการ upload

## HTTP worker supervisor

Python 3 on Windows/Linux can supervise a source worker without changing the
language/runtime separation:

```sh
python scripts/serve_http.py --runtime target/release/devrun --file examples/production-http/main.dev --health http://127.0.0.1:3000/healthz --max-rss-mib 256 --log out/http-worker.log
```

On Windows use `target/release/devrun.exe`. Use an absolute runtime path if the
executable is elsewhere. The worker's working directory is the source directory.
Keep the backend loopback-only and define a small `/healthz` route returning HTTP
200 with a finite body (at most 4096 bytes). Probes do not follow redirects.

Defaults: 15-second startup grace, 2-second polling interval, 1-second health
socket timeout, restart after 3 failed probes, and at most 5 restarts in a rolling
60-second window with capped exponential backoff. Unexpected worker exits,
including exit 0, are restarted: stop the supervisor to stop the service.
Exhausting the budget returns exit 1 for monitoring. Ctrl+C/SIGTERM stops the
worker; Windows also supports CTRL_BREAK. `--run-seconds` limits a qualification
run and exits 0 on normal stop. A supervisor launch/configuration error exits
nonzero instead of silently retrying a broken installation.

Worker stdout/stderr are drained in bounded chunks and rotated into one log with
three backups (`--log-bytes`, default 1 MiB per file; a final chunk may overshoot
by about 4 KiB). Supervisor events also appear on stdout. Monitor these events and
retain logs appropriately. A sampled RSS budget triggers restart, but it is not
a hard OS memory cap and can miss short allocation spikes; use OS resource limits
as well. Worker restarts interrupt in-flight requests and lose in-memory state.
Store durable data externally and use idempotent retry policies where applicable.

The health route must represent worker readiness. Long legitimate callbacks may
fail a short health budget because handlers are serial; tune budgets from measured
workloads. The supervisor does not repair application/database faults or manage
the external NGINX/TLS edge. Run it under a service manager for host boot/recovery.

```sh
python scripts/smoke_http_supervisor.py --bin-dir target/release
```

This regression creates a real DevLang worker, hangs its callback, closes its
listener and verifies recovery, checks restart exhaustion/RSS budgets, validates
log rotation and clean shutdown. CI executes it on Windows/Linux.

ภาษาไทย: supervisor มี health check, RSS budget, restart/backoff แบบจำกัดจำนวน
และ log rotation แต่การ restart ทำให้ request ที่กำลังทำงานถูกตัดและ state
ใน RAM หาย จึงต้องจัดการข้อมูลถาวรและ retry ของ app ให้เหมาะสม

The bundled `examples/production-http/main.dev` is a minimal loopback starter with
GET `/healthz`, GET `/`, 404 and 405 responses. Its health route checks listener
responsiveness only; extend readiness checks for application dependencies. It is
not a business application with authentication or durable storage.

## Windows production service

Windows supervision now places each worker in a Job Object **before it starts**.
The defaults enforce a 384 MiB committed-memory limit across the worker tree,
at most 8 processes and a 50% CPU hard rate (percentage of the machine's CPU
capacity). `--job-memory-mib`, `--job-processes` and `--cpu-percent` configure these.
RSS polling remains a separate early-warning/restart budget. Committed memory
and RSS measure different things. Closing the job or abruptly terminating the
supervisor kills the worker and descendants. Limit/assignment failures stop the
supervisor; it never falls back to an uncontained worker.

```powershell
python scripts/smoke_windows_job.py
python scripts/smoke_http_supervisor.py --bin-dir target/release
```

The kernel regression provokes allocation/process denial and forcibly terminates
an owner process to verify both its worker and grandchild exit. This was exercised
on Windows locally and is included in Windows CI. It does not qualify every
Windows version, service-wrapper policy or an arbitrary FFI application.

Prepare a dedicated deployment folder on the **production host**, for example:

```text
C:\ProgramData\DevLangHttp\
  devlang-http.exe          WinSW wrapper from official releases
  devlang-http.xml          supplied Windows Service configuration
  bin\devrun.exe
  scripts\serve_http.py
  scripts\soak_http_server.py
  scripts\windows_job.py
  app\main.dev             starter or your complete app/modules
  data\                    writable app state
  logs\                    writable rotating logs
```

Use [official WinSW releases](https://github.com/winsw/winsw/releases) and match
its executable/configuration naming. The supplied XML selects **LocalService**,
automatic delayed startup, two service-level recovery attempts then stop, and
bounded wrapper logs. Configure its absolute Python executable path for the
production installation before use. Python and its libraries must be readable
by LocalService. Keep code/binaries read-only to this account; allow Modify only
on data/logs. The following setup is for a new dedicated folder, in an elevated
PowerShell on the deployment host:

```powershell
icacls "C:\ProgramData\DevLangHttp" /inheritance:r /grant:r "*S-1-5-32-544:(OI)(CI)F" "*S-1-5-18:(OI)(CI)F" "*S-1-5-19:(OI)(CI)RX"
icacls "C:\ProgramData\DevLangHttp\data" /grant:r "*S-1-5-19:(OI)(CI)M"
icacls "C:\ProgramData\DevLangHttp\logs" /grant:r "*S-1-5-19:(OI)(CI)M"
Set-Location C:\ProgramData\DevLangHttp
.\devlang-http.exe install
.\devlang-http.exe start
.\devlang-http.exe status
# Maintenance:
.\devlang-http.exe stop
.\devlang-http.exe uninstall
```

The service profile is prepared; it has **not** been installed or boot-tested on
this development machine. Verify service start/stop, recovery and reboot on the
production host. Keep ports 3000/8080 loopback/private and run the TLS ingress as
a separate supervised service. Job Objects provide resource/process containment,
not a complete security sandbox. Service restarts still interrupt requests and
lose in-memory state. Persist application data under `--cwd` or external storage.

References: [Microsoft Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects),
[WinSW service configuration](https://github.com/winsw/winsw/blob/v3/docs/xml-config-file.md).

ภาษาไทย: เพิ่ม Windows Job Object ที่บังคับ memory/process/CPU limits จริง
พร้อมเก็บ process ลูกเมื่อ supervisor ตาย และเตรียม Windows Service แบบ
LocalService แล้ว แต่ยังไม่ได้ติดตั้ง service ลงเครื่องพัฒนานี้ ต้องตรวจ
startup/reboot/recovery บนเครื่อง production และตั้ง TLS แยกต่างหาก

## HTTPS ingress profile

`deploy/http/nginx-tls.conf` adds TLS 1.2/1.3 to the same body/header/concurrency
limits. It binds loopback port 8443 by default; configure the listen address and
`server_name` for the production host before public use. Keep the backend on
loopback port 3000. Install a valid certificate for your real hostname as
`certs/fullchain.pem` and its matching private key as `certs/privkey.pem` relative
to the NGINX prefix. Do not use the test certificate for public production.
Restrict private-key access to administrators and the ingress service account.

```powershell
nginx -t -p C:/ProgramData/DevLangIngress/ -c nginx-tls.conf
nginx -p C:/ProgramData/DevLangIngress/ -c nginx-tls.conf
# After replacing certificate files, validate first, then reload:
nginx -t -p C:/ProgramData/DevLangIngress/ -c nginx-tls.conf
nginx -p C:/ProgramData/DevLangIngress/ -c nginx-tls.conf -s reload
```

Check validation's exit status before executing reload. Certificate provisioning,
renewal and expiry alerts must be configured for the actual domain; they are not
automated by this template. Do not disable client hostname/certificate checks.
See [NGINX TLS directives](https://nginx.org/en/docs/http/ngx_http_ssl_module.html).

```sh
python scripts/smoke_http_ingress.py --bin-dir target/release --nginx nginx --tls --openssl openssl
```

This starts real HTTPS→NGINX→DevLang with a temporary localhost certificate,
trusted only by the test client. It verifies successful TLS 1.2 and TLS 1.3
connections, rejects an untrusted certificate and mismatched hostname, and runs
the existing body/header, slow-client and overload/recovery checks over TLS.
The test requires NGINX built with TLS and OpenSSL's CLI. It passed locally on
Windows; Linux CI also runs it. No certificate is installed in the OS trust store.
These results qualify this local profile, not public DNS, a production certificate
chain or business application readiness.

ภาษาไทย: มี profile HTTPS และทดสอบ TLS/certificate/hostname กับ server จริงแล้ว
แต่ต้องใช้ certificate ของโดเมนจริง พร้อมตั้ง renewal/expiry monitoring และ
ตรวจ configuration บนเครื่อง production ก่อนเปิดรับผู้ใช้ภายนอก

### Certificate and HTTPS health preflight

```powershell
python scripts/check_http_certificate.py --url https://your-domain.example/healthz --min-days 14
```

This verifies the certificate chain and hostname, requires the certificate to be
valid for at least the configured remaining days, then requires HTTP 200 with a
response body at most 4096 bytes. It prints JSON and returns 0 for success, 1 for
certificate/health failure. `expires_soon` signals renewal is needed. Private PKI
can use `--cafile PATH`; there is no option to disable verification. The timeout
is a socket I/O timeout, not a total DNS/probe deadline. Connect the exit status
and JSON to your monitoring system; certificate issuance/renewal is still an
operator-managed deployment step. The TLS integration regression tests both a
successful preflight and rejection of a one-day certificate by the 14-day budget.

ภาษาไทย: มีคำสั่งตรวจ certificate chain, hostname, วันหมดอายุ และ health endpoint
พร้อม JSON/exit code สำหรับ monitoring แล้ว แต่ยังต้องตั้งระบบออกและต่ออายุ
certificate ของโดเมนจริงบนเครื่อง production
