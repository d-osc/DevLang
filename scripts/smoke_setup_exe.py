"""Validate the standalone EXE without adjacent payload or script files."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import winreg


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('installer', type=Path)
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='devlang-exe-test-') as temporary:
        root = Path(temporary)
        setup = root / 'only installer.exe'
        shutil.copyfile(args.installer, setup)
        prefix = root / 'install with spaces'
        log = root / 'install.log'
        def install(extra=(), success=True, executable=setup):
            result = subprocess.run([str(executable), '--silent', '--prefix', str(prefix), '--log', str(log)] + list(extra))
            if (result.returncode == 0) != success:
                raise AssertionError(log.read_text(encoding='utf-8-sig') if log.exists() else 'Installer failed without a log')
        isolated = ['--no-path', '--no-registration']
        damaged = root / 'damaged.exe'
        data = bytearray(setup.read_bytes())
        offset = data.index(b'PK\x03\x04')
        data[offset + 60] ^= 1
        damaged.write_bytes(data)
        install(isolated, success=False, executable=damaged)
        assert not prefix.exists(), 'Damaged payload must not be installed'
        prefix.mkdir()
        (prefix / 'user.txt').write_text('keep')
        install(isolated, success=False)
        (prefix / 'user.txt').unlink()
        install(isolated)
        assert (prefix / 'stdlib/modules/io.dev').is_file()
        assert (prefix / 'examples/hello.dev').is_file()
        empty_path = os.environ.copy()
        empty_path['PATH'] = ''
        result = subprocess.run([str(prefix / 'd.exe'), '-e', 'print(42)'], env=empty_path, text=True, capture_output=True, check=True)
        assert result.stdout.strip() == '42'
        result = subprocess.run([str(prefix / 'd.exe'), str(prefix / 'examples/features/continuations.dev')], env=empty_path, text=True, capture_output=True, check=True)
        assert result.stdout.split() == ['42', '42']
        (prefix / 'user.txt').write_text('keep')
        install(isolated)
        assert (prefix / 'user.txt').read_text() == 'keep'
        powershell = str(Path(os.environ['WINDIR']) / 'System32/WindowsPowerShell/v1.0/powershell.exe')
        def uninstall():
            subprocess.run([powershell, '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', str(prefix / 'uninstall.ps1'), '-Prefix', str(prefix), '-Uninstall'], check=True)
        uninstall()
        assert not (prefix / 'd.exe').exists()
        assert (prefix / 'user.txt').read_text() == 'keep'
        (prefix / 'user.txt').unlink()

        key_name = r'Software\Microsoft\Windows\CurrentVersion\Uninstall\DevLang'
        try:
            with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_name):
                registered_before = True
        except FileNotFoundError:
            registered_before = False
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, 'Environment', 0, winreg.KEY_READ | winreg.KEY_SET_VALUE) as environment:
            try:
                original, kind = winreg.QueryValueEx(environment, 'Path')
            except FileNotFoundError:
                original, kind = None, winreg.REG_EXPAND_SZ
            try:
                options = ['--no-registration'] if registered_before else []
                install(options)
                assert str(prefix) in winreg.QueryValueEx(environment, 'Path')[0].split(';')
                if not registered_before:
                    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_name) as key:
                        assert winreg.QueryValueEx(key, 'InstallLocation')[0] == str(prefix)
                        assert 'uninstall.ps1' in winreg.QueryValueEx(key, 'UninstallString')[0]
                install(options)
                assert winreg.QueryValueEx(environment, 'Path')[0].split(';').count(str(prefix)) == 1
                uninstall()
                assert str(prefix) not in winreg.QueryValueEx(environment, 'Path')[0].split(';')
                if not registered_before:
                    try:
                        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_name):
                            raise AssertionError('Uninstall entry was not removed')
                    except FileNotFoundError:
                        pass
            finally:
                if original is None:
                    try:
                        winreg.DeleteValue(environment, 'Path')
                    except FileNotFoundError:
                        pass
                else:
                    winreg.SetValueEx(environment, 'Path', 0, kind, original)
        print('Passed: standalone EXE, corrupt payload, conflict rejection, compiler-free runtime, stdlib/examples, upgrade, PATH, Installed apps and uninstall')


if __name__ == '__main__':
    main()
