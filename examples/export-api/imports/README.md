# Import C++, Rust and Go into DevLang

The same `main.dev` calls one foreign library at a time using `extern fn`, `unsafe` and C ABI. Each implementation exports `foreign_add`, `foreign_scale` and `foreign_positive`.

See [the complete import guide](../../../docs/import-interop.md) for all build commands, runtime `--ffi-lib`, native link commands, expected output and platform constraints.

Verified on Windows x86_64 with C++/Rust and WSL Linux x86_64 with C++/Rust/Go. Each library was tested in runtime auto, AST and native builds. Runtime tests use an empty compiler PATH. Go on Windows is not covered.

```sh
python scripts/smoke_import_interop.py --d target/release/d
# Windows:
python scripts/smoke_import_interop.py --d target/release/d.exe --languages cpp,rust
```

The foreign source must first be compiled to a DLL/shared library. Running the DevLang source via the runtime does not require compiling that source first.
