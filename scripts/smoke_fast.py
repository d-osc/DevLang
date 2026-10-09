#!/usr/bin/env python3
"""Regression tests for executable caching, CPU profiles and TinyCC builds."""
from source_text import explicit_entry
import argparse
from pathlib import Path
import random
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--tcc")
    parser.add_argument("--cc", help="Explicit Clang/GCC backend for release/native cases")
    args = parser.parse_args()
    compiler = str(Path(args.compiler).resolve())
    count = 0
    with tempfile.TemporaryDirectory(prefix="dev-lang-fast-smoke-") as tmp:
        root = Path(tmp)
        source = root / "main.dev"
        output = root / "program"
        cache = root / "cache"
        def invoke(command="run", flags=(), ok=True):
            p = subprocess.run([compiler, command, str(source), "-o", str(output), "--cache-dir", str(cache), *flags], capture_output=True, text=True, cwd=root)
            if ok and p.returncode:
                raise AssertionError(p.stderr)
            if not ok and not p.returncode:
                raise AssertionError("unexpected success")
            return p

        source.write_text(explicit_entry("fn main() { print(42) }", path=source))
        first = invoke()
        assert first.stdout == "42\n" and "1 compiled" in first.stderr
        second = invoke()
        assert second.stdout == "42\n" and "0 compiled, 1 cached; link cached" in second.stderr
        count += 1
        output.unlink()
        third = invoke()
        assert third.stdout == "42\n" and "0 compiled, 1 cached" in third.stderr
        count += 1
        output.write_bytes(b"corrupt executable")
        p = invoke()
        assert p.stdout == "42\n" and "0 compiled, 1 cached" in p.stderr
        count += 1
        for artifact in cache.glob("exe-*.bin"):
            artifact.write_bytes(b"corrupt cache")
        p = invoke()
        assert p.stdout == "42\n" and "1 compiled, 0 cached" in p.stderr
        count += 1
        p = invoke(flags=("--release",))
        assert p.stdout == "42\n" and "1 compiled" in p.stderr
        count += 1
        p = invoke(flags=("--release", "--native"))
        assert p.stdout == "42\n" and "1 compiled" in p.stderr
        p = invoke(flags=("--release", "--native"))
        assert p.stdout == "42\n" and "0 compiled, 1 cached" in p.stderr
        count += 1
        source.write_text(explicit_entry("fn main() { print(43) }", path=source))
        p = invoke()
        assert p.stdout == "43\n" and "1 compiled" in p.stderr
        count += 1

        # Check modular-width lowering against independent Python arithmetic.
        # Include wide overflow, subtraction, nonmodular division/right shifts,
        # a commuted mask, nested masks, and a stateful call evaluated once.
        mask64 = (1 << 64) - 1
        mask32 = (1 << 32) - 1
        rng = random.Random(20261008)
        values = [0, 1, mask32, mask32 + 1, 1 << 63, mask64]
        values += [rng.getrandbits(64) for _ in range(40)]
        lines = ["fn f(p *u64) u64 { *p += 1; return *p }", "fn main() {"]
        expected = []
        for index, x in enumerate(values):
            y = values[(index + 7) % len(values)]
            lines += [f"let x{index} u64 = {x}", f"let y{index} u64 = {y}"]
            expressions = [
                (f"(x{index} * 1664525 + y{index}) & 0xffffffff", (x * 1664525 + y) & mask32),
                (f"4294967295 & ((x{index} - y{index}) * y{index})", ((x - y) * y) & mask32),
                (f"((x{index} / 7) + (y{index} >> 40)) & 0xffffffff", ((x // 7) + (y >> 40)) & mask32),
                (f"((x{index} | y{index}) ^ (x{index} & y{index})) & 0xffffffff", ((x | y) ^ (x & y)) & mask32),
                (f"(((x{index} + y{index}) & 0xffffffff) * x{index}) & 0xffffffff", ((x + y) * x) & mask32),
                (f"((x{index} * y{index}) >> 32) & 0xffffffff", (((x * y) & mask64) >> 32) & mask32),
            ]
            for expression, result in expressions:
                lines.append(f"print({expression})")
                expected.append(str(result))
        lines += ["let calls u64 = 4294967295", "print((f(&calls) * 3 + 7) & 0xffffffff)", "print(calls)",
                  "let s i64 = -1", "print((s * 7) & 0xffffffff)", "}"]
        expected += ["7", "4294967296", "4294967289"]
        source.write_text(explicit_entry("\n".join(lines), path=source))
        regular = ("--cc", args.cc) if args.cc else ()
        profiles = [regular + ("--release",), regular + ("--release", "--native")]
        if args.tcc:
            profiles.append(("--fast", "--cc", str(Path(args.tcc).resolve())))
        for profile in profiles:
            p = invoke(flags=profile)
            assert p.stdout.splitlines() == expected, (profile, p.stdout, p.stderr)
            count += 1

        if args.tcc:
            fast = ("--fast", "--cc", str(Path(args.tcc).resolve()))
            cases = [
                ('fn main() { print("ไทย ??/ %s"); print(42) }', "ไทย ??/ %s\n42\n"),
                ("fn main() { let x i64 = 9223372036854775807; x += 1; print(x); let y u64 = 18446744073709551615; y += 1; print(y) }", "-9223372036854775808\n0\n"),
                ("fn main() { let x i8 = 127; x += 1; print(x); let p = &x; volatile_store(p, -128); print(volatile_load(p)) }", "-128\n-128\n"),
                ("fn main() { let a [u64; 3] = [1, 2, 3]; let p = &a[0]; *(p + 1) = 42; print(a[1]); print(sizeof(a)) }", "42\n24\n"),
                ("fn f(n i64) i64 { if n < 2 { return n }; return f(n - 1) + f(n - 2) }\nfn main() { print(f(12)) }", "144\n"),
                ("fn main() { let x f32 = 2.5; print(x * 4) }", "10\n"),
                ("fn main() { let x u64 = 0xffffffff; print((x << 32) >> 32); print(x / 3); print(x % 3) }", "4294967295\n1431655765\n0\n"),
            ]
            for code, expected in cases:
                source.write_text(explicit_entry(code, path=source), encoding="utf-8")
                p = invoke(flags=fast)
                assert p.stdout == expected, (p.stdout, p.stderr)
                count += 1
            (root / "other.dev").write_text(explicit_entry("use main\nfn add(x i64) i64 { return x + 1 }", path=root / "other.dev"))
            source.write_text(explicit_entry("use other\nfn main() { print(other.add(41)) }", path=source))
            p = invoke(flags=fast)
            assert p.stdout == "42\n" and "2 compiled" in p.stderr
            p = invoke(flags=fast)
            assert "0 compiled, 2 cached" in p.stderr
            count += 1
            driver = root / "driver.c"
            driver.write_text(explicit_entry("#include <stdint.h>\nint32_t add(int32_t a, int32_t b) { return a+b; }", path=driver))
            source.write_text(explicit_entry("extern fn add(a i32, b i32) i32\nfn main() { print(add(20, 22)) }", path=source))
            p = invoke(flags=fast + ("--link", str(driver)))
            assert p.stdout == "42\n"
            before = output.read_bytes()
            driver.write_text(explicit_entry("invalid C!", path=driver))
            invoke(command="build", flags=fast + ("--link", str(driver)), ok=False)
            assert output.read_bytes() == before
            count += 1
            source.write_text(explicit_entry("fn main() { print(42) }", path=source))
            for conflict in ("--release", "--native", "--lib", "--freestanding"):
                p = invoke(command="build", flags=fast + (conflict,), ok=False)
                assert "--fast is for hosted" in p.stderr
                count += 1
    print(f"PASS: {count} fast/cache/profile scenarios")


if __name__ == "__main__":
    main()
