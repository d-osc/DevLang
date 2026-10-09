"""Archive verified component packages for a tagged GitHub release."""
import argparse
import hashlib
from pathlib import Path
import re
import tarfile
import zipfile


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--version', required=True)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'v\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?', args.version):
        parser.error('version must be a release tag such as v0.4.0')
    repo = Path(__file__).resolve().parents[1]
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    assets = []
    for platform in ('windows-x86_64', 'linux-x86_64'):
        for component in ('d', 'compiler', 'runtime', 'stdlib'):
            source = repo / 'dist' / component / platform
            manifest = source / 'SHA256SUMS'
            for line in manifest.read_text(encoding='utf-8').splitlines():
                digest, name = line.split('  ', 1)
                path = (source / name).resolve()
                if not path.is_relative_to(source.resolve()):
                    raise ValueError(f'unsafe package manifest path: {name}')
                if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
                    raise ValueError(f'package checksum mismatch: {path}')
            root = f'devlang-{component}-{args.version}-{platform}'
            if platform.startswith('windows'):
                asset = output / (root + '.zip')
                with zipfile.ZipFile(asset, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
                    for path in sorted(source.rglob('*')):
                        if path.is_file():
                            archive.write(path, root + '/' + path.relative_to(source).as_posix())
            else:
                asset = output / (root + '.tar.gz')
                def permissions(info):
                    info.uid = info.gid = 0
                    info.uname = info.gname = ''
                    # NTFS mode bits do not distinguish executable tools from docs.
                    info.mode = 0o755 if info.isdir() or Path(info.name).name in ('d', 'devc', 'devrun', 'tcc') else 0o644
                    return info
                with tarfile.open(asset, 'w:gz', compresslevel=9) as archive:
                    archive.add(source, arcname=root, filter=permissions)
            assets.append(asset)
            print(f'{asset.name}: {asset.stat().st_size:,} bytes')
    checksums = output / 'SHA256SUMS'
    checksums.write_text(''.join(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n' for path in assets), encoding='utf-8', newline='\n')
    print(checksums)


if __name__ == '__main__':
    main()
