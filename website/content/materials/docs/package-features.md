# Package tools (CLI 0.5.0)

These commands require a current source build of `d`. Runtime and compiler remain separate executables; their versions are independent of the CLI version. This update uses CLI 0.5.0, runtime 0.2.1 and compiler 0.4.1; build them together. Module mappings are passed via a JSON file (`--module-map FILE`) to avoid Windows command-line length limits. `--module-dir NAME=DIR` remains supported.

## Explicit scripts

```don
version: '1.0.0'
package: { name: 'app', entry: 'src/main.dev', modules: 'src' }
scripts: {
  start: ['d', 'run']
  check: ['d', 'check', 'src/main.dev']
  format: ['d', 'fmt', 'src']
  greeting: 'echo Hello'
}
```

```sh
d task
d start
d run start -- argument
d check
d format
d task start -- argument
d task check
d task format
```

Scripts can be invoked as `d NAME` or `d run NAME` (also `--run` / `-r`). A bare `d check` or `d format` runs the matching script when declared; `d check FILE.dev` keeps the built-in checker. Explicit `.dev` paths run source. Other built-in command names remain reserved; use `d task NAME` or `d run NAME` for collisions.

Array scripts launch an executable directly and forward arguments literally. `d` resolves to the running CLI. String scripts use the platform shell (`cmd` on Windows, `sh` elsewhere); argument forwarding requires an array. Scripts run in the package directory with `.dev/bin` added to PATH and preserve exit status. Installation never executes lifecycle scripts automatically.

## Dependency policy

```don
dependencies: { engine: { path: '../engine', version: '^1 || ^2' } }
optionalDependencies: { accelerator: { path: '../accelerator' } }
overrides: { math: { path: '../patched-math' } }
os: ['win32', 'linux', 'darwin']
cpu: ['x64', 'arm64']
```

`optionalDependencies` uses distinct aliases. A failed optional installation is reported and recorded in the lock's `omitted` section. Locked installs retain that omission; run a normal install to retry it. Changes to declarations or stale omissions invalidate the lock. An optional package's dependencies are ordinary required dependencies unless also declared optional.

Consumer-level `overrides` replaces dependencies with the same import alias throughout its graph. Override paths are relative to the consumer manifest and rebased for each child. Dependency constraints on the replaced declaration are replaced as well; test your chosen replacement. Overrides in a dependency's manifest do not alter the consumer's graph. Global snapshots preserve selected packages as local copies. Published archives omit consumer overrides and reject nonportable runtime/optional sources.

OS/CPU lists allow the current target when empty, when listed positively, or when using `any`. `!linux` excludes Linux. Exclusions win. Names follow Node-style `win32`, `darwin`, `linux`, `x64`, `arm64` (other Rust host names pass through). Policy checks the host running `d`, not a cross-compilation target.

## Ranges, version search and peers

Ranges support `^1.2`, `~1.2`, `=1.2.3`, `*`, `>=1.2, <2`, `>= 1.2 <2`, `1.2.0 - 2.3.0` and alternatives such as `^1 || ^2`. A partial hyphen upper bound expands to its next minor/major boundary. Prereleases require an explicit matching prerelease comparator. SemVer crate matching applies after normalization; this is not a promise of every npm edge-case rule.

Online installation searches Git tags, Store versions and static registry releases from newest to oldest, backtracking on version/peer constraints. Search defaults to 128 attempts; set `DEVLANG_RESOLUTION_ATTEMPTS` to an integer from 1 to 65536 to change the budget. Choices from abandoned branches are discarded. Transport, malformed-source and integrity errors propagate immediately. Locks retain the declared range and selected version, commit or actual registry URL/checksum. Locked and offline installs preserve the selection and do not search alternatives.

Version-2 graphs resolve peers from the nearest ancestor providers, with a package's own dependencies taking precedence. Version-1 locks use their flat namespace. When a shared package (including a wrapper with descendant peers) needs different ancestor providers, installation upgrades to a version-4 lock. Each dependency path can select its own compatible version, even with identical declarations. Runtime, native builds, locked installs and global launchers use physical instances under `.dev/contexts`. The lock's `resolutions` section retains every logical selection and original declaration; source hashes and pinned commits remain validated.

For example, both consumers can declare `plugin: { git: 'https://github.com/your-org/plugin.git', version: '^1 || ^2' }`. If plugin 1.1.0 requires peer `base: '^1'` and plugin 2.0.0 requires `base: '^2'`, a consumer providing base 1 selects plugin 1.1.0 while one providing base 2 selects plugin 2.0.0. No declaration workaround is needed. Git tags, Store versions and static registry releases use the same context-aware search.

Instances share a physical copy when their selected source and complete visible dependency scopes are equivalent. Comparing the complete scope preserves ancestor imports, including those not listed as peers; sharing is conservative and does not promise the smallest possible installation. Different providers keep separate copies. Graphs are bounded to 4096 logical instances, and copies/cache checkouts cost disk space. Ancestor cycles reuse an existing logical instance and must have compatible peer contexts. Version-1, version-2 and version-3 locks remain readable; frozen/offline installs preserve them, and a normal online install migrates version-3 contexts to version 4.

```sh
d pkg install --auto-peers
```

Peer installation is opt-in. It adds a missing peer to the root consumer's dependencies from the Package Store, then retries resolution. Set `DEVLANG_PEER_STORE` to a custom catalog for private/test stores. It does not replace already declared incompatible peers. A failed search restores the original consumer manifest.

## Frozen installs and reports

```sh
d pkg ci
d pkg ci --production
d pkg tree
d pkg why math
d pkg dedupe
```

`ci` installs/verifies exactly the matching lock; it does not wipe an existing installation or execute scripts. Production mode must match the lock. `tree` prints import paths, and `why` filters paths by alias, node ID or scoped Store identity. `dedupe` tries a compatible flat graph, falling back to isolated namespaces when sharing is impossible. It is a best-effort rewrite, not a global minimum-size solver.

## Cache and offline use

```sh
d pkg install --offline --locked
d pkg search math --offline
d pkg cache list
d pkg cache clean
```

HTTP(S) archives and metadata are cached under `DEVLANG_HOME/cache/downloads`, or `~/.devlang/cache/downloads`. Cached records bind URLs to SHA-256-verified blobs. Offline use prohibits Git clone/fetch and HTTP requests; it uses existing Git checkouts or previously downloaded metadata/archives. File URLs and local paths remain available. There is no shared global Git clone cache yet.

Cache cleaning removes only recognized regular download records/blobs. It preserves installed package snapshots, runtimes and project checkouts. Cache entries are not a cryptographic assertion about a publisher: an archive's declared checksum or a verified attestation remains the trust input.

## Advisories, signatures and provenance

```sh
d pkg audit
d pkg audit --database file:///absolute/path/advisories.don --offline
d pkg verify-tag v1.0.0 --key FULL_GPG_FINGERPRINT
d pkg verify math-1.0.0.tar.gz --github OWNER/REPOSITORY
```

Default audit queries [OSV's commit API](https://google.github.io/osv.dev/post-v1-query/) for public GitHub dependencies. Archive/local dependencies without coverage are `UNASSESSED`. Network/API errors fail the command. Exit codes: 1 for findings, 2 for unassessed dependencies, 0 for no known findings in the queried coverage. Zero findings is not a safety guarantee.

A custom DON/JSON database must explicitly declare coverage:

```don
schema_version: 1
coverage: ['@owner/math']
advisories: [
  { id: 'EXAMPLE-001', package: '@owner/math', affected: '<1.2.0', withdrawn: false }
]
```

`verify-tag` invokes Git/GPG and pins a full signing or primary key fingerprint; importing/trusting keys is your responsibility. `verify` delegates to [GitHub CLI attestation verification](https://cli.github.com/manual/gh_attestation_verify), restricts the repository and denies self-hosted runners. Verification errors propagate. Install does not automatically enforce signatures or attestations.

The package template's release workflow attests the exact archive/checksum bytes uploaded by `d pkg publish`. `--receipt FILE` writes their paths, hash, repository, tag and upload status for subsequent workflow steps. A dry-run receipt has `uploaded: false`; it is preparation metadata, not signed provenance. The workflow uses [actions/attest](https://github.com/actions/attest) to create the cryptographic attestation. Attestation failure requires repair even if release upload succeeded.
