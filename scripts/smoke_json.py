"""JSON contracts in both source engines, without a C compiler."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--runtime', required=True)
    binary = str(Path(parser.parse_args().runtime).resolve())
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-json-') as temporary:
        root = Path(temporary)
        env = dict(os.environ, PATH='', DEV_CC='missing-compiler')
        def run(code, expected=None, error=None):
            nonlocal count
            source = root / 'case.dev'
            source.write_text('use "std/json"\n'+code, encoding='utf-8')
            for engine in ('auto', 'ast'):
                result = subprocess.run([binary, str(source), '--engine', engine], cwd=root, env=env,
                                        capture_output=True, text=True, encoding='utf-8', timeout=10)
                if error:
                    assert result.returncode != 0 and error in result.stderr, result.stderr
                    assert 'case.dev:' in result.stderr, result.stderr
                else:
                    assert result.returncode == 0 and result.stdout == expected, (result.stdout,result.stderr,expected)
                count += 1
        def literal(text): return json.dumps(text, ensure_ascii=False)
        run('fn read(v json.Value) i64 { return json.int(json.get(v,"n")) }\n'
            'let v=json.parse("{\\"n\\":42}"); print(read(v))','42\n')
        for text in ('null','true','false','42','-9223372036854775808','18446744073709551615',
                     '123456789012345678901234567890','1e400','[1,true,null]', '{"a":[1,2]}', '"ภาษาไทย 😀"'):
            run(f'let v=json.parse({literal(text)}); print(json.valid(json.stringify(v))); print(json.stringify(v))',
                'true\n'+('1e+400' if text == '1e400' else text)+'\n')
        run('let x=json.object(); let before=x; x=json.set(x,"a",42); print(json.has(before,"a")); print(json.int(json.get(x,"a"))); x=json.remove(x,"a"); print(json.len(x))','false\n42\n0\n')
        run('let x=json.array(); x=json.push(x,true); print(json.bool(json.at(x,0))); print(json.kind(x)); print(json.len(x))','true\narray\n1\n')
        run('let m=Map<str,i64>(); m.set("answer",42); print(json.stringify(m)); print(json.stringify(Vec<i64>(19,23))); struct Point { x i64; y i64 }; print(json.stringify(Point(19,23)))',
            '{"answer":42}\n[19,23]\n{"x":19,"y":23}\n')
        run('let v=json.parse("{\\"b\\":2,\\"a\\":1}"); let keys=json.keys(v); print(keys[0]); print(keys[1]); print(json.uint(json.parse("18446744073709551615"))); print(json.float(json.parse("1.5"))); print(json.is_null(json.null_value()))',
            'a\nb\n18446744073709551615\n1.5\ntrue\n')
        run('let v=json.parse('+literal('"\\uD83D\\uDE00"')+'); print(json.string(v))','😀\n')
        run('let v=json.parse("{\\"a\\":1,\\"a\\":2}"); print(json.stringify(v))','{"a":2}\n')
        run('fn main() { let v=json.parse("[42]"); let xs=Vec<json.Value>(v); print(json.int(json.at(xs[0],0))); let job=spawn(fn() json.Value { return v }); print(json.stringify(await(job))) }\nmain()','42\n[42]\n')
        for text in ('', '{', '[1,]', '01', 'NaN', 'true false', '"\\uD800"', '"a\nb"'):
            run(f'print(json.valid({literal(text)}))','false\n')
            run(f'json.parse({literal(text)})',error='invalid JSON')
        for code,error in [
            ('json.int(json.parse("1.5"))','expected JSON i64'),
            ('json.int(json.parse("18446744073709551615"))','expected JSON i64'),
            ('json.uint(json.parse("-1"))','expected JSON u64'),
            ('json.float(json.parse("1e400"))','expected finite JSON number'),
            ('json.get(json.object(),"missing")','missing JSON key'),
            ('json.at(json.array(),-1)','invalid JSON index'),
            ('json.at(json.array(),0)','JSON index out of bounds'),
            ('json.get(json.array(),"x")','requires object'),
            ('json.bool(json.parse("1"))','expected JSON bool'),
            ('json.set(json.array(),"x",1)','requires object'),
            ('json.push(json.object(),1)','requires array'),
            ('json.stringify(1.0 / 0.0)','must be finite'),
            ('json.len(json.parse("42"))','requires object or array'),
            ('json.keys(json.array())','requires object'),
            ('json.parse()','expects 1 arguments'),
            ('let m=Map<i64,i64>(); m.set(1,2); json.stringify(m)','expected str'),
            ('let v=json.array(); for i in 0..140 { let p=json.array(); v=json.push(p,v) }','nesting limit'),
        ]: run(code,error=error)
    print(f'JSON contracts passed: {count} engine executions')

if __name__ == '__main__': main()
