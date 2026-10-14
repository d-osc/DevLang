"""Check package script shortcuts, aliases, forwarding and command collisions."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--bin', required=True)
binary = str(Path(parser.parse_args().bin).resolve())
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    (root / 'src').mkdir()
    (root / 'args.py').write_text('import sys; print(repr(sys.argv[1:])); sys.exit(7)', encoding='utf-8')
    scripts = {name: [sys.executable, 'args.py'] for name in ['start', 'check', 'format', 'build']}
    (root / 'package.don').write_text(json.dumps({'version': '1.0.0', 'package': {'name': 'test', 'entry': 'main.dev', 'modules': 'src'}, 'scripts': scripts}), encoding='utf-8')
    for prefix in [['start'], ['check'], ['format'], ['run', 'start'], ['--run', 'start'], ['-r', 'start'], ['task', 'start'], ['run', 'build']]:
        result = subprocess.run([binary, *prefix, '--', 'hello world', '&literal'], cwd=root, capture_output=True, text=True, timeout=15)
        assert result.returncode == 7, (prefix, result.stdout, result.stderr)
        assert result.stdout.strip() == "['hello world', '&literal']", (prefix, result.stdout)
    (root / 'main.dev').write_text('print("source")', encoding='utf-8')
    for prefix in [['main.dev'], ['run', 'main.dev']]:
        result = subprocess.run([binary, *prefix], cwd=root, capture_output=True, text=True, timeout=15)
        assert result.returncode == 0 and result.stdout.strip() == 'source', result
    result = subprocess.run([binary, 'check', 'main.dev'], cwd=root, capture_output=True, text=True, timeout=15)
    assert result.returncode == 0, result.stderr
print('PASS: script shortcuts, run aliases, literal arguments, exit status and explicit source/check')
