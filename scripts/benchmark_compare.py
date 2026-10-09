#!/usr/bin/env python3
"""Compare equivalent native Dev/C/Rust/Go programs with runtime inputs."""
import argparse
from contextlib import contextmanager
import datetime
import functools
import hashlib
import json
import os
from pathlib import Path
import platform
import random
import shutil
import statistics
import subprocess
import tempfile
import time

LANGUAGES = ("dev", "c", "rust", "go")
EXTENSIONS = {"dev": "dev", "c": "c", "rust": "rs", "go": "go"}
MASK64 = (1 << 64) - 1
MASK32 = (1 << 32) - 1


def xorshift_step(state):
    state ^= (state << 13) & MASK64
    state ^= state >> 7
    state ^= (state << 17) & MASK64
    return state & MASK64


def apply_matrix(matrix, value):
    result = 0
    while value:
        bit = value & -value
        result ^= matrix[bit.bit_length() - 1]
        value ^= bit
    return result


@functools.lru_cache(maxsize=None)
def reference(mode, n, seed, rounds):
    if mode == 1:
        blocks, rest = divmod(n, 256)
        return blocks * 32640 + rest * (rest - 1) // 2
    if mode == 2:
        matrix = [xorshift_step(1 << bit) for bit in range(64)]
        while n:
            if n & 1:
                seed = apply_matrix(matrix, seed)
            n >>= 1
            if n:
                matrix = [apply_matrix(matrix, value) for value in matrix]
        return seed
    if mode == 3:
        # Exponentiate the affine LCG transform modulo 2^32, independently of
        # the native programs' repeated memory updates.
        a, c, result_a, result_c = 1664525, 1013904223, 1, 0
        while rounds:
            if rounds & 1:
                result_a = (a * result_a) & MASK32
                result_c = (a * result_c + c) & MASK32
            c = (a * c + c) & MASK32
            a = (a * a) & MASK32
            rounds >>= 1
        return sum((result_a * ((seed + i) & MASK32) + result_c) & MASK32 for i in range(n))
    raise ValueError(mode)


def stats(values):
    ordered = sorted(values)
    return {
        "median_ms": round(statistics.median(values), 3),
        "min_ms": round(min(values), 3),
        "max_ms": round(max(values), 3),
        "p95_ms": round(ordered[min(len(ordered) - 1, int(len(ordered) * 0.95))], 3),
        "samples_ms": [round(value, 3) for value in values],
    }


def process(command, *, env=None, input_text=None, timeout=180, cwd=None):
    begin = time.perf_counter_ns()
    result = subprocess.run(command, input=input_text, capture_output=True, text=True,
                            encoding="utf-8", env=env, timeout=timeout, cwd=cwd)
    elapsed = (time.perf_counter_ns() - begin) / 1e6
    if result.returncode:
        raise RuntimeError(f"command failed: {command}\n{result.stdout}\n{result.stderr}")
    return elapsed, result


def run_checked(executable, values, env):
    duration, result = process([str(executable)], env=env,
                               input_text=" ".join(map(str, values)) + "\n", cwd=executable.parent)
    expected = str(reference(*values)) + "\n"
    if result.stdout != expected:
        raise AssertionError(f"{executable}, input={values}: {result.stdout!r} != {expected!r}")
    return duration


@contextmanager
def affinity(enabled):
    original = None
    selected = None
    if enabled and hasattr(os, "sched_getaffinity"):
        original = os.sched_getaffinity(0)
        selected = min(original)
        os.sched_setaffinity(0, {selected})
    try:
        yield selected
    finally:
        if original is not None:
            os.sched_setaffinity(0, original)


def cpu_model():
    path = Path("/proc/cpuinfo")
    if path.exists():
        for line in path.read_text().splitlines():
            if line.startswith("model name"):
                return line.split(":", 1)[1].strip()
    return platform.processor()


def markdown(report):
    label = {"dev": "Dev Lang", "c": "C", "rust": "Rust", "go": "Go"}
    lines = [
        "# Dev Lang vs C vs Rust vs Go", "",
        f"Measured {report['measured_at']} on {report['platform']}, {report['cpu_model']}.", "",
        "Runtime: median milliseconds across randomized rounds, including process startup, stdin parsing, allocation and printing. Lower is faster. All results match independent Python references.", "",
        "| Language | Masked sum | Xorshift64 | Array update |",
        "| --- | ---: | ---: | ---: |",
    ]
    for lang in LANGUAGES:
        row = [f"{report['runtime'][name]['languages'][lang]['median_ms']:.3f}" for name in report["runtime"]]
        lines.append(f"| {label[lang]} | {' | '.join(row)} |")
    lines += ["", "Full application rebuilds, with toolchain/standard-library dependencies already warm. No project cache is reused for Dev; Go main-package source changes every repetition. Optimized Dev and C use the same C backend.", "",
              "| Language | Build, no application optimization | Build, optimized | Optimized build min–max | Optimized binary KiB |", "| --- | ---: | ---: | ---: | ---: |"]
    for lang in LANGUAGES:
        unoptimized = report["builds"]["unoptimized"][lang]["median_ms"]
        optimized = report["builds"]["optimized"][lang]["median_ms"]
        size = report["binary_bytes"][lang] / 1024
        spread = report["builds"]["optimized"][lang]
        lines.append(f"| {label[lang]} | {unoptimized:.3f} ms | {optimized:.3f} ms | {spread['min_ms']:.3f}–{spread['max_ms']:.3f} ms | {size:.1f} |")
    lines += ["", "Build timings vary substantially and their ranges overlap; small median differences do not establish a stable compile-speed ranking."]
    if "--fast" in report["flags"]["unoptimized"]["dev"]:
        lines += ["", "Dev development builds use TinyCC, while C development builds use the reported GCC/Clang backend. This demonstrates the fast backend/profile, not an inherent advantage over C: C can use TinyCC too. TinyCC produces unoptimized runtime code; optimized runtime measurements below use GCC/Clang."]
    lines += ["", "Unchanged optimized builds (only drivers with caches configured in this experiment):", "",
              "| Driver | Median |", "| --- | ---: |"]
    for lang, values in report["unchanged_builds"].items():
        lines.append(f"| {label[lang]} | {values['median_ms']:.3f} ms |")
    lines += ["", f"The first Go build with an empty private GOCACHE took {report['bootstrap_builds']['go']['elapsed_ms']:.3f} ms, including compiling standard-library dependencies. This is one sample and is separate from the application rebuild table.", "",
              "## Parameters", "",
              f"- Runtime repetitions: {report['runtime_runs']}; warmups per workload/language: {report['warmups']}; build repetitions: {report['build_runs']}.",
              f"- Runtime CPU affinity: {report['runtime_cpu']}; Go runtime GOMAXPROCS=1, GOGC=100, CGO_ENABLED=0, GOAMD64={report.get('go_amd64', 'v1')}.",
              "- Inputs are supplied through stdin after compilation; no fixed-workload constant is available to the optimizer.",
        "- Dev compiler executable, benchmark sources, binaries and caches are staged on the local temporary filesystem (not /mnt/c on Linux).",
              "- Unsigned 64-bit operations; xorshift wraps at 64 bits; array elements wrap at 32 bits."]
    for name, workload in report["runtime"].items():
        lines.append(f"- {name}: mode/n/seed/rounds = {workload['input']}; checksum = {workload['expected']}.")
    lines += ["", "## Toolchains and flags", ""]
    for lang in LANGUAGES:
        lines.append(f"- {label[lang]}: `{report['toolchains'][lang]}`; optimized flags: `{report['flags']['optimized'][lang]}`.")
    if "tcc" in report["toolchains"]:
        lines.append(f"- Development backend: `{report['toolchains']['tcc']}`; Dev flags: `--fast`.")
    lines += ["", "Dev/C use raw pointers and malloc/free for the array; Rust uses a safe Vec and Go uses a checked slice. Rust/Go initialize their allocation to zero before the same explicit fill loop. This compares these ordinary implementations, including memory-safety/runtime costs; the compiler may eliminate redundant checks or writes.", "",
              "Binary sizes are as linked, without a separate stripping step. Rust/Go include more runtime/standard-library code; Dev/C dynamically link libc. Equal optimization-level numbers do not imply equal optimization passes. These results are specific to these kernels/toolchains/WSL and do not establish a universal language ranking.", "",
              "References: [Rust codegen flags](https://doc.rust-lang.org/rustc/codegen-options/), [Go build/cache flags](https://pkg.go.dev/cmd/go), [C optimization levels](https://clang.llvm.org/docs/CommandGuide/clang.html).", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--compiler", required=True)
    parser.add_argument("--cc", default="clang" if os.name == "nt" else "cc")
    parser.add_argument("--rustc", default="rustc")
    parser.add_argument("--go", default="go")
    parser.add_argument("--c-opt", choices=("2", "3"), default="3")
    parser.add_argument("--rust-opt", choices=("2", "3"), default="3")
    parser.add_argument("--native", action="store_true", help="CPU-specific flags for Dev/C/Rust and GOAMD64=v3")
    parser.add_argument("--fast-tcc", help="Use this TinyCC executable for Dev development build timings")
    parser.add_argument("--runs", type=int, default=11)
    parser.add_argument("--build-runs", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=2)
    parser.add_argument("--output", required=True)
    parser.add_argument("--markdown")
    parser.add_argument("--no-pin", action="store_true")
    parser.add_argument("--keep-workdir", action="store_true")
    args = parser.parse_args()
    if args.runs < 3 or args.build_runs < 3 or args.warmups < 1:
        parser.error("need >=3 runtime/build runs and >=1 warmup")
    compiler = str(Path(args.compiler).resolve())
    source_dir = Path(__file__).resolve().parents[1] / "benchmarks" / "comparison"
    sources = {lang: (source_dir / f"bench.{EXTENSIONS[lang]}").read_text() for lang in LANGUAGES}
    versions = {
        "dev": process([compiler, "--version"])[1].stdout.strip(),
        "c": process([args.cc, "--version"])[1].stdout.splitlines()[0],
        "rust": process([args.rustc, "--version"])[1].stdout.strip(),
        "go": process([args.go, "version"])[1].stdout.strip(),
    }
    if args.fast_tcc:
        versions["tcc"] = process([args.fast_tcc, "--version"])[1].stdout.splitlines()[0]
    flags = {
        "unoptimized": {"dev": "--fast / TinyCC" if args.fast_tcc else "default -O0", "c": "-std=c11 -O0 -fwrapv", "rust": "-C opt-level=0 -C overflow-checks=off -C debuginfo=0", "go": "-gcflags=-N -l (application only)"},
        "optimized": {"dev": "--release / -O3", "c": f"-std=c11 -O{args.c_opt} -fwrapv", "rust": f"-C opt-level={args.rust_opt} -C overflow-checks=off -C debuginfo=0", "go": "go build (default optimized)"},
    }
    if args.native:
        flags["optimized"]["dev"] += " --native / -march=native"
        flags["optimized"]["c"] += " -march=native"
        flags["optimized"]["rust"] += " -C target-cpu=native"
        flags["optimized"]["go"] += " GOAMD64=v3"
    report = {
        "measured_at": datetime.datetime.now(datetime.timezone(datetime.timedelta(hours=7))).isoformat(timespec="seconds"),
        "platform": platform.platform(), "cpu_model": cpu_model(), "cpu_count": os.cpu_count(),
        "runtime_runs": args.runs, "build_runs": args.build_runs, "warmups": args.warmups,
        "toolchains": versions, "flags": flags, "native": args.native,
        "go_amd64": "v3" if args.native else "v1",
        "source_sha256": {lang: hashlib.sha256(sources[lang].encode()).hexdigest() for lang in LANGUAGES},
        "devc_sha256": hashlib.sha256(Path(compiler).read_bytes()).hexdigest(),
        "bootstrap_builds": {}, "builds": {}, "unchanged_builds": {}, "runtime": {},
        "method": "Single-file programs, equivalent unsigned-64 kernels; runtime inputs; randomized language order; runtime pinned when supported; end-to-end process timings. Build table recompiles application with toolchain dependencies warm. Go's initial standard-library compilation reported separately.",
    }
    temporary = tempfile.TemporaryDirectory(prefix="dev-lang-compare-") if not args.keep_workdir else None
    root = Path(temporary.name if temporary else tempfile.mkdtemp(prefix="dev-lang-compare-"))
    try:
        print(f"Work directory: {root}", flush=True)
        local_compiler = root / ("devc-tool.exe" if os.name == "nt" else "devc-tool")
        shutil.copy2(compiler, local_compiler)
        compiler = str(local_compiler)
        env = os.environ.copy()
        env.update(GOCACHE=str(root / "go-cache"), CGO_ENABLED="0", GO111MODULE="off", GOFLAGS="", GOAMD64="v3" if args.native else "v1", GOMAXPROCS="8")
        binaries = {lang: root / (lang + (".exe" if os.name == "nt" else "")) for lang in LANGUAGES}
        paths = {lang: root / f"bench.{EXTENSIONS[lang]}" for lang in LANGUAGES}
        for lang in LANGUAGES:
            paths[lang].write_text(sources[lang])

        def command(lang, profile, cache):
            optimized = profile == "optimized"
            if lang == "dev":
                fast = not optimized and args.fast_tcc
                return [compiler, "build", str(paths[lang]), "-o", str(binaries[lang]), "--cc", args.fast_tcc if fast else args.cc, "--cache-dir", str(cache)] + (["--release"] if optimized else ["--fast"] if fast else []) + (["--native"] if optimized and args.native else [])
            if lang == "c":
                return [args.cc, "-std=c11", f"-O{args.c_opt}" if optimized else "-O0", "-fwrapv", str(paths[lang]), "-o", str(binaries[lang])] + (["-march=native"] if optimized and args.native else [])
            if lang == "rust":
                return [args.rustc, "--edition=2021", "--crate-name", "native_bench", "-C", f"opt-level={args.rust_opt}" if optimized else "opt-level=0", "-C", "overflow-checks=off", "-C", "debuginfo=0", str(paths[lang]), "-o", str(binaries[lang])] + (["-C", "target-cpu=native"] if optimized and args.native else [])
            return [args.go, "build"] + ([] if optimized else ["-gcflags=-N -l"]) + ["-o", str(binaries[lang]), str(paths[lang])]

        # The empty private Go cache includes building its standard library. Record
        # this separately; the other distributions ship already compiled runtimes.
        for lang in LANGUAGES:
            cmd = command(lang, "optimized", root / "bootstrap-cache")
            duration, _ = process(cmd, env=env, cwd=root)
            report["bootstrap_builds"][lang] = {"elapsed_ms": round(duration, 3), "command": cmd}
            print(f"Bootstrap {lang}: {duration:.1f} ms", flush=True)

        paths["go"].write_text(sources["go"] + "\n// application cache invalidation probe\n")
        probe = command("go", "optimized", root / "probe-cache")
        probe.insert(2, "-n")
        _, planned = process(probe, env=env, cwd=root)
        if " -p main " not in planned.stderr + planned.stdout:
            raise AssertionError("Go main-package source edit did not require recompilation")
        report["go_source_change_requires_main_compile"] = True

        rng = random.Random(20261008)
        last_commands = {}
        run_env = env.copy()
        run_env.update(GOMAXPROCS="1", GOGC="100")
        fixtures = [(1, 0, 0, 0), (1, 257, 0, 0), (1, 65537, 0, 0),
                    (2, 0, MASK64, 0), (2, 1, 1, 0), (2, 251, 88172645463325252, 0),
                    (3, 0, 0, 0), (3, 17, 123456789, 0), (3, 97, 123456789, 37)]
        for profile in ("unoptimized", "optimized"):
            timings = {lang: [] for lang in LANGUAGES}
            for iteration in range(args.build_runs):
                order = list(LANGUAGES)
                rng.shuffle(order)
                for lang in order:
                    # A changed comment invalidates the Go main-package cache. Dev
                    # instead gets a fresh object cache, since comments aren't emitted.
                    marker = "#" if lang == "dev" else "//"
                    paths[lang].write_text(sources[lang] + f"\n{marker} rebuild {profile} {iteration}\n")
                    if binaries[lang].exists():
                        binaries[lang].unlink()
                    cmd = command(lang, profile, root / f"cache-{profile}-{iteration}")
                    duration, result = process(cmd, env=env, cwd=root)
                    if lang == "dev" and "1 compiled, 0 cached" not in result.stderr:
                        raise AssertionError(result.stderr)
                    timings[lang].append(duration)
                    last_commands[lang] = cmd
            report["builds"][profile] = {lang: stats(values) for lang, values in timings.items()}
            for lang in LANGUAGES:
                for values in fixtures:
                    run_checked(binaries[lang], values, run_env)
            print(f"Build {profile}: " + ", ".join(f"{lang} {statistics.median(timings[lang]):.1f} ms" for lang in LANGUAGES), flush=True)
        report["commands"] = last_commands
        report["binary_bytes"] = {lang: binary.stat().st_size for lang, binary in binaries.items()}
        for lang in ("dev", "go"):
            values = []
            for _ in range(args.build_runs):
                duration, result = process(last_commands[lang], env=env, cwd=root)
                if lang == "dev" and "0 compiled, 1 cached; link cached" not in result.stderr:
                    raise AssertionError(result.stderr)
                values.append(duration)
            report["unchanged_builds"][lang] = stats(values)

        report["correctness_fixtures_per_language"] = len(fixtures)
        report["correctness_profiles"] = ["unoptimized", "optimized"]
        workloads = {
            "masked_sum": (1, 200000003, 0, 0),
            "xorshift64": (2, 30000000, 88172645463325252, 0),
            "array_update": (3, 262144, 123456789, 256),
        }
        with affinity(not args.no_pin) as selected:
            report["runtime_cpu"] = selected
            for name, values in workloads.items():
                expected = reference(*values)
                for _ in range(args.warmups):
                    order = list(LANGUAGES)
                    rng.shuffle(order)
                    for lang in order:
                        run_checked(binaries[lang], values, run_env)
                samples = {lang: [] for lang in LANGUAGES}
                orders = []
                for _ in range(args.runs):
                    order = list(LANGUAGES)
                    rng.shuffle(order)
                    orders.append(order)
                    for lang in order:
                        samples[lang].append(run_checked(binaries[lang], values, run_env))
                report["runtime"][name] = {"input": values, "expected": expected,
                                          "orders": orders, "languages": {lang: stats(v) for lang, v in samples.items()}}
                print(f"Runtime {name}: " + ", ".join(f"{lang} {statistics.median(samples[lang]):.3f} ms" for lang in LANGUAGES), flush=True)
        if args.keep_workdir:
            report["workdir"] = str(root)
        output = Path(args.output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2) + "\n")
        if args.markdown:
            path = Path(args.markdown)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(markdown(report))
        print(f"Report: {output}", flush=True)
    finally:
        if temporary:
            temporary.cleanup()


if __name__ == "__main__":
    main()
