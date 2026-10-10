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

Unix equivalent: `./.dev/bin/hello DevLang`. Launchers use the `d` executable that installed them. Reinstall if moving the project or its `d` executable. No global PATH changes or global install are performed.

Commands from the current package and all installed dependencies are available. `exec` runs source with devrun and consumer-project module mappings, without compiling Dev source. Program arguments after the command name are forwarded literally; use `--` to clearly separate them. Global options such as `-C` and `--package` go before the command name. Child exit status is preserved. Working directory is the consumer project root.

`pkg install --production` removes generated launchers for development-only tools, so `dev-check` becomes unavailable; `hello` and `math-add` remain. Locked installs refresh launchers too. It does not delete dependency caches or overwrite custom files in `.dev/bin`.

Command names begin with an ASCII letter or underscore and may contain letters, digits, underscores and hyphens. Windows reserved device names are rejected. Duplicate names, including case-only differences, fail install. Entry paths must be `.dev` files inside their owning package; absolute paths, `..` and escaping symlinks are rejected. Local bins do not override dependency bins.
