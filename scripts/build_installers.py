"""Build offline Windows and Linux installers from verified component packages."""
import argparse
import base64
import hashlib
import io
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', required=True)
    parser.add_argument('--windows-exe', action='store_true', help='Build only the single-file Windows GUI installer (requires Windows .NET Framework csc)')
    args = parser.parse_args()
    if not re.fullmatch(r'v\d+\.\d+\.\d+', args.version):
        parser.error('Expected vX.Y.Z')
    repo = Path(__file__).resolve().parents[1]
    output = repo / 'dist/releases' / args.version
    output.mkdir(parents=True, exist_ok=True)
    platforms = ('windows-x86_64',) if args.windows_exe else ('windows-x86_64', 'linux-x86_64')
    for platform in platforms:
        with tempfile.TemporaryDirectory(prefix='devlang-package-') as temporary:
            stage = Path(temporary) / 'payload'
            for component, destination in (('d', stage), ('stdlib', stage / 'stdlib')):
                source = repo / 'dist' / component / platform
                for line in (source / 'SHA256SUMS').read_text().splitlines():
                    digest, name = line.split('  ', 1)
                    file = source / name
                    if not file.resolve().is_relative_to(source.resolve()) or hashlib.sha256(file.read_bytes()).hexdigest() != digest:
                        raise ValueError(f'Invalid package checksum: {file}')
                shutil.copytree(source, destination)
            shutil.copytree(repo / 'examples', stage / 'examples', ignore=shutil.ignore_patterns('.dev', '.dev-cache', 'dev.lock', 'package-lock.don', 'node_modules', '__pycache__', 'target', 'out', '*.exe', '*.dll', '*.so'))
            (stage / 'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.relative_to(stage).as_posix() + '\n' for p in sorted(stage.rglob('*')) if p.is_file() and p != stage / 'SHA256SUMS'), encoding='utf-8')
            if platform.startswith('windows'):
                payload = Path(temporary) / 'payload.zip'
                with zipfile.ZipFile(payload, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
                    for p in sorted(stage.rglob('*')):
                        if p.is_file():
                            archive.write(p, p.relative_to(stage))
                install_script = (repo / 'installers/windows/install.ps1').read_text().replace('@VERSION@', args.version)
                if args.windows_exe:
                    csc = Path(os.environ.get('WINDIR', 'C:/Windows')) / 'Microsoft.NET/Framework64/v4.0.30319/csc.exe'
                    if not csc.is_file():
                        raise RuntimeError('Windows x64 .NET Framework C# compiler is required to build the EXE')
                    source = Path(temporary) / 'Setup.cs'
                    script = Path(temporary) / 'install.ps1'
                    source.write_text((repo / 'installers/windows/Setup.cs').read_text().replace('@VERSION@', args.version).replace('@ASSEMBLY_VERSION@', args.version[1:] + '.0').replace('@PAYLOAD_HASH@', hashlib.sha256(payload.read_bytes()).hexdigest()), encoding='utf-8')
                    script.write_text(install_script, encoding='utf-8')
                    asset = output / f'devlang-setup-{args.version}-{platform}.exe'
                    subprocess.run([str(csc), '/nologo', '/target:winexe', '/platform:x64', '/optimize+', '/reference:System.Windows.Forms.dll', '/reference:System.Drawing.dll', '/out:' + str(asset), '/win32manifest:' + str(repo / 'installers/windows/setup.manifest'), '/resource:' + str(payload) + ',DevLang.Payload', '/resource:' + str(script) + ',DevLang.InstallScript', str(source)], check=True)
                else:
                    asset = output / f'devlang-setup-{args.version}-{platform}.zip'
                    with zipfile.ZipFile(asset, 'w', zipfile.ZIP_DEFLATED) as archive:
                        archive.write(payload, 'payload.zip')
                        archive.write(repo / 'installers/windows/install.cmd', 'install.cmd')
                        archive.writestr('install.ps1', install_script)
                        archive.write(repo / 'docs/install.md', 'README.md')
            else:
                payload = io.BytesIO()
                def permissions(info):
                    info.uid = info.gid = 0
                    info.uname = info.gname = ''
                    info.mode = 0o755 if info.isdir() or Path(info.name).name in ('d', 'devc', 'devrun', 'tcc') else 0o644
                    return info
                with tarfile.open(fileobj=payload, mode='w:gz') as archive:
                    for p in sorted(stage.iterdir()):
                        archive.add(p, arcname=p.name, filter=permissions)
                asset = output / f'devlang-setup-{args.version}-{platform}.py'
                asset.write_text((repo / 'installers/linux/install.py').read_text().replace('@VERSION@', args.version).replace('@PAYLOAD@', base64.b85encode(payload.getvalue()).decode()), encoding='utf-8', newline='\n')
            print(asset)
    assets = sorted(p for p in output.iterdir() if p.is_file() and p.name != 'SHA256SUMS')
    (output / 'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.name + '\n' for p in assets), encoding='utf-8', newline='\n')


if __name__ == '__main__':
    main()
