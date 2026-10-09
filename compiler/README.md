# Dev compiler

This Rust crate contains the type checker, C emitter and native
build/link driver. It has no dependency on the Dev runtime or its source files.

Build from the repository with `cargo build --release`, or copy this directory
and sibling `syntax/`, then run `cargo build --release` there. The syntax crate
provides the shared lexer and parser; neither runtime nor stdlib source is needed. Compiling Dev programs also requires a C backend.

```sh
devc run main.dev
devc build main.dev --release
devc build hardware.dev --lib --freestanding
```

External modules and libraries are generic, explicit inputs:

```sh
devc run main.dev --module-dir mylib=/path/to/modules --link /path/to/library.a
```

`--module-dir NAME=DIR` maps `use "NAME/module"` to `DIR/module.dev` and can be
repeated for independent namespaces. Relative file imports continue to work.
`check` needs only module declarations; `emit` outputs Dev-generated C only.
Neither operation reads runtime C sources, headers or libraries. Linking the
emitted C requires the same explicitly selected external libraries.

There is no reserved runtime namespace, runtime auto-discovery, `DEV_RUNTIME`
environment variable or automatic runtime compilation. Applications choose their
libraries through normal module lookup and C ABI linkage.
