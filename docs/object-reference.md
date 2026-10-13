# Object Reference / คู่มือ Object

DevLang supports shared prototype objects in both the source runtime and hosted native compiler. Plain DON literals remain data values. Use `new Object({...})` for mutable object identity and prototype inheritance.

DevLang รองรับ object ที่มี identity ร่วมกันและสืบทอด prototype ทั้ง runtime และ native compiler ใช้ `new Object({...})` เมื่อต้องการพฤติกรรมนี้ ส่วน DON literal ปกติยังเป็นข้อมูล

```dev
use "std/json"
fn Person(this: json.Value, name: str) { this.name = name }
fn main() {
    Object.defineProperty(Person.prototype, "greet", {
        value: fn(this: json.Value): str { return "Hello " + this.name }
        writable: true
        configurable: true
    })
    let person = new Person("Dev")
    print(person.greet())
    print(person.constructor == Person)
    let alias = person
    Object.freeze(person)
    print(Object.isFrozen(alias))
}
main()
```

Methods, getters, setters and constructors receive an explicit first `this: json.Value` parameter. Methods assigned after construction are visible to existing instances. Calling a detached method requires supplying its receiver explicitly.

Method, getter, setter และ constructor ระบุ `this: json.Value` เป็น parameter แรก หากเพิ่ม method ลง prototype ภายหลัง instance เดิมจะเรียกใช้ได้ด้วย

| API | Behavior / การทำงาน |
| --- | --- |
| `new Object(data)` | Create an object; an existing prototype object keeps its identity / สร้าง object หรือคืน identity เดิม |
| `Object.prototype` | Shared root prototype / prototype หลักที่ใช้ร่วมกัน |
| `fnName.prototype` | Constructor prototype; can be replaced / prototype ของ constructor |
| `object.constructor` | Inherited constructor function / ฟังก์ชัน constructor ที่สืบทอดมา |
| `Object.construct(fn, ...args)` | Allocate instance and invoke constructor / สร้าง instance แล้วเรียก constructor |
| `Object.create(proto, descriptors)` | Inherit from proto; optional descriptors; null gives no parent / สืบทอด prototype หรือใช้ null |
| `Object.assign(target, ...sources)` | Copy enumerable own properties; evaluates getters/setters / คัดลอก property ของ object |
| `Object.defineProperty(obj, key, descriptor)` | Define data or accessor property / กำหนด property หรือ getter/setter |
| `Object.defineProperties(obj, descriptors)` | Define multiple properties / กำหนดหลาย property |
| `Object.delete(obj, key)` | Delete own configurable property; returns bool / ลบ property ที่อนุญาตให้ลบ |
| `Object.keys(obj)` | Enumerable own names, `Vec<str>` / ชื่อ property ที่ enumerable |
| `Object.values(obj)` | Enumerable own values, `Vec<json.Value>` / ค่า property |
| `Object.entries(obj)` | Enumerable `[key, value]` pairs / คู่ชื่อและค่า |
| `Object.fromEntries(pairs)` | Create an object from pairs / สร้าง object จากคู่ชื่อและค่า |
| `Object.getOwnPropertyNames(obj)` | All own names, including non-enumerable / ชื่อทั้งหมดของ object |
| `Object.getOwnPropertyDescriptor(obj, key)` | Descriptor object, or null when absent / ข้อมูล descriptor หรือ null |
| `Object.getOwnPropertyDescriptors(obj)` | Object containing all own descriptors / descriptor ทั้งหมด |
| `Object.groupBy(items, callback)` | Group by callback string key; optional index: i64 / จัดกลุ่มตาม string ที่ callback คืน |
| `Object.preventExtensions(obj)` | Block adding own properties / ห้ามเพิ่ม property |
| `Object.seal(obj)` | Block adding/removing properties / ห้ามเพิ่มหรือลบ property |
| `Object.freeze(obj)` | Seal and make own data properties read-only / seal และห้ามเปลี่ยน data property |
| `Object.isExtensible(obj)` | Whether properties can be added / ยังเพิ่ม property ได้หรือไม่ |
| `Object.isSealed(obj)` | Whether object is sealed / ตรวจ seal |
| `Object.isFrozen(obj)` | Whether object is frozen / ตรวจ freeze |
| `obj.toString()` / `Object.toString(obj)` | Returns `[object Object]` / คืนข้อความชนิด object |
| `obj.valueOf()` / `Object.valueOf(obj)` | Returns the same object identity / คืน object เดิม |

## Property descriptors / การกำหนด property

```dev
use "std/json"
fn main() {
    let counter = new Object({ count: 1 })
    Object.defineProperty(counter, "double", {
        get: fn(this: json.Value): i64 { return this.count * 2 }
        set: fn(this: json.Value, value: i64) { this.count = value / 2 }
        enumerable: true
        configurable: true
    })
    counter.double = 10
    print(counter.count)
    print(json.stringify(counter))
}
main()
```

Data descriptors use `value` and `writable`; accessors use `get` and/or `set`. Do not mix both forms. Missing flags on a new property default to false. Updating an existing descriptor preserves omitted flags. Frozen objects are shallow: referenced children and accessor setters can still have their own state.

Data descriptor ใช้ `value` กับ `writable` ส่วน accessor ใช้ `get`/`set` ห้ามผสมสองแบบ ค่า flag ที่ไม่ระบุใน property ใหม่เป็น false ส่วน property เดิมจะคง flag ที่ไม่ได้ระบุไว้ freeze มีผลระดับเดียว

## Serialization and limits / การแปลงข้อมูลและข้อจำกัด

JSON/DON serialization evaluates own enumerable getters and omits function properties and inherited properties. Names are sorted deterministically. Missing descriptors use null; DevLang has no JavaScript undefined. Native callback arguments and results support strict scalar types, struct/enum values, Array/Vec/Slice/Map/Ref and Dev function values. Aggregate types must match exactly; DON object literals do not implicitly become structs. Use explicit type annotations for aggregate/function method results. Managed arguments and results retain ownership and collection copy-on-write behavior. Prototype allocations in native programs live until process exit; the source runtime collects unreachable prototype cycles through arrays, maps, references and closures. GC.collect() requests a pass. Cycles hidden in foreign resources or Task internals remain conservative roots. This is prototype inheritance with DevLang's strict types, not full JavaScript compatibility.

JSON/DON แปลงเฉพาะ own enumerable property โดยเรียก getter และไม่รวม function หรือ property ที่สืบทอดมา ชื่อเรียงลำดับแน่นอน ไม่มี undefined แบบ JavaScript Native callback รับส่ง struct/enum, Array/Vec/Slice/Map/Ref และ Dev function ได้แล้ว ชนิดต้องตรงกัน ห้ามใช้ DON literal แทน struct โดยอัตโนมัติ และควรระบุชนิดของผลลัพธ์ที่เป็น aggregate/function allocation ของ native prototype อยู่จนโปรแกรมจบ runtime เก็บวงจร prototype ผ่าน array/map/reference/closure ได้แล้ว ใช้ GC.collect() ขอเก็บเองได้ แต่ cycle ผ่าน foreign resource หรือภายใน Task ยังเก็บแบบ conservative

## Constructor syntax / Syntax constructor

`new Object()` creates an empty object; `new Object({...})` initializes properties. `new Person(args)` creates an instance using `Person.prototype` and calls `Person(this, args)`. Constructor aliases and module-qualified names such as `new lib.Person(args)` are supported. Parentheses are required. Existing `Object(...)` and `Object.construct(...)` calls remain compatible.

## Direct updates

```dev
use "std/json"
fn Person(this: json.Value, name: str) { this.name = name }
fn main() {
    Person.prototype.greet = fn(this: json.Value): str { return this.name }
    let p = new Person("Dev")
    print(p.greet())
    Object.prototype.common = "root"
    print(p.common)
}
main()
```

## Typed callback values / ค่าที่มีชนิดใน callback

```dev
use "std/json"
struct Point { x: i64; y: i64 }
fn main() {
    let object = new Object({
        sum: fn(this: json.Value, point: Point): i64 {
            return point.x + point.y
        }
        copy: fn(this: json.Value, point: Point): Point { return point }
    })
    let point = Point(3, 4)
    print(object.sum(point))
    let copied: Point = object.copy(point)
    print(copied.x)
}
main()
```

The callback boundary checks nominal type identity, array length, collection element types and function signatures before invoking native code. Pass declared typed arrays rather than untyped array literals. A captured Dev function keeps its context when passed or returned. Native typed arguments borrow a retained caller value for the invocation, without serializing collection contents. JSON/DON encoding is lazy. Escaping property/result payloads hold retained copies until process exit, following the native arena lifetime. Native JSON/DON conversion is separate: payload enums, non-string-key maps and recursive reference layouts can be passed as typed values but do not gain JSON serialization support. Raw pointer parameters and Task values remain unsupported at the Object callback boundary. This does not change the C extern/callback ABI.

callback ตรวจชนิด struct/enum, ความยาว array, ชนิดสมาชิก collection และ signature ของ function ก่อนเรียก native code closure จะเก็บ context ไว้ได้ argument แบบ typed ยืมค่าที่ถูก retain ไว้ระหว่างเรียก โดยไม่ serialize สมาชิก collection จนกว่าจะต้องใช้ JSON/DON ส่วนค่าที่เก็บลง property หรือส่งคืนจะมี retained payload อยู่ใน arena จนจบโปรแกรม การรับส่ง enum payload, Map ที่ key ไม่ใช่ str และ Ref แบบ recursive ไม่ได้ทำให้แปลง JSON/DON ได้ ส่วน raw pointer และ Task ยังไม่รองรับผ่าน boundary นี้ และไม่เปลี่ยน ABI ของ C FFI

[Typed structs, arrays and constructors / ตัวอย่าง](#/examples/object-aggregates) · [Managed values and recursive enums / ตัวอย่าง managed](#/examples/object-aggregates-managed) · [Captured functions / ตัวอย่าง closure](#/examples/object-aggregate-functions)
