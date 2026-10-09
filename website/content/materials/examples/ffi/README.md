# C FFI

Run directly; no manual build step:

```powershell
.\target\release\d.exe examples/ffi/main.dev
```

```sh
./target/release/d examples/ffi/main.dev
```

`puts` resolves from the system C library. If no loaded or adjacent shared
library provides `device_add`, the runtime finds its adjacent C source and asks
the separate `devc` executable to prepare a shared library automatically.
The first run needs Clang/GCC (or `DEV_CC`) and `devc` beside `devrun`.
Dev source is interpreted directly; only the C dependency is compiled.
Later runs reuse `.dev-cache/native/` without invoking a C backend. Changes to
adjacent C files or local headers invalidate the cache. Remove that directory
to rebuild after external toolchain/system-header changes.

C files in the declaration module's directory are compiled together; local
headers may be in subdirectories. Imported declaration modules work too.
Windows exports matching declared function names automatically with Clang.
The C library must match the runtime OS, architecture and default C ABI.

Use repeatable `--ffi-lib PATH` for explicit libraries. Explicit and adjacent
prebuilt libraries take precedence over automatic C preparation. The optional
`python examples/ffi/build.py` prepares an adjacent library manually; remove it
to switch back to automatic source preparation.

The native compiler remains supported:

```sh
d build examples/ffi/main.dev --link examples/ffi/device.c -o out/ffi
```
