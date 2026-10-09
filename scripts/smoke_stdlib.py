#!/usr/bin/env python3
"""Exercise runtime modules via the actual native compiler and cache paths."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--cc")
    parser.add_argument("--tcc")
    args = parser.parse_args()
    compiler = str(Path(args.compiler).resolve())
    repo = Path(__file__).resolve().parents[1]
    count = 0
    with tempfile.TemporaryDirectory(prefix="dev-runtime-") as directory:
        root = Path(directory)
        runtime = root / "stdlib"
        shutil.copytree(repo / "stdlib", runtime)
        entry = root / "main.dev"
        exe = root / ("app.exe" if os.name == "nt" else "app")
        cache = root / "cache"
        regular = ["--cc", args.cc] if args.cc else []
        cc = args.cc or ("clang" if os.name == "nt" else "cc")
        library = runtime / "lib/libdevruntime.a"
        def build_runtime(ok=True):
            result = subprocess.run([os.sys.executable, str(runtime / "build.py"), "--cc", cc,
                                     "--output", str(library)], cwd=root, capture_output=True, text=True, timeout=120)
            assert (result.returncode == 0) == ok, (result.stdout, result.stderr)
            return result
        build_runtime()

        # Compiler alone works and does not auto-discover an adjacent runtime.
        relocated = root / ("devc.exe" if os.name == "nt" else "devc")
        shutil.copy2(compiler, relocated)
        entry.write_text('fn main() { print("standalone compiler") }')
        env = os.environ.copy()
        env.pop("DEV_RUNTIME", None)
        result = subprocess.run([str(relocated), "run", str(entry), "-o", str(exe), "--cache-dir", str(cache), *regular],
                                cwd=root, env=env, capture_output=True, text=True, timeout=120)
        assert result.returncode == 0 and result.stdout == "standalone compiler\n", (result.stdout, result.stderr)
        count += 1
        entry.write_text('use "std/strings"\nfn main() {}')
        result = subprocess.run([str(relocated), "check", str(entry)], cwd=root, capture_output=True, text=True)
        assert result.returncode != 0 and "cannot import" in result.stderr, result.stderr
        count += 1

        def invoke(command="run", flags=(), input_text=None, ok=True, output=None):
            cmd = [compiler, command, str(entry), "--module-dir", f"std={runtime / 'modules'}", "--cache-dir", str(cache), *regular, *flags]
            if command in ("run", "build"):
                cmd += ["-o", str(output or exe), "--link", str(library)]
            result = subprocess.run(cmd, cwd=root, capture_output=True, text=True, encoding="utf-8", input=input_text, timeout=120)
            if ok and result.returncode:
                raise AssertionError((cmd, result.stdout, result.stderr))
            if not ok and not result.returncode:
                raise AssertionError("unexpected success")
            return result

        cases = [
            ('use "std/memory"\nfn main() { let p = memory.alloc_array(8, 1) as *u8; if p == null { return 1 }; print(p[7]); memory.fill(p as *void, 65, 8); memory.copy((p + 1) as *void, p as *void, 7); print(p[7]); let q = memory.resize(p as *void, 16) as *u8; if q == null { memory.free(p as *void); return 1 }; print(q[7]); memory.free(q as *void); print(memory.alloc_array(18446744073709551615, 8) == null); memory.free(null); memory.copy(null, null, 0); memory.fill(null, 0, 0) }', "0\n65\n65\ntrue\n"),
            ('use "std/strings"\nfn main() { let s = strings.new("ภาษาไทย"); if s == null { return 1 }; print(strings.len(s)); strings.append(s, " Dev"); print(strings.view(s)); let copy = strings.clone(s); print(strings.equal(s, copy)); strings.clear(s); print(strings.len(s)); print(strings.view(copy)); strings.free(copy); strings.free(s); strings.free(null); print(strings.append(null, "bad")) }', "21\nภาษาไทย Dev\ntrue\n0\nภาษาไทย Dev\nfalse\n"),
            ('use "std/strings"\nfn main() { let s = strings.new("0123456789abcdef0123456789abcdef"); if s == null { return 1 }; print(strings.append(s, strings.view(s))); print(strings.len(s)); let bytes [u8; 3] = [65, 0, 66]; print(strings.append_bytes(s, &bytes[0], 3)); let copy = strings.clone(s); print(strings.len(copy)); print(strings.equal(s, copy)); strings.free(copy); strings.free(s) }', "true\n64\ntrue\n67\ntrue\n"),
            ('use "std/io"\nfn main() { let f = io.open("ไฟล์-runtime.bin", "wb"); if f == null { return 1 }; print(io.write_file(f, "ภาษาไทย")); let bytes [u8; 3] = [0, 255, 66]; print(io.write_bytes(f, &bytes[0], 3)); print(io.flush_file(f)); print(io.close(f)); f = io.open("ไฟล์-runtime.bin", "rb"); if f == null { return 2 }; let data [u8; 32] = [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]; print(io.read(f, &data[0], 32)); print(data[21]); print(data[22]); print(io.eof(f)); print(io.error(f)); io.close(f); print(io.open("missing-file", "rb") == null); print(io.open("bad-mode", "invalid") == null); print(io.close(null)); print(io.error(null)) }', "21\n3\ntrue\ntrue\n24\n0\n255\ntrue\nfalse\ntrue\ntrue\nfalse\ntrue\n"),
            ('use "std/io"\nfn main() { io.write("hello "); io.writeln("ไทย"); print(io.flush()) }', "hello ไทย\ntrue\n"),
            ('use "std/time"\nfn main() { let start = time.now_ns(); print(start > 0); print(time.sleep_ms(1)); let end = time.now_ns(); print(end >= start); print(time.now_ms() > 0) }', "true\ntrue\ntrue\ntrue\n"),
        ]
        profiles = [[], ["--release", "--native"]]
        if args.tcc:
            profiles.append(["--fast", "--cc", str(Path(args.tcc).resolve())])
        for profile in profiles:
            for code, expected in cases:
                entry.write_text(code, encoding="utf-8")
                result = invoke(flags=profile)
                assert result.stdout == expected, (profile, result.stdout, expected)
                count += 1
                result = invoke(flags=profile)
                assert result.stdout == expected and "0 compiled" in result.stderr and "link cached" in result.stderr, result.stderr
                count += 1
        assert (root / "ไฟล์-runtime.bin").read_bytes() == "ภาษาไทย".encode() + bytes([0, 255, 66])
        entry.write_text('use "std/strings"\nfn main() { let s = strings.new("emitted"); if s == null { return 1 }; print(strings.view(s)); strings.free(s) }')
        generated = root / "generated"
        invoke("emit", flags=["-o", str(generated)])
        cc = args.cc or ("clang" if os.name == "nt" else "cc")
        command = [cc, "-std=c11", *map(str, generated.glob("*.c")), str(library), "-o", str(exe)]
        subprocess.run(command, cwd=root, check=True, capture_output=True, timeout=120)
        assert subprocess.check_output([str(exe)], text=True) == "emitted\n"
        count += 1
        archive = library
        consumer = root / "consumer.c"
        consumer.write_text('#include "dev_runtime.h"\n#include <assert.h>\nint main(void) { void *s = dvr_string_new("C ABI"); assert(s && dvr_string_len(s) == 5); dvr_string_free(s); return 0; }')
        subprocess.run([cc, "-std=c11", "-I", str(runtime / "include"), str(consumer), str(archive), "-o", str(exe)], cwd=root, check=True, capture_output=True, timeout=120)
        subprocess.run([str(exe)], check=True, timeout=30)
        count += 1
        entry.write_text('use "std/io"\nfn main() { let data [u8; 5] = [0,0,0,0,0]; print(io.read_line(&data[0], 5)); print(data[0]); print(io.read_line(&data[0], 5)); print(data[0]); print(io.read_line(&data[0], 5)); print(io.read_line(null, 0)) }')
        result = invoke(input_text="ab\r\n\nz")
        assert result.stdout == "2\n97\n0\n0\n1\n-2\n", result.stdout
        result = invoke(input_text="")
        assert result.stdout == "-1\n0\n-1\n0\n-1\n-2\n", result.stdout
        count += 2

        # Rebuild the runtime independently; Dev objects stay cached on relink.
        entry.write_text('use "std/memory"\nfn main() { memory.free(null); print(42) }')
        invoke()
        source = runtime / "src/memory.c"
        header = runtime / "include/dev_runtime.h"
        header.write_text(header.read_text() + '\n#define DVR_TEST_MESSAGE "source changed"\n')
        source.write_text('#include <stdio.h>\n' + source.read_text().replace(
            'void dvr_free(void *data) { free(data); }',
            'void dvr_free(void *data) { if (!data) puts(DVR_TEST_MESSAGE); free(data); }'))
        build_runtime()
        result = invoke()
        assert result.stdout == "source changed\n42\n" and "0 compiled" in result.stderr and "link yes" in result.stderr, result.stderr
        count += 1
        header.write_text(header.read_text().replace('"source changed"', '"header changed"'))
        build_runtime()
        result = invoke()
        assert result.stdout == "header changed\n42\n" and "0 compiled" in result.stderr, result.stderr
        before = library.read_bytes()
        source.write_text("invalid C source")
        build_runtime(ok=False)
        assert library.read_bytes() == before
        assert invoke().stdout == "header changed\n42\n"
        count += 2
        entry.write_text('use "std/memory"\nfn main() { memory.free(null) }')
        invoke("check", flags=["--freestanding", "--lib"])
        count += 1
        entry.write_text('use "std/unknown"\nfn main() {}')
        result = invoke("check", ok=False)
        assert "cannot import" in result.stderr
        count += 1
        entry.write_text('use "std/../secret"\nfn main() {}')
        result = invoke("check", ok=False)
        assert "cannot escape" in result.stderr
        count += 1
        entry.write_text('use "custom/strings" as text\nfn main() { let s = text.new("generic modules"); print(text.view(s)); text.free(s) }')
        result = invoke(flags=["--module-dir", f"custom={runtime / 'modules'}"])
        assert result.stdout == "generic modules\n"
        count += 1
    print(f"PASS: {count} runtime native/cache scenarios")


if __name__ == "__main__":
    main()
