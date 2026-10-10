# DevLang for VS Code

Extension ID: `n-devs.devlang-language`. Build the latest repository runtime
with `cargo build --release`, then configure `devlang.executablePath` to the
absolute `target/release/d[.exe]` path (or put its containing directory on PATH).
All three executables, d/devrun/devc, should stay together.

From this folder with Node.js 22 or newer: `npm ci`, `npm run check`, `npm run package`.
Install the resulting `.vsix` with VS Code's **Extensions: Install from VSIX**.
Open a trusted project folder and a `.dev` file to start `d lsp --stdio`.
Use Format Document, completion, hover, Go to Definition and Outline.
On Windows, Shift+Alt+F runs Format Document using DevLang by default.
Formatting calls `d fmt --stdout` independently of the language server and
includes unsaved edits. Configure `devlang.executablePath` to a current CLI
with `fmt` support; the CLI is not bundled in this extension.
For file icons, select **Preferences: File Icon Theme** → **DevLang File Icons**.
The language also supplies a default .dev icon; other icon themes may override it.
DevLang syntax colors apply only to DevLang token scopes, keeping your existing
VS Code theme. Keywords are lavender, strings green, numbers peach,
functions gold and types mint. Override these using editor.tokenColorCustomizations.
Diagnostics cover lexer/parser errors, compiler frontend semantic checks and
unread `let` variables. Type mismatches, unknown names, duplicate declarations,
invalid arguments/returns/conditions, invalid operations, unsafe usage and
out-of-range typed literals are checked against the same rules as native builds.
Checks use unsaved editor text, including open imported files, without compiling
or running code. Editing an imported file also refreshes dependent open files.
Independent statement errors are collected (up to 100); parser and declaration/
generic resolution failures currently stop their phase at the first error.
Runtime-only JSON files are excluded from native semantic checks; parser checks
and unused-variable hints remain available. This is not a full TypeScript-style
incremental type checker or runtime memory-safety proof.
Unused
declarations appear faded when editor.showUnused is enabled (the DevLang default).
This needs the latest `d lsp`; rebuilding/updating the extension alone does not
update the CLI. Names beginning with `_` suppress unused hints. Analysis respects
blocks, parameters, closure captures and match bindings; it does not analyze
control-flow liveness or report unused parameters.
Symbols, hover and
definition cover top-level functions and types in the current open document;
there is no persistent workspace index, cross-file navigation or rename yet.
Formatting preserves tokens/newlines and uses four spaces; it does not reflow
expressions or convert a one-line body into multiple lines.

For native debugging install [CodeLLDB](https://github.com/vadimcn/codelldb),
run `d debug --vscode` from a project root and press F5 with the `.dev` entry file
active. Existing launch/tasks files are never overwritten. The DevLang extension
enables .dev breakpoints; CodeLLDB owns the debug adapter. A Clang/GCC C backend
is required. Native symbols use generated names for locals and functions, and
generated helpers may appear while stepping. Runtime/JSON programs cannot use
this native debugger. CLI `d debug FILE.dev` launches an installed LLDB;
`--debugger gdb` selects GDB instead.
