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

## Store integration

```sh
d pkg search math
d pkg info @OWNER/REPOSITORY
d pkg add @OWNER/REPOSITORY --as math --version '^1'
d pkg outdated
d pkg upgrade math
d pkg upgrade math --latest --dry-run
```

`@OWNER/REPOSITORY` is a scoped GitHub identity; `math` is the local import alias. Replace OWNER/REPOSITORY with your repository. The generated lock pins the installed commit. Store installation stops if a tag moved from the commit checked by the catalog.

The optional `devlang` manifest field is a SemVer requirement on the **d CLI version**, independent of the language/compiler release number. `d pkg compatibility --json` checks it. Source inspection does not reject a different compatible range, but install/run does.

## CI and GitHub Releases

The included `.github/workflows/devlang-package.yml` runs tests in both engines on main/PRs. Pushing a matching `vVERSION` tag runs the checks and publishes a tar.gz plus sha256.txt via `d pkg publish --github "$GITHUB_REPOSITORY"`. The release job has contents:write; other jobs have read-only access. It builds the current DevLang checkout; pin the toolchain checkout to a commit for reproducible CI.

Manual preparation and upload:

```sh
d pkg publish --github OWNER/REPOSITORY --dry-run
d pkg publish --github OWNER/REPOSITORY
```

The actual upload requires gh authentication, a clean committed source tree, and the matching local tag already pushed to the selected GitHub repository. Existing releases/assets are not overwritten. Dry-run retains prepared files in a temporary directory and uploads nothing.
