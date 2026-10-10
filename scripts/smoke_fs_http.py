"""Exercise source-runtime filesystem and HTTP against real local files/server."""
import argparse
import gzip
import json
import os
from pathlib import Path
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


class Handler(BaseHTTPRequestHandler):
    protocol_version = "HTTP/1.1"
    def handle(self):
        try:
            super().handle()
        except (ConnectionAbortedError, ConnectionResetError, BrokenPipeError):
            pass  # Expected when the timeout check disconnects the client.
    def log_message(self, *args):
        pass
    def do_GET(self):
        if self.path == "/redirect":
            self.send_response(302); self.send_header("Location", "/hello")
            self.send_header("Content-Length", "0"); self.end_headers(); return
        if self.path == "/slow":
            time.sleep(.2)
        status = 404 if self.path == "/missing" else 200
        data = b"\x00\xff\x80A" if self.path == "/binary" else "hello ไทย".encode()
        self.send_response(status)
        if self.path == "/gzip":
            data = gzip.compress(data); self.send_header("Content-Encoding", "gzip")
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("X-Repeated", "one"); self.send_header("X-Repeated", "two")
        self.send_header("Content-Length", str(len(data))); self.end_headers()
        try: self.wfile.write(data)
        except (BrokenPipeError, ConnectionResetError, ConnectionAbortedError): pass
    def do_HEAD(self):
        self.send_response(200); self.send_header("Content-Length", "999"); self.end_headers()
    def do_POST(self):
        data = self.rfile.read(int(self.headers.get("Content-Length", "0")))
        if self.path == "/binary":
            body = data
        else:
            body = json.dumps({"method": self.command, "body": data.decode(),
                               "header": self.headers.get("X-Dev", "")}).encode()
        self.send_response(201); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body)
    do_PUT = do_POST
    do_PATCH = do_POST
    do_DELETE = do_POST
    do_OPTIONS = do_POST


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bin-dir", required=True)
    parser.add_argument("--https", action="store_true", help="also verify a live HTTPS request")
    options = parser.parse_args()
    binary = Path(options.bin_dir).resolve() / ("d.exe" if os.name == "nt" else "d")
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
    url = f"http://127.0.0.1:{server.server_port}"
    count = 0
    try:
        with tempfile.TemporaryDirectory(prefix="dev-fs-http-") as temporary:
            root = Path(temporary)
            source = root / "main.dev"
            def run(code, expected=None, error=None):
                nonlocal count
                source.write_text(code, encoding="utf-8")
                for engine in ("auto", "ast"):
                    env = dict(os.environ, DEV_CC="missing-compiler")
                    result = subprocess.run([str(binary), str(source), "--engine", engine],
                        cwd=root, env=env, text=True, encoding="utf-8", capture_output=True, timeout=20)
                    if error:
                        assert result.returncode != 0 and error.lower() in result.stderr.lower(), (code, result.stdout, result.stderr)
                    else:
                        assert result.returncode == 0, (code, result.stderr)
                        if expected is not None: assert result.stdout == expected, (code, result.stdout, expected)
                    count += 1
                return result.stdout
            run('''use "std/fs"
fn main() {
    fs.create_dirs("data/nested")
    fs.write_text("data/ไทย.txt", "hello")
    fs.append_text("data/ไทย.txt", " world")
    print(fs.read_text("data/ไทย.txt"))
    print(fs.size("data/ไทย.txt"))
    print(fs.exists("data/ไทย.txt"))
    print(fs.is_file("data/ไทย.txt"))
    print(fs.is_dir("data/nested"))
    print(fs.exists("absent"))
    fs.copy("data/ไทย.txt", "data/copy.txt")
    fs.rename("data/copy.txt", "data/moved.txt")
    let files = fs.read_dir("data")
    print(files.len())
    print(files[0])
    fs.remove_file("data/moved.txt")
    let bytes = Vec<u8>()
    bytes.push(0 as u8)
    bytes.push(255 as u8)
    fs.write_bytes("data/binary", bytes)
    let read = fs.read_bytes("data/binary")
    print(read.len())
    print(read[1])
    print(fs.is_dir(fs.current_dir()))
    print(fs.is_file(fs.join("data", "binary")))
    fs.remove_file("data/binary")
    fs.remove_file("data/ไทย.txt")
    fs.remove_dir("data/nested")
    fs.remove_dir("data")
}
main()
''', "hello world\n11\ntrue\ntrue\ntrue\nfalse\n3\nmoved.txt\n2\n255\ntrue\ntrue\n")
            run('use "std/fs"\nfs.read_text("missing")', error="fs.read_text:")
            (root / "invalid-utf8").write_bytes(b"\xff")
            run('use "std/fs"\nfs.read_text("invalid-utf8")', error="UTF-8")
            run('use "std/fs"\nfs.write_bytes("bad", Vec<i64>())', error="Vec<u8>")
            assert not (root / "bad").exists()
            run('use "std/fs"\nfs.read_text(12)', error="expected str")
            for route in ("hello", "redirect", "gzip"):
                run(f'use "std/http"\nlet r = http.get("{url}/{route}")\nprint(r.status)\nprint(r.ok)\nprint(r.body)', "200\ntrue\nhello ไทย\n")
            run(f'use "std/http"\nlet r = http.get("{url}/missing")\nprint(r.status)\nprint(r.ok)', "404\nfalse\n")
            run(f'use "std/http"\nlet r = http.head("{url}/hello")\nprint(r.status)\nprint(r.bytes.len())', "200\n0\n")
            run(f'use "std/http"\nlet r = http.get("{url}/binary")\nprint(r.bytes.len())\nprint(r.bytes[1])', "4\n255\n")
            for method in ("post", "put", "patch"):
                out = run(f'use "std/http"\nlet r = http.{method}("{url}/echo", "payload")\nprint(r.status)\nprint(r.body)')
                assert out.splitlines()[0] == "201"
                assert json.loads(out.splitlines()[1])["method"] == method.upper()
            out = run(f'''use "std/http"
let headers = Map<str,str>()
headers.set("X-Dev", "custom")
let r = http.request("POST", "{url}/echo", headers, "text", 1000)
print(r.body)
''')
            assert json.loads(out)["header"] == "custom"
            run(f'''use "std/http"
let data = Vec<u8>()
data.push(255 as u8)
let r = http.request_bytes("POST", "{url}/binary", Map<str,str>(), data, 1000)
print(r.bytes[0])
''', "255\n")
            run('use "std/http"\nhttp.get("file:///private")', error="HTTP URL")
            run('use "std/http"\nhttp.get()', error="expects 1 arguments")
            run(f'use "std/http"\nhttp.request("GET", "{url}/slow", Map<str,str>(), "", 20)', error="timeout")
            run(f'use "std/http"\nhttp.request("GET", "{url}/hello", Map<str,str>(), "", 0)', error="timeout_ms")
            run(f'use "std/http"\nlet h = Map<str,str>()\nh.set("X-Test", "bad\\r\\nInjected: yes")\nhttp.request("GET", "{url}/hello", h, "", 1000)', error="invalid request/header")
            if options.https:
                run('use "std/http"\nlet r = http.get("https://example.com/")\nprint(r.status)\nprint(r.ok)', "200\ntrue\n")
        print(f"PASS: {count} filesystem/HTTP checks across auto and AST runtimes")
    finally:
        server.shutdown(); server.server_close()


if __name__ == "__main__":
    main()
