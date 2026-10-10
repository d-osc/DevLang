#!/usr/bin/env python3
"""Publish separate components and the unified d command bundle."""
import argparse
import hashlib
from pathlib import Path
import shutil


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--runtime-library", required=True)
    parser.add_argument("--runtime", required=True)
    parser.add_argument("--cli", required=True)
    parser.add_argument("--platform", required=True, choices=("windows-x86_64", "linux-x86_64"))
    parser.add_argument("--backend")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    compiler = repo / "dist/compiler" / args.platform
    runtime = repo / "dist/stdlib" / args.platform
    compiler.mkdir(parents=True, exist_ok=True)
    runtime.mkdir(parents=True, exist_ok=True)
    shutil.copy2(args.compiler, compiler / ("devc.exe" if args.platform.startswith("windows") else "devc"))
    shutil.copy2(repo / "compiler/README.md", compiler / "README.md")
    shutil.copy2(repo / "LICENSE", compiler / "LICENSE")
    if args.backend:
        shutil.copytree(args.backend, compiler / "backend", dirs_exist_ok=True)
    for directory in ("include", "modules", "src", "tests"):
        shutil.copytree(repo / "stdlib" / directory, runtime / directory, dirs_exist_ok=True)
    for name in ("build.py", "README.md"):
        shutil.copy2(repo / "stdlib" / name, runtime / name)
    shutil.copy2(repo / "LICENSE", runtime / "LICENSE")
    shutil.copy2(repo / "docs/stdlib.md", runtime / "API.md")
    (runtime / "lib").mkdir(exist_ok=True)
    shutil.copy2(args.runtime_library, runtime / "lib/libdevruntime.a")
    metadata = Path(args.runtime_library).with_suffix(".json")
    if metadata.is_file():
        shutil.copy2(metadata, runtime / "lib/libdevruntime.json")
    interpreter = repo / "dist/runtime" / args.platform
    interpreter.mkdir(parents=True, exist_ok=True)
    shutil.copy2(args.runtime, interpreter / ("devrun.exe" if args.platform.startswith("windows") else "devrun"))
    shutil.copy2(repo / "runtime/README.md", interpreter / "README.md")
    shutil.copy2(repo / "docs/runtime.md", interpreter / "API.md")
    shutil.copy2(repo / "LICENSE", interpreter / "LICENSE")
    shutil.copy2(repo / "runtime/THIRD_PARTY_NOTICES.md", interpreter / "THIRD_PARTY_NOTICES.md")
    bundle = repo / "dist/d" / args.platform
    bundle.mkdir(parents=True, exist_ok=True)
    suffix = ".exe" if args.platform.startswith("windows") else ""
    for source, name in ((args.cli, "d"), (args.compiler, "devc"), (args.runtime, "devrun")):
        shutil.copy2(source, bundle / (name + suffix))
    if (compiler / "backend").is_dir():
        shutil.copytree(compiler / "backend", bundle / "backend", dirs_exist_ok=True)
    shutil.copy2(repo / "cli/README.md", bundle / "README.md")
    shutil.copytree(repo / "editors/vscode", bundle / "editors/vscode", dirs_exist_ok=True,
                    ignore=shutil.ignore_patterns("node_modules", "*.vsix"))
    shutil.copy2(repo / "LICENSE", bundle / "LICENSE")
    shutil.copy2(repo / "runtime/THIRD_PARTY_NOTICES.md", bundle / "THIRD_PARTY_NOTICES.md")
    for directory in (compiler, runtime, interpreter, bundle):
        for document in ("advanced.md","language.md","runtime.md","json.md","tooling.md","packages.md","don.md","map-performance.md","map-benchmark-windows.json","map-benchmark-linux.json"):
            shutil.copy2(repo / "docs" / document,directory / document)
        shutil.copytree(repo / "docs/modules", directory / "docs/modules", dirs_exist_ok=True)
        lines = [hashlib.sha256(p.read_bytes()).hexdigest() + "  " + p.relative_to(directory).as_posix()
                 for p in sorted(directory.rglob("*")) if p.is_file() and p.name != "SHA256SUMS"]
        (directory / "SHA256SUMS").write_text("\n".join(lines) + "\n")
        print(directory)


if __name__ == "__main__":
    main()
