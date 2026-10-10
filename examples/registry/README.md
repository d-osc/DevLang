# Static package registry

Create a registry outside the library source, then serve it over HTTP:

```powershell
d -C examples/registry/library pkg publish --registry ../repository
python -m http.server 8080 --directory examples/registry/repository
```

In another terminal, discover and install the library:

```powershell
d pkg search --registry http://localhost:8080/index.don math
d -C examples/registry/app pkg add math --registry http://localhost:8080/index.don --version '^1.2'
d -C examples/registry/app run
d -C examples/registry/app exec answer
d -C examples/registry/app pkg install --locked
```

Both programs print `42`. `pkg search` works without a project. Use a `file:///.../index.don` URL for a local registry without a server, or host the static files with HTTPS for remote consumers. Publish writes local files; it does not upload to a hosting provider.

`pkg add --registry` resolves the highest matching SemVer, verifies the archive SHA-256, then stores an exact version and archive URL/checksum in `package.don`. Installation, run/build and lock validation reuse existing archive dependency rules. Registry location and the original range are not retained as a live dependency source: `pkg update` keeps the chosen archive. Re-run `pkg add --registry ... --version ...` to select a newer registry release. `--dev` is supported; source options and `--peer` cannot be combined with `--registry`.

Published versions cannot be overwritten. Increment `version` to publish again. A package must have a version and module directory, and each declared bin must exist. Production dependencies must use HTTP(S) URL/checksum or a Git tag over HTTPS, not local paths, workspace members, Git branches or an unpinned Git source. The packed manifest removes `devDependencies` and workspace metadata; peer constraints remain. Sources, examples and other files are included, excluding `.git`, `.dev`, `.dev-cache`, build output, node_modules, Python caches and both lock names. Symlinks and special files are rejected. The publisher validates the archive against consumer extraction limits before updating the index. Failed publication removes staging and releases its writer lock; a process crash may leave `.publish.lock` for manual recovery after confirming no writer is active.

There is no hosted central registry, user accounts or remote upload protocol. Use HTTPS with a trusted registry: a checksum verifies bytes against the index, not the publisher's identity. Registry index schema:

```don
version: 1
packages: {
  math: {
    '1.2.0': {
      url: 'packages/math/1.2.0.tar.gz'
      sha256: 'GENERATED_64_HEX_DIGITS'
    }
  }
}
```

Relative archive URLs resolve against the index URL. The publisher generates the real checksum. Index downloads are limited to 1 MiB; package archives use the existing 64 MiB compressed, 128 MiB expanded, 16 MiB per file and 8192-entry limits. The example application needs the `pkg add` step before running.

Run isolated real HTTP verification without modifying this example:

```powershell
python scripts/smoke_registry.py --d target/release/d.exe
```
