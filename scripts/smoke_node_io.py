"""Validate Node-style filesystem/tasks and actual DevLang HTTP server callbacks."""
import argparse
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time
import urllib.request
import urllib.error


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True)
    d = Path(parser.parse_args().bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    with tempfile.TemporaryDirectory(prefix='dev-node-io-') as temp:
        root = Path(temp)
        source = root / 'main.dev'
        for engine in ('auto', 'ast'):
            source.write_text('''use "std/fs"
use "std/fs/promises"
fn main() {
    fs.mkdirSync("data/nested", true)
    fs.writeFileSync("data/file", "hello")
    fs.appendFileSync("data/file", " world")
    print(fs.readFileSync("data/file", "utf8"))
    print(fs.existsSync("data/file"))
    fs.copyFileSync("data/file", "data/copy")
    fs.renameSync("data/copy", "data/moved")
    print(fs.readdirSync("data").len())
    print(await(promises.readFile("data/file", "utf8")))
    await(promises.writeFile("data/file", "async"))
    await(promises.appendFile("data/file", "!"))
    print(fs.readFileSync("data/file", "utf-8"))
    await(promises.rename("data/moved", "data/renamed"))
    await(promises.unlink("data/renamed"))
    fs.unlinkSync("data/file")
    fs.rmdirSync("data/nested")
    fs.rmdirSync("data")
}
main()
''', encoding='utf-8')
            result = subprocess.run([str(d), str(source), '--engine', engine], cwd=root,
                                    capture_output=True, text=True, encoding='utf-8', timeout=15,
                                    env=dict(os.environ, DEV_CC='missing-compiler'))
            assert result.returncode == 0, result.stderr
            assert result.stdout == 'hello world\ntrue\n3\nhello world\nasync!\n', result.stdout
            with socket.socket() as reserve:
                reserve.bind(('127.0.0.1', 0))
                port = reserve.getsockname()[1]
            source.write_text('''use "std/http"
fn main() {
    let marker = "captured:"
    let server = http.createServer(fn(req http.IncomingMessage, res http.ServerResponse) {
        if req.url == "/invalid" {
            res.setHeader("X-Test", "bad\\r\\nInjected: yes")
        }
        let headers = Map<str,str>()
        headers.set("Content-Type", "text/plain; charset=utf-8")
        res.writeHead(201, headers)
        res.setHeader("X-Dev", "yes")
        res.write(marker + req.method + ":")
        res.end(req.url + ":" + req.body)
    })
    server.listenOn(PORT, "127.0.0.1")
    print("LISTEN_RETURNED")
}
main()
'''.replace('PORT', str(port)), encoding='utf-8')
            proc = subprocess.Popen([str(d), str(source), '--engine', engine], cwd=root,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                    env=dict(os.environ, DEV_CC='missing-compiler'))
            try:
                base = f'http://127.0.0.1:{port}'
                for _ in range(100):
                    if proc.poll() is not None:
                        raise AssertionError(proc.communicate())
                    try:
                        with urllib.request.urlopen(base + '/hello?x=1', timeout=1) as reply:
                            assert reply.status == 201
                            assert reply.headers['X-Dev'] == 'yes'
                            assert reply.read() == b'captured:GET:/hello?x=1:'
                        break
                    except urllib.error.URLError:
                        time.sleep(.03)
                else:
                    raise AssertionError('server did not listen')
                for method in ('POST', 'PUT', 'DELETE'):
                    request = urllib.request.Request(base+'/echo', data=b'payload', method=method)
                    with urllib.request.urlopen(request, timeout=3) as reply:
                        assert reply.read() == f'captured:{method}:/echo:payload'.encode()
                # Invalid headers never reach the wire; failing handlers terminate with a located error.
                try:
                    urllib.request.urlopen(base+'/invalid', timeout=3)
                    raise AssertionError('invalid response header accepted')
                except urllib.error.HTTPError as error:
                    assert error.code == 500
                out, err = proc.communicate(timeout=5)
                assert proc.returncode != 0 and b'invalid response header' in err, err
                assert b'LISTEN_RETURNED' in out, out
            finally:
                if proc.poll() is None:
                    proc.kill(); proc.communicate(timeout=5)
            # Replace the request listener, capture the server, then close it from the callback.
            source.write_text('''use "std/http"
fn main() {
    let server = http.createServer(fn(req http.IncomingMessage, res http.ServerResponse) { res.end("initial") })
    server.on("request", fn(req http.IncomingMessage, res http.ServerResponse) {
        res.statusCode = 202
        res.end("closed")
        server.close()
    })
    server.listenOn(PORT, "127.0.0.1")
}
main()
'''.replace('PORT', str(port)), encoding='utf-8')
            proc = subprocess.Popen([str(d), str(source), '--engine', engine], cwd=root,
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            try:
                for _ in range(100):
                    if proc.poll() is not None:
                        raise AssertionError(proc.communicate())
                    try:
                        with urllib.request.urlopen(base+'/stop', timeout=1) as reply:
                            assert reply.status == 202
                            assert reply.read() == b'closed'
                        break
                    except urllib.error.URLError:
                        time.sleep(.03)
                else:
                    raise AssertionError('close-test server did not listen')
                out, err = proc.communicate(timeout=5)
                assert proc.returncode == 0, (out, err)
            finally:
                if proc.poll() is None:
                    proc.kill(); proc.communicate(timeout=5)
            source.write_text('''use "std/http"
fn main() {
    let owner = http.createServer(fn(req http.IncomingMessage, res http.ServerResponse) { res.end("owner") })
    let task = spawn(fn() {
        let other = http.createServer(fn(req http.IncomingMessage, res http.ServerResponse) { res.end("other") })
        owner.close()
        other.close()
    })
    await(task)
}
main()
''', encoding='utf-8')
            result = subprocess.run([str(d), str(source), '--engine', engine], cwd=root,
                                    capture_output=True, text=True, encoding='utf-8', timeout=10)
            assert result.returncode != 0 and 'unknown or closed HTTP server handle' in result.stderr, result.stderr
        print('PASS: Node-style fs, async tasks and HTTP server in auto/AST engines')


if __name__ == '__main__':
    main()
