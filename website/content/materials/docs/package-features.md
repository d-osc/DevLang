# Package tools (CLI 0.4.0)

These commands require a current source build of `d`. Runtime and compiler remain separate executables; their versions are independent of the CLI version. This update uses CLI 0.4.0, runtime 0.2.1 and compiler 0.4.1; build them together. Module mappings are passed via a JSON file (`--module-map FILE`) to avoid Windows command-line length limits. `--module-dir NAME=DIR` remains supported.

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
d task start -- argument
d task check
d task format
```

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

Version-2 graphs resolve peers from the nearest ancestor providers, with a package's own dependencies taking precedence. Version-1 locks use their flat namespace. When a shared package (including a wrapper with descendant peers) needs different ancestor providers, installation upgrades to a version-3 lock. Each dependency path gets a physical copy under `.dev/contexts`, and runtime, native builds, locked installs and global launchers use that context. Source hashes and original declarations remain validated. Copies cost additional disk space; graphs are bounded to 4096 instances. Ancestor cycles reuse the existing instance and must have compatible peer contexts. Version-1 and version-2 locks remain supported.

Context instances share the selected source version for an identical normalized dependency declaration. They isolate peer providers, but do not independently select different package versions per peer context. If one selected plugin version cannot accept both providers, declare distinct compatible plugin version ranges in the two consumers.

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
