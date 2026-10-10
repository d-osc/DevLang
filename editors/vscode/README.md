# DevLang for VS Code

Extension ID: `n-devs.devlang-language`. Build the latest repository runtime
with `cargo build --release`, then configure `devlang.executablePath` to the
absolute `target/release/d[.exe]` path (or put its containing directory on PATH).
All three executables, d/devrun/devc, should stay together.

From this folder with Node.js 22 or newer: `npm ci`, `npm run check`, `npm run package`.
Install the resulting `.vsix` with VS Code's **Extensions: Install from VSIX**.
Open a trusted project folder and a `.dev` or `.don` file to start `d lsp --stdio`.
Use Format Document, completion, hover, Go to Definition and Outline.
On Windows, Shift+Alt+F runs Format Document using DevLang by default.
Formatting calls `d fmt --stdout` independently of the language server and
includes unsaved edits. Configure `devlang.executablePath` to a current CLI
with `fmt` support; the CLI is not bundled in this extension.
When Material Icon Theme is active, DevLang automatically adds its .dev/.don SVG icons
through that theme's supported custom file association setting. Other icons and
explicit custom .dev/.don associations are preserved. The SVG is stored in a sibling
devlang-file-icons directory under the installed extensions directory, so Material
updates do not delete it. Disable `devlang.autoFileIcon` in User Settings to remove
the automatic association. Integration activates in trusted local workspaces at
startup and refreshes when the theme or installed extensions change.
Other icon themes can select **Preferences: File Icon Theme** → **DevLang File Icons**.
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

## Dev Object Notation (.don)

Version 0.2.0 registers **Dev Object Notation** (language ID `don`), with its own
file icon and syntax colors for keys, strings, triple-double-quoted strings,
numbers, booleans/null, comments, escapes and `@root.path` references. A DON
language configuration provides bracket/quote pairs and comment toggling.

Format Document / Shift+Alt+F uses `d don fmt-source` on the **unsaved text**.
It keeps comments, references, key order, separators and quoted/multiline string
contents; indentation uses four spaces and line endings outside literal tokens
use LF. Invalid input returns an error without applying edits. The older
`d don fmt` still serializes resolved data and discards comments/references.
DON support needs the updated `d` executable; the extension does not bundle it.

The same LSP serves DevLang and DON. DON diagnostics validate syntax, duplicate
keys, escapes, nesting/size budgets and missing/cyclic references. They refresh
on edits and clear on fixes/close. Outline shows nested keys/arrays; hover shows
value kinds; Go to Definition follows references inside the document; completion
suggests true/false/null and root-reference paths. Untitled DON documents also
work. There is no generic schema validation, cross-file DON reference/import,
rename or evaluation of expressions. Parser diagnostics report the first error.

Material Icon Theme receives `.don` independently of `.dev`, retaining explicit
user mappings. Other icon themes use the contributed language icon if supported,
or select **DevLang File Icons** manually; the extension does not switch themes.

Run `node --test tests/*.test.js` after building d, or set DEVLANG_TEST_BIN to it.
The suite checks real TextMate tokenization, icon integration, and the formatter
provider against the real CLI. `python scripts/smoke_don_editor.py --bin PATH`
from the repository root checks the LSP wire protocol and UTF-16 diagnostics.
