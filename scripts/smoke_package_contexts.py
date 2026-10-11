"""Real runtime/native/locked/global regressions for contextual peer instances."""
import argparse, json, os, shutil, subprocess, tempfile
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
d = Path(parser.parse_args().d).resolve()
checks = 0
with tempfile.TemporaryDirectory(prefix='dev peer contexts ') as tmp:
    root = Path(tmp)
    env = {**os.environ, 'DEVLANG_HOME': str(root/'home'), 'DEVLANG_NO_PATH_UPDATE': '1', 'GIT_CONFIG_GLOBAL': str(root/'gitconfig'), 'GIT_CONFIG_NOSYSTEM': '1'}
    # Do not inherit a developer's long-path setting and hide Windows regressions.
    (root/'gitconfig').write_text('[core]\nlongpaths = false\n', encoding='utf-8')
    def run(*args, cwd=None, error=None):
        global checks
        result = subprocess.run([str(d), '-C', str(cwd or root/'app'), *map(str, args)], env=env, capture_output=True, text=True, encoding='utf-8', timeout=90)
        assert (result.returncode != 0 and error in result.stderr) if error else result.returncode == 0, (args, result.stdout, result.stderr)
        checks += 1
        return result.stdout.strip()
    def package(name, body, version='1.0.0', deps=None, peers=None, bins=None):
        at = root/name
        (at/'src').mkdir(parents=True)
        (at/'package.don').write_text(json.dumps({'version': version, 'package': {'name': name}, 'dependencies': deps or {}, 'peerDependencies': peers or {}, 'bin': bins or {}}), encoding='utf-8')
        (at/'src/lib.dev').write_text(body, encoding='utf-8')
        return at
    package('base1', 'fn value() i64 { return 10 }\n')
    package('base2', 'fn value() i64 { return 20 }\n', version='2.0.0')
    package('plugin', 'use "base/lib" as base\nfn value() i64 { return base.value() }\n', peers={'base': '^1 || ^2'})
    package('wrapper', 'use "plugin/lib" as plugin\nfn value() i64 { return plugin.value() }\n', deps={'plugin': {'path': '../plugin'}})
    for name, provider in [('left', 'base1'), ('right', 'base2')]:
        package(name, 'use "wrapper/lib" as wrapper\nfn value() i64 { return wrapper.value() }\n', deps={'base': {'path': '../'+provider}, 'wrapper': {'path': '../wrapper'}})
    app = package('app', '', deps={'left': {'path': '../left'}, 'right': {'path': '../right'}}, bins={'peer-answer': 'src/main.dev'})
    (app/'src/main.dev').write_text('use "left/lib" as left\nuse "right/lib" as right\nfn main() { print(left.value()); print(right.value()) }\nmain()\n', encoding='utf-8')
    run('pkg', 'install')
    lock = json.loads(run('don', 'to-json', app/'package-lock.don'))
    assert lock['version'] == 4
    plugins = [p for p in lock['packages'].values() if p['source']['path'] == '../plugin']
    assert len(plugins) == 2 and plugins[0]['root'] != plugins[1]['root']
    for engine in ['auto', 'ast']:
        assert run('run', '--engine', engine) == '10\n20'
    mapping = root/'explicit maps.json'
    mapping.write_text(json.dumps(['manual='+str(root/'base1/src')]), encoding='utf-8')
    assert run('run', '--module-map', mapping) == '10\n20'
    run('check', 'src/main.dev', '--module-map', mapping)
    mapping.write_text('["bad/name=."]', encoding='utf-8')
    run('run', '--module-map', mapping, error='identifier')
    run('check', 'src/main.dev', '--module-map', mapping, error='identifier')
    mapping.write_text('["same=.", "same=."]', encoding='utf-8')
    run('run', '--module-map', mapping, error='duplicate module namespace')
    run('check', 'src/main.dev', '--module-map', mapping, error='duplicate module namespace')
    run('check', 'src/main.dev')
    native = root/('answer.exe' if os.name == 'nt' else 'answer')
    run('build', 'src/main.dev', '--output', native)
    result = subprocess.run([str(native)], capture_output=True, text=True, encoding='utf-8', timeout=30)
    assert result.returncode == 0 and result.stdout.strip() == '10\n20', result
    run('pkg', 'ci'); run('pkg', 'install', '--locked', '--offline')
    assert 'wrapper -> plugin' in run('pkg', 'tree')
    assert 'plugin' in run('pkg', 'why', 'plugin')
    saved = (app/'package-lock.don').read_bytes()
    altered = json.loads(json.dumps(lock))
    first = next(id for id, item in altered['packages'].items() if item['source_key'] == plugins[0]['source_key'])
    altered['packages'][first]['source_root'] = '../absent'
    (app/'package-lock.don').write_text(json.dumps(altered), encoding='utf-8')
    run('run', error='inconsistent source context packages')
    altered = json.loads(json.dumps(lock)); altered['packages'][first]['root'] = '.dev/contexts/invalid'
    (app/'package-lock.don').write_text(json.dumps(altered), encoding='utf-8')
    run('run', error='invalid package context path')
    altered = json.loads(json.dumps(lock)); altered['edges']['root']['left'] = altered['edges']['root']['right']
    (app/'package-lock.don').write_text(json.dumps(altered), encoding='utf-8')
    run('run', error='graph changed')
    (app/'package-lock.don').write_bytes(saved)
    copied = app/plugins[0]['root']/'src/lib.dev'; original = copied.read_bytes()
    copied.write_bytes(original+b'\n'); run('run', error='context content changed'); copied.write_bytes(original)
    relocated = root/'relocated'
    for name in ['app', 'base1', 'base2', 'plugin', 'wrapper', 'left', 'right']:
        shutil.copytree(root/name, relocated/name)
    assert run('run', cwd=relocated/'app') == '10\n20'
    run('pkg', 'ci', cwd=relocated/'app')
    run('pkg', 'install', '--global')
    launcher = root/'home/bin'/('peer-answer.cmd' if os.name == 'nt' else 'peer-answer')
    result = subprocess.run([str(launcher)], env=env, cwd=root, shell=os.name == 'nt', capture_output=True, text=True, encoding='utf-8', timeout=30)
    assert result.returncode == 0 and result.stdout.strip() == '10\n20', (result.stdout, result.stderr)
    run('pkg', 'uninstall', '--global', 'app')
    # A source cycle reuses the ancestor instance instead of expanding indefinitely.
    wrapper_manifest = root/'wrapper/package.don'
    m = json.loads(wrapper_manifest.read_text(encoding='utf-8'))
    m['dependencies']['loop'] = {'path': '../wrapper'}
    wrapper_manifest.write_text(json.dumps(m), encoding='utf-8')
    run('pkg', 'install'); run('pkg', 'ci')
    assert run('run') == '10\n20'
    # Git-backed peer copies retain and verify their original pinned checkout.
    plugin = root/'plugin'
    for args in [('init',), ('config', 'user.name', 'Context Test'), ('config', 'user.email', 'context@example.invalid'), ('add', '.'), ('commit', '-m', 'fixture'), ('tag', 'v1.0.0')]:
        result = subprocess.run(['git', '-C', str(plugin), *args], env=env, capture_output=True, text=True, encoding='utf-8', timeout=30)
        assert result.returncode == 0, result.stderr
    m['dependencies']['plugin'] = {'git': plugin.as_uri(), 'version': '^1'}
    wrapper_manifest.write_text(json.dumps(m), encoding='utf-8')
    run('pkg', 'install'); run('pkg', 'ci'); run('pkg', 'install', '--locked', '--offline')
    assert run('run') == '10\n20'
    run('build', 'src/main.dev', '--output', native)
    result = subprocess.run([str(native)], capture_output=True, text=True, encoding='utf-8', timeout=30)
    assert result.returncode == 0 and result.stdout.strip() == '10\n20', result
    # Identical declarations select independently in incompatible peer scopes.
    plugin_manifest = plugin/'package.don'
    for version, peer, extra in [('1.1.0', '^1', 100), ('2.0.0', '^2', 200)]:
        manifest = json.loads(plugin_manifest.read_text(encoding='utf-8'))
        manifest['version'] = version
        manifest['peerDependencies']['base'] = peer
        plugin_manifest.write_text(json.dumps(manifest), encoding='utf-8')
        (plugin/'src/lib.dev').write_text(f'use "base/lib" as base\nfn value() i64 {{ return base.value() + {extra} }}\n', encoding='utf-8')
        for args in [('add', '.'), ('commit', '-m', version), ('tag', 'v'+version)]:
            result = subprocess.run(['git', '-C', str(plugin), *args], env=env, capture_output=True, text=True, encoding='utf-8', timeout=30)
            assert result.returncode == 0, result.stderr
    m['dependencies']['plugin']['version'] = '^1 || ^2'
    wrapper_manifest.write_text(json.dumps(m), encoding='utf-8')
    run('pkg', 'install')
    lock = json.loads(run('don', 'to-json', app/'package-lock.don'))
    selected = [p for p in lock['resolutions']['packages'].values() if p['source'].get('git') == plugin.as_uri()]
    assert {p['resolved_version'] for p in selected} == {'1.1.0', '2.0.0'}, selected
    for engine in ['auto', 'ast']:
        assert run('run', '--engine', engine) == '110\n220'
    run('pkg', 'ci'); run('pkg', 'install', '--locked', '--offline')
    run('build', 'src/main.dev', '--output', native)
    result = subprocess.run([str(native)], capture_output=True, text=True, encoding='utf-8', timeout=30)
    assert result.returncode == 0 and result.stdout.strip() == '110\n220', result
    run('pkg', 'install', '--global')
    result = subprocess.run([str(launcher)], env=env, cwd=root, shell=os.name == 'nt', capture_output=True, text=True, encoding='utf-8', timeout=30)
    assert result.returncode == 0 and result.stdout.strip() == '110\n220', (result.stdout, result.stderr)
    run('pkg', 'uninstall', '--global', 'app')
    # Compatible scopes share physical instances while preserving logical selections.
    right_manifest = root/'right/package.don'
    right = json.loads(right_manifest.read_text(encoding='utf-8'))
    right['dependencies']['base']['path'] = '../base1'
    right_manifest.write_text(json.dumps(right), encoding='utf-8')
    run('pkg', 'install')
    shared = json.loads(run('don', 'to-json', app/'package-lock.don'))
    instances = [p for p in shared['packages'].values() if p['source'].get('git') == plugin.as_uri()]
    assert len(instances) == 1 and instances[0]['resolved_version'] == '1.1.0', instances
    assert run('run') == '110\n110'
    run('pkg', 'ci'); run('pkg', 'install', '--locked', '--offline')
    right['dependencies']['base']['path'] = '../base2'
    right_manifest.write_text(json.dumps(right), encoding='utf-8')
    run('pkg', 'install')
    assert run('run') == '110\n220'
    # Root self-import fallback cannot shadow a nearer ancestor peer provider.
    root_manifest = json.loads((app/'package.don').read_text(encoding='utf-8'))
    root_manifest['package']['name'] = 'base'
    root_manifest['version'] = '3.0.0'
    (app/'package.don').write_text(json.dumps(root_manifest), encoding='utf-8')
    (app/'src/lib.dev').write_text('fn value() i64 { return 999 }\n', encoding='utf-8')
    run('pkg', 'install'); run('pkg', 'ci')
    assert run('run') == '110\n220'
    # The logical resolution graph is also checked before execution.
    saved = (app/'package-lock.don').read_bytes()
    tampered = json.loads(run('don', 'to-json', app/'package-lock.don'))
    tampered['resolutions']['edges']['root']['left'] = tampered['resolutions']['edges']['root']['right']
    (app/'package-lock.don').write_text(json.dumps(tampered), encoding='utf-8')
    run('run', error='graph changed')
    (app/'package-lock.don').write_bytes(saved)
    print(f'Contextual peers: {checks} CLI checks passed, plus native execution and global launcher')
