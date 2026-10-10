# Per-module API examples

These examples accompany [the module API index](../../docs/modules/index.md).
Each folder is one module. Run runtime examples with `d examples/modules-api/NAME/main.dev`.
Native folders require `python stdlib/build.py`, then `d build FILE --release
--module-dir std=stdlib/modules --link stdlib/lib/libdevruntime.a -o out/example`.

`python scripts/build_module_docs.py --check --d target/release/d.exe` validates
all 42 runtime examples with auto/AST and no compiler on PATH, and builds/runs
all four native examples with the real C stdlib. Runtime examples use temporary
working directories, loopback ephemeral ports and no external services. The TLS example only inspects options; connection coverage remains
in the corresponding smoke suites. Examples explicitly release opened handles.
