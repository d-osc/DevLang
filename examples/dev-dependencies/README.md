# Development dependencies

From the repository root:

```powershell
.\target\release\d.exe -C examples/dev-dependencies/app pkg install
.\target\release\d.exe -C examples/dev-dependencies/app run
.\target\release\d.exe -C examples/dev-dependencies/app run tests/main.dev
.\target\release\d.exe -C examples/dev-dependencies/app pkg install --locked
```

The app prints `42`; the test program prints `PASS`. The app's `dependencies` contains math; its `devDependencies` contains testkit. Both are available after a normal install. Each supports path/Git/archive/workspace sources and the same version checks.

```don
package: { name: 'dev_dependencies_app' }
dependencies: { math: { path: '../math' } }
devDependencies: { testkit: { path: '../testkit' } }
```

Only the current project's development dependencies are included. A dependency's own `devDependencies` are not installed transitively. The math example declares a deliberately unavailable private development tool to demonstrate this; installing math itself in development mode needs that private package, while consuming math works without it.

Production installation:

```powershell
.\target\release\d.exe -C examples/dev-dependencies/app pkg install --production
.\target\release\d.exe -C examples/dev-dependencies/app run
.\target\release\d.exe -C examples/dev-dependencies/app pkg install --production --locked
```

Production still prints `42`. The testkit namespace is absent, so running `tests/main.dev` in this mode fails. The generated `package-lock.don` records `production: true`; a development lock and a production lock describe different graphs. Switching modes requires a normal install first. `--locked` requires the requested mode to match the lock. Production mode does not delete previous cached downloads.

Return to development mode with `pkg install`. Add/remove tools using:

```powershell
d pkg add testkit --path ../testkit --dev
d pkg remove testkit --dev
```

Run these two commands from the app directory. Add/remove rewrites the manifest and installs in development mode. A namespace cannot be declared in both dependencies sections. `pkg install/update --workspace --production` applies production mode to each selected package. Peer dependencies validate a consumer-supplied version; see `../peer-dependencies`.
