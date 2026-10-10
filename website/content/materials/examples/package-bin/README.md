# Package bin commands

`bin` maps command names to `.dev` entry files, relative to the package root:

```don
package: { name: 'bin_app' }
bin: {
    hello: 'src/cli.dev'
}
dependencies: { math: { path: '../math' } }
devDependencies: { testkit: { path: '../testkit' } }
```

From the repository root:

```powershell
.\target\release\d.exe -C examples/package-bin/app pkg install
.\target\release\d.exe -C examples/package-bin/app pkg bin
.\target\release\d.exe -C examples/package-bin/app exec hello -- DevLang
.\target\release\d.exe -C examples/package-bin/app exec math-add
.\target\release\d.exe -C examples/package-bin/app exec dev-check
```

Outputs: hello prints `Hello from package bin` then `DevLang`; math-add prints `42`; dev-check prints `PASS`.

Install creates local launchers in `app/.dev/bin`: `.cmd` files on Windows and executable shell scripts on Unix. From the app directory:

```powershell
.\.dev\bin\hello.cmd DevLang
.\.dev\bin\math-add.cmd
```

Unix equivalent: `./.dev/bin/hello DevLang`. Launchers use the `d` executable that installed them. Reinstall if moving the project or its `d` executable. Ordinary install does not change global PATH.

## Global installation

```powershell
.\target\release\d.exe -C examples/package-bin/app pkg install -g
# Reopen the terminal application after the first PATH update.
hello DevLang
math-add
```

`--global` is an alias of `-g`. It installs the current package graph and exports its bins in `%USERPROFILE%\.devlang\bin` on Windows (or `~/.devlang/bin` on Unix). Windows adds that directory to the user PATH; Unix appends an export to `.bashrc`, `.zshrc` or `.profile` for the detected shell. A running parent shell cannot have its PATH changed by a child process; reopen the terminal application after first install. Existing PATH entries are retained.

Global installation copies the source graph into `.devlang/packages` and caches the runtime in `.devlang/runtime`. Commands can be invoked from any directory after moving/deleting the original project and runtime; their working directory is the snapshot root. Reinstall to pick up source edits. There is no public registry lookup.

Use `pkg install -g --production` to exclude development tools, `pkg update --global` to update dependencies and refresh exported bins, and `pkg uninstall --global` from this project to remove its global launchers. Uninstall leaves PATH, sources and caches intact. Another project's command with the same name and custom global launchers are not overwritten. `--global --workspace` is rejected; choose a member using `--package NAME` instead.

Set `DEVLANG_HOME` to an absolute custom home directory to relocate the global registry and bin directory. `DEVLANG_NO_PATH_UPDATE=1` skips profile/PATH changes when managing PATH yourself or testing. Verify global command lookup without changing real user settings:

```powershell
python scripts/smoke_global_bins.py --d target/release/d.exe
```

Commands from the current package and all installed dependencies are available. `exec` runs source with devrun and consumer-project module mappings, without compiling Dev source. Program arguments after the command name are forwarded literally; use `--` to clearly separate them. Global options such as `-C` and `--package` go before the command name. Child exit status is preserved. Working directory is the consumer project root.

`pkg install --production` removes generated launchers for development-only tools, so `dev-check` becomes unavailable; `hello` and `math-add` remain. Locked installs refresh launchers too. It does not delete dependency caches or overwrite custom files in `.dev/bin`.

Command names begin with an ASCII letter or underscore and may contain letters, digits, underscores and hyphens. Windows reserved device names are rejected. Duplicate names, including case-only differences, fail install. Entry paths must be `.dev` files inside their owning package; absolute paths, `..` and escaping symlinks are rejected. Local bins do not override dependency bins.

From any directory, use `d pkg list --global` and `d pkg uninstall --global bin_app`, including after deleting the original project. Uninstall removes exported commands but retains snapshots and runtime caches.
