#!/usr/bin/env python3
"""Measure Dev's old/new/backend profiles together; no timing assertions."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import random
import shutil
import tempfile
from benchmark_compare import affinity, cpu_model, process, run_checked, stats


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--before", required=True)
    parser.add_argument("--after", required=True)
    parser.add_argument("--cc", default="cc")
    parser.add_argument("--clang", default="clang")
    parser.add_argument("--tcc", required=True)
    parser.add_argument("--runs", type=int, default=15)
    parser.add_argument("--build-runs", type=int, default=5)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    if args.runs < 3 or args.build_runs < 3:
        parser.error("at least three runtime/build samples required")
    source = (Path(__file__).resolve().parents[1] / "benchmarks/comparison/bench.dev").read_text()
    report = {"measured_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
              "cpu_model": cpu_model(), "runs": args.runs, "build_runs": args.build_runs,
              "warmups": 2, "runtime": {}, "builds": {}, "toolchains": {},
              "source_sha256": hashlib.sha256(source.encode()).hexdigest(),
              "method": "All Dev profiles in one randomized session, same source and runtime inputs, runtime pinned. Full builds use fresh caches; checksums verified on every execution. No universal ranking."}
    with tempfile.TemporaryDirectory(prefix="dev-lang-profiles-") as directory:
        root = Path(directory)
        compilers = {}
        for name, path in (("before", args.before), ("after", args.after)):
            destination = root / (name + (".exe" if os.name == "nt" else ""))
            shutil.copy2(Path(path).resolve(), destination)
            compilers[name] = str(destination)
            report["toolchains"][name] = process([str(destination), "--version"])[1].stdout.strip()
            report[name + "_sha256"] = hashlib.sha256(destination.read_bytes()).hexdigest()
        for name, path in (("gcc", args.cc), ("clang", args.clang), ("tcc", args.tcc)):
            report["toolchains"][name] = process([path, "--version"])[1].stdout.splitlines()[0]
        profiles = {
            "before_O2": ("before", args.cc, ["--release"]),
            "after_O3": ("after", args.cc, ["--release"]),
            "after_O3_native": ("after", args.cc, ["--release", "--native"]),
            "after_clang_O3": ("after", args.clang, ["--release"]),
            "after_clang_O3_native": ("after", args.clang, ["--release", "--native"]),
            "before_development": ("before", args.cc, []),
            "after_development": ("after", args.cc, []),
            "after_fast": ("after", args.tcc, ["--fast"]),
        }
        report["profiles"] = profiles
        paths = {name: root / (name + ".dev") for name in profiles}
        binaries = {name: root / (name + "-app") for name in profiles}
        for path in paths.values():
            path.write_text(source)
        rng = random.Random(2026100802)
        samples = {name: [] for name in profiles}
        last = {}
        for iteration in range(args.build_runs):
            names = list(profiles)
            rng.shuffle(names)
            for name in names:
                compiler, backend, flags = profiles[name]
                command = [compilers[compiler], "build", str(paths[name]), "--cc", backend,
                           "-o", str(binaries[name]), "--cache-dir", str(root / f"cache-{name}-{iteration}"), *flags]
                elapsed, output = process(command, cwd=root)
                if "1 compiled, 0 cached" not in output.stderr:
                    raise AssertionError(output.stderr)
                samples[name].append(elapsed)
                last[name] = command
        report["builds"] = {name: stats(values) for name, values in samples.items()}
        report["unchanged_builds"] = {}
        for name in profiles:
            elapsed = []
            for _ in range(args.build_runs):
                duration, result = process(last[name], cwd=root)
                if "0 compiled, 1 cached; link cached" not in result.stderr:
                    raise AssertionError(result.stderr)
                elapsed.append(duration)
            report["unchanged_builds"][name] = stats(elapsed)
            for values in ((1, 257, 0, 0), (2, 251, 88172645463325252, 0), (3, 97, 123456789, 37)):
                run_checked(binaries[name], values, os.environ.copy())
            print(f"Build {name}: {report['builds'][name]['median_ms']:.3f} ms", flush=True)
        runtime_names = [name for name in profiles if "development" not in name and name != "after_fast"]
        workloads = {"masked_sum": (1, 200000003, 0, 0),
                     "xorshift64": (2, 30000000, 88172645463325252, 0),
                     "array_update": (3, 262144, 123456789, 256)}
        with affinity(True) as cpu:
            report["runtime_cpu"] = cpu
            for name, values in workloads.items():
                samples = {profile: [] for profile in runtime_names}
                orders = []
                for iteration in range(args.runs + 2):
                    order = runtime_names.copy()
                    rng.shuffle(order)
                    for profile in order:
                        duration = run_checked(binaries[profile], values, os.environ.copy())
                        if iteration >= 2:
                            samples[profile].append(duration)
                    if iteration >= 2:
                        orders.append(order)
                report["runtime"][name] = {"input": values, "orders": orders,
                                           "profiles": {p: stats(v) for p, v in samples.items()}}
                print(name + ": " + ", ".join(f"{p} {stats(v)['median_ms']:.3f} ms" for p, v in samples.items()), flush=True)
        Path(args.output).write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
