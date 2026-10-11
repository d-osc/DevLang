# Math API

Import the installed module with `use "math/lib" as math`.

## `add(a i64, b i64) i64`

Returns the sum of two signed 64-bit integers.

```dev
use "math/lib" as math

fn main() {
    print(math.add(19, 23))
    return 0
}
return main()
```

The checked result is `42`. Operands must be `i64`; integer overflow follows the selected execution mode, so callers should keep sums within the supported range.
