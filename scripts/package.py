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
    parser.add_argument("--platform", required=True, choices=("windows-x86_64", "linux-x86_64", "windows-arm64", "linux-arm64"))
    parser.add_argument("--backend")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    for executable in (args.compiler, args.runtime, args.cli):
        data = Path(executable).read_bytes()
        if args.platform.startswith("windows"):
            pe = int.from_bytes(data[60:64], "little")
            machine = int.from_bytes(data[pe+4:pe+6], "little")
            expected = 0xAA64 if args.platform.endswith("arm64") else 0x8664
            valid = data[:2] == b"MZ" and data[pe:pe+4] == b"PE\0\0" and machine == expected
        else:
            machine = int.from_bytes(data[18:20], "little")
            expected = 183 if args.platform.endswith("arm64") else 62
            valid = data[:4] == b"\x7fELF" and data[4:6] == b"\x02\x01" and machine == expected
        if not valid: raise ValueError(f"wrong executable architecture for {args.platform}: {executable}")
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
    for directory in (bundle, interpreter):
        (directory / "scripts").mkdir(exist_ok=True)
        for name in ("serve_http.py", "windows_job.py", "soak_http_server.py", "check_http_certificate.py"):
            shutil.copy2(repo / "scripts" / name, directory / "scripts" / name)
        shutil.copytree(repo / "deploy/http", directory / "deploy/http", dirs_exist_ok=True,
                        ignore=shutil.ignore_patterns("logs", "temp", "certs"))
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
