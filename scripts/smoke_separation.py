#!/usr/bin/env python3
"""Verify a compiler-only source checkout and relocated separate packages."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", default="windows-x86_64" if os.name == "nt" else "linux-x86_64")
    parser.add_argument("--cargo", default="cargo")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="dev-separation-") as directory:
        root = Path(directory)
        source = root / "compiler-source"
        shutil.copytree(repo / "compiler", source, ignore=shutil.ignore_patterns("target"))
        shutil.copytree(repo / "syntax", root / "syntax")
        assert not (root / "runtime").exists()
        subprocess.run([args.cargo, "build", "--release"], cwd=source, check=True, timeout=180)
        binary = source / "target/release" / ("devc.exe" if os.name == "nt" else "devc")
        entry = root / "main.dev"
        entry.write_text('fn main() { print("compiler alone") }')
        result = subprocess.run([str(binary), "run", str(entry), "-o", str(root / "program")], cwd=root,
                                capture_output=True, text=True, timeout=120)
        assert result.returncode == 0 and result.stdout == "compiler alone\n", (result.stdout, result.stderr)

        isolated = root / "interpreter-alone"
        isolated.mkdir()
        shutil.copytree(repo / "runtime", isolated / "runtime")
        shutil.copytree(repo / "syntax", isolated / "syntax")
        subprocess.run([args.cargo, "build", "--release"], cwd=isolated / "runtime", check=True, timeout=180)
        interpreter = isolated / "runtime/target/release" / ("devrun.exe" if os.name == "nt" else "devrun")
        result = subprocess.run([str(interpreter), str(entry)], cwd=root, env=dict(os.environ, PATH=""), capture_output=True, text=True, timeout=10)
        assert result.returncode == 0 and result.stdout == "compiler alone\n", result.stderr

        compiler = root / "compiler-package"
        runtime = root / "independent-runtime-package"
        shutil.copytree(repo / "dist/compiler" / args.platform, compiler)
        shutil.copytree(repo / "dist/stdlib" / args.platform, runtime)
        assert not (compiler / "stdlib").exists()
        assert not list(runtime.rglob("*.rs")) and not list(runtime.rglob("devc*"))
        entry.write_text('use "std/strings"\nfn main() { let s = strings.new("separate packages"); if s == null { return 1 }; print(strings.view(s)); strings.free(s) }')
        binary = compiler / ("devc.exe" if os.name == "nt" else "devc")
        module = ["--module-dir", f"std={runtime / 'modules'}"]
        for command in ("check", "emit"):
            flags = ["-o", str(root / "generated")] if command == "emit" else []
            subprocess.run([str(binary), command, str(entry), *module, *flags], cwd=root, check=True, timeout=60)
        result = subprocess.run([str(binary), "run", str(entry), *module, "--link", str(runtime / "lib/libdevruntime.a"),
                                 "-o", str(root / "program")], cwd=root, capture_output=True, text=True, timeout=120)
        assert result.returncode == 0 and result.stdout == "separate packages\n", (result.stdout, result.stderr)
    print("PASS: independent compiler/interpreter source builds, execution without a compiler, relocated packages and declaration-only check/emit")


if __name__ == "__main__":
    main()
