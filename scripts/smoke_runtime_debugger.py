"""Exercise the real source debugger through piped interactive commands."""
import argparse
import pathlib
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--bin-dir', default='target/debug')
args = parser.parse_args()
binary = pathlib.Path(args.bin_dir).resolve() / ('devrun.exe' if __import__('os').name == 'nt' else 'devrun')

with tempfile.TemporaryDirectory() as directory:
    root = pathlib.Path(directory).resolve()
    source = root / 'main.dev'
    library = root / 'lib.dev'
    library.write_text('export fn twice(x: i32): i32 {\n    return x * 2\n}\n', encoding='utf-8')
    source.write_text('use "lib.dev" as lib\nfn main() {\n    let x: i32 = 1\n    let y = lib.twice(x)\n    while x < 3 {\n        x += 1\n    }\n    print(y + x)\n}\nmain()\n', encoding='utf-8')

    def run(commands=None):
        command = [str(binary), str(source)]
        if commands is not None:
            command += ['--debug-runtime', '--engine', 'auto']
        return subprocess.run(command, input=commands, text=True, capture_output=True, timeout=20)

    normal = run()
    assert normal.returncode == 0 and normal.stdout.strip() == '5', normal
    stepped = run('step\nnext\nlocals\nstep\nlocals\nstack\nfinish\nlocals\ncontinue\n')
    assert stepped.returncode == 0 and stepped.stdout.strip() == '5', stepped
    assert 'x = 1' in stepped.stderr and 'y = 2' in stepped.stderr, stepped.stderr
    assert 'twice' in stepped.stderr and '#1 main' in stepped.stderr, stepped.stderr
    loop = run(f'break {source}:5\ncontinue\nlocals\ncontinue\nlocals\ncontinue\nlocals\nclear\ncontinue\n')
    assert loop.returncode == 0 and loop.stdout.strip() == '5', loop
    for value in (1, 2, 3):
        assert f'x = {value}' in loop.stderr, loop.stderr
    imported = run(f'break {library}:2\ncontinue\nlocals\nstack\ncontinue\n')
    assert imported.returncode == 0 and 'x = 1' in imported.stderr and 'twice' in imported.stderr, imported
    for commands in ('quit\n', ''):
        stopped = run(commands)
        assert stopped.returncode == 0 and not stopped.stdout, stopped
    invalid = run('break 0\nbreak missing.dev:2\nbreak oops\nhelp\nquit\n')
    assert 'line must be positive' in invalid.stderr and 'unknown loaded source' in invalid.stderr and 'usage: break' in invalid.stderr, invalid
    cli = binary.with_name('d.exe' if __import__('os').name == 'nt' else 'd')
    if cli.exists():
        result = subprocess.run([str(cli), 'run', str(source), '--debug-runtime'], input='continue\n', text=True, capture_output=True, timeout=20)
        assert result.returncode == 0 and result.stdout.strip() == '5' and 'paused' in result.stderr, result
    source.write_text('fn main() {\n    let x = 1\n    if true {\n        let x = 2\n        print(x)\n    }\n    print(x)\n}\nmain()\n', encoding='utf-8')
    shadow = run('break 5\nbreak 7\ncontinue\nlocals\ncontinue\nlocals\ncontinue\n')
    assert shadow.returncode == 0 and shadow.stdout.split() == ['2', '1'] and 'x = 2' in shadow.stderr and 'x = 1' in shadow.stderr, shadow
    source.write_text('fn main() {\n    let zero = 0\n    print(1 / zero)\n}\nmain()\n', encoding='utf-8')
    error = run('continue\n')
    assert error.returncode != 0 and 'zero' in error.stderr.lower(), error
print('runtime debugger: stepping, locals, stack, loops, imported breakpoints, quit/EOF passed')
