"""Compare identical Map workloads; compilation is excluded from native timing."""
import argparse
import json
import hashlib
import platform
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', required=True)
    parser.add_argument('--baseline-bin-dir', required=True)
    parser.add_argument('--sizes', default='250,1000,4000')
    parser.add_argument('--repeat', type=int, default=3)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    if args.repeat < 1:
        parser.error('--repeat must be positive')
    sizes = [int(n) for n in args.sizes.split(',')]
    if not sizes or min(sizes) < 1:
        parser.error('--sizes must contain positive integers')
    ext = '.exe' if os.name == 'nt' else ''
    rows = []
    binaries = {label: {name: hashlib.sha256((Path(folder) / (name + ext)).read_bytes()).hexdigest() for name in ("devc", "devrun")} for label, folder in [("baseline", args.baseline_bin_dir), ("hash", args.bin_dir)]}
    with tempfile.TemporaryDirectory(prefix='dev-map-benchmark-') as directory:
        root = Path(directory)
        for n in sizes:
            source = root / 'main.dev'
            source.write_text(f'fn main() {{ let m=Map<i64,i64>(); for i in 0..{n} {{ m.set(i,i) }}; let sum=0; for i in 0..{n} {{ sum+=m.get(i) }}; for i in 0..{n} {{ m.remove(i) }}; print(sum); print(m.len()) }}; main()', encoding='utf-8')
            expected = f'{n * (n - 1) // 2}\n0\n'
            for label, folder in [('baseline', args.baseline_bin_dir), ('hash', args.bin_dir)]:
                folder = Path(folder).resolve()
                for mode in ['runtime', 'native']:
                    if mode == 'native':
                        binary = root / (label + ext)
                        build = subprocess.run([str(folder / ('devc' + ext)), 'build', str(source), '--release', '-o', str(binary)], capture_output=True, text=True, timeout=120)
                        if build.returncode:
                            raise RuntimeError(build.stderr)
                        command, env = [str(binary)], None
                    else:
                        command = [str(folder / ('devrun' + ext)), str(source)]
                        env = dict(os.environ, PATH='', DEV_CC='missing-compiler')
                    times = []
                    for iteration in range(args.repeat + 1):
                        start = time.perf_counter()
                        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=120)
                        elapsed = time.perf_counter() - start
                        assert result.returncode == 0 and result.stdout == expected, (label, mode, result.stdout, result.stderr)
                        if iteration:
                            times.append(elapsed)
                    row = dict(entries=n, implementation=label, mode=mode, median_seconds=statistics.median(times), samples_seconds=times)
                    rows.append(row)
                    print(json.dumps(row), flush=True)
    Path(args.output).write_text(json.dumps(dict(note='Wall time includes process startup and runtime parsing; native compilation is excluded. One warmup, identical insert/get/remove workload, no shared snapshots.', platform=platform.platform(), processor=os.environ.get("PROCESSOR_IDENTIFIER", platform.processor()), binary_sha256=binaries, repetitions=args.repeat, rows=rows), indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
