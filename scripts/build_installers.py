"""Build offline Windows and Linux installers from verified component packages."""
import argparse
import base64
import hashlib
import io
from pathlib import Path
import re
import shutil
import tarfile
import tempfile
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'v\d+\.\d+\.\d+', args.version):
        parser.error('Expected vX.Y.Z')
    repo = Path(__file__).resolve().parents[1]
    output = repo / 'dist/releases' / args.version
    output.mkdir(parents=True, exist_ok=True)
    for platform in ('windows-x86_64', 'linux-x86_64'):
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
            shutil.copytree(repo / 'examples', stage / 'examples', ignore=shutil.ignore_patterns('.dev-cache', '*.exe', '*.dll', '*.so'))
            (stage / 'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  ' + p.relative_to(stage).as_posix() + '\n' for p in sorted(stage.rglob('*')) if p.is_file() and p != stage / 'SHA256SUMS'), encoding='utf-8')
            if platform.startswith('windows'):
                payload = Path(temporary) / 'payload.zip'
                with zipfile.ZipFile(payload, 'w', zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
                    for p in sorted(stage.rglob('*')):
                        if p.is_file():
                            archive.write(p, p.relative_to(stage))
                asset = output / f'devlang-setup-{args.version}-{platform}.zip'
                with zipfile.ZipFile(asset, 'w', zipfile.ZIP_DEFLATED) as archive:
                    archive.write(payload, 'payload.zip')
                    archive.write(repo / 'installers/windows/install.cmd', 'install.cmd')
                    archive.writestr('install.ps1', (repo / 'installers/windows/install.ps1').read_text().replace('@VERSION@', args.version))
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
