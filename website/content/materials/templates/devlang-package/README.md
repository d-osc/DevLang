# DevLang package template

A small module named `math`. Rename the package namespace in `package.don` and imports when creating your own package. Repository names can contain hyphens; package namespaces must be valid DevLang identifiers.

## Layout

Keep `package.don` at the repository root. Put exported module declarations in `src/`, runnable examples in `examples/`, and checks in `tests/`. Runnable files explicitly call `main()`.

## Check locally

```sh
d pkg inspect --json
d examples/main.dev
d tests/main.dev
d run tests/main.dev --engine ast
```

Use a current DevLang checkout with `pkg inspect` support. See https://github.com/d-osc/DevLang for CLI installation/build instructions.

## Publish on GitHub

1. Create a public repository and upload this directory's contents at level 0.
2. Set a useful About description and add topics `devlang` and `devlang-package`. Optionally add `devlang-module` or `devlang-cli`.
3. Add a LICENSE file appropriate for your project and run the checks above.
4. Commit the source. Create and push tag `v1.0.0` for manifest version `1.0.0` (a leading `v` in the manifest is also accepted).
5. Visit https://d-osc.github.io/DevLang/#/packages. The scheduled index scans eligible public repositories every six hours, subject to GitHub's search indexing delay.

Install by replacing OWNER/REPOSITORY with your actual repository:

```sh
d pkg add math --git https://github.com/OWNER/REPOSITORY.git --tag v1.0.0
d pkg install --locked
```

Then use `use "math/lib" as math` in a consumer program. Store validation checks manifest and file layout, not package safety or test results. Git tags can move; keep releases immutable and commit the generated `package-lock.don` in consumer applications.
