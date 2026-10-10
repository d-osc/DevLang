"""Run real core module contracts in auto/AST without a C compiler."""
import argparse
from pathlib import Path
import os
import queue
import socket
import subprocess
import tempfile
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True)
    d = Path(parser.parse_args().bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-core-') as temporary:
        root = Path(temporary)
        source = root / 'main.dev'
        env = dict(os.environ, DEV_CC='missing-compiler', DEV_CORE_SMOKE='core')
        def run(code, expected=None, error=None):
            nonlocal count
            source.write_text(code, encoding='utf-8')
            for engine in ('auto', 'ast'):
                result = subprocess.run([str(d),str(source),'--engine',engine],cwd=root,
                                        env=env,capture_output=True,text=True,encoding='utf-8',timeout=20)
                if error:
                    assert result.returncode != 0 and error in result.stderr,(code,result.stderr)
                    assert 'main.dev:' in result.stderr,result.stderr
                else:
                    assert result.returncode == 0,(code,result.stderr)
                    if expected is not None: assert result.stdout == expected,(code,result.stdout)
                count += 1
            return result.stdout
        run('''use "std/path"
fn main() {
    print(path.basename("dir/file.txt"))
    print(path.dirname("dir/file.txt"))
    print(path.extname("dir/file.txt"))
    print(path.extname(".gitignore"))
    print(path.normalize("a/../b"))
    print(path.normalize(""))
    print(path.isAbsolute(path.resolve(".","missing")))
    print(path.relative(path.resolve(".","a"),path.resolve(".","a/b")))
    let parts = path.parse("dir/file.txt")
    print(parts.base)
    print(parts.name)
    print(path.format(parts) == path.join("dir","file.txt"))
    let names = Vec<str>()
    names.push("a")
    names.push("..")
    names.push("b")
    print(path.joinMany(names))
}
main()
''','file.txt\ndir\n.txt\n\nb\n.\ntrue\nb\nfile.txt\nfile\ntrue\nb\n')
        run('''use "std/os"
use "std/process"
use "std/fs"
use "std/module"
fn main() {
    print(os.platform() == process.platform())
    print(os.arch() == process.arch())
    print(os.availableParallelism() > 0)
    print(os.totalmem() > 0)
    print(os.freemem() >= 0)
    print(os.uptime() >= 0)
    print(fs.is_dir(os.homedir()))
    print(fs.is_dir(os.tmpdir()))
    print(process.pid() > 0)
    print(process.uptime() >= 0.0)
    print(process.hrtime() >= 0)
    print(process.getenv("DEV_CORE_SMOKE"))
    print(process.env().get("DEV_CORE_SMOKE"))
    print(process.hasEnv("DEV_CORE_SMOKE"))
    print(process.hasEnv("DEV_CORE_MISSING_VARIABLE"))
    print(fs.is_file(process.execPath()))
    print(module.isBuiltin("std/net"))
    print(module.isBuiltin("buffer"))
    print(module.isBuiltin("not-a-builtin"))
    print(module.isBuiltin("don") && module.isBuiltin("math") && module.isBuiltin("random") && module.isBuiltin("datetime") && module.isBuiltin("test") && module.isBuiltin("log"))
    print(module.loaded().len() >= 4 as usize)
    print(module.entry() == process.argv()[1])
    print(fs.is_file(module.resolve("main", module.entry())))
    let previous = process.cwd()
    fs.create_dir("cwd-test")
    process.chdir("cwd-test")
    print(pathless())
    process.chdir(previous)
    fs.remove_dir("cwd-test")
}
fn pathless() bool { return process.cwd() != "" }
main()
''',('true\n'*11)+'core\ncore\ntrue\nfalse\ntrue\ntrue\ntrue\nfalse\ntrue\ntrue\ntrue\ntrue\ntrue\n')
        run('''use "std/buffer"
fn main() {
    let b = buffer.from("hello", "utf8")
    print(b.length)
    print(b.toString("hex"))
    print(b.toString("base64"))
    print(b.toBytes()[4])
    let raw = buffer.alloc(8, 0 as u8)
    raw.writeUInt32LE(0x01020304 as u32, 0)
    raw.writeUInt16BE(0x0506 as u16, 4)
    print(raw.toString("hex"))
    print(raw.readUInt32LE(0))
    print(raw.readUInt16BE(4))
    let slice = raw.slice(0,4)
    print(slice.toString("hex"))
    let twin = buffer.from("aGVsbG8=", "base64")
    print(b.equals(twin))
    let all = Vec<buffer.Buffer>()
    all.push(b)
    all.push(twin)
    let joined = buffer.concat(all)
    print(joined.toString("utf8"))
    raw.fill(255 as u8)
    raw.copy(raw, 1, 0, 4)
    print(raw.readUInt8(1))
    print(buffer.byteLength("ไทย", "utf8"))
    b.close()
    raw.close()
    slice.close()
    twin.close()
    joined.close()
}
main()
''','5\n68656c6c6f\naGVsbG8=\n111\n0403020105060000\n16909060\n1286\n04030201\ntrue\nhellohello\n255\n9\n')
        run('use "std/buffer"\nlet b = buffer.alloc(2, 0 as u8)\nb.readUInt32LE(0)',error='out of bounds')
        run('use "std/buffer"\nbuffer.from("gg", "hex")',error='invalid hex')
        run('use "std/buffer"\nlet b = buffer.alloc(1, 0 as u8)\nb.close()\nb.toBytes()',error='closed')
        run('''use "std/events"
fn main() {
    let emitter = events.createEmitter()
    emitter.once("tick", fn(value str) { print("once:" + value); emitter.emit("tick", "nested") })
    let token = emitter.on("tick", fn(value str) { print("regular:" + value) })
    print(emitter.listenerCount("tick"))
    emitter.emit("tick", "outer")
    print(emitter.listenerCount("tick"))
    print(emitter.eventNames()[0])
    print(emitter.off("tick", token))
    print(emitter.emit("tick", "ignored"))
    emitter.close()
}
main()
''','2\nonce:outer\nregular:nested\nregular:outer\n1\ntick\ntrue\nfalse\n')
        run('use "std/events"\nlet e = events.createEmitter()\ne.emit("error", "failure")',error='unhandled error event')
        run('''use "std/url"
use "std/path"
fn main() {
    let u = url.parse("https://example.com:8443/a?q=hello#fragment")
    print(u.hostname)
    print(u.port)
    print(u.pathname)
    print(u.search)
    print(u.hash)
    print(u.toString())
    u.pathname = "/b"
    print(url.format(u))
    print(url.resolve("https://example.com/a/b", "../c").href)
    let params = url.searchParams("q=one&q=two&name=hello+world")
    print(params.getAll("q").len())
    params.set("q", "three")
    params.append("x", "a b")
    print(params.get("name"))
    print(params.toString())
    params.delete("name")
    print(params.has("name"))
    params.close()
    let absolute = path.resolve(".", "ไทย space#.txt")
    print(url.fileURLToPath(url.pathToFileURL(absolute)) == absolute)
    print(url.domainToASCII("bücher.de"))
}
main()
''','example.com\n8443\n/a\n?q=hello\n#fragment\nhttps://example.com:8443/a?q=hello#fragment\nhttps://example.com:8443/b?q=hello#fragment\nhttps://example.com/c\n2\nhello world\nq=three&name=hello+world&x=a+b\nfalse\ntrue\nxn--bcher-kva.de\n')
        run('use "std/url"\nurl.parse("not a URL")',error='relative URL')
        (root/'source.bin').write_bytes(b'\x00\xffhello')
        run('''use "std/stream"
use "std/fs"
fn main() {
    let reader = stream.createReadStream("source.bin")
    let bytes = reader.read(2)
    print(bytes[1])
    reader.close()
    let input = stream.createReadStream("source.bin")
    let output = stream.createWriteStream("copy.bin", false)
    print(input.pipe(output))
    print(fs.read_bytes("copy.bin").len())
}
main()
''','255\n7\n7\n')
        assert (root/'copy.bin').read_bytes()==(root/'source.bin').read_bytes()
        large=b'x'*(9*1024*1024)
        (root/'large.bin').write_bytes(large)
        run('use "std/stream"\nlet reader = stream.createReadStream("large.bin")\nlet writer = stream.createWriteStream("large-copy.bin", false)\nprint(reader.pipe(writer))',str(len(large))+'\n')
        assert (root/'large-copy.bin').read_bytes()==large
        run('use "std/stream"\nlet r = stream.createReadStream("source.bin")\nr.close()\nr.read(1)',error='closed')

        tcp=socket.socket();tcp.bind(('127.0.0.1',0));tcp.listen();tcp.settimeout(10)
        udp=socket.socket(socket.AF_INET,socket.SOCK_DGRAM);udp.bind(('127.0.0.1',0));udp.settimeout(10)
        def tcp_echo():
            try:
                for _ in range(2):
                    connection,_=tcp.accept()
                    with connection:
                        data=connection.recv(512);connection.sendall(data)
            finally: tcp.close()
        def udp_echo():
            try:
                for _ in range(2):
                    data,address=udp.recvfrom(65536);udp.sendto(data,address)
            finally: udp.close()
        tcp_port=tcp.getsockname()[1];udp_port=udp.getsockname()[1]
        threads=[threading.Thread(target=tcp_echo,daemon=True),threading.Thread(target=udp_echo,daemon=True)]
        for thread in threads:thread.start()
        run('''use "std/net"
use "std/buffer"
fn main() {
    print(net.isIP("127.0.0.1"))
    print(net.isIPv6("::1"))
    print(net.isIP("invalid"))
    let socket = net.connect(PORT,"127.0.0.1",1000)
    socket.setNoDelay(true)
    socket.setTimeout(1000)
    let message = buffer.from("hello","utf8")
    print(socket.write(message.toBytes()))
    let received = Vec<u8>()
    while received.len() < 5 as usize {
        let part = socket.read(5)
        if part.len() == 0 as usize { break }
        for i in 0..(part.len() as i64) { received.push(part[i]) }
    }
    let reply = buffer.fromBytes(received)
    print(reply.toString("utf8"))
    print(socket.remoteAddress().port == PORT)
    socket.destroy()
    message.close()
    reply.close()
}
main()
'''.replace('PORT',str(tcp_port)),'4\ntrue\n0\n5\nhello\ntrue\n')
        run('''use "std/dgram"
use "std/buffer"
fn main() {
    let socket = dgram.createSocket("udp4")
    socket.bind(0,"127.0.0.1")
    socket.setBroadcast(false)
    let message = buffer.from("datagram","utf8")
    print(socket.send(message.toBytes(),PORT,"127.0.0.1"))
    let received = socket.recv(512,1000)
    let reply = buffer.fromBytes(received.data)
    print(reply.toString("utf8"))
    print(received.port == PORT)
    print(socket.address().port > 0)
    message.close()
    reply.close()
    socket.close()
}
main()
'''.replace('PORT',str(udp_port)),'8\ndatagram\ntrue\ntrue\n')
        for thread in threads:thread.join(timeout=10);assert not thread.is_alive()
        run('use "std/net"\nnet.connect(65536,"127.0.0.1",100)',error='port must')
        run('use "std/dgram"\ndgram.createSocket("invalid")',error='udp4 or udp6')
        run('use "std/dgram"\nlet s = dgram.createSocket("udp4")\ns.bind(0,"127.0.0.1")\ns.recv(32,20)',error='std/dgram')
        run('''use "std/net"
use "std/buffer"
use "std/io"
fn main() {
    let server = net.createServer(fn(socket net.Socket) {
        socket.on("data", fn(bytes Vec<u8>) { socket.write(bytes) })
    })
    server.listenOn(0,"127.0.0.1")
    let client = net.connect(server.address().port,"127.0.0.1",1000)
    client.on("data", fn(bytes Vec<u8>) {
        let reply = buffer.fromBytes(bytes)
        io.write(reply.toString("utf8"))
        reply.close()
    })
    client.on("end", fn(bytes Vec<u8>) {
        print("")
        client.destroy()
        server.close()
    })
    let message = buffer.from("self-echo","utf8")
    client.end(message.toBytes())
    message.close()
}
main()
''','self-echo\n')
        # Callback servers choose a free port and close themselves after one real packet.
        for engine in ('auto','ast'):
            for kind in ('tcp','udp'):
                code = '''use "std/net"
fn main() {
    let server = net.createServer(fn(socket net.Socket) {})
    server.on("connection", fn(socket net.Socket) {
        socket.on("data", fn(bytes Vec<u8>) { socket.write(bytes); socket.destroy(); server.close() })
    })
    server.listenOn(0,"127.0.0.1")
    print(server.address().port)
}
main()
''' if kind=='tcp' else '''use "std/dgram"
fn main() {
    let socket = dgram.createSocket("udp4")
    socket.bind(0,"127.0.0.1")
    socket.on("message", fn(message dgram.Message) {
        socket.send(message.data,message.port,message.address)
        socket.close()
    })
    print(socket.address().port)
}
main()
'''
                source.write_text(code,encoding='utf-8')
                proc=subprocess.Popen([str(d),str(source),'--engine',engine],cwd=root,env=env,
                                      stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
                ready=queue.Queue()
                threading.Thread(target=lambda:ready.put(proc.stdout.readline()),daemon=True).start()
                try:
                    line=ready.get(timeout=10)
                    assert line.strip().isdigit(),(line,proc.communicate(timeout=5))
                    port=int(line)
                    if kind=='tcp':
                        with socket.create_connection(('127.0.0.1',port),timeout=3) as client:
                            client.sendall(b'hello');assert client.recv(512)==b'hello'
                    else:
                        with socket.socket(socket.AF_INET,socket.SOCK_DGRAM) as client:
                            client.settimeout(3);client.sendto(b'hello',('127.0.0.1',port));assert client.recvfrom(512)[0]==b'hello'
                    out,err=proc.communicate(timeout=5);assert proc.returncode==0,(out,err)
                    count+=1
                finally:
                    if proc.poll() is None:proc.kill();proc.communicate(timeout=5)
        print(f'PASS: {count} core module executions including real TCP/UDP clients and callback servers')


if __name__=='__main__':main()
