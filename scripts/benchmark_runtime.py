"""Measure real source-runtime compute workloads; check every result."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time

WORKLOADS = {
    'integer_loop': ('''fn sum(n i64) i64 {
 let i = 0; let total = 0
 while i < n { total += i & 255; i += 1 }
 return total
}
print(sum(500000))''', str(sum(i & 255 for i in range(500000)))),
    'float_loop': ('''fn compute(n i64) f64 {
 let i = 0; let total = 0.0
 while i < n { total += (i as f64) * 0.5; i += 1 }
 return total
}
print(compute(300000) as i64)''', str(300000 * 299999 // 4)),
    'array_loop': ('''fn compute(n i64) i64 {
 let a [i64; 256] = [''' + ','.join(str(i) for i in range(256)) + ''']
 let i = 0; let total = 0
 while i < n { let j = i & 255; a[j] += 1; total += a[j]; i += 1 }
 return total
}
print(compute(20000))''', str(sum((i & 255) + i // 256 + 1 for i in range(20000)))),
    'function_loop': ('''fn mix(x i64) i64 { return (x * 17 + 3) & 65535 }
fn compute(n i64) i64 {
 let i = 0; let total = 0
 while i < n { total += mix(i); i += 1 }
 return total
}
print(compute(30000))''', str(sum((i * 17 + 3) & 65535 for i in range(30000)))),
}

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runtime', required=True)
    parser.add_argument('--before')
    parser.add_argument('--runs', type=int, default=5)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    assert args.runs > 0
    binaries = {'after': Path(args.runtime).resolve()}
    if args.before:
        binaries['before'] = Path(args.before).resolve()
    env = dict(os.environ, PATH='', CC='missing-compiler', DEV_CC='missing-compiler')
    report = {'platform': platform.platform(), 'runs': args.runs,
              'method': 'End-to-end process wall time, one warmup, alternating before/after order; median/min/max. Source and data in OS-local temporary directory; empty compiler PATH; every output checked. Fixed workloads; no native performance claim.',
              'binaries': {k: {'path': str(v), 'sha256': hashlib.sha256(v.read_bytes()).hexdigest()} for k,v in binaries.items()},
              'workloads': {}}
    with tempfile.TemporaryDirectory(prefix='dev-runtime-bench-') as d:
        root = Path(d)
        for name, (code, expected) in WORKLOADS.items():
            source = root / (name + '.dev')
            source.write_text(code, encoding='utf-8')
            times = {k: [] for k in binaries}
            for iteration in range(args.runs + 1):
                order = list(binaries)
                if iteration % 2:
                    order.reverse()
                for label in order:
                    start = time.perf_counter()
                    result = subprocess.run([str(binaries[label]), str(source)], cwd=root,
                                            env=env, capture_output=True, text=True, timeout=120)
                    elapsed = (time.perf_counter() - start) * 1000
                    assert result.returncode == 0 and result.stdout.strip() == expected, (name, result.stdout, result.stderr)
                    if iteration:
                        times[label].append(elapsed)
            row = {'output': expected, 'source_sha256': hashlib.sha256(code.encode()).hexdigest()}
            for label, samples in times.items():
                row[label] = {'median_ms': round(statistics.median(samples), 3),
                              'min_ms': round(min(samples), 3), 'max_ms': round(max(samples), 3),
                              'samples_ms': samples}
            if 'before' in times:
                row['speedup'] = round(statistics.median(times['before']) / statistics.median(times['after']), 2)
            report['workloads'][name] = row
            print(name, json.dumps(row))
        assert all(p.suffix == '.dev' for p in root.rglob('*')), 'runtime generated build artifacts'
    Path(args.output).write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')

if __name__ == '__main__':
    main()
