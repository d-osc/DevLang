# DevLang developer tools

These tools are included in the latest source build (`cargo build --release`),
not in the existing v0.4.0 installers. Keep `d`, `devrun`, `devc` together.
The tools are part of `d`; compiler and runtime remain separate executables.

## Projects and dependencies

```sh
d new hello
cd hello
d run
d build --release -o out/hello
d fmt --check
```

`d new` creates `package.don` using [Dev Object Notation](don.md). Legacy `dev.toml` manifests remain supported; DON takes precedence if both exist. The TOML example below is the legacy equivalent. Commands without a filename use its entry.
An explicit file still works. The CLI searches ancestor directories of the
entry file for a manifest; it never downloads dependencies while running/building.
Use `d -C PROJECT ...` to select a working directory.

```toml
[package]
name = "hello"
entry = "src/main.dev"
modules = "src"

[dependencies.math]
path = "../math"
```

Create `../math/dev.toml` with `[package]`, `name = "math"`,
`entry = "src/lib.dev"`, `modules = "src"`. Its `src/lib.dev` may contain
`fn answer() i64 { return 42 }`; imported files contain declarations only.
In the application's source use `use "math/lib"` and `print(lib.answer())`.
Dependency names define namespaces; `modules` defines their directory.
Entry/modules paths must be relative and cannot contain `..`.
Local dependency paths may use `..` and are resolved relative to their manifest.

```sh
d pkg add math --path ../math
# Git URL below is a placeholder for your actual library repository:
d pkg add math --git https://github.com/your-org/math.git --tag v1.0.0
d pkg install
d pkg install --locked
d pkg update
d pkg list
d pkg remove math
```

Dependencies need their own `package.don` or legacy `dev.toml`. Path and Git dependencies can have
transitive dependencies. Names form one flat namespace; conflicting versions or
local sources for the same name are rejected. `std` is reserved. There is no
public registry or package publishing yet. Workspace members and SemVer Git-tag
requirements are supported; see [Workspaces and dependency versions](packages.md)
for commands, lock behavior and the flat resolver limitations.
Git URLs accept HTTPS and file URLs; Git must be installed for Git dependencies.
Credential-bearing HTTPS URLs are rejected; use Git's existing credential setup.
Git dependencies are checked out in `.dev/packages/NAME-COMMIT`, without running
package install/build scripts. Submodules and symlink packages are unsupported.

Commit your manifest and `dev.lock`; ignore `.dev/` and `out/`. The versioned TOML
lockfile records source declarations, pinned Git commits, module directories and
SHA-256 hashes of package files (excluding .git/.dev/target/dist/out/node_modules and dev.lock files).
`pkg install` keeps existing matching Git commits pinned, resolves new sources,
and accepts intentional local content changes. `pkg update` resolves requested
Git refs again. `--locked` restores missing Git checkouts at the pinned commit
and verifies exact content without rewriting the lockfile; restoration needs
network for HTTPS sources. Cached dependencies work offline. Changed Git caches
are rejected: remove the affected checkout and reinstall. Local files with
different newline bytes have different hashes, so regenerate locks when changing
those bytes. Cross-drive local paths may remain absolute and are machine-specific.

`d run` and `d build` verify locked dependencies and supply `--module-dir` to
their separate engines. The runtime supports the same namespace mapping as the
compiler, including nested relative imports. Workers reuse already loaded modules.
Direct `devrun`/`devc` commands need explicit `--module-dir NAME=DIR`; they do not
read manifests. Namespace imports reject traversal and symlinks outside their root.

## Formatter

```sh
d fmt                     # project modules directory, normally src/
d fmt main.dev src/lib.dev
d fmt src examples
d fmt --check src          # exit 1 when changes are needed
d fmt --stdout main.dev    # exactly one file; leaves it untouched
```

Formatting uses four spaces, trims ordinary trailing whitespace and normalizes
line endings to LF with a final newline. It preserves comments, strings, existing
line breaks and tokens; it does not reflow expressions or expand one-line bodies.
All inputs are parsed and token-checked before any file is written. Syntax errors
leave files unchanged. Directory traversal skips symlinks, .git, .dev, target,
dist, out and node_modules. `--check` works in CI without changing source.

## Language server and VS Code

`d lsp --stdio` implements JSON-RPC with LSP Content-Length framing and UTF-16
positions. Standard output contains protocol messages only. Full document sync
supports didOpen/didChange/didClose; stale versions are ignored.

| Capability | Current boundary |
| --- | --- |
| Diagnostics | Lexer/parser errors, compiler frontend type/name/argument/return checks and unread-variable hints; cleared after fixes/close |
| Completion | Keywords, types and current-document top-level functions/types |
| Hover | Current-document top-level function signatures and types |
| Go to Definition | Current-document top-level declarations |
| Outline | Top-level functions/structs/enums |
| Format Document | Same formatter as `d fmt`, fixed four-space indentation |

The language server uses the compiler frontend to check unsaved editor text and
its imported modules. It checks mismatched variable/argument/return types, unknown
names, duplicate declarations, conditions, operators, typed-literal ranges and
unsafe operations. Changing or closing an imported document refreshes other open
documents using the latest editor text or the file on disk. Diagnostics never
execute code, invoke a C compiler, or write edited source to disk.

Independent statement errors are collected, capped at 100. Parser errors and
declaration/generic resolution errors still stop their phase at the first error.
Runtime-only JSON files retain parser/unused checks but skip native semantic
checks. This does not yet provide a persistent incremental workspace index,
cross-file navigation, local-variable definition lookup, rename, signature help
or semantic tokens. Unknown requests return protocol errors.

The extension is published as [`n-devs.devlang-language`](https://marketplace.visualstudio.com/items?itemName=n-devs.devlang-language).
Install it from the Marketplace or with `code --install-extension n-devs.devlang-language`.
Its source is in `editors/vscode`.
Build a VSIX from that directory with Node.js 22 or newer,
`npm ci`, `npm run check`, `npm run package`.
Install via **Extensions: Install from VSIX**. Configure
`devlang.executablePath` with the latest absolute `d` path, or put the directory
containing all three binaries on PATH. Open a trusted project and a `.dev` file.
The extension supplies syntax highlighting, LSP, formatting and .dev breakpoints.
When Material Icon Theme is active in a trusted local workspace, the extension
automatically associates its .dev SVG icon using Material's custom file settings.
Other icons and existing custom .dev mappings are preserved. Disable
`devlang.autoFileIcon` in User Settings to remove the generated association.
Other themes can use the language's default icon when supported or explicitly
select the separate DevLang File Icons theme.
**DevLang: Restart Language Server** reconnects with the configured executable;
reload the extension host after changing its executable path.

## Native debugger integration

```sh
d debug src/main.dev                    # build, then launch installed LLDB
d debug src/main.dev --debugger gdb
d debug src/main.dev --no-launch        # build only; print executable path
d debug src/main.dev -- argument1
d debug --vscode                        # create .vscode/tasks.json and launch.json
```

`d debug` creates `out/debug/app[.exe]` using the native compiler with `-O0`,
DWARF debug symbols and frame pointers. `d build --debug` uses the same symbols.
Windows Clang targeting MSVC requires LLD (`-fuse-ld=lld`, `/debug:dwarf`), because
link.exe truncates DWARF section names. MinGW GCC and Linux Clang/GCC use their
normal linkers. `--release` and TinyCC `--fast` are not debug-symbol builds.
Source mapping uses function and statement `#line` directives. Local variables
and non-exported functions currently retain generated C names; helper frames and
less precise locations can appear when stepping over generated cleanup code.

Install a working LLDB/GDB for CLI debugging. Some Windows LLVM distributions
need a matching Python DLL on PATH (the tested LLVM 18.1.7 needed Python 3.10).
The launcher forwards debugger exit status and does not install external tools.
For scripted checks, `--batch` and repeated `--command "DEBUGGER COMMAND"` run
commands before/while launching under the selected debugger.

For VS Code install [CodeLLDB](https://github.com/vadimcn/codelldb) plus the
DevLang extension, run `d debug --vscode`, select the `.dev` entry file and press
F5. Existing launch/tasks files are never overwritten. CodeLLDB provides the
adapter; DevLang supplies build configuration, symbols and source locations.
Debugging currently covers native builds only: the source interpreter, JSON
runtime-only programs and an interpreter DAP adapter are not supported yet.

Protocol and debugger integration follow the [LSP specification](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
and [CodeLLDB configuration](https://github.com/vadimcn/codelldb/blob/master/MANUAL.md).

## Validation

`python scripts/smoke_tooling.py --bin-dir target/release` exercises new projects,
path/transitive and local-Git dependencies, pinned updates/tamper checks, runtime
and native execution, formatter idempotence/token preservation, LSP requests and
notifications, non-overwriting VS Code templates, and debug symbols when
llvm-dwarfdump is available. Add `--lldb` to require an actual source breakpoint
and local-value inspection through the CLI with an installed working LLDB.

DON editor support: [VS Code and lossless formatting](don.md#vs-code-support).
The LSP dispatches `.don` to the DON parser and keeps DevLang semantic checks separate.
