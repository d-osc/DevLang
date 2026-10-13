"""Compare DON objects in the source engines and hosted native executables."""
import argparse
import os
import subprocess
import tempfile
from pathlib import Path

CASES = {
    "array_search": 'fn needle(): i64 { print("needle once"); return 2 }\nfn main() {\n    const values = [1, 2, 3, 2]\n    print(values.includes(2))\n    print(values.indexOf(2))\n    print(values.lastIndexOf(2))\n    print(values.indexOf(9))\n    print(values.includes(9))\n    let slice = values[1..3]\n    print(slice.indexOf(2))\n    print(slice.lastIndexOf(3))\n    let vector = [...values]\n    print(vector.indexOf(needle()))\n    let names = ["Dev", "Other", "Dev"]\n    print(names.lastIndexOf("D" + "ev"))\n    let empty = Vec<i64>()\n    print(empty.indexOf(1))\n    print([true, false].includes(false))\n    print([1.5, 2.5].indexOf(2.5))\n}\nmain()\n',
    "array_order": """fn side(n:i64):i64 {print(n);return n}
fn make():[i64;2] {print(2);return [1,2]}
fn use_values(a:i64,xs:[i64;2],b:i64):i64 {return a+xs[0]+b}
fn main() {
  print(use_values(side(1),make(),side(3)))
  let xs=[side(4),side(5)]
  print(xs[1])
  print([6,7][0])
}
main()
""",

    "array_managed": """fn make():[Ref<i64>;2] {return [ref(1),ref(2)]}
fn copy(xs:[Ref<i64>;2]):[Ref<i64>;2] {return xs}
fn main() {
  let xs=make()
  let f=copy
  let ys=f(xs)
  print(deref(ys[1]))
  let captured=fn():[Ref<i64>;2] {return xs}
  let zs=captured()
  print(deref(zs[0]))
  let repeated=[ref(3);2]
  let vector=[...repeated,ref(4)]
  print(deref(vector[2]))
  xs=ys
  let selected=true ? xs : ys
  print(deref(selected[0]))
  let names=['A','B']
  for name in names {print(name)}
  let values=Vec<i64>();values.push(1);values.push(2)
  for value in values {print(value);values.push(9)}
  print(values.len())
  let job=spawn(fn():[i64;2] {return [5,6]})
  let result=await(job)
  print(result[1])
}
main()
""",

    "array_features": """fn repeated():i64 {print('repeat once');return 7}
fn make():[i64;3] {return [1,2,3]}
fn sum(xs:[i64;3]):i64 {xs[0]=10;return xs[0]+xs[1]+xs[2]}
fn main() {
  let xs=make()
  print(sum(xs))
  print(xs[0])
  let copy=xs
  copy[0]=99
  print(xs[0])
  let repeated_values=[repeated();3]
  print(repeated_values[2])
  let [a,b,...rest]=xs
  print(a)
  print(b)
  print(rest[0])
  let combined=[0,...xs,4,...rest]
  for value in combined { if value==2 {continue}; print(value) }
  let part=xs[1..3]
  print(part[0])
  print(part.len())
  let object={items:[5,6],first:fn():i64 {return this.items[0]}}
  print(object.first())
  let data={items:[1,2,3]}
  for item in data.items {print(item)}
}
main()
""",

    "method_object_keys": """fn key():str {print('key once');return 'name'}
fn main() {
  let person={name:'Dev',age:18,greet:fn():str {return "Hello ${this.name}"}}
  let x='name'
  print(person[x])
  person[key()]='Alice'
  print(person.greet())
  x='age'
  person[x]+=2
  print(person[x])
  const snapshot=person
  print(snapshot['name'])
}
main()
""",

    "object_methods": """use "std/json"
fn main() {
  let user={name:'Dev',age:18,
    greet:fn():str {return "Hello ${this.name}"},
    older:fn(n:i64):i64 {return this.age+n},
    message:fn():str {return this.greet()}
  }
  print(user.greet())
  print(user.older(2))
  print(user.message())
  user.name='Other'
  print(user.greet())
  print(json.stringify(user))
  let {name,age}=user
  print(name)
  print(age)
}
main()
""",

    "switch": """fn value():i64 {print('once');return 2}
fn main() {
  switch value() {
    case 1: {print('one')}
    case 2, 3: {print('two-or-three')}
    default: {print('other')}
  }
  let selected='D'+'ev'
  switch selected {
    case 'Other': {print('wrong')}
    case 'Dev': {print('name')}
  }
  switch 9 {case 1: {print('wrong')} default: {print('fallback')}}
  for i in 0..3 {
    switch i {case 1: {continue} default: {print(i)}}
  }
}
main()
""",

    "ternary": """fn side(n:i64):i64 {print(n); return n}
fn make(n:i64):Vec<i64> {let v=Vec<i64>(); v.push(n); return v}
fn main() {
  let age=18
  print(age>=18 ? 'adult' : 'child')
  print(false ? side(1) : side(2))
  print(true ? 7 : 1/0)
  print(false ? 1/0 : 9)
  print(false ? 1 : true ? 2 : 3)
  print(true || false ? 4 : 5)
  let n:i32=6
  let value=false ? n : 7
  print(value+ (1 as i32))
  let typed:i32=true ? 10 : 11
  print(typed)
  let user=true ? {name:'Dev'} : {name:'Other'}
  print(user.name)
  let vector=false ? make(1) : make(2)
  print(vector[0])
  let captured=12
  let f=fn():i64 {return true ? captured : 0}
  print(f())
  print("${true ? 'yes' : 'no'}")
}
main()
""",

    "constants": """fn main() {
  const user={name:'Dev',age:10,items:[1,2]}
  const {name, age}=user
  const {age: years}=user
  const count:i32=3
  print(name)
  print(age)
  print(years)
  print(count)
  let copy=user
  copy.age=11
  print(user.age)
  print(copy.age)
  if true { let age=20; age+=1; print(age) }
}
main()
""",

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
    "array_slice_reverse": "let x=[1,2]\nlet y=x[2..1]",
    "array_slice_end": "let x=[1,2]\nlet y=x[0..3]",
    "array_foreach_object": "let x={}\nfor item in x {print(item)}",

    "method_key_missing": "let p={name:'Dev',age:18,greet:fn() {print(this.name)}}\nprint(p['missing'])",
    "method_key_wrong_type": "let p={name:'Dev',age:18,greet:fn() {print(this.name)}}\np['age']='wrong'",

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
        const_errors = {
            'const_method_key': "const p={name:'Dev',get:fn():str {return this.name}}\np['name']='Other'",
            'this_key_mutation': "let p={name:'Dev',change:fn() {this['name']='Other'}}",
            'this_mutation': 'let x={n:1,change:fn() {this.n=2}}',
            'const_assign': 'const x=1\nx=2',
            'const_compound': 'const x=1\nx+=2',
            'const_field': 'const x={a:1}\nx.a=2',
            'const_index': 'const x={a:[1]}\nx.a[0]=2',
            'const_destructure': 'const {a}={a:1}\na=2',
            'const_rename': 'const {a:b}={a:1}\nb=2',
            'const_collection': 'const x=Vec<i64>()\nx.push(1)',
            'const_capture': 'const x=1\nlet f=fn():void { x=2 }',
        }
        ternary_errors = {
            'object_capture': 'let n=1\nlet x={get:fn():i64 {return n}}',
            'switch_default_order': 'switch 1 { default: {} case 1: {} }',
            'switch_duplicate_default': 'switch 1 { default: {} default: {} }',
            'ternary_condition': 'print(1 ? 2 : 3)',
            'ternary_type': "print(true ? 1 : 'wrong')",
        }
        for name,code in ternary_errors.items():
            source=root/(name+'.dev');source.write_text(code,encoding='utf-8')
            commands=[[str(folder/('devrun'+suffix)),str(source),'--engine',engine] for engine in ['auto','ast']]
            commands.append([str(folder/('devc'+suffix)),'build',str(source),'-o',str(root/(name+suffix))])
            for command in commands:
                result=run(command)
                assert result.returncode != 0 and ('switch' if name.startswith('switch') else 'object methods' if name.startswith('object') else 'ternary') in result.stderr,(name,result.stderr)
            print('PASS:',name)
        for name,code in const_errors.items():
            source=root/(name+'.dev');source.write_text(code,encoding='utf-8')
            commands=[[str(folder/('devrun'+suffix)),str(source),'--engine',engine] for engine in ['auto','ast']]
            commands.append([str(folder/('devc'+suffix)),'build',str(source),'-o',str(root/(name+suffix))])
            for command in commands:
                result=run(command)
                assert result.returncode != 0 and 'cannot modify const' in result.stderr,(name,result.stderr)
            print('PASS:',name)
        for name,code in {**CASES,**FAILURES}.items():
            source=root/(name+'.dev');source.write_text(code,encoding='utf-8',newline='\n')
            results=[run([str(folder/('devrun'+suffix)),str(source),'--engine',engine]) for engine in ['auto','ast']]
            exe=root/(name+suffix)
            built=run([str(folder/('devc'+suffix)),'build',str(source),'--release','-o',str(exe)])
            assert built.returncode==0,(name,'build',built.stderr)
            results.append(run([str(exe)]))
            if name in FAILURES:
                assert all(r.returncode!=0 and r.stderr.strip() for r in results),(name,[(r.stdout,r.stderr) for r in results])
                assert os.path.normcase(str(source)) in os.path.normcase(results[-1].stderr),(name,results[-1].stderr)
            else:
                assert all(r.returncode==0 for r in results),(name,[(r.stdout,r.stderr) for r in results])
                assert len({r.stdout for r in results})==1,(name,[(r.stdout,r.stderr) for r in results])
                if name == 'array_search':
                    assert results[0].stdout.splitlines()==['true','1','3','-1','false','0','1','needle once','1','2','-1','true','1'],results[0].stdout
                if name == 'array_order':
                    assert results[0].stdout.splitlines()==['1','2','3','5','4','5','5','6'],results[0].stdout
                if name == 'array_managed':
                    assert results[0].stdout.splitlines()==['2','1','4','1','A','B','1','2','4','6'],results[0].stdout
                if name == 'array_features':
                    assert results[0].stdout.splitlines()==['15','1','1','repeat once','7','1','2','3','0','1','3','4','3','2','2','5','1','2','3'],results[0].stdout
                if name == 'method_object_keys':
                    assert results[0].stdout.splitlines() == ['Dev','key once','Hello Alice','20','Alice'],results[0].stdout
                if name == 'object_methods':
                    assert results[0].stdout.splitlines() == ['Hello Dev','20','Hello Dev','Hello Other','{"age":18,"name":"Other"}','Other','18'],results[0].stdout
                if name == 'switch':
                    assert results[0].stdout.splitlines() == ['once','two-or-three','name','fallback','0','2'],results[0].stdout
                if name == 'ternary':
                    assert results[0].stdout.splitlines() == ['adult','2','2','7','9','2','4','8','10','Dev','2','12','yes'],results[0].stdout
            print('PASS:',name)
    print(f'PASS: {len(CASES)} DON object scenarios and {len(FAILURES)} errors in auto/AST/native')

if __name__=='__main__':main()
