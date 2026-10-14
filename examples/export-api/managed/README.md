# Python, .NET and Node.js / TypeScript interoperability

See [the full guide](../../../docs/managed-interop.md) for setup, complete source links, commands, ownership and verified platforms.

- Python ctypes / .NET PInvoke load the shared C adapter (`devbridge.dll` or `libdevbridge.so`).
- JavaScript and TypeScript call the same DevLang static library through the handwritten Node-API addon (`devmath.node`). TypeScript is checked with `tsc --strict`.
- DevLang source runtime calls Python/.NET/JS/TS JSON workers using bounded stdin/stdout and separate process arguments. This reverse path is runtime-only.

```sh
python examples/export-api/managed/prepare_node_sdk.py --output out/node-sdk
python examples/export-api/managed/build.py --d target/release/d.exe --node-sdk out/node-sdk
python scripts/smoke_managed_interop.py --d target/release/d.exe --node-sdk out/node-sdk
```

On Linux use `target/release/d --cc gcc --ar ar` in the build/test command. To use Python/.NET alone, omit `--node-sdk` when building and pass `--languages python,dotnet` to the test.

Requirements: Python 3.12+, C compiler/archiver and current d/devc; .NET 9 SDK for C#; Node and TypeScript 5.9.3 on PATH for JS/TS. Node SDK preparation downloads verified matching headers/import libraries from nodejs.org.

Verified locally: Windows x64 all four consumers/workers; WSL Linux x64 Python/JS/TS. CI includes .NET on Linux as well. The test checks negative input, failed workers, UTF-8 and paths containing spaces.
