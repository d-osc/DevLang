# GitHub Package Store

DevLang's [Package Store](https://d-osc.github.io/DevLang/#/packages) discovers GitHub repositories, validates manifest/source layout and provides Git-tag installation commands. It is a catalog, separate from the archive registry supported by `d pkg publish` and `d pkg search --registry`.

## Repository contract

- One package per public repository. `package.don` must be a regular file at repository level 0.
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

Monorepos may use DevLang workspaces locally. Git dependencies currently do not select a package subdirectory, so publish store packages in separate repositories with root manifests.

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

The crawler checks a default-branch commit snapshot and up to 30 tags returned by GitHub. Matching versions are sorted by SemVer; the highest stable version is preferred over prereleases. A valid source repository without accepted tags is shown without an installation command. The default crawl limit is 100 repositories, configurable up to GitHub's 1,000-result search ceiling. A partial-results flag is shown on the page when applicable.

The browser reads static `packages.json`; it receives no GitHub credentials and does not make unauthenticated API requests. “Reload list” reloads the published snapshot, not GitHub's API. API failure stops deployment and keeps the previously deployed site available. Individual invalid repositories are listed in the catalog's `rejected` diagnostics.

## Trust boundary

A topic is self-assigned metadata, not a security certification. Validation confirms manifest structure, source layout and tag/version agreement. It does not run package code, confirm tests, verify author identity or guarantee safe dependencies. Read source, ownership and license before installing. Git tags can move; the catalog shows the commit checked at crawl time, and your lock records the commit actually installed.

Crawler manifest parsing uses the real `d pkg inspect --json` parser in a temporary directory. No remote source or build scripts are executed. It rejects symlink manifests and source entries, manifests over 64 KiB and truncated Git trees.

## Maintain the catalog locally

```sh
cargo build --release -p dev-cli
python scripts/build_package_store.py --d target/release/d
python website/build.py --skip-validation
```

On Windows, pass `target/release/d.exe`. An optional `GITHUB_TOKEN` or `GH_TOKEN` increases the API allowance; keep it out of files and the browser. Optional `--repo owner/repo` seeds still must meet all public/topic/root/tag requirements. The JSON catalog is not a `d pkg search --registry` archive index.
