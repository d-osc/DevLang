#!/usr/bin/env python3
"""Reproducible measurements; reports process latency as well as compiler phases."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import statistics
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--cc")
    parser.add_argument("--fast-tcc", help="Use TinyCC for development/cache measurements")
    parser.add_argument("--native", action="store_true", help="Use CPU-specific release flags")
    parser.add_argument("--output", required=True)
    parser.add_argument("--runs", type=int, default=7)
    args = parser.parse_args()
    if args.runs < 3:
        parser.error("use at least three runs")
    compiler = str(Path(args.compiler).resolve())
    cc = args.cc or os.environ.get("DEV_CC") or ("clang" if os.name == "nt" else "cc")
    extra = ["--cc", cc]

    def timed(cmd):
        begin = time.perf_counter()
        p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", timeout=120, cwd=root)
        duration = (time.perf_counter() - begin) * 1000
        if p.returncode:
            raise RuntimeError(f"{cmd}\n{p.stdout}\n{p.stderr}")
        return duration, p

    def summarize(values):
        return {"median_ms": round(statistics.median(values), 3), "min_ms": round(min(values), 3), "max_ms": round(max(values), 3)}

    report = {
        "platform": platform.platform(),
        "cpu_count": os.cpu_count(),
        "compiler": subprocess.check_output([compiler, "--version"], text=True).strip(),
        "compiler_sha256": hashlib.sha256(Path(compiler).read_bytes()).hexdigest(),
        "cc": subprocess.check_output([cc, "--version"], text=True).splitlines()[0],
        "runs": args.runs,
        "method": "Wall-clock process latency, median across runs. Sources/cache on local temp filesystem. C comparison uses the same loop and -O3 -fwrapv. Runtime includes process startup and printing; no universal speed claim. Fixed runtime input is visible to optimizers; use benchmark_compare.py for runtime-input comparisons.",
        "development_backend": args.fast_tcc or cc,
        "native": args.native,
        "development_profile": "--fast" if args.fast_tcc else "default -O0",
        "release_flags": "--release / -O3" + (" --native / -march=native" if args.native else ""),
    }
    with tempfile.TemporaryDirectory(prefix="dev-lang-benchmark-") as tmp:
        root = Path(tmp)
        local_compiler = root / ("devc-tool.exe" if os.name == "nt" else "devc-tool")
        shutil.copy2(compiler, local_compiler)
        compiler = str(local_compiler)
        exe = root / ("app.exe" if os.name == "nt" else "app")
        single = root / "hello.dev"
        single.write_text('fn main() { print("hello"); print(40 + 2) }\n', encoding="utf-8")

        def build_cmd(entry, cache, flags=()):
            selected = ["--cc", args.fast_tcc, "--fast"] if args.fast_tcc and "--release" not in flags else extra
            native = ["--native"] if args.native and "--release" in flags else []
            return [compiler, "build", str(entry), "-o", str(exe), "--cache-dir", str(cache), "--timings"] + selected + list(flags) + native

        cold, warm, frontend, frontend_process = [], [], [], []
        for i in range(args.runs):
            cache = root / f"cold-{i}"
            elapsed, p = timed(build_cmd(single, cache))
            cold.append(elapsed)
            assert "1 compiled" in p.stderr
            elapsed, p = timed(build_cmd(single, cache))
            warm.append(elapsed)
            assert "0 compiled, 1 cached; link cached" in p.stderr
            elapsed, p = timed([compiler, "check", str(single), "--timings"])
            frontend_process.append(elapsed)
            frontend.append(float(re.search(r"frontend ([0-9.]+) ms", p.stderr)[1]))
        report["single_module"] = {"cold_build": summarize(cold), "warm_build": summarize(warm), "frontend_only_phase": summarize(frontend), "check_process": summarize(frontend_process)}

        imports, calls = [], []
        for m in range(32):
            imports.append(f"use m{m}")
            calls.append(f"total += m{m}.f0(1)")
            functions = [f"fn f{i}(x i64) i64 {{ return x + {i} }}" for i in range(8)]
            (root / f"m{m}.dev").write_text("\n".join(functions), encoding="utf-8")
        multi = root / "main.dev"
        multi.write_text("\n".join(imports) + "\nfn main() { let total = 0; " + "; ".join(calls) + "; print(total) }\n", encoding="utf-8")
        cache = root / "multi-cache"
        cold_elapsed, p = timed(build_cmd(multi, cache))
        assert "33 compiled" in p.stderr
        warm, incremental, phases = [], [], []
        for i in range(args.runs):
            elapsed, p = timed(build_cmd(multi, cache))
            warm.append(elapsed)
            assert "0 compiled, 33 cached; link cached" in p.stderr
            # Mutate one implementation without changing its interface.
            module = root / "m0.dev"
            module.write_text(f"fn f0(x i64) i64 {{ return x + {i + 1000} }}\n" + "\n".join(f"fn f{j}(x i64) i64 {{ return x + {j} }}" for j in range(1, 8)), encoding="utf-8")
            elapsed, p = timed(build_cmd(multi, cache))
            incremental.append(elapsed)
            assert "1 compiled, 32 cached" in p.stderr
            phases.append(float(re.search(r"frontend ([0-9.]+) ms", p.stderr)[1]))
        report["33_modules_257_functions"] = {"cold_build_one_sample_ms": round(cold_elapsed, 3), "warm_build": summarize(warm), "one_body_changed": summarize(incremental), "frontend_phase": summarize(phases)}

        # Compare a real release executable with an equivalent C executable.
        runtime = root / "loop.dev"
        runtime.write_text("fn sum_loop(n i64) i64 { let total = 0; let i = 0; while i < n { total += i & 255; i += 1 }; return total }\nfn main() { print(sum_loop(100000000)) }\n", encoding="utf-8")
        timed(build_cmd(runtime, root / "runtime-cache", ("--release",)))
        c_source = root / "loop.c"
        c_source.write_text('#include <stdint.h>\n#include <stdio.h>\nint64_t sum_loop(int64_t n) { int64_t total=0; int64_t i=0; while(i<n) { total += i & 255; i += 1; } return total; }\nint main(void) { printf("%lld\\n", (long long)sum_loop(100000000)); return 0; }\n', encoding="utf-8")
        c_exe = root / ("c-app.exe" if os.name == "nt" else "c-app")
        timed([cc, "-std=c11", "-O3", "-fwrapv", str(c_source), "-o", str(c_exe)] + (["-march=native"] if args.native else []))
        dev_times, c_times = [], []
        for _ in range(args.runs):
            elapsed, dev = timed([str(exe)])
            dev_times.append(elapsed)
            elapsed, c = timed([str(c_exe)])
            c_times.append(elapsed)
            assert dev.stdout == c.stdout == "12750000000\n", (dev.stdout, c.stdout)
        report["100_million_iteration_runtime"] = {"dev_release": summarize(dev_times), "equivalent_c_O3": summarize(c_times), "output": "12750000000"}

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
