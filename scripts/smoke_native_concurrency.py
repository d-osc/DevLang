"""Concurrent cold C dependency preparation must publish one reusable library."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', required=True)
    folder = Path(parser.parse_args().bin_dir).resolve()
    ext = '.exe' if os.name == 'nt' else ''
    with tempfile.TemporaryDirectory(prefix='dev-concurrent-native-') as directory:
        root = Path(directory)
        (root / 'device.c').write_text('long long device_add(long long a,long long b){return a+b;}\n', encoding='utf-8')
        command = [str(folder / ('devc' + ext)), 'native-build', str(root), '--symbol', 'device_add']
        def prepare(_):
            result = subprocess.run(command, capture_output=True, text=True, timeout=45)
            assert result.returncode == 0, result.stderr
            return result.stdout.strip()
        with ThreadPoolExecutor(max_workers=8) as pool:
            paths = list(pool.map(prepare, range(8)))
        assert len(set(paths)) == 1, paths
        library = Path(paths[0])
        before = library.stat().st_mtime_ns
        env = dict(os.environ, PATH='')
        result = subprocess.run(command, env=env, capture_output=True, text=True, timeout=45)
        assert result.returncode == 0 and result.stdout.strip() == paths[0], result.stderr
        assert library.stat().st_mtime_ns == before
        assert not list((root / '.dev-cache' / 'native').glob('*.lock'))
    print('PASS: 8 concurrent cold builders, warm compiler-free cache reuse, lock cleanup')


if __name__ == '__main__':
    main()
