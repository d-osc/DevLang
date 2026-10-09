#!/usr/bin/env python3
"""Offline per-user DevLang installer. Requires Python 3.9 or newer."""
import argparse
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import tarfile
import tempfile

VERSION = '@VERSION@'
PAYLOAD = '@PAYLOAD@'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prefix', type=Path, default=Path.home() / '.local/share/devlang')
    parser.add_argument('--bin-dir', type=Path, default=Path.home() / '.local/bin')
    parser.add_argument('--uninstall', action='store_true')
    args = parser.parse_args()
    prefix = args.prefix.expanduser().absolute()
    bin_dir = args.bin_dir.expanduser().absolute()
    marker = prefix / '.devlang-install.json'
    previous = json.loads(marker.read_text()) if marker.is_file() else None
    if previous and (previous.get('product') != 'DevLang' or previous.get('prefix') != str(prefix)):
        raise ValueError('Invalid installation marker')

    def owned_file(name):
        path = prefix / name
        if not path.resolve().is_relative_to(prefix.resolve()) or path.is_symlink():
            raise ValueError('Unsafe manifest path: ' + name)
        return path

    def remove_files():
        for name in previous['files']:
            owned_file(name).unlink(missing_ok=True)

    if args.uninstall:
        if not previous:
            raise ValueError('No managed DevLang installation here')
        for name in previous['links']:
            link = Path(previous['bin_dir']) / name
            if link.is_symlink() and os.readlink(link) == str(prefix / name):
                link.unlink()
        remove_files()
        marker.unlink()
        print('DevLang removed; other files were preserved.')
        return
    if prefix.exists() and not previous and any(prefix.iterdir()):
        raise ValueError('Choose an empty directory or a managed DevLang installation')
    if previous and previous['bin_dir'] != str(bin_dir):
        raise ValueError('Use the original --bin-dir when upgrading')
    for name in ('d', 'devc', 'devrun'):
        link = bin_dir / name
        if os.path.lexists(link) and not (link.is_symlink() and os.readlink(link) == str(prefix / name) and previous and name in previous['links']):
            raise ValueError('Will not overwrite existing command: ' + str(link))
    with tempfile.TemporaryDirectory(prefix='devlang-install-') as temporary:
        stage = Path(temporary)
        with tarfile.open(fileobj=io.BytesIO(base64.b85decode(PAYLOAD)), mode='r:gz') as archive:
            for member in archive.getmembers():
                if not (stage / member.name).resolve().is_relative_to(stage) or not (member.isfile() or member.isdir()):
                    raise ValueError('Unsafe archive entry')
            archive.extractall(stage)
        for line in (stage / 'SHA256SUMS').read_text().splitlines():
            digest, name = line.split('  ', 1)
            file = stage / name
            if not file.resolve().is_relative_to(stage) or hashlib.sha256(file.read_bytes()).hexdigest() != digest:
                raise ValueError('Checksum mismatch: ' + name)
        files = [str(p.relative_to(stage)) for p in stage.rglob('*') if p.is_file()]
        for name in files:
            path = owned_file(name)
            if path.exists() and (not previous or name not in previous['files']):
                raise ValueError('Unowned destination file: ' + name)
        prefix.mkdir(parents=True, exist_ok=True)
        if previous:
            remove_files()
        shutil.copytree(stage, prefix, dirs_exist_ok=True)
        bin_dir.mkdir(parents=True, exist_ok=True)
        links = previous['links'] if previous else []
        for name in ('d', 'devc', 'devrun'):
            if not os.path.lexists(bin_dir / name):
                (bin_dir / name).symlink_to(prefix / name)
                links.append(name)
        marker.write_text(json.dumps(dict(product='DevLang', version=VERSION, prefix=str(prefix), bin_dir=str(bin_dir), files=files, links=links)))
    print(f'Installed DevLang {VERSION} in {prefix}')
    print(f'Add {bin_dir} to PATH if needed, then run: d --help')
    print('Uninstall with this installer and the same --prefix/--bin-dir, plus --uninstall.')


if __name__ == '__main__':
    main()
