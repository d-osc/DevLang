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
