# Peer dependencies

The plugin declares `peerDependencies: { math: '^1.2' }`. Its consumer explicitly declares math in `dependencies`. Peer dependencies are version constraints, not download sources; they are never fetched automatically.

```powershell
.\target\release\d.exe -C examples/peer-dependencies/app pkg install
.\target\release\d.exe -C examples/peer-dependencies/app run
.\target\release\d.exe -C examples/peer-dependencies/app pkg install --locked
```

Output: `42`. Removing the math declaration or changing math to `2.0.0` makes install fail. All installed packages' peers are checked after graph resolution and again before run/build/check/LSP mappings. Providers must declare a valid SemVer package version. The current root package can provide a peer using its own package name/version. Development dependencies can provide peers only in development mode; production install rejects missing providers.

Edit root constraints with `d pkg add math --peer --version '^1.2'` and `d pkg remove math --peer`. The provider must already be available in the current graph when adding, or the original manifest is restored. Package source options and `--dev` cannot be combined with `--peer`.

Optional peer metadata, automatic peer installation and multi-version solving are not implemented.
