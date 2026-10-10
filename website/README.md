# DevLang documentation website

Static Thai learning guide, complete reference documentation, executable examples and a downloadable AI skill. No browser interpreter is advertised or included.

Build from the repository root:

```sh
python -m pip install --target website/.build-deps -r website/requirements.txt
cargo build --release
python website/build.py --d target/release/d.exe
python -m http.server 4173 --directory website/dist --bind 127.0.0.1
```

Edit `content/guide.md` for the Thai guide. Reference pages are generated from repository docs. Examples are read from `examples/`; the build captures output with the actual source runtime. The website includes a ZIP generated from `skills/devlang`, `llms.txt`, plain Markdown documents and source downloads.

On Linux pass `--d target/release/d`. The JSON example needs the latest source
runtime, not the old v0.4.0 binary. `dev-runtime` fences in the guide are checked
with both source engines; `dev` fences also receive native frontend checks.

`dist` is a complete static deployment directory. GitHub Pages publishes it at https://d-osc.github.io/DevLang/ through `.github/workflows/pages.yml`. Changes to website source, reference docs, examples or the AI skill on `main` trigger deployment; the workflow can also be started manually. It builds with `--skip-validation`, checking example source hashes against the validated outputs in `content/example-outputs.json`. If an example changes, run a validated local build and commit its refreshed snapshot before publishing.

Assets and downloads use relative URLs, and documentation navigation uses hash routes, so the site works under the `/DevLang/` project path. The Sites hosting manifest retains the separate hosted Site identity; use the Sites source workflow for that host.

Standalone Site clones use the tracked snapshots in `content/materials` and `content/example-outputs.json`. Run `python build.py --skip-validation` to rebuild with those outputs, or supply `--d /path/to/d` for fresh runtime validation. In the DevLang checkout, builds refresh the snapshots from the current repository docs/examples/skill.

## Example catalog

`content/examples.json` defines 100 focused and integrated examples, their topic,
mode, commands, prerequisites, input and related documentation page. It covers
conditions/logical aliases, operators, types/casts, loops, functions/recursion,
modules, structs/methods, enums/matching, Ref/recursive layouts, Vec/Map/Slice,
generic inference/constraints/const parameters, traits, closures, threaded tasks,
callbacks, raw memory, volatile access, variadic/aggregate C ABI, export, runtime
I/O/files/input/arguments/time and native stdlib.

A full build validates common programs with native frontend checking and both
runtime engines; runtime API programs use both engines; native-only examples
are compiled and executed. File I/O runs in temporary directories and console
input receives the catalog's recorded stdin. Build the native stdlib first with
`python stdlib/build.py` when refreshing its example output. C-source FFI examples
require the sibling compiler and a C backend. Diagnostics/outputs do not claim
that physical hardware was exercised.

Captured-output freshness hashes include the catalog entry, imported Dev sources,
C/header dependencies and bundled stdlib source. CI uses these snapshots without
requiring a Windows executable. Example ZIPs preserve paths from the repository
root and include imports/C dependencies; the native stdlib example also includes
its buildable source. Every example page lists the exact validation mode.

Per-module API pages are registered by content/module-docs.json. Rebuild/check them with python scripts/build_module_docs.py [--check] [--d target/release/d.exe] from the repository root. Each module links to its independently validated example.
