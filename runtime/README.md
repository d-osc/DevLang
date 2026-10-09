# Dev source runtime

`devrun` interprets `.dev` source directly. It does not invoke `devc`, a C
compiler, linker, or Rust during execution, and creates no build artifacts.
Compiler and runtime are independent executables; only the syntax crate is shared.

```sh
cargo build --release -p dev-runtime
./target/release/devrun examples/hello.dev
./target/release/devrun examples/modules/main.dev
./target/release/devrun examples/runtime/main.dev -- hello
./target/release/devrun -e 'print(40 + 2)'
./target/release/devrun --eval 'print(40 + 2)' --timings
```

Windows uses `devrun.exe`. Distribute the binary alone; users do not need Rust
or the compiler to run it. Source builds need Rust and the sibling `syntax/`
crate, but not `compiler/` or `stdlib/`.

Supports functions, relative imports (including cycles), local scopes, typed
numbers, strings, arrays, casts, conditions, loops, break/continue and print.
The entry point remains `fn main()`. `-e` wraps statements in that entry point.
Strings and arrays are owned automatically; arrays use value copies.

This first release executes an AST and has no JIT. It is intended for direct
execution and scripting; it does not claim native performance. Raw pointers,
volatile hardware access and external C functions require `devc`. Array
assignment currently supports a named array indexed by an expression; nested
array assignment is unsupported. Recursion is limited to 128 calls.

See `docs/runtime.md` for intrinsic APIs and `stdlib/` for the separate native
C library. Validate with `python scripts/smoke_runtime.py --runtime target/release/devrun`.
