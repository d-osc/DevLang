# Foundational source-runtime libraries

The current source runtime adds `std/math`, `std/random`, `std/datetime`,
`std/test`, `std/log`, and expands `std/strings`. Import with `use "std/math"`.
These APIs run without a C compiler. They are not yet provided by native stdlib
or existing release installers. Shared signatures let the editor check calls.
Arguments are required; there are no optional/variadic arguments.

## Math

All numeric arguments/results use **f64**; use `9.0` or an explicit `as f64` cast.
Angles are radians. Invalid domains/nonfinite results raise runtime errors.

| API | Meaning |
| --- | --- |
| `pi()`, `e()`, `tau()` | Constants as functions |
| `abs(x)`, `sqrt(x)`, `cbrt(x)` | Absolute value and roots |
| `pow(x,y)`, `exp(x)` | Powers/exponential |
| `log(x)`, `log2(x)`, `log10(x)` | Natural, base-2 and base-10 logarithms |
| `sin(x)`, `cos(x)`, `tan(x)` | Trigonometry |
| `asin(x)`, `acos(x)`, `atan(x)`, `atan2(y,x)` | Inverse trigonometry |
| `floor(x)`, `ceil(x)`, `round(x)`, `trunc(x)` | Rounding; `round` ties away from zero |
| `min(x,y)`, `max(x,y)`, `clamp(x,low,high)` | Bounds; clamp rejects low > high |
| `hypot(x,y)` | Hypotenuse |
| `isFinite(x)`, `isNaN(x)` | f64 predicates returning bool |

This is floating-point math, not arbitrary-precision arithmetic. Platform math
implementations can differ slightly; use `test.near` for approximate comparisons.

## Random

| API | Meaning |
| --- | --- |
| `seed(value i64)` | Reset the interpreter's SplitMix64 state |
| `float() f64` | Uniform 53-bit fraction in `[0,1)` |
| `int(min i64,max i64) i64` | Uniform integer, minimum included, maximum excluded |
| `bool() bool` | Random boolean |
| `bytes(size i64) Vec<u8>` | PRNG bytes, up to 8 MiB |

Without an explicit seed, the OS supplies the initial seed. Equal seeds and equal
call sequences produce repeatable results. Each task interpreter has separate
state. Integer generation uses rejection sampling to avoid modulo bias.
These APIs are **not cryptographically secure**; do not use them for passwords,
tokens, keys or security nonces. There is no crypto module in this change.

## Strings

| API | Result |
| --- | --- |
| `len(text)` | UTF-8 **byte** length as usize (existing behavior) |
| `charLength(text)` | Unicode scalar count as i64 |
| `charAt(text,index)` | One Unicode scalar as str |
| `substring(text,start,end)` | Unicode scalar range `[start,end)` |
| `concat(a,b)`, `equal(a,b)` | Existing text operations |
| `trim(text)`, `trimStart(text)`, `trimEnd(text)` | Unicode whitespace removal |
| `toLowerCase(text)`, `toUpperCase(text)` | Unicode casing, without locale tailoring |
| `contains(text,pattern)`, `startsWith(text,pattern)`, `endsWith(text,pattern)` | bool |
| `indexOf(text,pattern)`, `lastIndexOf(text,pattern)` | UTF-8 **byte** offset, or -1 |
| `split(text,separator)` | Vec<str>; literal separator, keeps empty entries |
| `join(parts Vec<str>,separator)` | str |
| `replace(text,pattern,replacement)` | Replace first literal occurrence |
| `replaceAll(text,pattern,replacement)` | Replace all literal occurrences |
| `repeat(text,count)` | Repeated str |

An empty split separator returns Unicode scalars, with no extra empty entries.
Unicode scalars are not grapheme clusters (Thai marks and combined emoji can
contain multiple scalars). Negative/out-of-range character indices fail;
substring does not clamp or swap indices. An empty replacement pattern follows
UTF-8 character boundaries. Generated text is limited to 8 MiB and split to 65,536
parts / 8 MiB of text. There are no regex or Unicode normalization APIs yet.

## Datetime

Timestamps are **signed i64 Unix milliseconds**, distinct from the monotonic
elapsed time in `std/time`. Supported calendar range is bounded by Chrono.

| API | Meaning |
| --- | --- |
| `now() i64` | Current system clock |
| `parse(text) i64` | RFC3339 with explicit Z/offset; fractions truncated to milliseconds |
| `iso(timestamp) str` | UTC RFC3339, with three fractional digits |
| `format(timestamp,pattern) str` | UTC strftime-style format, e.g. `%F %T` |
| `formatOffset(timestamp,offsetMinutes,pattern) str` | Format in a fixed UTC offset |
| `parts(timestamp,offsetMinutes) Parts` | Calendar fields in a fixed UTC offset |
| `utc(year,month,day,hour,minute,second,millisecond) i64` | Construct validated UTC timestamp |
| `addMilliseconds(timestamp,duration) i64` | Checked signed-duration addition |
| `isLeapYear(year) bool`, `daysInMonth(year,month) i64` | Gregorian calendar helpers |

`Parts` fields: year, month, day, hour, minute, second, millisecond, weekday,
offsetMinutes (all i64). Month/day start at 1; weekday is Monday=1 to Sunday=7.
Fixed offsets must be -1439..1439 minutes; Bangkok uses 420. Invalid dates,
leap seconds, unsupported format directives and out-of-range dates raise errors.
This release does not include IANA timezone rules, DST or localized formatting.
System clock corrections can move `now()` backwards; use `std/time` for durations.

## Tests

| API | Meaning |
| --- | --- |
| `expect(condition bool,message str)` | Raise an assertion error when false |
| `equal<T>(actual T,expected T,message str)` | Exact structural comparison of supported JSON-encodable Dev values |
| `near(actual f64,expected f64,tolerance f64,message str)` | Absolute error tolerance; finite values/nonnegative tolerance required |
| `case(name str,body fn() void)` | Register a named callback |
| `run() Report` | Run callbacks in registration order; catch errors and continue |

`Report` contains total/passed/failed i64 and summary str. Running consumes the
queue. At most 1024 uniquely named cases are accepted. Nested runs and registering
cases during a run fail. Summary rows are truncated after 4096 UTF-8 bytes.
Callbacks follow DevLang's usual capture-by-value rules. `equal` excludes function
values/tasks/non-null raw pointers and does not compare underlying handle resources.
There are no mocks, fixture hooks, directory discovery or async test scheduler yet.
The runner **does not change the process exit code**; propagate a failure explicitly:

```dev-runtime
use "std/test"
use "std/math"
fn main() {
    test.case("sqrt", fn() {
        test.near(math.sqrt(9.0), 3.0, 0.000001, "square root")
    })
    let report = test.run()
    print(report.summary)
    if report.failed > 0 { return 1 }
    return 0
}
return main()
```

## Logging

| API | Meaning |
| --- | --- |
| `setLevel(level str)` | debug/info/warn/error/off; default info |
| `debug(message)`, `info(message)`, `warn(message)`, `error(message)` | Emit at the selected level |
| `toFile(path str)` | Open append-only log output; replace this interpreter's destination |
| `toStderr()` | Flush/close file output and switch to stderr |
| `flush()` | Flush current output |

Lines contain UTC RFC3339 timestamp, level and message. CR/LF in a message are
escaped to keep each message on one line. Messages are limited to 8 MiB; writes
are synchronous and flushed. I/O errors propagate. Files close when switched or
when the interpreter exits. Task interpreters have separate log settings; writes
from multiple processes/tasks are not globally coordinated. Rotation, structured
JSON logging, remote sinks and automatic log retention are not implemented.

## Examples and validation

```sh
d examples/basic-libs/main.dev
python scripts/smoke_basic_libs.py --bin-dir out/core-io/release
```

The smoke suite checks both source engines, seeded randomness, Unicode text,
calendar boundaries, continued execution after failed tests, exit status and real
file logging, with PATH empty and no C compiler.
