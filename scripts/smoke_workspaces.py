"""Test workspace projects and SemVer Git tags without external network access."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--bin-dir', required=True)
    d = Path(p.parse_args().bin_dir).resolve() / ('d.exe' if os.name == 'nt' else 'd')
    count = 0
    with tempfile.TemporaryDirectory(prefix='dev-workspaces-') as tmp:
        root = Path(tmp)

        def command(*args, cwd=root, error=None, env=None):
            nonlocal count
            r = subprocess.run([str(d), *map(str, args)], cwd=cwd, env=env, capture_output=True, text=True, encoding='utf-8', timeout=45)
            if error:
                assert r.returncode != 0 and error in r.stderr, (args, r.stdout, r.stderr)
            else:
                assert r.returncode == 0, (args, r.stdout, r.stderr)
            count += 1
            return r

        def package(at, name, version='1.0.0', dependencies=None, workspace=None):
            at.mkdir(parents=True, exist_ok=True)
            value = dict(version=version, package=dict(name=name), dependencies=dependencies or {})
            if workspace is not None:
                value['workspace'] = dict(members=workspace)
            (at/'package.don').write_text(json.dumps(value, ensure_ascii=False), encoding='utf-8')
            (at/'src').mkdir(exist_ok=True)
            (at/'src/main.dev').write_text('fn main() { print(42) }\nmain()\n')

        workspace = root/'workspace'
        package(workspace, 'monorepo', workspace=['apps/app', 'libs/math', 'libs/base'])
        app, math, base = workspace/'apps/app', workspace/'libs/math', workspace/'libs/base'
        package(app, 'app', dependencies={'math': dict(workspace=True, version='^1.0')})
        package(math, 'math', dependencies={'base': dict(workspace=True, version='~1.0')})
        package(base, 'base')
        (base/'src/lib.dev').write_text('fn value() i64 { return 42 }\n')
        (math/'src/lib.dev').write_text('use "base/lib"\nfn value() i64 { return lib.value() }\n')
        (app/'src/main.dev').write_text('use "math/lib"\nfn main() { print(lib.value()) }\nmain()\n')
        assert all(name in command('pkg', 'workspace', cwd=app).stdout for name in ('app', 'math', 'base'))
        command('pkg', 'install', '--workspace', cwd=app)
        previous = {at: (at/'package-lock.don').read_bytes() for at in (workspace, app, math, base)}
        command('pkg', 'install', '--workspace', '--locked', cwd=workspace)
        assert all((at/'package-lock.don').read_bytes() == value for at, value in previous.items())
        no_cc = dict(os.environ, PATH='', DEV_CC='missing')
        for engine in ('auto', 'ast'):
            assert command('run', '--package', 'app', '--engine', engine, cwd=workspace, env=no_cc).stdout == '42\n'
            assert command('run', cwd=app/'src', env=no_cc).stdout == '42\n'
        command('check', '--package', 'app', cwd=workspace)
        command('build', '--package', 'app', '--release', '-o', root/'native.exe', cwd=workspace)
        assert subprocess.check_output([str(root/'native.exe')], text=True) == '42\n'
        command('fmt', '--package', 'app', '--stdout', 'src/main.dev', cwd=workspace)
        command('pkg', 'list', '--package', 'app', cwd=base)
        command('run', '--package', 'missing', cwd=workspace, error='not found')
        command('run', '--package', 'app', '--package', 'app', cwd=workspace, error='duplicate')
        # Dependency lock metadata must not create circular workspace hashes.
        (base/'package-lock.don').write_text('# extra lock comment\n' + previous[base].decode())
        assert command('run', '--package', 'app', cwd=workspace).stdout == '42\n'
        (base/'src/lib.dev').write_text('fn value() i64 { return 43 }\n')
        command('run', '--package', 'app', cwd=workspace, error='content changed')
        command('pkg', 'install', '--workspace', '--locked', cwd=workspace, error='content changed')
        command('pkg', 'install', '--workspace', cwd=workspace)
        assert command('run', '--package', 'app', cwd=workspace).stdout == '43\n'
        original = (app/'package.don').read_bytes()
        command('pkg', 'add', 'base', '--workspace', '--version', '=9.0.0', cwd=app, error='does not satisfy')
        assert (app/'package.don').read_bytes() == original
        command('pkg', 'add', 'base', '--workspace', '--version', '^1', cwd=app)
        combined_lock = (app/'package-lock.don').read_bytes()
        command('pkg', 'install', '--locked', cwd=app)
        assert (app/'package-lock.don').read_bytes() == combined_lock
        assert command('run', cwd=app).stdout == '43\n'
        command('pkg', 'remove', 'base', cwd=app)
        original = (app/'package.don').read_bytes()
        command('pkg', 'add', 'missing', '--workspace', cwd=app, error='not found')
        assert (app/'package.don').read_bytes() == original
        command('pkg', 'add', 'base', '--workspace', '--path', base, cwd=app, error='exactly one')
        command('pkg', 'add', 'base', '--workspace', '--rev', 'v1', cwd=app, error='Git-only')
        command('pkg', 'add', 'base', '--workspace', '--version', '1 || 2', cwd=app, error='SemVer')
        # Changing workspace membership invalidates workspace references.
        metadata = json.loads((workspace/'package.don').read_text())
        for members, error in [(['../elsewhere'], "without '..'"), (['apps/app', 'apps/app'], 'overlap'), (['libs/math', 'libs'], 'overlap'), ([], '1..128')]:
            edited = dict(metadata, workspace=dict(members=members))
            (workspace/'package.don').write_text(json.dumps(edited))
            command('pkg', 'workspace', cwd=workspace, error=error)
        (workspace/'package.don').write_text(json.dumps(metadata))
        command('pkg', 'workspace', cwd=workspace)
        standalone = root/'standalone'
        package(standalone, 'standalone')
        command('run', '--package', 'app', cwd=standalone, error='workspace root')
        command('pkg', 'add', 'base', '--workspace', cwd=standalone, error='workspace root')
        # Real Git repo containing stable, prerelease and non-SemVer tags.
        repo = root/'remote'
        package(repo, 'remote')
        def git(*args):
            return subprocess.check_output(['git', '-C', str(repo), *args], text=True, stderr=subprocess.DEVNULL).strip()
        git('init'); git('config', 'user.name', 'Workspace Test'); git('config', 'user.email', 'test@example.invalid')
        git('config', 'core.autocrlf', 'false')
        commits = {}
        def release(version, value, tag=None):
            package(repo, 'remote', version)
            (repo/'src/lib.dev').write_text(f'fn value() i64 {{ return {value} }}\n')
            git('add', '.'); git('commit', '-m', version)
            git('tag', tag or 'v'+version)
            commits[version] = git('rev-parse', 'HEAD')
        release('1.0.0', 10)
        release('1.2.0', 12)
        release('1.3.0-beta.1', 13)
        release('2.0.0', 20)
        git('tag', 'not-a-version')
        consumer = root/'consumer'
        package(consumer, 'consumer')
        (consumer/'src/main.dev').write_text('use "remote/lib"\nprint(lib.value())\n')
        command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', '^1.0', cwd=consumer)
        lock = lambda: json.loads(subprocess.check_output([str(d), "don", "to-json", str(consumer/'package-lock.don')], text=True, encoding="utf-8"))['packages']['remote']
        assert lock()['resolved_version'] == '1.2.0' and lock()['commit'] == commits['1.2.0']
        assert command('run', cwd=consumer).stdout == '12\n'
        release('1.2.1', 121)
        command('pkg', 'install', cwd=consumer)
        assert lock()['resolved_version'] == '1.2.0'
        command('pkg', 'update', cwd=consumer)
        assert lock()['resolved_version'] == '1.2.1'
        assert command('run', cwd=consumer).stdout == '121\n'
        pinned = (consumer/'package-lock.don').read_bytes()
        (consumer/lock()['root']).rename(root/'saved-checkout')
        command('pkg', 'install', '--locked', cwd=consumer)
        assert (consumer/'package-lock.don').read_bytes() == pinned
        for req, expected in [('=1.0.0', '1.0.0'), ('~1.2', '1.2.1'), ('>=1.0, <1.2', '1.0.0'), ('=1.3.0-beta.1', '1.3.0-beta.1'), ('*', '2.0.0')]:
            command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', req, cwd=consumer)
            assert lock()['resolved_version'] == expected
        before = (consumer/'package.don').read_bytes()
        before_lock = (consumer/'package-lock.don').read_bytes()
        command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', '=9.0.0', cwd=consumer, error='no Git SemVer tag')
        assert (consumer/'package.don').read_bytes() == before and (consumer/'package-lock.don').read_bytes() == before_lock
        command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', '^1', '--rev', 'HEAD', cwd=consumer, error='cannot be combined')
        command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', '^1', '--version', '^2', cwd=consumer, error='duplicate')
        release('3.0.0', 30, 'v4.0.0')
        command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', '=4.0.0', cwd=consumer, error='does not satisfy')
        release('3.1.0',31,'v3.2.0')
        command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', '^3', cwd=consumer, error='differs from package')
        git('tag', '2.0.0', commits['2.0.0'])
        command('pkg', 'add', 'remote', '--git', repo.as_uri(), '--version', '=2.0.0', cwd=consumer, error='ambiguous')
        # Path version constraints use the package's manifest version.
        command('pkg', 'add', 'local', '--path', base, '--version', '=1.0.0', cwd=standalone)
        command('pkg', 'install', '--locked', cwd=standalone)
        # Legacy v1 hashes included package-lock.don. Preserve them on locked install.
        legacy = json.loads(subprocess.check_output([str(d), "don", "to-json", str(standalone/'package-lock.don')], text=True, encoding="utf-8"))
        h = hashlib.sha256()
        files = sorted([f for f in base.rglob('*') if f.is_file()], key=lambda f: f.relative_to(base).as_posix())
        for file in files:
            name = file.relative_to(base).as_posix().encode()
            content = file.read_bytes()
            h.update(len(name).to_bytes(8, 'little')); h.update(name)
            h.update(len(content).to_bytes(8, 'little')); h.update(content)
        text = (standalone/'package-lock.don').read_text().replace(legacy['packages']['local']['sha256'], h.hexdigest())
        (standalone/'package-lock.don').write_text(text)
        command('pkg', 'install', '--locked', cwd=standalone)
        assert (standalone/'package-lock.don').read_text() == text
        command('--version')
    print(f'PASS: {count} workspace/SemVer commands; runtime, native, local Git tags, lock restoration and legacy hashes')


if __name__ == '__main__':
    main()
