#!/usr/bin/env python3
"""Build the standalone C runtime; no Dev compiler or Rust installation needed."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--cc", default="clang" if os.name == "nt" else "cc")
    parser.add_argument("--ar", default="llvm-ar" if os.name == "nt" else "ar")
    parser.add_argument("--output", default=str(Path(__file__).resolve().parent / "lib/libdevruntime.a"))
    parser.add_argument("--debug", action="store_true")
    parser.add_argument("--cflag", action="append", default=[])
    args = parser.parse_args()
    root = Path(__file__).resolve().parent
    output = Path(args.output).resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    flags = ["-std=c11", "-O0" if args.debug else "-O3", "-fwrapv", "-Wall", "-Wextra", "-Werror",
             "-I", str(root / "include"), *args.cflag]
    with tempfile.TemporaryDirectory(prefix=".runtime-build-", dir=output.parent) as directory:
        stage = Path(directory)
        objects = []
        for source in sorted((root / "src").glob("*.c")):
            obj = stage / (source.stem + ".o")
            subprocess.run([args.cc, *flags, "-c", str(source), "-o", str(obj)], check=True)
            objects.append(str(obj))
        if not objects:
            raise RuntimeError("runtime C sources are missing")
        archive = stage / "libdevruntime.a"
        subprocess.run([args.ar, "rcs", str(archive), *objects], check=True)
        os.replace(archive, output)
    manifest = {"runtime_abi": 1, "library": output.name,
                "platform": os.name, "cc": args.cc, "flags": flags,
                "sha256": hashlib.sha256(output.read_bytes()).hexdigest()}
    output.with_suffix(".json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Built standalone runtime: {output}")


if __name__ == "__main__":
    main()
