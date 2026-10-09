"""Exercise offline install, upgrade, command execution, conflicts and uninstall."""
import argparse
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('installer', type=Path)
    args = parser.parse_args()
    installer = args.installer.resolve()
    windows = os.name == 'nt'
    with tempfile.TemporaryDirectory(prefix='devlang-installer-test-') as temporary:
        root = Path(temporary)
        prefix = root / 'install with spaces'
        if windows:
            extracted = root / 'setup'
            with zipfile.ZipFile(installer) as archive:
                archive.extractall(extracted)
            command = ['powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', str(extracted / 'install.ps1'), '-Prefix', str(prefix), '-NoPath', '-NoRegistration']
        else:
            command = [sys.executable, str(installer), '--prefix', str(prefix), '--bin-dir', str(root / 'bin')]
        def run(extra=(), success=True):
            result = subprocess.run(command + list(extra), text=True, capture_output=True)
            if (result.returncode == 0) != success:
                raise AssertionError(result.stdout + result.stderr)
        prefix.mkdir()
        (prefix / 'user.txt').write_text('keep')
        run(success=False)
        (prefix / 'user.txt').unlink()
        if windows:
            payload = extracted / 'payload.zip'
            original_payload = payload.read_bytes()
            with zipfile.ZipFile(payload) as archive:
                entries = [(item, archive.read(item)) for item in archive.infolist()]
            with zipfile.ZipFile(payload, 'w') as archive:
                for item, data in entries:
                    archive.writestr(item, b'corrupted' if item.filename == 'd.exe' else data)
            run(success=False)
            assert not (prefix / 'd.exe').exists()
            payload.write_bytes(original_payload)
        run()
        exe = prefix / ('d.exe' if windows else 'd')
        result = subprocess.run([str(exe), '-e', 'print(42)'], capture_output=True, text=True, check=True)
        assert result.stdout.strip() == '42', result
        (prefix / 'user.txt').write_text('keep')
        run()
        assert (prefix / 'user.txt').read_text() == 'keep'
        assert (prefix / 'stdlib/modules/io.dev').is_file()
        assert (prefix / 'examples/hello.dev').is_file()
        run(['-Uninstall' if windows else '--uninstall'])
        assert not exe.exists()
        assert not (prefix / '.devlang-install.json').exists()
        assert (prefix / 'user.txt').read_text() == 'keep'
        if not windows:
            assert not os.path.lexists(root / 'bin/d')
        else:
            # Exercise the real user PATH branch, restoring the original value even on failure.
            integration = root / 'path test.ps1'
            integration.write_text(r"""param([string]$Setup, [string]$Prefix)
$ErrorActionPreference = 'Stop'
$original = [Environment]::GetEnvironmentVariable('Path', 'User')
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\DevLang'
$skipRegistration = Test-Path $key
try {
    & $Setup -Prefix $Prefix -NoRegistration:$skipRegistration
    if (-not $skipRegistration -and (Get-ItemProperty $key).InstallLocation -ne $Prefix) { throw 'Uninstall registration missing' }
    if (-not ([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' -contains $Prefix)) { throw 'PATH missing installation' }
    & $Setup -Prefix $Prefix -NoRegistration:$skipRegistration
    if (@([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' | Where-Object { $_ -eq $Prefix }).Count -ne 1) { throw 'Duplicate PATH entries' }
    & $Setup -Prefix $Prefix -Uninstall
    if ([Environment]::GetEnvironmentVariable('Path', 'User') -split ';' -contains $Prefix) { throw 'PATH entry not removed' }
    if (-not $skipRegistration -and (Test-Path $key)) { throw 'Uninstall registration not removed' }
} finally { [Environment]::SetEnvironmentVariable('Path', $original, 'User') }
""", encoding='utf-8')
            subprocess.run(['powershell.exe', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', str(integration), str(extracted / 'install.ps1'), str(root / 'path installation')], check=True)
        print('Passed: install, upgrade, execution, examples/stdlib, conflict rejection and uninstall preservation')


if __name__ == '__main__':
    main()
