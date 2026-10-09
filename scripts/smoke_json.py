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
        def run(code, expected=None, error=None, imports=True):
            nonlocal count
            source = root / 'case.dev'
            source.write_text(('use "std/json"\n' if imports else '')+code, encoding='utf-8')
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
        run('let user={name:"Dev"}; print(user.name)', 'Dev\n', imports=False)
        run('fn main() { let user={name:"Dev"}; let job=spawn(fn() str { return user.name }); print(await(job)) }; main()', 'Dev\n')
        run('fn index() i64 { print("index"); return 0 }; let x={a:[1]}; x.a[index()]+=2; print(x.a[0])', 'index\n3\n')
        run('let user={name:"Dev", age:18, address:{city:"Bangkok"}, scores:[1,"two",true,null,{}], empty:[]}; '
            'let saved=user; user.age+=1; user.address.city="Chiang Mai"; user.scores[0]=42; '
            'user["active"]=true; user.name=23; print(saved.age); print(saved.address.city); '
            'print(user.age); print(user.address.city); print(user.scores[0]); print(user.active); '
            'print(user.name); print(user.scores[3]==null); print(json.stringify(user.empty))',
            '18\nBangkok\n19\nChiang Mai\n42\ntrue\n23\ntrue\n[]\n')
        run('let x={"first-name":"Dev",\n nested:{},\n}; x.nested.list=[1,"hello",null]; '
            'print(x["first-name"]); print(json.stringify(x.nested.list)); print(json.stringify({}))',
            'Dev\n[1,"hello",null]\n{}\n')
        run('let x=json.parse("{\\"n\\":2,\\"a\\":[true]}"); x.n*=3; print(x.n); print(x.a[0])','6\ntrue\n')
        run('fn make() json.Value { return {n:42} }; print(make().n)','42\n')
        for code,error in [
            ('let x={n:1}; print(x.missing)','missing JSON key'),
            ('let x={a:[]}; x.a[0]=1','JSON index out of bounds'),
            ('let x={}; x.missing.n=1','missing JSON key'),
            ('let x={a:[1]}; print(x.a[-1])','invalid JSON index'),
            ('let x={a:1}; print(x.a.n)','field access requires'),
            ('let x={a:1,a:2}','duplicate object literal key'),
            ('let x={a 1}','expected'),
            ('let x={a:1 b:2}','expected'),
        ]: run(code,error=error)
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
