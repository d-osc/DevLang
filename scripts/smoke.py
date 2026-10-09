#!/usr/bin/env python3
"""Exercise the actual native build/link/run path, including cache invalidation."""
from source_text import explicit_entry
import argparse
import os
from pathlib import Path
import shlex
import shutil
import struct
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--cc")
    args = parser.parse_args()
    compiler = str(Path(args.compiler).resolve())
    extra = ["--cc", args.cc] if args.cc else []
    count = 0
    with tempfile.TemporaryDirectory(prefix="dev-lang-smoke-") as tmp:
        root = Path(tmp)
        exe = root / ("app.exe" if os.name == "nt" else "app")
        cache = root / "cache"

        def write(name, source):
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(explicit_entry(source, path=path), encoding="utf-8")
            return path

        def invoke(command, entry="main.dev", flags=(), ok=True):
            cmd = [compiler, command, str(root / entry), "--cache-dir", str(cache)] + extra + list(flags)
            if command in ("run", "build") and "-o" not in flags:
                cmd += ["-o", str(exe)]
            p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", timeout=60)
            if ok and p.returncode != 0:
                raise AssertionError(f"{cmd}\n{p.stdout}\n{p.stderr}")
            if not ok and p.returncode == 0:
                raise AssertionError(f"unexpected success: {cmd}")
            return p

        def run_source(source, expected, flags=()):
            nonlocal count
            write("main.dev", source)
            p = invoke("run", flags=flags)
            assert p.stdout == expected, (p.stdout, expected, p.stderr)
            count += 1
            return p

        run_source('fn main() { print("Hello ก Dev"); print(40 + 2) }', "Hello ก Dev\n42\n")
        run_source(r'fn main() { print("??/ %s \"quoted\"") }', '??/ %s "quoted"\n')
        run_source("fn main() { let x u8 = 255; x += 1; print(x); let y i8 = -128; print(y) }", "0\n-128\n")
        isize_min = -(1 << (struct.calcsize("P") * 8 - 1))
        run_source(f"fn main() {{ let x i64 = -9223372036854775808; let y u64 = 18446744073709551615; let z isize = {isize_min}; print(x); print(y); print(z) }}", f"-9223372036854775808\n18446744073709551615\n{isize_min}\n")
        run_source("fn main() { let x i32 = 2; print(x * 3 as i32); print(2.5 * 4.0); print(true and not false) }", "6\n10\ntrue\n")
        run_source("fn main() { let x i32 = 4; let p = &x; *p = 12; volatile_store(p, 42); print(volatile_load(p)); let a [u8; 3] = [1, 2, 3]; *( &a[0] + 1 ) = 9; print(a[1]); print(sizeof(a)) }", "42\n9\n3\n")
        run_source('fn main() { let x = "ก"; print(x[0]); print(sizeof("abc") == sizeof(x)) }', "224\ntrue\n")
        run_source("fn main() { let p *i32 = null; print(p == null) }", "true\n")
        run_source("fn main() { let i = 0; let sum = 0; while i < 10 { i += 1; if i == 3 { continue }; if i == 8 { break }; sum += i }; print(sum); if sum == 25 { print(true) } else { print(false) } }", "25\ntrue\n")
        run_source("fn f() i32 { return 42 }\nfn main() { print(f()) }", "42\n")
        run_source("fn f(n i64) i64 { if n < 2 { return n }; return f(n - 1) + f(n - 2) }\nfn main() { print(f(12)) }", "144\n", flags=("--release",))

        write("math.dev", "fn add(a i64, b i64) i64 { return a + b }")
        write("greet.dev", 'fn greet() { print("modules") }')
        multi = "use math\nuse greet\nfn main() { greet.greet(); print(math.add(20, 22)) }"
        p = run_source(multi, "modules\n42\n")
        assert "3 compiled" in p.stderr
        p = invoke("build")
        assert "0 compiled, 3 cached; link cached" in p.stderr, p.stderr
        count += 1
        write("math.dev", "fn add(a i64, b i64) i64 { return a + b + 1 }")
        p = invoke("run")
        assert p.stdout == "modules\n43\n" and "1 compiled, 2 cached" in p.stderr, p.stderr
        count += 1
        # A signature change invalidates its consumers, while an unrelated module stays cached.
        write("math.dev", "fn add(a i32, b i32) i32 { return a + b }")
        p = invoke("run")
        assert p.stdout == "modules\n42\n" and "2 compiled, 1 cached" in p.stderr, p.stderr
        count += 1
        exe.write_bytes(b"tampered")
        p = invoke("run")
        assert p.stdout == "modules\n42\n" and "0 compiled, 3 cached; link yes" in p.stderr, p.stderr
        count += 1
        # Corrupt every cached object, without changing its saved hash.
        for obj in cache.glob("*.o"):
            obj.write_bytes(b"corrupt")
        p = invoke("run")
        assert p.stdout == "modules\n42\n" and "3 compiled" in p.stderr, p.stderr
        count += 1
        write("a.dev", "use b\nfn a(n i64) i64 { if n == 0 { return 42 }; return b.b(n - 1) }")
        write("b.dev", "use a\nfn b(n i64) i64 { return a.a(n - 1) }")
        run_source("use a\nfn main() { print(a.a(4)) }", "42\n")

        c = write("driver.c", "#include <stdint.h>\nint32_t device_add(int32_t a, int32_t b) { return a+b; }\n")
        ffi = 'extern fn puts(text str) i32\nextern fn device_add(a i32, b i32) i32\nfn main() { puts("C ABI"); print(device_add(20, 22)) }'
        run_source(ffi, "C ABI\n42\n", flags=("--link", str(c)))
        before = exe.read_bytes()
        c.write_text(explicit_entry("invalid C code!", path=c), encoding="utf-8")
        p = invoke("build", flags=("--link", str(c)), ok=False)
        assert exe.read_bytes() == before and "failed" in p.stderr
        count += 1

        # C consumes exported Dev functions from a freestanding static archive.
        write("library.dev", "export fn increment(value i32) i32 { return value + 1 }")
        archive = root / "libdev.a"
        invoke("build", entry="library.dev", flags=("--lib", "--freestanding", "-o", str(archive)))
        host = write("host.c", "#include <stdint.h>\n#include <stdio.h>\nextern int32_t increment(int32_t);\nint main(void) { printf(\"%d\\n\", increment(41)); return 0; }")
        cc = args.cc or os.environ.get("DEV_CC") or ("clang" if os.name == "nt" else "cc")
        p = subprocess.run([cc, str(host), str(archive), "-o", str(exe)], capture_output=True, text=True)
        assert p.returncode == 0, p.stderr
        assert subprocess.check_output([str(exe)], text=True) == "42\n"
        count += 1

        # Compiler discovery is cached until the resolved executable changes.
        real_cc = shutil.which(cc)
        assert real_cc, cc
        log = root / "version-calls.log"
        wrapper = root / ("compiler.cmd" if os.name == "nt" else "compiler")
        if os.name == "nt":
            wrapper_text = f'@echo off\nif "%~1"=="--version" echo version>>"{log}"\n"{real_cc}" %*\n'
        else:
            wrapper_text = f'#!/bin/sh\nif [ "$1" = "--version" ]; then echo version >> {shlex.quote(str(log))}; fi\nexec {shlex.quote(real_cc)} "$@"\n'
        wrapper.write_text(explicit_entry(wrapper_text, path=wrapper), encoding="utf-8")
        wrapper.chmod(0o755)
        write("main.dev", "fn main() { print(42) }")
        invoke("build", flags=("--cc", str(wrapper)))
        p = invoke("build", flags=("--cc", str(wrapper)))
        assert "0 compiled, 1 cached; link cached" in p.stderr, p.stderr
        assert log.read_text().splitlines() == ["version"]
        wrapper.write_text(explicit_entry(wrapper_text + ("rem changed\n" if os.name == "nt" else "# changed\n"), path=wrapper), encoding="utf-8")
        p = invoke("build", flags=("--cc", str(wrapper)))
        assert "1 compiled, 0 cached; link yes" in p.stderr, p.stderr
        assert log.read_text().splitlines() == ["version", "version"]
        count += 1

        # Exported volatile operations exercise a real allocated register through C.
        write("register.dev", "export fn load(address usize) u32 { return volatile_load(address as *u32) }\nexport fn store(address usize, value u32) { volatile_store(address as *u32, value) }")
        invoke("build", entry="register.dev", flags=("--lib", "--freestanding", "-o", str(archive)))
        host = write("register.c", "#include <stdint.h>\n#include <stdio.h>\nextern uint32_t load(uintptr_t);\nextern void store(uintptr_t, uint32_t);\nint main(void) { uint32_t reg = 0; store((uintptr_t)&reg, 42); printf(\"%u %u\\n\", load((uintptr_t)&reg), reg); return 0; }")
        p = subprocess.run([cc, str(host), str(archive), "-o", str(exe)], capture_output=True, text=True)
        assert p.returncode == 0, p.stderr
        assert subprocess.check_output([str(exe)], text=True) == "42 42\n"
        count += 1

        write("main.dev", "fn main() { return 7 }")
        p = invoke("run", ok=False)
        assert p.returncode == 7
        count += 1
        write("main.dev", "fn main() { print(42) }")
        generated = root / "generated"
        invoke("emit", flags=("-o", str(generated)))
        sources = list(generated.glob("*.c"))
        p = subprocess.run([cc] + [str(s) for s in sources] + ["-o", str(exe)], capture_output=True, text=True)
        assert p.returncode == 0, p.stderr
        assert subprocess.check_output([str(exe)], text=True) == "42\n"
        count += 1

        for source, diagnostic in [
            ("fn main() { let x u8 = 256 }", "out of range"),
            ("fn main() { missing() }", "unknown function"),
            ("fn f() i32 { if true { return 1 } }", "every path"),
            ("use missing\nfn main() {}", "cannot import"),
            ("fn main() { let a = [1]; a[1] = 0 }", "out of bounds"),
            ("fn main() { let x = 1; x = true }", "expected i64"),
            ("fn main() { break }", "while loop"),
            ("fn main() { let s = \"hi\"; s[0] = 65 }", "writable"),
            ("fn main() { let p = null; print(*p) }", "cast *void"),
            ("fn main() { print(1) }", "freestanding"),
        ]:
            write("main.dev", source)
            flags = ("--freestanding",) if diagnostic == "freestanding" else ()
            p = invoke("check", flags=flags, ok=False)
            assert diagnostic in p.stderr and "main.dev:" in p.stderr, p.stderr
            count += 1
    print(f"PASS: {count} native/compiler scenarios ({os.name}, {cc})")


if __name__ == "__main__":
    main()
