# Workspaces and version requirements

For a complete runnable `package.don` example covering manifest fields, workspace and path dependencies, Git templates, DON references and lock commands, see [the full package example](../examples/package-don/README.md).

Use the current source-built `d`, `devrun` and `devc`. These additions extend the existing path/Git package manager; there is no public package registry or publish command yet.

## Workspace layout

A workspace root has `package.don` (or legacy `dev.toml`) with a normal package section and explicit member directories:

```don
version: '0.1.0'
package: { name: 'monorepo' }
workspace: {
  members: ['apps/app', 'libs/math']
}
```

Each member has its own manifest. Member names come from `package.name`, not directory names. For example `libs/math/package.don`:

```don
version: '1.2.0'
package: { name: 'math', entry: 'src/lib.dev', modules: 'src' }
```

`apps/app/package.don` links it without a relative dependency path:

```don
version: '0.1.0'
package: { name: 'app' }
dependencies: {
  math: { workspace: true, version: '^1.0' }
}
```

Source imports still use `use "math/lib"`. Workspace dependencies resolve only by the declared member name; they do not silently fall back to Git or a registry. Transitive member dependencies work. Only the root and its declared members may use workspace dependencies; cached external packages cannot borrow host-workspace members.

```sh
d -C examples/workspace pkg workspace
d -C examples/workspace pkg install --workspace
d -C examples/workspace run --package app
d -C examples/workspace build --package app --release -o out/app
d -C examples/workspace pkg install --workspace --locked
```

`--package NAME` selects a member and changes the command's working directory before execution. It works with run/build/check/emit and tooling commands such as fmt/pkg/debug. Relative source paths, outputs and program file I/O therefore use that member directory. `--package` after the program argument separator `--` is passed to the program. The root's own package name may also be selected.

`pkg workspace` lists members. `pkg add math --workspace [--version REQUIREMENT]` adds a member dependency; run it from the consumer or with `--package app`. Each member keeps its own `package-lock.don` and `.dev/packages` cache. `pkg install --workspace` / `pkg update --workspace` process the root and every member in name order. `--locked` is available only for install and needs existing locks for all packages.

Members must be 1–128 explicit relative directories with no `..`, globs, repeated/overlapping paths, symlinks, duplicate package names, or nested member workspaces. The root package must have a different name from its members. Its entry may be unused when selecting members.

Group installation is not a transaction across all packages: an error in a later member leaves earlier successful installs intact. Dependencies remain per-package; this is not a shared workspace-wide version solver or deduplicated global cache. Running/building does not install missing dependencies automatically.

## SemVer dependencies

Archive dependencies use `url: 'https://host/math.zip'` and a required `sha256` containing the archive's 64 hexadecimal SHA-256 characters. Supported formats are ZIP, TAR and TAR.GZ/TGZ; HTTP(S) and file URLs work. An optional `version` validates the extracted package version. Archive sources cannot use Git selectors. Use `pkg add NAME --url URL --sha256 HEX [--version REQUIREMENT]`. See [the complete archive URL example](../examples/archive-url/README.md) for packaging, checksums, install commands and a real localhost HTTP demo. Locked installs verify package content and can restore missing caches from checksum-verified downloads.

Git dependencies may select an explicit remote branch with `branch: 'main'` or `pkg add NAME --git URL --branch main`. Branch names such as `feature/new-api` are supported; invalid or missing branches fail. Branch selection resolves the remote branch even when a tag has the same name. `branch`, `tag` and dependency `version` are mutually exclusive. Initial install locks the branch commit; regular and locked installs preserve it, while `pkg update` selects the current branch commit.

Use `tag: 'v1.2.0'` or `pkg add NAME --git URL --tag v1.2.0` to select a Git ref. `rev` and `--rev` remain legacy aliases for existing manifests and locks; newly written manifests and locks use `tag`. Do not specify both names or combine `tag` with a dependency `version`.

The optional root manifest `version` supplies the package's version. A dependency may add `version` to path, Git or workspace sources:

```don
dependencies: {
  math: { git: 'https://github.com/your-org/math.git', version: '^1.2' }
  local: { path: '../local', version: '=1.0.0' }
}
```

```sh
d pkg add math --git https://github.com/your-org/math.git --version '^1.2'
d pkg add local --path ../local --version '=1.0.0'
```

The URL above is a placeholder. Git version resolution enumerates repository tags, parses `1.2.3` or `v1.2.3`, and selects the highest matching SemVer precedence. Non-version tags are ignored. Equivalent tags with the same precedence are rejected as ambiguous (including v/non-v duplicates and differing build metadata). The chosen tag must point to a package whose manifest version agrees with that tag. A root version prefixed with `v` is accepted and stored canonically without it.

| Requirement | Meaning |
|---|---|
| `=1.2.3` | Exact version |
| `^1.2`, `1.2.3` | Compatible range; bare versions use caret semantics |
| `~1.2` | Matching minor release range |
| `>=1.2, <2` | Intersection of comparators |
| `*`, `1.*` | Wildcard stable range |
| `=1.3.0-beta.1` | Explicit prerelease |

Requirements follow [the semver crate syntax](https://docs.rs/semver/latest/semver/struct.VersionReq.html); they are not npm's full range syntax. `||`, space-separated comparator intersections and hyphen ranges are unsupported. Ordinary ranges/wildcards do not automatically opt into prereleases. Requirements are capped at 256 bytes.

Local/workspace requirements validate the member's manifest version; they do not fetch another version. Git requirements select a tag during initial install or explicit `pkg update`. Regular install keeps a matching locked commit pinned. `--locked` restores the exact old commit and verifies its contents even if newer tags exist. `resolved_version` in package-lock.don records the selected version. `tag` remains available for arbitrary Git refs but cannot be combined with a dependency version.

Within one package graph, namespaces remain flat. Different version declarations for the same source can share an already selected version when it satisfies both requirements; all declarations are recorded in the lock. Different sources, incompatible constraints or a later constraint that excludes the selected version are rejected. There is no backtracking to a lower tag, multi-version namespace support, or npm-style global solver yet. Dependencies with and without version requirements for the same namespace are not merged.

`pkg add` / `pkg remove` restore the original manifest bytes if dependency installation fails. Completed cache fetches may remain; failed/verification fetch directories are retained for inspection. Successful mutation still rewrites DON formatting/comments as before.

## Package commands (`bin`)

```don
package: { name: 'my_cli' }
bin: { mycli: 'src/cli.dev' }
```

`pkg install` creates local launchers in `.dev/bin` (`NAME.cmd` on Windows; executable shell scripts on Unix). `d exec NAME [-- arguments]` runs a command from the current package or installed dependency with devrun and the consumer project's module mappings. `d pkg bin` lists commands. Program arguments are forwarded literally and child exit codes are preserved. Working directory is the consumer project root. No global PATH changes are made. Reinstall to regenerate launchers after moving the project or `d`.

Command names support ASCII letters, digits, underscore and hyphen, beginning with a letter or underscore; reserved Windows device names are rejected. Entries are `.dev` files inside the package. Duplicate names (case-insensitive), missing files and escaping paths fail install. Production installation removes generated development-only launchers but preserves custom files and cached dependencies. Locked installs refresh launchers. See [the complete bin example](../examples/package-bin/README.md).

## Development dependencies

`devDependencies` uses the same dependency source objects as `dependencies`:

```don
package: { name: 'app' }
dependencies: { math: { path: '../math' } }
devDependencies: { testkit: { path: '../testkit' } }
```

Normal `pkg install/update` includes the current package's development dependencies. Dependencies' own development dependencies are not installed transitively. `pkg install/update --production` excludes them. Each mode writes its graph to `package-lock.don`; production locks include `production: true`. `--locked` requires a matching mode, so switching modes needs an ordinary install first. Run/build/check and LSP mappings follow the lock's mode. Existing cached downloads are retained when switching to production.

Use `pkg add NAME SOURCE_OPTIONS --dev` and `pkg remove NAME --dev` to edit the development section. Add/remove installs in development mode and preserves manifest rollback on failure. Namespaces cannot appear in both sections. Workspace install/update supports `--production` for all packages. Peer dependencies remain unsupported. See [the runnable devDependencies example](../examples/dev-dependencies/README.md).

## Lock and content hashes

The canonical lock is `package-lock.don`, serialized as DON. Old `dev.lock` TOML files remain readable. A successful install (including `--locked`) creates the new DON file when migrating from the legacy lock, preserving resolved commits and content checks. The old file is retained; when both exist the new name takes precedence. New hashes exclude both lock names.

New dependency hashes exclude `package-lock.don` anywhere in the tree, alongside existing excluded build/cache directories. Locks describe resolution metadata, so changing a member lock does not invalidate every consumer or introduce a recursive workspace hash. Source files and manifests are still hashed by exact bytes. Older version-1 hashes that included lockfiles remain verifiable; locked installation preserves them. Ordinary install refreshes local hashes and migrates matching cached hashes to the new rule.

See [the complete workspace example](../examples/workspace/README.md) and `scripts/smoke_workspaces.py` for runtime/native builds, local Git version tags, pinned restoration, legacy hashes and failed-add rollback.
