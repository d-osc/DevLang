"""Compare DON objects in the source engines and hosted native executables."""
import argparse
import os
import subprocess
import tempfile
from pathlib import Path

CASES = {
    "dynamic_array_methods": 'use "std/json"\nfn main(){\n let xs=new Array("Dev",42,true,null,0.1)\n print(xs.includes(42))\n print(xs.indexOf("Dev"))\n print(xs.lastIndexOf(null))\n print(xs.join("|"))\n print(xs.toString())\n let sorted=new Array(10,2,1).toSorted()\n print(sorted.toString())\n let ns=new Array(10,2,1)\n ns.sort(fn(a:json.Value,b:json.Value):i64{return json.int(a)-json.int(b)})\n print(ns.toString())\n let nested=new Array([1,[2,3]],[],[4])\n print(json.stringify(nested.flat()))\n print(nested.flat().toString())\n print(json.stringify(nested.flat(2)))\n print(nested.flat(2).toString())\n let p=new Object({name:"Dev"})\n let os=new Array(p,p,new Object({name:"Other"}))\n print(os.indexOf(p))\n print(os.lastIndexOf(p))\n print(os.includes(new Object({name:"Dev"})))\n print(os.join(","))\n Object.defineProperty(p,"toString",{value:fn(this:json.Value):str{return this.name},configurable:true})\n print(os.join(","))\n let nestedObjects=new Array([p])\n let flatObjects=nestedObjects.flat()\n print(flatObjects.includes(p))\n print(flatObjects.join(","))\n}\nmain()\n',
    "math_order": 'fn side(n:i64):i64 {print(n);return n}\nfn main(){print(Math.max(side(1),side(2),side(3)));print(Math.min(side(4),side(5)))}\nmain()\n',
    "math_reference": 'fn main(){\n print(Math.abs((Math.abs(-3))-(3.0))<0.000000000001) // abs\n print(Math.abs((Math.acos(1))-(0.0))<0.000000000001) // acos\n print(Math.abs((Math.acosh(1))-(0.0))<0.000000000001) // acosh\n print(Math.abs((Math.asin(0))-(0.0))<0.000000000001) // asin\n print(Math.abs((Math.asinh(0))-(0.0))<0.000000000001) // asinh\n print(Math.abs((Math.atan(0))-(0.0))<0.000000000001) // atan\n print(Math.abs((Math.atan2(1,0))-(Math.PI/2))<0.000000000001) // atan2\n print(Math.abs((Math.atanh(0))-(0.0))<0.000000000001) // atanh\n print(Math.abs((Math.cbrt(27))-(3.0))<0.000000000001) // cbrt\n print(Math.abs((Math.ceil(1.1))-(2.0))<0.000000000001) // ceil\n print(Math.abs((Math.clz32(1))-(31.0))<0.000000000001) // clz32\n print(Math.abs((Math.cos(0))-(1.0))<0.000000000001) // cos\n print(Math.abs((Math.cosh(0))-(1.0))<0.000000000001) // cosh\n print(Math.abs((Math.exp(0))-(1.0))<0.000000000001) // exp\n print(Math.abs((Math.expm1(0))-(0.0))<0.000000000001) // expm1\n print(Math.abs((Math.f16round(1.0004882812500002))-(1.0009765625))<0.000000000001) // f16round\n print(Math.abs((Math.floor(-1.1))-(-2.0))<0.000000000001) // floor\n print(Math.abs((Math.fround(16777217))-(16777216.0))<0.000000000001) // fround\n print(Math.abs((Math.log(Math.E))-(1.0))<0.000000000001) // log\n print(Math.abs((Math.log10(100))-(2.0))<0.000000000001) // log10\n print(Math.abs((Math.log1p(0))-(0.0))<0.000000000001) // log1p\n print(Math.abs((Math.log2(8))-(3.0))<0.000000000001) // log2\n print(Math.abs((Math.max(1,8,-2))-(8.0))<0.000000000001) // max\n print(Math.abs((Math.min(1,8,-2))-(-2.0))<0.000000000001) // min\n print(Math.abs((Math.pow(2,10))-(1024.0))<0.000000000001) // pow\n print(Math.abs((Math.round(-1.5))-(-1.0))<0.000000000001) // round\n print(Math.abs((Math.sign(-2))-(-1.0))<0.000000000001) // sign\n print(Math.abs((Math.sin(Math.PI/2))-(1.0))<0.000000000001) // sin\n print(Math.abs((Math.sinh(0))-(0.0))<0.000000000001) // sinh\n print(Math.abs((Math.sqrt(9))-(3.0))<0.000000000001) // sqrt\n print(Math.abs((Math.tan(0))-(0.0))<0.000000000001) // tan\n print(Math.abs((Math.tanh(0))-(0.0))<0.000000000001) // tanh\n print(Math.abs((Math.trunc(-1.9))-(-1.0))<0.000000000001) // trunc\n\n print(Math.clz32(-1)==0.0)\n print(Math.clz32(4294967296)==32.0)\n print(Math.clz32(-4294967295)==31.0)\n print(Math.f16round(1.00048828125)==1.0)\n print(Math.f16round(1.00146484375)==1.001953125)\n print(Math.f16round(0.0000000298023223876953125)==0.0)\n print(Math.f16round(0.00000008940696716308594)==0.00000011920928955078125)\n print(Math.f16round(65519)==65504.0)\n print(Math.f16round(65520)>65504.0)\n print(Math.atan2(Math.round(-0.5),-1)<0.0)\n print(Math.atan2(Math.min(0.0,-0.0),-1)<0.0)\n print(Math.atan2(Math.max(0.0,-0.0),-1)>0.0)\n print(Math.min()>0.0)\n print(Math.max()<0.0)\n let nan=Math.sqrt(-1)\n print(nan!=nan)\n let inf=Math.log(0)\n print(inf<0.0)\n let odd=Math.pow(-1,Math.exp(1000))\n print(odd!=odd)\n let r=Math.random()\n print(r>=0.0&&r<1.0)\n print(Math.LN2==Math.log(2))\n print(Math.LN10==Math.log(10))\n print(Math.abs(Math.SQRT2*Math.SQRT1_2-1.0)<0.000000000001)\n let power=Math.pow(1,nan)\n print(power!=power)\n print(Math.pow(nan,0)==1.0)\n print(Math.atan2(Math.sign(-0.0),-1)<0.0)\n let minimum=Math.min(1,nan)\n print(minimum!=minimum)\n let maximum=Math.max(1,nan)\n print(maximum!=maximum)\n print(Math.abs(Math.expm1(0.0000000000000001)-0.0000000000000001)<0.000000000000000000000000000001)\n}\nmain()\n',
    "array_constructor": 'use "std/json"\nfn main(){\n let empty=new Array()\n empty.push("Dev")\n empty.unshift(42)\n print(json.stringify(empty))\n print(empty.length)\n let slots=new Array(3)\n print(json.stringify(slots))\n slots[0]=1\n slots[1]="Hi"\n slots[2]=true\n print(json.stringify(slots))\n print(json.stringify(slots.pop()))\n let mixed=new Array("Dev",42,true,null,{name:"test"})\n print(json.stringify(mixed))\n print(Array.isArray(mixed))\n mixed.reverse()\n print(json.stringify(mixed))\n print(json.stringify(mixed.toSpliced(1,2,"new")))\n print(json.stringify(new Array("single")))\n print(new Array(0).length)\n print(new Array(2.0).length)\n}\nmain()\n',
    "array_constructor_boxed": 'use "std/json"\nfn main(){let xs=new Array(json.value(3));print(json.stringify(xs))}\nmain()\n',
    "new_constructors": 'use "std/json"\nfn Person(this: json.Value,name: str){this.name = name}\nfn main(){\n    let empty = new Object()\n    empty.name = "Dev"\n    print(empty.name)\n    let data = new Object({name:"Literal"})\n    print(data.name)\n    Person.prototype.greet = fn(this: json.Value): str{return this.name}\n    let p = new Person("Constructor")\n    print(p.greet())\n    print(p.constructor==Person)\n    print(new Person("Inline").greet())\n    let C = Person\n    print(new C("Alias").name)\n}\nmain()\n',
    "prototype_alias": 'use "std/json"\nfn main(){let O=Object;let o=O();let offset=3;O.defineProperty(o,"n",{get:fn(this:json.Value):i64{return offset}});print(o.n)}\nmain()\n',
    "prototype_direct": 'use "std/json"\nfn Person(this:json.Value,name:str){this.name=name}\nfn main(){\n Person.prototype.greet=fn(this:json.Value):str{return this.name}\n let p=Object.construct(Person,"Dev")\n print(p.greet())\n Object.prototype.common="root"\n print(p.common)\n Person.prototype=Object({newValue:7})\n let q=Object.construct(Person,"Next")\n print(q.newValue)\n}\nmain()\n',
    "prototype": 'use "std/json"\nfn Person(this: json.Value, name: str) { this.name = name }\nfn main() {\n    Object.defineProperty(Person.prototype, "greet", {\n        value: fn(this: json.Value): str { return "Hello " + this.name }\n        writable: true\n        configurable: true\n    })\n    let person = Object.construct(Person, "Dev")\n    print(person.greet())\n    print(person.constructor == Person)\n    Object.assign(person, {age: 18})\n    print(json.stringify(person))\n    Object.defineProperty(Person.prototype, "label", {value: "prototype", writable: true, configurable: true})\n    print(person.label)\n    Object.defineProperty(Person.prototype, "label", {value: "updated"})\n    print(person.label)\n    print(Object.keys(person).join(","))\n    print(Object.getOwnPropertyDescriptor(person,"name").writable)\n    let alias = person\n    Object.freeze(person)\n    print(Object.isFrozen(alias))\n    print(Object.delete(alias,"age"))\n    let entries = Object.entries(person)\n    print(json.stringify(Object.fromEntries(entries)))\n    let groups = Object.groupBy([1,2,3,4],fn(n:i64):str {return n % 2 == 0 ? "even" : "odd"})\n    print(json.stringify(groups))\n    print(person.toString())\n}\nmain()\n',
    "prototype_descriptors": 'use "std/json"\nfn main() {\n let obj=Object({count:1})\n let offset=10\n Object.defineProperties(obj,{\n   total:{get:fn(this:json.Value):i64 {return this.count+offset},set:fn(this:json.Value,n:i64){this.count=n-offset},enumerable:true,configurable:true}\n   hidden:{value:"secret"}\n })\n print(obj.total)\n obj.total=25\n print(obj.count)\n print(Object.keys(obj).join(","))\n print(Object.getOwnPropertyNames(obj).join(","))\n print(json.stringify(obj))\n print(Object.isExtensible(obj))\n Object.seal(obj)\n print(Object.isSealed(obj))\n print(Object.isFrozen(obj))\n print(Object.delete(obj,"count"))\n obj.count=7\n print(obj.total)\n print(Object.valueOf(obj)==obj)\n let child=Object.create(obj)\n print(child.total)\n print(Object.keys(child).length)\n let pairs=Object.entries(obj)\n print(pairs[0][0])\n print(pairs[0].length)\n}\nmain()\n',
    "array_basic_managed": 'fn main() {\n    let xs=Vec<Ref<i64>>(ref(1),ref(2),ref(3),ref(4))\n    let snapshot=xs.slice(0,4)\n    xs.copyWithin(1,0,3)\n    for n in xs {print(deref(n))}\n    let removed=xs.splice(1,2,xs.at(3))\n    for n in removed {print(deref(n))}\n    print(deref(xs.delete(0)))\n    for n in snapshot {print(deref(n))}\n    let nested=Vec<Vec<Vec<Ref<i64>>>>(Vec<Vec<Ref<i64>>>(xs,removed))\n    let flat=nested.flat(2)\n    for n in flat {print(deref(n))}\n    let changed=[1,2,3].toSpliced(1,1,9)\n    print(changed.toString())\n    print(snapshot[1..3].toSpliced(0,1,ref(9)).map(fn(n:Ref<i64>):i64 {return deref(n)}).toString())\n    let dense=Vec<i64>(1,2,3,4)\n    dense.copyWithin(-2,0,2)\n    print(dense.toString())\n    print(dense.splice(2).toString())\n    print(dense.toString())\n    print(dense.splice(-9223372036854775808,9223372036854775807,8).toString())\n    print(dense.toString())\n    let empty=Vec<i64>()\n    print(empty.toString())\n    print(empty.toSpliced(100,4).length)\n}\nmain()\n',
    "array_basic": 'fn main() {\n    const values = [1, 2, 3, 4]\n    print(values.length)\n    print(values.length())\n    print(values.toString())\n    print(Array.isArray(values))\n    print(Array.isArray(42))\n    let xs = values.toVec()\n    let snapshot = xs.slice(0, 4)\n    let removed = xs.splice(1, 2, 9, 8, 7)\n    print(removed.toString())\n    print(xs.toString())\n    print(snapshot.toString())\n    let changed = xs.toSpliced(-2, 1, 6)\n    print(changed.toString())\n    print(xs.toString())\n    print(xs.delete(-1))\n    xs.copyWithin(1, 0, 3)\n    print(xs.toString())\n    let nested = Vec<Vec<i64>>(Vec<i64>(1, 2), Vec<i64>(3, 4))\n    print(nested.flat().toString())\n    print(nested.flat(0).length)\n    print([true, false].toString())\n    print(["Hello", "Dev"].toString())\n    let data = { items: [1, "Dev", true, null], missing: null }\n    print(Array.isArray(data.items))\n    print(Array.isArray(data))\n    print(Array.isArray(data.missing))\n}\nmain()\n',
    "array_index": 'fn main() {\n    const numbers = [10, 20, 30]\n    numbers.forEach(fn(value: i64, index: i64) {\n        print("${index}: ${value}")\n    })\n    let mapped = numbers.map(fn(value: i64, index: i64): i64 { return value + index })\n    print(mapped.at(2))\n    let selected = numbers.filter(fn(value: i64, index: i64): bool { return index % 2 == 0 })\n    print(selected.len())\n    print(numbers.some(fn(value: i64, index: i64): bool { return index == 1 }))\n    print(numbers.every(fn(value: i64, index: i64): bool { return value > index }))\n    print(numbers.findIndex(fn(value: i64, index: i64): bool { return index == 1 }))\n    print(numbers.findLastIndex(fn(value: i64, index: i64): bool { return index > 0 }))\n    print(numbers.reduce(fn(total: i64, value: i64, index: i64): i64 { return total + value + index }, 0))\n    numbers[1..3].forEach(fn(value: i64, index: i64) { print("slice ${index}: ${value}") })\n    numbers.forEach(fn(value: i64) { print(value) })\n}\nmain()\n',
    "array_sort_target": 'fn index():i64 {print("index once");return 0}\nfn main() {\n    let nested=Vec<Vec<i64>>(Vec<i64>(3,1,2))\n    nested[index()].sort()\n    print(nested[0].at(0))\n}\nmain()\n',
    "array_sort": 'fn main() {\n    const values = [10, 2, 1, 2, -3]\n    let sorted = values.toSorted()\n    for n in sorted { print(n) }\n    print(values.at(0))\n    let descending = values.toSorted(fn(a: i64, b: i64): i64 {\n        return a < b ? 1 : (a > b ? -1 : 0)\n    })\n    print(descending.at(0))\n    let queue = values.toVec()\n    let snapshot = queue.slice(0, 5)\n    queue.sort()\n    print(queue.at(0))\n    print(snapshot.at(0))\n    let words = ["zebra", "apple", "banana"].toSorted()\n    print(words.join(","))\n    let view = values[1..4].toSorted()\n    print(view.at(0))\n    let empty = Vec<i64>()\n    empty.sort()\n    print(empty.len())\n    print([7].toSorted().at(0))\n}\nmain()\n',
    "array_sort_managed": 'fn main() {\n    let xs=Vec<Ref<i64>>(ref(21),ref(10),ref(22),ref(11),ref(23))\n    let snapshot=xs.slice(0,5)\n    let divisor=ref(10)\n    xs.sort(fn(a:Ref<i64>,b:Ref<i64>):i64 {return deref(a)/deref(divisor)-deref(b)/deref(divisor)})\n    for n in xs {print(deref(n))}\n    print(deref(snapshot.at(0)))\n    let zero=xs.toSorted(fn(a:Ref<i64>,b:Ref<i64>):i64 {return 0})\n    for n in zero {print(deref(n))}\n    let asc=Vec<i64>(9223372036854775807,-9223372036854775808,0)\n    asc.sort()\n    print(asc.at(0))\n    print(asc.at(-1))\n    let floats=Vec<f64>(2.5,-1.5,0.0)\n    floats.sort()\n    print(floats.at(0))\n}\nmain()\n',
    "array_callbacks_order": 'fn receive():[i64;3] {print("receiver");return [1,2,3]}\nfn reducer():fn(i64,i64):i64 {print("callback");return fn(a:i64,b:i64):i64 {print(b);return a+b}}\nfn seed():i64 {print("initial");return 10}\nfn predicate(n:i64):bool {print(n);return n==2}\nfn main() {\n    print(receive().reduce(reducer(),seed()))\n    let xs=[1,2,3]\n    print(xs.some(predicate))\n    print(xs.every(predicate))\n    print(xs.findIndex(predicate))\n    print(xs.findLastIndex(predicate))\n}\nmain()\n',
    "array_methods": 'fn main() {\n    const values = [1, 2, 3, 2]\n    print(values.at(-1))\n    let doubled = values.map(fn(n: i64): i64 { return n * 2 })\n    print(doubled.at(0))\n    let even = values.filter(fn(n: i64): bool { return n % 2 == 0 })\n    print(even.len())\n    print(values.reduce(fn(sum: i64, n: i64): i64 { return sum + n }, 0))\n    print(values.some(fn(n: i64): bool { return n > 2 }))\n    print(values.every(fn(n: i64): bool { return n > 0 }))\n    print(values.findIndex(fn(n: i64): bool { return n == 2 }))\n    print(values.findLastIndex(fn(n: i64): bool { return n == 2 }))\n    values[1..3].forEach(fn(n: i64) { print(n) })\n    let combined = values.concat([4, 5])\n    print(combined.at(-1))\n    let reversed = values.toReversed()\n    print(reversed.at(0))\n    let queue = values.toVec()\n    queue.unshift(0)\n    print(queue.shift())\n    queue.reverse()\n    print(queue.at(0))\n    print(values.at(0))\n    const words = ["Hello", "Dev"]\n    print(words.join(" "))\n    print(words[1..2].join("-"))\n    let empty = Vec<i64>()\n    print(empty.every(fn(n: i64): bool { return n > 0 }))\n    print(empty.some(fn(n: i64): bool { return n > 0 }))\n    print(empty.findIndex(fn(n: i64): bool { return true }))\n    print(empty.reduce(fn(a: i64, b: i64): i64 { return a + b }, 10))\n    let multiplier = 3\n    print(values.map(fn(n: i64): i64 { return n * multiplier }).at(0))\n}\nmain()\n',
    "array_methods_managed": 'fn main() {\n    let refs=[ref(1),ref(2)]\n    let xs=refs.toVec()\n    let snapshot=xs.slice(0,2)\n    xs.unshift(ref(3))\n    print(deref(xs.shift()))\n    xs.reverse()\n    print(deref(xs.at(0)))\n    print(deref(snapshot.at(0)))\n    let more=xs.concat(refs)\n    print(deref(more.at(-1)))\n    let offset=ref(10)\n    let mapped=refs.map(fn(x:Ref<i64>):Ref<i64> {return ref(deref(x)+deref(offset))})\n    print(deref(mapped.at(0)))\n    let filtered=mapped.filter(fn(x:Ref<i64>):bool {return deref(x)>11})\n    print(deref(filtered.at(0)))\n    let total=[1,2,3].reduce(fn(a:Ref<i64>,n:i64):Ref<i64> {return ref(deref(a)+n)},ref(0))\n    print(deref(total))\n    let names=[1,2].map(fn(n:i64):str {return "value ${n}"})\n    print(names.join(","))\n    let nested=[1,2].map(fn(n:i64):Vec<i64> {return Vec<i64>(n)})\n    print(nested.at(1).at(0))\n    let empty=Vec<str>()\n    print(empty.join("-"))\n}\nmain()\n',
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
    "array_constructor_negative": 'fn main(){let xs=new Array(-1);print(xs.length)}\nmain()\n',
    "array_constructor_fraction": 'fn main(){let xs=new Array(1.5);print(xs.length)}\nmain()\n',
    "array_constructor_limit": 'fn main(){let xs=new Array(1000001);print(xs.length)}\nmain()\n',
    "prototype_type": 'fn main(){let a=Object({n:1});let n:i32=a.n;print(n)}\nmain()\n',
    "prototype_frozen": 'fn main(){let a=Object({n:1}); let b=a;Object.freeze(a);b.n=2}\nmain()\n',
    "prototype_nonextensible": 'fn main(){let a=Object();Object.preventExtensions(a);a.n=2}\nmain()\n',
    "prototype_nonconfigurable": 'fn main(){let a=Object();Object.defineProperty(a,"n",{value:1});Object.defineProperty(a,"n",{value:2})}\nmain()\n',
    "array_delete_min": "let xs=Vec<i64>(1);xs.delete(-9223372036854775808)",
    "array_delete_empty": "let xs=Vec<i64>();xs.delete(0)",
    "array_delete_negative": "let xs=Vec<i64>(1,2);xs.delete(-3)",
    "array_at_negative": "let x=[1,2];print(x.at(-3))",
    "array_at_end": "let x=[1,2];print(x.at(2))",
    "array_at_min": "let x=[1,2];print(x.at(-9223372036854775808))",
    "array_shift_empty": "let x=Vec<i64>();x.shift()",
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
        math_errors = {
            'math_type': 'fn main(){print(Math.sin("wrong"))}\nmain()\n',
            'math_arity': 'fn main(){print(Math.pow(2))}\nmain()\n',
            'math_unknown': 'fn main(){print(Math.nope(2))}\nmain()\n',
            'math_constant': 'fn main(){print(Math.MISSING)}\nmain()\n',
        }
        for name,code in math_errors.items():
            source=root/(name+'.dev');source.write_text(code,encoding='utf-8')
            for command in [[str(folder/('devrun'+suffix)),str(source),'--engine',engine] for engine in ['auto','ast']] + [[str(folder/('devc'+suffix)),'check',str(source)]]:
                result=run(command);assert result.returncode!=0 and 'Math' in result.stderr,(name,result.stderr)
            print('PASS:',name)
        const_errors = {
            'const_splice': 'const xs=Vec<i64>(1);xs.splice(0,1)',
            'const_delete': 'const xs=Vec<i64>(1);xs.delete(0)',
            'const_copyWithin': 'const xs=Vec<i64>(1);xs.copyWithin(0,0)',
            'const_method_key': "const p={name:'Dev',get:fn():str {return this.name}}\np['name']='Other'",
            'this_key_mutation': "let p={name:'Dev',change:fn() {this['name']='Other'}}",
            'this_mutation': 'let x={n:1,change:fn() {this.n=2}}',
            'const_sort': 'const xs=Vec<i64>(2,1);xs.sort()',
            'const_reverse': 'const x=Vec<i64>(1);x.reverse()',
            'const_shift': 'const x=Vec<i64>(1);x.shift()',
            'const_unshift': 'const x=Vec<i64>();x.unshift(1)',
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
        method_errors = {
            "splice_array": "let xs=[1,2];xs.splice(0,1)",
            "splice_element": "let xs=Vec<i64>();xs.splice(0,0,true)",
            "copyWithin_index": "let xs=Vec<i64>();xs.copyWithin(true,0)",
            "flat_depth": "let xs=[1,2];let depth=1;xs.flat(depth)",
            "flat_negative": "let xs=[1,2];xs.flat(-1)",
            "length_write": "let xs=Vec<i64>();xs.length=1",
            "callback_index_type": "let xs=[1];xs.map(fn(value:i64,index:str):i64 {return value})",
            "callback_index_arity": "let xs=[1];xs.forEach(fn(value:i64,index:i64,extra:i64) {})",
            "reduce_index_type": "let xs=[1];xs.reduce(fn(a:i64,b:i64,index:bool):i64 {return a+b},0)",
            'sort_array': 'let xs=[1,2];xs.sort()',
            'sort_default_ref': 'let xs=Vec<Ref<i64>>(ref(1));xs.sort()',
            'sort_callback_type': 'let xs=[1,2];xs.toSorted(fn(a:i64,b:i64):bool {return a<b})',
            'sort_callback_param': 'let xs=[1,2];xs.toSorted(fn(a:i32,b:i32):i64 {return 0})',
            'sort_arity': 'let xs=[1,2];xs.toSorted(1,2)',
            'map_void': 'let xs=[1,2];xs.map(fn(n:i64) {})',
            'map_parameter': 'let xs=[1,2];xs.map(fn(n:str):str {return n})',
            'filter_result': 'let xs=[1,2];xs.filter(fn(n:i64):i64 {return n})',
            'forEach_result': 'let xs=[1,2];xs.forEach(fn(n:i64):i64 {return n})',
            'reduce_signature': 'let xs=[1,2];xs.reduce(fn(n:i64):i64 {return n},0)',
            'reduce_seed': "let xs=[1,2];xs.reduce(fn(a:i64,b:i64):i64 {return a+b},'bad')",
            'concat_type': 'let xs=[1,2];xs.concat([true])',
            'at_type': 'let xs=[1,2];xs.at(true)',
            'join_type': "let xs=[1,2];xs.join(',')",
            'array_reverse': 'let xs=[1,2];xs.reverse()',
            'slice_reverse': 'let xs=[1,2];let view=xs.slice(0,2);view.reverse()',
            'unshift_type': "let xs=Vec<i64>();xs.unshift('bad')",
        }
        for name,code in method_errors.items():
            source=root/(name+'.dev');source.write_text(code,encoding='utf-8')
            commands=[[str(folder/('devrun'+suffix)),str(source),'--engine',engine] for engine in ['auto','ast']]
            commands.append([str(folder/('devc'+suffix)),'build',str(source),'-o',str(root/(name+suffix))])
            for command in commands:
                result=run(command)
                assert result.returncode != 0,(name,result.stdout,result.stderr)
            print('PASS:',name)
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
                # Windows runners can use an 8.3 TEMP path; diagnostics use the canonical long path.
                assert os.path.normcase(str(source.resolve())) in os.path.normcase(results[-1].stderr),(name,results[-1].stderr)
            else:
                assert all(r.returncode==0 for r in results),(name,[(r.stdout,r.stderr) for r in results])
                assert len({r.stdout for r in results})==1,(name,[(r.stdout,r.stderr) for r in results])
                if name == 'prototype':
                    assert results[0].stdout.splitlines()==['Hello Dev','true','{"age":18,"name":"Dev"}','prototype','updated','age,name','true','true','false','{"age":18,"name":"Dev"}','{"even":[2,4],"odd":[1,3]}','[object Object]'],results[0].stdout
                if name == 'math_order':
                    assert results[0].stdout.splitlines()==['1','2','3','3','4','5','4'],results[0].stdout
                if name == 'math_reference':
                    assert results[0].stdout.splitlines()==['true']*60,results[0].stdout
                if name == 'array_basic_managed':
                    assert results[0].stdout.splitlines()==['1','1','2','3','1','2','1','1','2','3','4','3','3','1','2','1,9,3','9,3','1,2,1,2','1,2','1,2','1,2','8','','0'],results[0].stdout
                if name == 'array_basic':
                    assert results[0].stdout.splitlines()==['4','4','1,2,3,4','true','false','2,3','1,9,8,7,4','1,2,3,4','1,9,8,6,4','1,9,8,7,4','4','1,1,9,8','1,2,3,4','2','true,false','Hello,Dev','true','false','false'],results[0].stdout
                if name == 'dynamic_array_methods':
                    assert results[0].stdout.splitlines()==['true','0','3','Dev|42|true||0.10000000000000001','Dev,42,true,,0.10000000000000001','1,10,2','1,2,10','[1,[2,3],4]','1,2,3,4','[1,2,3,4]','1,2,3,4','0','1','false','[object Object],[object Object],[object Object]','Dev,Dev,[object Object]','true','Dev'],results[0].stdout
                if name == 'array_index':
                    assert results[0].stdout.splitlines()==['0: 10','1: 20','2: 30','32','2','true','true','1','2','63','slice 0: 20','slice 1: 30','10','20','30'],results[0].stdout
                if name == 'array_sort_target':
                    assert results[0].stdout.splitlines()==['index once','1'],results[0].stdout
                if name == 'array_sort':
                    assert results[0].stdout.splitlines()==['-3','1','2','2','10','10','10','-3','10','apple,banana,zebra','1','0','7'],results[0].stdout
                if name == 'array_sort_managed':
                    assert results[0].stdout.splitlines()==['10','11','21','22','23','21','10','11','21','22','23','-9223372036854775808','9223372036854775807','-1.5'],results[0].stdout
                if name == 'array_callbacks_order':
                    assert results[0].stdout.splitlines()==['receiver','callback','initial','1','2','3','16','1','2','true','1','false','1','2','1','3','2','1'],results[0].stdout
                if name == 'array_methods':
                    assert results[0].stdout.splitlines()==['2','2','2','8','true','true','1','3','2','3','5','2','0','2','1','Hello Dev','Dev','true','false','-1','10','3'],results[0].stdout
                if name == 'array_methods_managed':
                    assert results[0].stdout.splitlines()==['3','2','1','2','11','12','6','value 1,value 2','2',''],results[0].stdout
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
