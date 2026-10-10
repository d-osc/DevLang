"""Exercise DNS and schema-based CLI parsing in both source engines offline."""
import argparse
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import threading


def strings(items):
    return 'let input=Vec<str>()\n' + ''.join(
        'input.push(' + json.dumps(item, ensure_ascii=False) + ')\n' for item in items)


SCHEMA = '''use "std/cli"
let definitions=Vec<cli.Option>()
definitions.push(cli.Option("verbose","v",false))
definitions.push(cli.Option("quiet","q",false))
definitions.push(cli.Option("port","p",true))
definitions.push(cli.Option("name","n",true))
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True)
    args = parser.parse_args()
    d = Path(args.bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-system-') as temporary:
        root = Path(temporary)
        source = root / 'main.dev'
        env = dict(os.environ, PATH='', DEV_CC='missing-compiler')

        def run(code, expected=None, error=None, arguments=()):
            nonlocal count
            source.write_text(code, encoding='utf-8')
            results = []
            for engine in ('auto', 'ast'):
                command = [str(d), str(source), '--engine', engine]
                if arguments:
                    command += ['--', *arguments]
                result = subprocess.run(command, cwd=root, env=env, capture_output=True,
                                        text=True, encoding='utf-8', timeout=30)
                if error:
                    assert result.returncode != 0 and error in result.stderr, (code, result.stdout, result.stderr)
                    assert 'main.dev:' in result.stderr, result.stderr
                else:
                    assert result.returncode == 0, (code, result.stdout, result.stderr)
                    if expected is not None:
                        assert result.stdout == expected, (code, result.stdout, result.stderr)
                results.append(result)
                count += 1
            return results

        run(SCHEMA + strings(['--port', '3000', '-vq', 'serve', '--name=ไทย 🚀', '--', '--literal', '-']) + '''
let parsed=cli.parse(input,definitions)
print(parsed.values.get("port"))
print(parsed.values.get("name"))
print(parsed.flags.get("verbose"))
print(parsed.flags.contains("quiet"))
print(parsed.positionals.len())
print(parsed.positionals[0])
print(parsed.positionals[1])
print(parsed.positionals[2])
''', '3000\nไทย 🚀\ntrue\ntrue\n3\nserve\n--literal\n-\n')
        for tokens in (['-vp3000'], ['-v', '-p3000'], ['--verbose', '--port=3000'], ['-vp', '3000'], ['-v', '-p=3000']):
            run(SCHEMA + strings(tokens) + '''let parsed=cli.parse(input,definitions)
print(parsed.values.get("port"))
print(parsed.flags.get("verbose"))
''', '3000\ntrue\n')
        run(SCHEMA + strings(['--name=', '--port', '-1', '', '-', '--verbose']) + '''
let parsed=cli.parse(input,definitions)
print(parsed.values.get("name") == "")
print(parsed.values.get("port"))
print(parsed.positionals.len())
print(parsed.flags.contains("quiet"))
''', 'true\n-1\n2\nfalse\n')
        run(SCHEMA + strings(['--name', '--verbose']) + '''
let parsed=cli.parse(input,definitions)
print(parsed.values.get("name"))
print(parsed.flags.contains("verbose"))
''', '--verbose\nfalse\n')
        run(SCHEMA + strings([]) + '''let parsed=cli.parse(input,definitions)
print(parsed.values.len())
print(parsed.flags.len())
print(parsed.positionals.len())
''', '0\n0\n0\n')
        run(SCHEMA + '''let input=cli.args()
let parsed=cli.parse(input,definitions)
print(parsed.values.get("port"))
print(parsed.positionals[0])
print(input.len())
''', '8080\nhello world\n3\n', arguments=['--port=8080', '--', 'hello world'])
        run('use "std/cli"\nprint(cli.args().len())', '0\n')
        for tokens, error in [
            (['--unknown'], 'unknown option'), (['-z'], 'unknown option'),
            (['-vé'], 'unknown option'), (['--port'], 'requires a value'),
            (['-vp'], 'requires a value'), (['--port', '--'], 'requires a value'),
            (['--verbose=false'], 'does not accept a value'),
            (['--verbose', '-v'], 'duplicate option'), (['-vv'], 'duplicate option'),
            (['--port=1', '-p2'], 'duplicate option'), (['--=x'], 'unknown option')]:
            run(SCHEMA + strings(tokens) + 'cli.parse(input,definitions)', error=error)
        for definition, error in [
            ('cli.Option("", "", false)', 'must start with a letter'),
            ('cli.Option("-bad", "", false)', 'must start with a letter'),
            ('cli.Option("bad_name", "", false)', 'must start with a letter'),
            ('cli.Option("ไทย", "", false)', 'must start with a letter'),
            ('cli.Option("a", "ab", false)', 'one ASCII letter'),
            ('cli.Option("a", "1", false)', 'one ASCII letter'),
            ('cli.Option("a", "é", false)', 'one ASCII letter')]:
            run('use "std/cli"\nlet options=Vec<cli.Option>()\noptions.push(' + definition +
                ')\ncli.parse(Vec<str>(),options)', error=error)
        run(SCHEMA + 'definitions.push(cli.Option("port","",false))\ncli.parse(Vec<str>(),definitions)', error='duplicate CLI definition')
        run(SCHEMA + 'definitions.push(cli.Option("other","v",false))\ncli.parse(Vec<str>(),definitions)', error='duplicate CLI short')
        run('''use "std/cli"
let definitions=Vec<cli.Option>()
definitions.push(cli.Option("output-path","",true))
let input=Vec<str>()
input.push("--output-path=x")
print(cli.parse(input,definitions).values.get("output-path"))
''', 'x\n')
        run('''use "std/cli"
let input=Vec<str>()
for i in 0..4097 { input.push("x") }
cli.parse(input,Vec<cli.Option>())
''', error='4096 items')
        run('''use "std/cli"
let input=Vec<str>()
for i in 0..4096 { input.push("x") }
print(cli.parse(input,Vec<cli.Option>()).positionals.len())
''', '4096\n')
        run('use "std/cli"\nlet input=Vec<str>()\ninput.push("a\\0b")\ncli.parse(input,Vec<cli.Option>())', error='NUL')
        full_schema = 'use "std/cli"\nlet definitions=Vec<cli.Option>()\n'
        full_schema += ''.join(f'definitions.push(cli.Option("option{i}","",false))\n' for i in range(128))
        run(full_schema + strings(['--option127']) + 'print(cli.parse(input,definitions).flags.contains("option127"))', 'true\n')
        run('''use "std/cli"
let definitions=Vec<cli.Option>()
for i in 0..129 { definitions.push(cli.Option("a","",false)) }
cli.parse(Vec<str>(),definitions)
''', error='128 options')
        run('''use "std/cli"
use "std/strings"
let input=Vec<str>()
input.push(strings.repeat("a",8388608))
input.push("x")
cli.parse(input,Vec<cli.Option>())
''', error='8 MiB')

        run('''use "std/dns"
print(dns.isIP("127.0.0.1"))
print(dns.isIP("2001:db8::1"))
print(dns.isIP("localhost"))
print(dns.isIP("[::1]"))
print(dns.isIP("127.0.0.1:80"))
print(dns.isIPv4("::1"))
print(dns.isIPv6("::1"))
print(dns.isIP("127.00.0.1"))
print(dns.isIPv4("127.0.0.1"))
''', '4\n6\n0\n0\n0\nfalse\ntrue\n0\ntrue\n')
        run('''use "std/dns"
print(dns.lookup("127.0.0.1",0)[0])
print(dns.lookup("127.0.0.1",4)[0])
print(dns.lookup("127.0.0.1",6).len())
print(dns.lookup("0:0:0:0:0:0:0:1",6)[0])
print(dns.lookupOne("::1",0).family)
print(dns.lookupOne("127.0.0.1",4).address)
''', '127.0.0.1\n127.0.0.1\n0\n::1\n6\n127.0.0.1\n')
        expected = set(item[4][0] for item in socket.getaddrinfo('localhost', 0, type=socket.SOCK_STREAM))
        results = run('''use "std/dns"
let addresses=dns.lookup("localhost",0)
for i in 0..addresses.len() as i64 { print(addresses[i as usize]) }
''')
        for result in results:
            lines = result.stdout.splitlines()
            assert set(lines) == expected and len(lines) == len(set(lines)), (lines, expected)
        run('use "std/dns"\ndns.lookupOne("127.0.0.1",6)', error='no matching address')
        for family in (-1, 1, 5, 64):
            run(f'use "std/dns"\ndns.lookup("localhost",{family})', error='family must be')
        for host in ('', 'bad host', '[::1]', 'localhost:80', 'https://example.com', 'a..b', '-a', 'a-', 'a' * 64, 'a' * 254, 'ไทย'):
            run('use "std/dns"\ndns.lookup(' + json.dumps(host, ensure_ascii=False) + ',0)', error='DNS hostname')

        # The resolved address is usable by the existing TCP API, without relying
        # on the public internet or spawning an external compiler/server program.
        server = socket.socket()
        server.bind(('127.0.0.1', 0))
        server.listen()
        server.settimeout(15)
        failures = []

        def serve():
            try:
                for _ in range(2):
                    connection, _ = server.accept()
                    with connection:
                        connection.sendall(b'DNS TCP OK')
            except Exception as error:
                failures.append(error)
            finally:
                server.close()

        worker = threading.Thread(target=serve)
        worker.start()
        port = server.getsockname()[1]
        try:
            run('''use "std/dns"
use "std/net"
use "std/encoding"
fn main() {
let address=dns.lookupOne("localhost",4)
let connection=net.connect(PORT,address.address,1000)
let received=Vec<u8>()
while received.len() < 10 as usize {
    let part=connection.read(64)
    if part.len() == 0 as usize { break }
    for i in 0..part.len() as i64 { received.push(part[i as usize]) }
}
print(encoding.decode(received,"utf8"))
connection.destroy()
}
main()
'''.replace('PORT', str(port)), 'DNS TCP OK\n')
        finally:
            worker.join(timeout=16)
        assert not worker.is_alive() and not failures, failures
    print(f'PASS: {count} system-library executions across auto/AST; OS DNS and real TCP verified')


if __name__ == '__main__':
    main()
