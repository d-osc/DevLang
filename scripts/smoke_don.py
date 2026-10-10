"""Exercise DON source execution and package resolution, without a C compiler."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--bin-dir', required=True)
    args = parser.parse_args()
    binary = Path(args.bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-don-') as temporary:
        root = Path(temporary)
        env = dict(os.environ, DEV_CC='missing-compiler', PATH='')
        def run(*args, cwd=root, error=None):
            nonlocal count
            result = subprocess.run([str(binary), *map(str,args)], cwd=cwd, env=env, capture_output=True, text=True, encoding='utf-8', timeout=30)
            if error:
                assert result.returncode != 0 and error in result.stderr, result
            else:
                assert result.returncode == 0, (args,result.stdout,result.stderr)
            count += 1
            return result.stdout
        source = root / 'main.dev'
        config = root / 'config.don'
        config.write_text("# data\nname='Dev'\nitems:[1\n2,]\n",encoding='utf-8')
        run('don','check',config)
        parsed = json.loads(run('don','to-json',config))
        assert parsed == {'name':'Dev','items':[1,2]}
        formatted = root / 'formatted.don'
        formatted.write_text(run('don','fmt',config),encoding='utf-8')
        assert json.loads(run('don','to-json',formatted)) == parsed
        json_file = root/'config.json'
        json_file.write_text(json.dumps(parsed),encoding='utf-8')
        formatted.write_text(run('don','from-json',json_file),encoding='utf-8')
        assert json.loads(run('don','to-json',formatted)) == parsed
        assert config.read_text(encoding='utf-8').startswith('# data')
        for engine in ('auto','ast'):
            source.write_text('''use "std/don"
use "std/json"
let data = don.parse("# test\\nname='Dev'\\nport:1_024\\nitems:[1\\n2,]\\nflag:true")
print(data.name)
print(data.items[1])
data.port = 8080
let copy = don.parse(don.stringify(data))
print(copy.port)
print(don.int(don.get(copy,"port")))
print(don.has(copy,"missing"))
print(don.bool(don.get(copy,"flag")))
print(don.valid("x:1\\nx:2"))
print(don.valid("x:'\\\\uD800'"))
print(don.fromJSON(don.toJSON(data)).port)
print(json.parse(don.toJSON(data)).name)
print(don.toJSON({message: "hello", numbers: [1, 2]}))
let refs = don.parse("version:'v1.0.0'\\nutils:{rev:@version}\\nport:3000\\ncopy:@port")
print(refs.utils.rev)
print(refs.copy)
''',encoding='utf-8')
            lines = run(source,'--engine',engine).splitlines()
            assert lines[:10] == ['Dev','2','8080','8080','false','true','false','false','8080','Dev'], lines
            assert json.loads(lines[10]) == {'message':'hello','numbers':[1,2]}
            assert lines[11:] == ['v1.0.0','3000'], lines
            for text,error in [("a:1\\na:2","duplicate"),("a:'unterminated","unterminated"),("a:word","quoted"),("[1 2]","separator"),("x:1__0","numeric")]:
                source.write_text('use "std/don"\ndon.parse("'+text+'")\n',encoding='utf-8')
                run(source,'--engine',engine,error=error if error!='separator' else 'between entries')
        run('new','app'); run('new','lib')
        app,lib = root/'app',root/'lib'
        assert (app/'package.don').is_file() and not (app/'dev.toml').exists()
        # Handwritten DON manifest with comments, single quotes and no root braces.
        (lib/'package.don').write_text("package:{name:'lib'\\nentry:'src/main.dev'\\nmodules:'src'}\\ndependencies:{}".replace('\\n','\n'),encoding='utf-8')
        (lib/'src/lib.dev').write_text('fn answer() i64 { return 42 }\n',encoding='utf-8')
        run('pkg','add','math','--path',lib,cwd=app)
        manifest = app/'package.don'
        content = manifest.read_text(encoding='utf-8')
        assert 'version:' in content and '0.1.0' in content
        (app/'src/main.dev').write_text('use "math/lib"\nprint(lib.answer())\n',encoding='utf-8')
        for engine in ('auto','ast'): assert run('run','--engine',engine,cwd=app)=='42\n'
        run('pkg','install','--locked',cwd=app)
        assert 'math' in run('pkg','list',cwd=app)
        run('pkg','remove','math',cwd=app)
        # Legacy TOML remains usable and DON takes precedence over an invalid TOML.
        (app/'dev.toml').write_text('invalid TOML',encoding='utf-8')
        run('pkg','list',cwd=app)
        (app/'package.don').unlink()
        (app/'dev.toml').write_text('[package]\nname="app"\nentry="src/main.dev"\nmodules="src"\n',encoding='utf-8')
        (app/'src/main.dev').write_text('print(7)\n',encoding='utf-8')
        assert run('run',cwd=app)=='7\n'
    print(f'PASS: {count} DON runtime/package checks; auto/AST and no C compiler')

if __name__ == '__main__': main()
