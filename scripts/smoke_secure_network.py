"""Exercise TLS and WebSocket against independent Python socket peers."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import socket
import ssl
import struct
import subprocess
import tempfile
import threading


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--bin-dir', required=True)
    args = p.parse_args()
    d = Path(args.bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-secure-') as tmp:
        root = Path(tmp)
        cert, key, ca = root / 'cert.pem', root / 'key.pem', root / 'ca.pem'
        openssl = 'C:/Program Files/OpenSSL/bin/openssl.exe' if os.name == 'nt' else 'openssl'
        def openssl_run(*arguments):
            subprocess.run([openssl, *map(str, arguments)], check=True, capture_output=True)
        openssl_run('req', '-x509', '-newkey', 'rsa:2048', '-nodes', '-keyout', root/'ca.key', '-out', ca, '-days', '1', '-subj', '/CN=Dev Test CA', '-addext', 'basicConstraints=critical,CA:TRUE', '-addext', 'keyUsage=critical,keyCertSign,cRLSign')
        openssl_run('req', '-newkey', 'rsa:2048', '-nodes', '-keyout', key, '-out', root/'cert.csr', '-subj', '/CN=localhost')
        (root/'extensions.txt').write_text('basicConstraints=critical,CA:FALSE\nsubjectAltName=DNS:localhost,IP:127.0.0.1\nkeyUsage=critical,digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\n')
        openssl_run('x509', '-req', '-in', root/'cert.csr', '-CA', ca, '-CAkey', root/'ca.key', '-CAcreateserial', '-out', cert, '-days', '1', '-extfile', root/'extensions.txt')
        server_context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
        server_context.load_cert_chain(cert, key)
        client_context = ssl.create_default_context(cafile=str(ca))
        source = root / 'main.dev'
        lit = lambda x: json.dumps(str(x), ensure_ascii=False)

        def run(code, expected, engine):
            nonlocal count
            source.write_text(code, encoding='utf-8')
            r = subprocess.run([str(d), str(source), '--engine', engine], capture_output=True, text=True, encoding='utf-8', timeout=20, env=dict(os.environ, PATH='', DEV_CC='missing'))
            assert r.returncode == 0 and r.stdout == expected, (code, r.stdout, r.stderr)
            count += 1

        def exact(s, n):
            data = b''
            while len(data) < n:
                chunk = s.recv(n - len(data))
                assert chunk, 'unexpected EOF'
                data += chunk
            return data

        def handshake(s, server):
            if not server:
                s.sendall(b'GET /echo?q=1 HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: MDEyMzQ1Njc4OWFiY2RlZg==\r\nSec-WebSocket-Version: 13\r\nOrigin: https://example.test\r\n\r\n')
            data = b''
            while not data.endswith(b'\r\n\r\n'):
                data += exact(s, 1)
                assert len(data) <= 8192
            if server:
                fields = dict(line.split(b': ', 1) for line in data.split(b'\r\n')[1:] if b': ' in line)
                accept = base64.b64encode(hashlib.sha1(fields[b'Sec-WebSocket-Key'] + b'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest())
                s.sendall(b'HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ' + accept + b'\r\n\r\n')
            else:
                assert data.startswith(b'HTTP/1.1 101'), data

        def frame(s, opcode, payload, masked):
            assert len(payload) < 126
            mask = b'abcd' if masked else b''
            output = bytes(v ^ mask[i % 4] for i, v in enumerate(payload)) if masked else payload
            s.sendall(bytes([128 | opcode, len(payload) | (128 if masked else 0)]) + mask + output)

        def receive(s, expect_mask):
            h = exact(s, 2)
            assert bool(h[1] & 128) == expect_mask
            n = h[1] & 127
            if n == 126:
                n = struct.unpack('!H', exact(s, 2))[0]
            elif n == 127:
                n = struct.unpack('!Q', exact(s, 8))[0]
            mask = exact(s, 4) if expect_mask else b''
            payload = exact(s, n)
            if mask:
                payload = bytes(v ^ mask[i % 4] for i, v in enumerate(payload))
            return h[0] & 15, payload

        for engine in ('auto', 'ast'):
            # TLS clients: trusted CA, rejected CA and rejected hostname.
            for failure in ('', 'untrusted', 'hostname'):
                listener = socket.socket()
                listener.bind(('127.0.0.1', 0))
                listener.listen()
                errors = []
                def tls_peer():
                    try:
                        raw, _ = listener.accept()
                        raw.settimeout(5)
                        try:
                            with server_context.wrap_socket(raw, server_side=True) as s:
                                if not failure:
                                    assert exact(s, 5) == b'hello'
                                    s.sendall(b'world')
                                    exact(s, 1)  # wait for Dev close_notify / EOF
                        except (ssl.SSLError, AssertionError) as e:
                            if failure or str(e) == 'unexpected EOF':
                                pass
                            else:
                                raise
                    except Exception as e:
                        errors.append(e)
                    finally:
                        listener.close()
                thread = threading.Thread(target=tls_peer)
                thread.start()
                trusted_ca = '' if failure == 'untrusted' else str(ca)
                name = 'wrong.test' if failure == 'hostname' else 'localhost'
                body = f'let options=tls.Options({lit(name)},{lit(trusted_ca)},2000)\n'
                if failure:
                    body += f'print(result.isErr(result.run(fn() {{ tls.connect({listener.getsockname()[1]},"127.0.0.1",options) }})))'
                    expected = 'true\n'
                else:
                    body += f'let s=tls.connect({listener.getsockname()[1]},"127.0.0.1",options)\ns.write(encoding.encode("hello","utf8"))\nprint(encoding.decode(s.read(5),"utf8"))\ns.close()'
                    expected = 'world\n'
                run('use "std/tls"\nuse "std/result"\nuse "std/encoding"\nfn main() {\n' + body + '\n}\nmain()\n', expected, engine)
                thread.join(8)
                assert not thread.is_alive() and not errors, errors

            # WS/WSS client protocol and masking, binary/text, ping/pong and close.
            for secure in (False, True):
                listener = socket.socket()
                listener.bind(('127.0.0.1', 0))
                listener.listen()
                port = listener.getsockname()[1]
                errors = []
                def ws_peer():
                    try:
                        s, _ = listener.accept()
                        s.settimeout(5)
                        if secure:
                            s = server_context.wrap_socket(s, server_side=True)
                        with s:
                            handshake(s, True)
                            assert receive(s, True) == (1, 'ไทย'.encode())
                            frame(s, 9, b'ping', False)
                            frame(s, 1, b'echo', False)
                            assert receive(s, True) == (10, b'ping')
                            assert receive(s, True) == (2, b'bytes')
                            frame(s, 2, b'binary', False)
                            frame(s, 8, struct.pack('!H', 1000) + b'bye', False)
                            assert receive(s, True)[0] == 8
                    except Exception as e:
                        errors.append(e)
                    finally:
                        listener.close()
                thread = threading.Thread(target=ws_peer)
                thread.start()
                run(f'''use "std/websocket"
use "std/encoding"
fn main() {{
let s=websocket.connect("{'wss' if secure else 'ws'}://localhost:{port}/echo?q=1",2000,{lit(ca) if secure else '""'})
s.sendText("ไทย")
print(s.receive().text)
s.sendBytes(encoding.encode("bytes","utf8"))
print(encoding.decode(s.receive().data,"utf8"))
let closed=s.receive()
print(closed.kind)
print(closed.code)
print(closed.text)
s.close()
}}
main()
''', 'echo\nbinary\nclose\n1000\nbye\n', engine)
                thread.join(8)
                assert not thread.is_alive() and not errors, errors

            # Reject unmasked server frames and oversized frames before payload allocation.
            for malformed in ('masked', 'oversized'):
                listener = socket.socket()
                listener.bind(('127.0.0.1', 0))
                listener.listen()
                port = listener.getsockname()[1]
                errors = []
                def bad_peer():
                    try:
                        s, _ = listener.accept()
                        s.settimeout(5)
                        with s:
                            handshake(s, True)
                            if malformed == 'masked':
                                frame(s, 1, b'bad', True)
                            else:
                                s.sendall(bytes([130, 127]) + struct.pack('!Q', 8 * 1024 * 1024 + 1))
                            s.recv(1)
                    except Exception as e:
                        errors.append(e)
                    finally:
                        listener.close()
                thread = threading.Thread(target=bad_peer)
                thread.start()
                run(f'''use "std/websocket"
use "std/result"
fn main() {{
let s=websocket.connect("ws://127.0.0.1:{port}",1000,"")
print(result.isErr(result.run(fn() {{ s.receive() }})))
result.run(fn() {{ s.close() }})
}}
main()
''', 'true\n', engine)
                thread.join(8)
                assert not thread.is_alive() and not errors, errors

            # Dev TLS/WS/WSS servers against Python clients.
            for kind in ('tls', 'ws', 'wss'):
                module = 'tls' if kind == 'tls' else 'websocket'
                creator = f'tls.createServer({lit(cert)},{lit(key)},fn(s tls.Socket) {{}})' if kind == 'tls' else (f'websocket.createSecureServer({lit(cert)},{lit(key)},fn(s websocket.Socket) {{}})' if kind == 'wss' else 'websocket.createServer(fn(s websocket.Socket) {})')
                handler = 'print(encoding.decode(s.read(5),"utf8"))\ns.write(encoding.encode("world","utf8"))' if kind == 'tls' else 'print(s.path())\nprint(s.origin())\nprint(s.receive().text)\ns.sendText("world")'
                source.write_text(f'''use "std/{module}"
use "std/timers"
use "std/encoding"
fn main() {{
let server={creator}
server.listenOn(0,"127.0.0.1")
let deadline=timers.setTimeout(fn(t timers.Timer) {{ server.close() }},8000)
server.on("connection",fn(s {module}.Socket) {{
{handler}
s.close()
server.close()
deadline.cancel()
}})
print(server.address().port)
}}
main()
''', encoding='utf-8')
                proc = subprocess.Popen([str(d), str(source), '--engine', engine], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
                try:
                    port = int(proc.stdout.readline())
                    s = socket.create_connection(('127.0.0.1', port), timeout=5)
                    if kind != 'ws':
                        s = client_context.wrap_socket(s, server_hostname='localhost')
                    with s:
                        if kind == 'tls':
                            s.sendall(b'hello')
                            assert exact(s, 5) == b'world'
                        else:
                            handshake(s, False)
                            frame(s, 1, b'hello', True)
                            assert receive(s, False) == (1, b'world')
                            assert receive(s, False)[0] == 8
                    out, err = proc.communicate(timeout=12)
                    assert proc.returncode == 0 and out == ('hello\n' if kind == 'tls' else '/echo?q=1\nhttps://example.test\nhello\n'), (out, err)
                    count += 1
                finally:
                    if proc.poll() is None:
                        proc.kill()
                        proc.communicate()
            run('''use "std/websocket"
use "std/tls"
use "std/result"
fn main() {
print(result.isErr(result.run(fn() { websocket.connect("https://localhost",1000,"") })))
print(result.isErr(result.run(fn() { websocket.connect("ws://user:pass@localhost",1000,"") })))
print(result.isErr(result.run(fn() { websocket.connect("ws://localhost/#fragment",1000,"") })))
print(result.isErr(result.run(fn() { tls.connect(0,"localhost",tls.options()) })))
print(result.isErr(result.run(fn() { websocket.connect("ws://localhost",0,"") })))
}
main()
''', 'true\n' * 5, engine)
    print(f'secure network smoke: {count} executions PASS (auto/AST, Python TLS/WS/WSS peers)')


if __name__ == '__main__':
    main()
