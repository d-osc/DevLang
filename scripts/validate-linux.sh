#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Build on Linux's native filesystem to avoid measuring /mnt/c filesystem overhead.
task_stage="$(mktemp -d /tmp/dev-lang-validation-XXXXXX)"
cp Cargo.toml Cargo.lock "$task_stage/"
cp -r compiler scripts examples runtime syntax stdlib cli "$task_stage/"
cd "$task_stage"
cargo test
cargo build --release
python3 stdlib/build.py
python3 scripts/smoke.py --compiler target/release/devc
python3 scripts/smoke_fast.py --compiler target/release/devc
python3 scripts/smoke_runtime.py --runtime target/release/devrun
python3 scripts/smoke_json.py --runtime target/release/devrun
python3 scripts/smoke_numeric.py --runtime target/release/devrun
python3 scripts/smoke_features.py --bin-dir target/release
python3 scripts/smoke_safety.py --bin-dir target/release
python3 scripts/smoke_reference_native.py --compiler target/release/devc --sanitize
python3 scripts/smoke_cli.py --bin-dir target/release
python3 scripts/smoke_tooling.py --bin-dir target/release
python3 scripts/smoke_entry.py --runtime target/release/devrun --compiler target/release/devc
python3 scripts/smoke_ffi.py --bin-dir target/release
python3 scripts/smoke_native_auto.py --bin-dir target/release
python3 scripts/smoke_native_concurrency.py --bin-dir target/release
python3 scripts/smoke_stdlib.py --compiler target/release/devc
python3 stdlib/tests/test_native.py --sanitize
python3 scripts/test_benchmark_compare.py
python3 scripts/benchmark.py --compiler target/release/devc --output benchmarks-linux.json
printf 'Validated Linux workspace: %s\n' "$task_stage"
