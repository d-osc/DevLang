# Math Reference / คู่มือ Math

`Math` is a built-in namespace in DevLang. No import is needed. These methods work in the source runtime and hosted native compiler. Numeric arguments are converted to f64 for this API; strings, bool, null and objects are not implicitly converted. Every method returns f64, including clz32. Constants are read-only f64 values.

Math เป็น namespace ในตัว ไม่ต้อง use รองรับทั้ง runtime และ native compiler แปลง argument ที่เป็นตัวเลขเป็น f64 เฉพาะ API นี้ ไม่แปลง string, bool, null หรือ object อัตโนมัติ ทุก method คืน f64 รวมถึง clz32 ส่วนค่าคงที่เป็น f64 และแก้ไขไม่ได้

```dev
fn main() {
    print(Math.sqrt(81))
    print(Math.max(2, 8, 3))
    print(Math.round(-1.5)) // -1
    print(Math.sin(Math.PI / 2))
}
main()
```

## Methods / ฟังก์ชัน

| Name | Behavior / การทำงาน |
| --- | --- |
| `Math.abs(x)` | Magnitude / ค่าสัมบูรณ์ |
| `Math.acos(x)` | Inverse cosine, radians / อาร์กโคไซน์ หน่วยเรเดียน |
| `Math.acosh(x)` | Inverse hyperbolic cosine / ไฮเพอร์โบลิกโคไซน์ผกผัน |
| `Math.asin(x)` | Inverse sine, radians / อาร์กไซน์ หน่วยเรเดียน |
| `Math.asinh(x)` | Inverse hyperbolic sine / ไฮเพอร์โบลิกไซน์ผกผัน |
| `Math.atan(x)` | Inverse tangent, radians / อาร์กแทนเจนต์ หน่วยเรเดียน |
| `Math.atan2(y, x)` | Angle for a coordinate pair / มุมจากพิกัด y และ x |
| `Math.atanh(x)` | Inverse hyperbolic tangent / ไฮเพอร์โบลิกแทนเจนต์ผกผัน |
| `Math.cbrt(x)` | Cube root / รากที่สาม |
| `Math.ceil(x)` | Round toward positive infinity / ปัดขึ้น |
| `Math.clz32(x)` | Leading zero bits after uint32 conversion / จำนวนบิตศูนย์ด้านหน้า หลังแปลงเป็น uint32 |
| `Math.cos(x)` | Cosine, radians / โคไซน์ |
| `Math.cosh(x)` | Hyperbolic cosine / ไฮเพอร์โบลิกโคไซน์ |
| `Math.exp(x)` | Exponential e^x / e ยกกำลัง x |
| `Math.expm1(x)` | Accurate e^x - 1 near zero / e ยกกำลัง x ลบ 1 แม่นยำใกล้ศูนย์ |
| `Math.f16round(x)` | Nearest binary16 value, ties to even / ปัดเป็น float16 กรณีกึ่งกลางเลือกค่าคู่ |
| `Math.floor(x)` | Round toward negative infinity / ปัดลง |
| `Math.fround(x)` | Nearest binary32 value / ปัดเป็น float32 |
| `Math.log(x)` | Natural logarithm / ลอการิทึมธรรมชาติ |
| `Math.log10(x)` | Base-10 logarithm / ลอการิทึมฐาน 10 |
| `Math.log1p(x)` | Accurate ln(1 + x) near zero / ln(1+x) แม่นยำใกล้ศูนย์ |
| `Math.log2(x)` | Base-2 logarithm / ลอการิทึมฐาน 2 |
| `Math.max(...values)` | Largest value; no values gives negative infinity / ค่าสูงสุด ไม่มี argument คืนลบอนันต์ |
| `Math.min(...values)` | Smallest value; no values gives positive infinity / ค่าต่ำสุด ไม่มี argument คืนบวกอนันต์ |
| `Math.pow(x, y)` | Exponentiation x^y / x ยกกำลัง y |
| `Math.random()` | Pseudo-random f64 in [0, 1) / สุ่มค่า f64 ตั้งแต่ 0 แต่ไม่ถึง 1 |
| `Math.round(x)` | Nearest integer, half ties toward positive infinity / ปัดจำนวนเต็ม กึ่งกลางปัดไปทางบวก |
| `Math.sign(x)` | -1, +1, zero or NaN / ตรวจเครื่องหมาย คง signed zero และ NaN |
| `Math.sin(x)` | Sine, radians / ไซน์ |
| `Math.sinh(x)` | Hyperbolic sine / ไฮเพอร์โบลิกไซน์ |
| `Math.sqrt(x)` | Square root / รากที่สอง |
| `Math.tan(x)` | Tangent, radians / แทนเจนต์ |
| `Math.tanh(x)` | Hyperbolic tangent / ไฮเพอร์โบลิกแทนเจนต์ |
| `Math.trunc(x)` | Remove fractional part toward zero / ตัดเศษทศนิยมเข้าหาศูนย์ |

## Constants / ค่าคงที่

| Name | f64 value |
| --- | --- |
| `Math.E` | 2.718281828459045 |
| `Math.LN2` | 0.6931471805599453 |
| `Math.LN10` | 2.302585092994046 |
| `Math.LOG10E` | 0.4342944819032518 |
| `Math.LOG2E` | 1.4426950408889634 |
| `Math.PI` | 3.141592653589793 |
| `Math.SQRT1_2` | 0.7071067811865476 |
| `Math.SQRT2` | 1.4142135623730951 |

## Rounding and domains / การปัดและโดเมน

`Math.round(-1.5)` is -1; `Math.round(-0.5)` preserves negative zero. `f16round` converts directly to binary16 using ties-to-even, then returns that rounded value as f64; it does not floor a number. `Math.f16round(1.0004882812500002)` is 1.0009765625. Subnormal values, overflow, infinity, NaN and signed zero are handled.

round ปัดกึ่งกลางไปทางบวก ส่วน f16round ปัดเป็น float16 โดยเลือกค่าคู่เมื่ออยู่กึ่งกลาง แล้วคืนผลเป็น f64 ไม่ใช่การปัดลงเป็นจำนวนเต็ม

Negative sqrt and out-of-domain inverse functions return NaN. `Math.log(0)` is negative infinity. Variadic min/max propagate NaN and distinguish signed zero. Required arguments and numeric types are checked. Transcendental results can vary by a few last bits between system math libraries; compare using a tolerance. random is pseudo-random, not a cryptographic API.

sqrt ของค่าติดลบหรือฟังก์ชันผกผันที่อยู่นอกโดเมนคืน NaN ส่วน log(0) คืนลบอนันต์ min/max รองรับหลาย argument ตรวจจำนวน argument และชนิดตัวเลข ผลฟังก์ชันบางตัวอาจต่างเล็กน้อยตาม math library จึงควรเปรียบเทียบด้วย tolerance random ไม่ใช่ API สำหรับการเข้ารหัส

The existing imported `std/math` API keeps its strict finite-input/domain-error behavior. Math methods currently use direct namespace calls; wrap them in typed functions for callbacks. Freestanding targets do not support hosted Math methods. Direct Math calls on Linux automatically link libm. Consumers of separately built external archives must still supply their linker dependencies.

std/math เดิมยังคงตรวจค่าที่ต้องเป็น finite และแจ้ง domain error ใช้ Math.method(...) โดยตรง หากต้องการ callback ให้ห่อด้วยฟังก์ชันที่ระบุชนิด ไม่รองรับ Math methods ใน freestanding

The float16 rounding behavior follows [ECMAScript Math.f16round](https://tc39.es/ecma262/multipage/numbers-and-dates.html#sec-math.f16round).
