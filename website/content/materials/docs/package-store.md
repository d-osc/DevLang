# GitHub Package Store

DevLang's [Package Store](https://d-osc.github.io/DevLang/#/packages) discovers GitHub repositories, validates manifest/source layout and provides Git-tag installation commands. It is a catalog, separate from the archive registry supported by `d pkg publish` and `d pkg search --registry`.

## Repository contract

- A public repository must have a regular `package.don` at repository level 0. It may contain one package or declare explicit workspace members.
- Add the `devlang-package` topic in GitHub's About settings. `devlang`, `devlang-module` or `devlang-cli` describe the ecosystem/package kind; only `devlang-package` controls discovery.
- Use a valid package namespace, e.g. `math`. Repository names such as `devlang-math` can differ from the namespace.
- The configured `package.modules` directory must contain a regular `.dev` file. Every declared `bin` file must exist.
- Give the repository a README, useful About description and LICENSE. The Store reads description/license from GitHub, not unsupported manifest properties.
- Use SemVer in top-level `version` and a matching tag such as `v1.0.0` or `1.0.0`. A leading `v` in the manifest version is accepted. The namespace must match between the default branch and each accepted tag.
- Forks, archived repositories and private repositories are excluded.

```text
devlang-math/
├── package.don
├── src/lib.dev
├── examples/main.dev
├── tests/main.dev
├── README.md
└── LICENSE
```

```don
version: '1.0.0'
package: {
  name: 'math'
  modules: 'src'
}
```

Download the [starter template](https://d-osc.github.io/DevLang/downloads/devlang-package-template.zip), extract it into your repository root, and replace the sample namespace and LICENSE copyright. It includes a working library, example and test.

Monorepos declare explicit `workspace.members` in the root manifest. Each member needs its own versioned `package.don` and `.dev` module source. The root may be an aggregator without source/version. Nested manifests alone are not discovered.

```don
package: { name: 'workspace' }
workspace: { members: ['packages/math', 'packages/http'] }
```

A member named `math` in `packages/math` is indexed as `@owner/repo/math` with `subdir: 'packages/math'`. Publish a matching `math/v1.0.0` tag for its `version: '1.0.0'`. Namespace tags separate member releases. Install with `d pkg add @owner/repo/math --version '^1'`; the CLI carries the checked subdirectory automatically. A root package continues to use `@owner/repo` and `v1.0.0` tags. The GitHub archive publisher currently publishes a standalone root package; workspace Store members install from Git tags.

## Validate and release

```sh
d pkg inspect --json
d examples/main.dev
d tests/main.dev
git add .
git commit -m "Release 1.0.0"
git tag v1.0.0
git push origin main --tags
```

`pkg inspect` validates the manifest schema, dependency selectors and SemVer without installing dependencies or running package code. Source existence checks are performed separately by the Store crawler. This command requires a current CLI built from source; older released binaries may not include it.

## Install a package

Replace OWNER/REPOSITORY with the actual repository shown in the Store:

```sh
d pkg add math --git https://github.com/OWNER/REPOSITORY.git --tag v1.0.0
d pkg install --locked
```

For this template, use `use "math/lib" as math` in your program. Keep the generated `package-lock.don` in consumer applications; it records the resolved Git commit. CLI packages can declare `bin`; see the [package reference](packages.md) for global installation.

## Discovery and identity

The Pages workflow rebuilds the catalog on website updates, manual dispatch and every six hours (UTC schedule). GitHub topic search indexing may delay discovery. The scheduled workflow can also be delayed by GitHub; this is not an immediate submission API.

The key is `owner/repo`, not just the namespace. Repositories can share namespaces; the Store displays ownership so consumers can choose explicitly. The initial catalog may be empty until eligible repositories exist. No sample packages are presented as published packages.

The crawler checks a default-branch commit snapshot and up to 400 tags returned by GitHub. Matching versions are sorted by SemVer; the highest stable version is preferred over prereleases. A valid source repository without accepted tags is shown without an installation command. The default repository budget is 10,000; searches exceeding 1,000 matches split into disjoint creation-date shards. A partial-results flag reports budget truncation, incomplete GitHub responses or shards exceeding 1,000 repos on a single day.

The browser reads static `packages.json`; it receives no GitHub credentials and does not make unauthenticated API requests. “Reload list” reloads the published snapshot, not GitHub's API. API failure stops deployment and keeps the previously deployed site available. Individual invalid repositories are listed in the catalog's `rejected` diagnostics.

## Trust boundary

A topic is self-assigned metadata, not a security certification. Validation confirms manifest structure, source layout and tag/version agreement. It does not run package code or verify author identity or guarantee safe dependencies. Read source, ownership and license before installing. Git tags can move; the catalog shows the commit checked at crawl time, and your lock records the commit actually installed.

Crawler manifest parsing uses the real `d pkg inspect --json` parser in a temporary directory. No remote source or build scripts are executed. It rejects symlink manifests and source entries, manifests over 64 KiB and truncated Git trees.

## Maintain the catalog locally

```sh
cargo build --release -p dev-cli
python scripts/build_package_store.py --d target/release/d
python website/build.py --skip-validation
```

On Windows, pass `target/release/d.exe`. An optional `GITHUB_TOKEN` or `GH_TOKEN` increases the API allowance; keep it out of files and the browser. Optional `--repo owner/repo` seeds still must meet all public/topic/root/tag requirements. The JSON catalog is not a `d pkg search --registry` archive index.

## CLI search, scoped identity and upgrades

Use CLI 0.5.0 and the runtime built from current source. Component versions are independent of the compiler/language release. See [Package tools](package-features.md) for scripts, peer installation, resolution, offline and security checks.

```sh
d pkg search math
d pkg info @OWNER/REPOSITORY
d pkg add @OWNER/REPOSITORY --as math --version '^1'
d pkg outdated
d pkg upgrade math --dry-run
d pkg upgrade math
d pkg upgrade math --latest
```

`--store URL` selects another catalog. Names without owners work only when unique. `@owner/repository` is ownership identity, while `math` is the local import alias. `@owner/namespace` also works if unique; use the repo identity when ambiguous. Manifests retain the Git URL, Store URL, scoped package identity and requested version range. Store resolution selects only validated catalog versions and rejects moved tags. Cached locked installs do not require Store access.

`outdated` shows CURRENT/WANTED/LATEST for direct runtime/dev dependencies with Git, Store or tracked registry sources. `upgrade` preserves ranges; `--latest` opts into the newest stable major with a caret range. Dry-run reads metadata without changing manifest/lock. `pkg update` resolves Git ranges again; `upgrade` also refreshes archives using saved registry/range metadata. Older archive manifests can be re-added to track a registry.

## Documents, quality and indexing status

The package page reads README.md, API.md, CHANGELOG.md and Markdown in docs/ from the checked default-branch commit and indexed release commits. Select a release to read its own snapshot. The crawler snapshots documents for up to ten newest versions by default; older/deferred versions link to the exact GitHub commit. Workspace documentation is scoped to the member directory. Raw HTML and remote image loading are disabled; links resolve to GitHub. Documents load separately from the catalog. Search results have 12 cards per page.

The [repo status page](https://d-osc.github.io/DevLang/#/packages/status) shows passed, rejected and deferred repos, invalid-tag reasons, snapshot time and cache/request counts. A missing repo may lack the topic, be excluded, await GitHub indexing or exceed the search cap. It does not imply rejection if absent.

CI badges report GitHub checks at the default-branch snapshot: passed, failed, pending, skipped, unknown, unavailable or partial. They do not imply release-tag testing or certify safety. Optional top-level `devlang: '^0.5.0'` is a d CLI SemVer requirement. `d pkg compatibility --json` reports compatibility; install/run enforce it. Manifest inspection remains available for incompatible packages.

## Cached crawling and bounds

`--cache-dir DIR` stores public API data with ETag revalidation and immutable commit caching. Pages restores this cache between runs. The default crawl request budget is 900; remaining repos are deferred, not marked invalid. `--max-repos` supports 1–10,000, `--max-tags` 1–10,000 (default 400), and `--max-version-docs` 0–400 (default 10). Each snapshot reads up to ten regular UTF-8 Markdown files of 64 KiB each. Creation-date sharding works around a single 1,000-result search, but the overall request/time budget still limits a crawl. A truncated tag list can omit old versions. API failures preserve the previous deployment.

## GitHub Releases

```sh
d pkg publish --github OWNER/REPOSITORY --dry-run
d pkg publish --github OWNER/REPOSITORY
```

The publisher prepares and verifies tar.gz and SHA-256 files. Actual upload uses authenticated gh, requires clean committed source, and verifies matching local/remote tags identify HEAD. It never overwrites existing releases/assets. Dry-run uploads nothing and retains prepared files for inspection. The template tests both engines, publishes tag releases after success, then attests the exact uploaded assets. See [signature and provenance verification](package-features.md#Advisories-signatures-and-provenance). Store crawling never runs repo source or uploads on owners' behalf.
