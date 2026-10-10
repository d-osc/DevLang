# Project and local dependency example

From the DevLang repository root with the newest d/devrun/devc on PATH:

```sh
d -C examples/tooling/app pkg install
d -C examples/tooling/app run
d -C examples/tooling/app fmt --check
d -C examples/tooling/app build --release -o out/app
d -C examples/tooling/app debug src/main.dev
```

Output: `42`. The app imports a declaration-only library through the `math`
namespace, mapped from its manifest. The checked-in lockfile captures the library
contents; run `pkg install` to record intentional local edits, or `pkg install
--locked` to verify unchanged source. Git dependencies use the same import syntax.
See [Developer tools](../../docs/tooling.md) for Git locks, LSP and VS Code setup.
