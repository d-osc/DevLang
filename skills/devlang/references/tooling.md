# Developer tooling (latest source build)

Use `cargo build --release` when tooling commands are missing; existing v0.4.0
installers do not include them. d/devrun/devc stay together.

`d new NAME` creates `dev.toml`, src/main.dev and .gitignore; destination must not
exist and NAME is a Dev identifier. `d run/build/check` without a filename use
`[package].entry` (default src/main.dev). `[package].modules` defaults to src.
Import dependencies as `use "NAME/lib"`; aliases follow ordinary import rules.
Dependencies require their own dev.toml and declaration-only imported .dev files.

`d pkg add NAME --path DIR` or `--git HTTPS_URL [--rev COMMIT_OR_TAG]` updates the
manifest and installs. `install` keeps existing matching Git commits pinned;
`update` resolves refs again. Commit dev.toml/dev.lock; ignore .dev/out. `install
--locked` restores/verifies the pinned lock without rewriting it; missing HTTPS
checkouts require network. Run/build never fetch. Cached Git content changes are
errors; intentional local changes require install and a new lock. Local sources
use relative paths where possible. A lock hashes source bytes, so CRLF changes
matter. Dependencies are transitive with one flat namespace and conflict errors;
no registry, semver resolution or publication exists. std is reserved. No hooks
or package build scripts are intentionally run; symlinks/submodules unsupported.

`d fmt [FILES/DIRS]` defaults to the project module directory; `--check` changes
nothing and returns 1 if needed, `--stdout` takes exactly one file. The formatter
uses four spaces/LF and preserves tokens, comments and existing line breaks.
It validates all input before writes, and does not expand one-line bodies.

`d lsp --stdio` exposes full sync, parser/compiler-frontend diagnostics,
unused-variable hints, keywords/current-file
top-level completion, signatures/hover, local top-level definitions, outline and
formatting. Semantic checks include unsaved imported files; independent statement
errors are collected up to 100. Parser/declaration/generic-resolution failures
stop their phase at the first error; runtime-only JSON skips native semantics.
No persistent workspace index/cross-file navigation/local variable definitions/
rename yet. Never claim runtime validation from editor diagnostics. VS Code
source extension: editors/vscode; build npm ci/package, configure
devlang.executablePath to d and install the VSIX in a trusted workspace.

`d debug FILE [--debugger lldb|gdb] [--no-launch]` builds native out/debug/app[.exe]
with O0/DWARF/frame pointers. MSVC-targeting Windows Clang requires LLD to keep
DWARF section names. `--batch --command COMMAND` repeats scripted debugger
commands. Source #line mapping supports .dev breakpoints; symbols/locals still
have generated C names. This does not debug the source interpreter or JSON.
`d debug --vscode` writes launch/tasks only when absent. Install CodeLLDB plus the
DevLang extension for F5/.dev breakpoints. LLDB must have its runtime dependencies
on PATH (some Windows builds require a matching Python DLL). Verify a real
breakpoint with smoke_tooling.py --bin-dir target/release --lldb when available.
