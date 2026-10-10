# Workspace example

From the DevLang repository root:

```sh
d -C examples/workspace pkg workspace
d -C examples/workspace pkg install --workspace
d -C examples/workspace run --package app
d -C examples/workspace run --package app --engine ast
d -C examples/workspace pkg install --workspace --locked
```

The app prints `42`. Both source engines run without a C compiler. For a native executable use `d -C examples/workspace build --package app --release -o out/app`; native compilation needs a C backend. Paths after --package use the app directory. Regenerate local locks after changing source/newline bytes. The root's own entry is unused.

The math version `1.2.0` satisfies the app's `^1.0` constraint. Versions are root manifest fields. [The package guide](../../docs/packages.md) documents Git-tag ranges, lock pinning, group installation and current limits.
