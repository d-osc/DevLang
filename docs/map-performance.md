# Map hash table measurements

Measured on 2026-10-10, on this Windows computer and its Ubuntu WSL environment.
Both versions insert N integer keys, read all keys into a checked checksum, then
remove all keys and verify an empty map. The baseline is the previous packaged
linear Map implementation. Both versions use the same source and scalar types.

Each measurement has one warmup and three timed processes; results are medians.
Wall time includes process startup. Runtime uses its default auto engine without
a compiler in PATH. Native uses `--release`; compilation is excluded. No shared
snapshots are created, so these numbers do not measure COW cloning.

| 4,000 entries | Previous linear Map | Hash Map | Speedup |
| --- | ---: | ---: | ---: |
| Windows source runtime | 1,234.09 ms | 30.81 ms | 40.0x |
| Windows native | 48.35 ms | 12.57 ms | 3.8x |
| Ubuntu WSL source runtime | 1,120.81 ms | 11.79 ms | 95.0x |
| Ubuntu WSL native | 5.76 ms | 0.69 ms | 8.3x |

These are workload measurements, not a claim about all programs. Small native
maps are dominated by startup cost, and repeated runs can change their ordering.
Sub-millisecond native samples are sensitive to process
startup and scheduling noise. String hashing depends on key length; deterministic
hash collisions can still produce O(n) lookup, and COW writes copy shared storage.

Raw samples, platform metadata and binary SHA-256 identities:
[Windows](map-benchmark-windows.json), [Linux](map-benchmark-linux.json).

To compare against a saved baseline installation:

```text
python scripts/benchmark_map.py --bin-dir target/release --baseline-bin-dir OLD_BIN_DIR --repeat 3 --output result.json
```

Keep the old binaries separately before packaging a newer build. The script
checks program output on every run and never includes C compilation in its native
execution timing.
