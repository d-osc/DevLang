"""Compare DON objects in the source engines and hosted native executables."""
import argparse
import os
import subprocess
import tempfile
from pathlib import Path

CASES = {
    "destructuring": """use "std/json"
fn make():json.Value { print('once'); return {name:'test',age:10} }
fn main() {
  let user=make()
  let {name, age} = make()
  print(name)
  print(age)
  let {name: display, age: years,} = user
  print(display)
  print(years)
}
main()
""",

    "notation": r'''let name = 'Dev'
let data = {
    name = name
    greeting: "Hello ${name}"
    'display-name': 'DevLang'
    nested: { enabled = true; city: 'Bangkok' }
    array: [
        'one'
        2
        true
        null
        { a = 'nested' }
        []
    ]
    /* multiline
       comment */
    note: """first
second"""
}
print(data.name)
print(data.greeting)
print(data['display-name'])
print(data.array[4].a)
print(data.note)
print(data.nested.enabled)
''',
    "array_boundaries": '''use "std/json"
fn add(n:i64):i64 {return n}
let x={items:[
    [1]
    []
    -2
    (1
      +2)
    add(1
        +2)
    """first
second"""+' tail'
]}
print(json.stringify(x))
''',
    "json_parse": r'''use "std/json"
let text='{ "name":"Dev", "unicode":"\\u0e44\\u0e17\\u0e22", "emoji":"\\ud83d\\ude00", "items":[1,true,null], "duplicate":1, "duplicate":2 }'
let x=json.parse(text)
print(x.name)
print(x.unicode)
print(x.emoji)
print(x.duplicate)
print(json.stringify(x.items))
print(json.valid('[1,2]'))
print(json.valid('[1,]'))
print(json.valid('01'))
print(json.valid('true false'))
print(json.valid('"\\ud800"'))
print(json.valid('"\\u0000"'))
let keys=json.keys(x)
print(keys.len())
print(keys[0])
''',
    "encoding": '''use "std/json"
struct User { name:str, age:i64 }
let user=User('Dev',18)
let rows=Vec<User>()
rows.push(user)
let map=Map<str,i64>()
map.set('x',1)
let values=[1,2,3]
let x={user:user, rows:rows, map:map, values:values, reference:ref(42)}
print(json.stringify(x))
print(json.pretty({a:[1,true,null]}))
''',
    "mutations": '''use "std/json"
let data={items:[1,2], inner:{name:'Old'}, a:1}
let old=data
data.items[0]+=10
data.inner.name+=' name'
data.a='changed'
data.extra={empty:[], list:[false,null]}
let key='extra'
data[key].list[0]=true
print(json.stringify(old))
print(json.stringify(data))
data.items=[{item:1},'two',null]
print(json.stringify(data.items))
''',
    "scalar_locals": """let x={n:10, label:'Dev', f:1.5}
let n=x.n
n+=2
n=20
let label=x.label
label+='Lang'
x.f+=1
print(n)
print(label)
print(x.f+1)
print(x.n)
""",
    "scalars": '''fn twice(n: i64): i64 { return n*2 }
let x={n:21, f:1.5, flag:true, text:'Dev'}
let n: i64=x.n
let text: str=x.text
print(twice(x.n))
print(n)
print(text)
print(x.n+1)
print(1+x.n)
print(x.n>10 && x.flag)
print(!x.flag)
print(-x.n)
print(x.text+'Lang')
print("Hello ${x.text}: ${x.n}")
print(x.n as i32)
''',
    "string_fields": '''let x={name:'Dev'}
let b:u8=x.name[0]
print(b)
let c=x.name[1]
c=255
c+=1
print(c)
print(x.name[2]+1)
''',
    "unsigned": '''let big:u64=18446744073709551615
let x={n:big}
print(x.n - 9223372036854775808)
let n=x.n
n=9223372036854775808
n-=1
print(n)
n=1
print(n)
print((x.n-x.n) as u64)
print(x.n as i32)
''',
    "api": '''use "std/json"
use "std/don"
let x=json.object()
x=json.set(x,'name','Dev')
x=json.set(x,'value',json.null_value())
let xs=json.array()
xs=json.push(xs,1)
xs=json.push(xs,{a:true})
x=json.set(x,'items',xs)
print(json.has(x,'value'))
print(json.is_null(json.get(x,'value')))
print(json.kind(x.items))
print(json.len(xs))
print(json.int(json.at(xs,0)))
print(json.string(json.get(x,'name')))
print(json.value('Dev'))
print(json.stringify(json.remove(x,'value')))
print(don.toJSON(x))
print(don.stringify({name:'Dev', value:'literal ${name}', list:[1,2]}))
''',
    "evaluation_order": '''use "std/json"
fn item(n: i64): json.Value { print(n); return {value:n} }
fn key(): str { print('key'); return 'value' }
fn number(): i64 { print('rhs'); return 2 }
let x={a:item(1), b:item(2)}
print(item(3)[key()])
x.a[key()]+=number()
print(x.a.value)
print(item(4).value + item(5).value)
''',
    "modules": '''use "objects.dev" as objects
let x=objects.make('Dev')
let y=objects.copy(x)
y.inner.name='New'
print(x.inner.name)
print(y.inner.name)
print(objects.read(x))
''',
    "generics": '''fn identity<T>(value: T): T { return value }
let x={n:42, inner:{name:'Dev'}}
print(identity<i64>(x.n))
let y=identity(x)
y.inner.name='New'
print(x.inner.name)
print(y.inner.name)
''',
}
FAILURES = {
    "missing": "let x={a:1}\nprint(x.missing)",
    "bounds": "let x={a:[1]}\nprint(x.a[1])",
    "negative": "let x={a:[1]}\nprint(x.a[-1])",
    "bad_key": "let x={a:1}\nprint(x[1])",
    "missing_parent": "let x={}\nx.parent.name='Dev'",
    "wrong_parent": "let x={a:1}\nx.a.b=2",
    "division": "let x={n:1}\nx.n/=0",
    "wrong_type": "let x={n:'text'}\nlet n:i64=x.n",
    "implicit_narrow": "let x={n:1}\nlet n:i32=x.n",
    "compound_types": "let x={n:1}\nx.n+='text'",
    "scalar_local_type": "let x={n:1}\nlet n=x.n\nn='text'",
    "narrow_operand": "let x={n:1}\nx.n+=(1 as i32)",
    "unsigned_operand": "let x={n:1}\nx.n+=(1 as u64)",
    "json_invalid": "use \"std/json\"\njson.parse(\"{\")",
    "boxed_arithmetic": "use \"std/json\"\nlet x=json.value(1)\nprint(x+1)",
    "boxed_implicit": "use \"std/json\"\nlet x=json.value(1)\nlet n:i64=x",
    "boxed_interpolation": "use \"std/json\"\nlet x=json.value(1)\nprint(\"${x}\")",
    "boxed_string_interpolation": "use \"std/json\"\nlet x=json.value(\"Dev\")\nprint(\"${x}\")",
    "boxed_key": "use \"std/json\"\nlet x={a:1}\nlet key=json.value(\"a\")\nprint(x[key])",
    "boxed_key_assignment": "use \"std/json\"\nlet x={a:1}\nlet key=json.value(\"a\")\nx[key]=2",
    "string_write": "let x={name:'Dev'}\nx.name[0]=65 as u8",
    "byte_shift": "let x={name:'Dev'}\nprint(x.name[0]<<8)",
    "shift": "let x={n:1}\nprint(x.n<<64)",
}

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--bin-dir',required=True)
    folder=Path(parser.parse_args().bin_dir).resolve()
    suffix='.exe' if os.name=='nt' else ''
    def run(argv):
        return subprocess.run(argv,capture_output=True,text=True,encoding='utf-8',timeout=60)
    with tempfile.TemporaryDirectory() as tmp:
        root=Path(tmp)
        (root/'objects.dev').write_text('''use "std/json"
fn make(name:str):json.Value { return {inner:{name:name}} }
fn copy(x:json.Value):json.Value { return x }
fn read(x:json.Value):str { return x.inner.name }
''',encoding='utf-8',newline='\n')
        for name,code in {**CASES,**FAILURES}.items():
            source=root/(name+'.dev');source.write_text(code,encoding='utf-8',newline='\n')
            results=[run([str(folder/('devrun'+suffix)),str(source),'--engine',engine]) for engine in ['auto','ast']]
            exe=root/(name+suffix)
            built=run([str(folder/('devc'+suffix)),'build',str(source),'--release','-o',str(exe)])
            assert built.returncode==0,(name,'build',built.stderr)
            results.append(run([str(exe)]))
            if name in FAILURES:
                assert all(r.returncode!=0 and r.stderr.strip() for r in results),(name,[(r.stdout,r.stderr) for r in results])
                assert str(source) in results[-1].stderr,(name,results[-1].stderr)
            else:
                assert all(r.returncode==0 for r in results),(name,[(r.stdout,r.stderr) for r in results])
                assert len({r.stdout for r in results})==1,(name,[(r.stdout,r.stderr) for r in results])
            print('PASS:',name)
    print(f'PASS: {len(CASES)} DON object scenarios and {len(FAILURES)} errors in auto/AST/native')

if __name__=='__main__':main()
