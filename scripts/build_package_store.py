"""Discover public DevLang GitHub packages and validate root manifests/tag versions."""
import argparse
import base64
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
from urllib.error import HTTPError
from urllib.parse import quote, urlencode, urlparse
from urllib.request import Request, build_opener, HTTPRedirectHandler

REPOSITORY = re.compile(r'^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$')
VERSION = re.compile(r'^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$')
class NoRedirect(HTTPRedirectHandler):
    def redirect_request(self, *args, **kwargs): return None
class Api:
    def __init__(self, base='https://api.github.com'):
        parsed = urlparse(base)
        if base != 'https://api.github.com' and not (parsed.scheme == 'http' and parsed.hostname in ('127.0.0.1', 'localhost')):
            raise ValueError('API base must be GitHub or a localhost test server')
        self.base = base.rstrip('/')
        self.token = (os.environ.get('GITHUB_TOKEN') or os.environ.get('GH_TOKEN')) if base == 'https://api.github.com' else None
        self.opener = build_opener(NoRedirect())
    def get(self, path, **query):
        url = self.base + path + ('?' + urlencode(query) if query else '')
        headers = {'Accept': 'application/vnd.github+json', 'User-Agent': 'DevLang-Package-Store', 'X-GitHub-Api-Version': '2026-03-10'}
        if self.token: headers['Authorization'] = 'Bearer ' + self.token
        with self.opener.open(Request(url, headers=headers), timeout=30) as response:
            data = response.read(8 * 1024 * 1024 + 1)
        if len(data) > 8 * 1024 * 1024: raise ValueError('GitHub response exceeds 8 MiB')
        return json.loads(data)

def inspect(api, full_name, ref, d):
    entries = api.get('/repos/' + full_name + '/git/trees/' + quote(ref, safe=''), recursive='1')
    if entries.get('truncated'): raise ValueError('source tree is truncated; cannot validate package')
    files = {x['path']: x for x in entries.get('tree', [])}
    root = files.get('package.don')
    if not root or root.get('type') != 'blob' or root.get('mode') not in ('100644', '100755'):
        raise ValueError('package.don must be a regular file at repository level 0')
    data = api.get('/repos/' + full_name + '/contents/package.don', ref=ref)
    if data.get('type') != 'file' or data.get('encoding') != 'base64' or data.get('size', 65537) > 65536:
        raise ValueError('manifest must be a UTF-8 file no larger than 64 KiB')
    contents = base64.b64decode(''.join(data['content'].split()), validate=True)
    if len(contents) > 65536: raise ValueError('manifest exceeds 64 KiB')
    contents.decode('utf-8')
    with tempfile.TemporaryDirectory(prefix='dev-store-manifest-') as temporary:
        Path(temporary, 'package.don').write_bytes(contents)
        result = subprocess.run([str(d), '-C', temporary, 'pkg', 'inspect', '--json'], text=True, encoding='utf-8', capture_output=True, timeout=15)
        if result.returncode: raise ValueError(result.stderr.strip())
        manifest = json.loads(result.stdout)
    version = manifest.get('version', '').removeprefix('v')
    if not VERSION.fullmatch(version): raise ValueError('package needs a SemVer version')
    modules = manifest['package'].get('modules', 'src').rstrip('/')
    if '\\' in modules: raise ValueError('module directory must use forward slashes')
    source_prefix = modules + '/'
    if not any(path.startswith(source_prefix) and path.endswith('.dev') and item.get('type') == 'blob' and item.get('mode') in ('100644', '100755') for path, item in files.items()):
        raise ValueError('module directory must contain a regular .dev source file')
    for entry in manifest.get('bin', {}).values():
        if files.get(entry, {}).get('mode') not in ('100644', '100755'): raise ValueError('package bin entry is missing')
    return manifest, version

def version_key(version):
    major, minor, patch, pre = VERSION.fullmatch(version).groups()
    prerelease = tuple((0, int(x)) if x.isdigit() else (1, x) for x in (pre or '').split('.'))
    return int(major), int(minor), int(patch), pre is None, prerelease

def collect(api, d, explicit=(), limit=100):
    names = list(explicit)
    total = 0
    incomplete = False
    for page in range(1, (limit + 99) // 100 + 1):
        response = api.get('/search/repositories', q='topic:devlang-package fork:false archived:false is:public', sort='updated', per_page=min(100, limit), page=page)
        total = response.get('total_count', 0)
        incomplete = incomplete or response.get('incomplete_results', False)
        items = response.get('items', [])
        names.extend(x['full_name'] for x in items)
        if len(items) < min(100, limit) or page * 100 >= total: break
    packages, rejected = [], []
    for name in dict.fromkeys(names[:limit]):
        if not REPOSITORY.fullmatch(name): raise ValueError('invalid repository identity')
        try:
            metadata = api.get('/repos/' + name)
            if metadata.get('private') or metadata.get('fork') or metadata.get('archived'): raise ValueError('repository must be public, active and not a fork')
            if 'devlang-package' not in metadata.get('topics', []): raise ValueError('repository needs the devlang-package topic')
            full_name = metadata['full_name']
            if not REPOSITORY.fullmatch(full_name): raise ValueError('invalid repository identity')
            branch = api.get('/repos/' + full_name + '/branches/' + quote(metadata['default_branch'], safe=''))
            commit = branch['commit']['sha']
            manifest, current_version = inspect(api, full_name, commit, d)
            tags = api.get('/repos/' + full_name + '/tags', per_page=30)
            versions, ignored_tags = [], []
            for tag in tags:
                version = tag['name'].removeprefix('v')
                if not VERSION.fullmatch(version): continue
                try:
                    tagged, checked = inspect(api, full_name, tag['commit']['sha'], d)
                    if checked != version or tagged['package']['name'] != manifest['package']['name']: raise ValueError('tag and package version/name differ')
                    versions.append({'version': checked, 'tag': tag['name'], 'commit': tag['commit']['sha']})
                except (ValueError, HTTPError) as error:
                    if isinstance(error, HTTPError) and error.code not in (404, 422): raise
                    ignored_tags.append({'tag': tag['name'], 'reason': str(error)[:300]})
            versions.sort(key=lambda x: version_key(x['version']), reverse=True)
            stable = [v for v in versions if '-' not in v['version'].split('+')[0]]
            latest = (stable or versions or [None])[0]
            license = metadata.get('license') or {}
            packages.append({'id': full_name, 'name': manifest['package']['name'], 'owner': full_name.split('/')[0], 'repository': 'https://github.com/' + full_name,
                'description': metadata.get('description') or '', 'topics': metadata.get('topics', []), 'stars': metadata.get('stargazers_count', 0),
                'license': license.get('spdx_id') or '', 'updated_at': metadata.get('pushed_at'), 'checked_commit': commit, 'source_version': current_version,
                'modules': manifest['package']['modules'], 'bin': manifest.get('bin', {}), 'dependencies': list(manifest.get('dependencies', {})), 'peers': manifest.get('peerDependencies', {}),
                'versions': versions, 'latest': latest, 'ignored_tags': ignored_tags})
        except (ValueError, HTTPError) as error:
            if isinstance(error, HTTPError) and error.code not in (404, 422): raise
            rejected.append({'repository': name, 'reason': str(error)[:300]})
    return {'schema_version': 1, 'generated_at': datetime.now(timezone.utc).isoformat(), 'discovery_total': total, 'limited': incomplete or total > limit,
            'packages': packages, 'rejected': rejected, 'policy': {'topic': 'devlang-package', 'manifest': 'package.don', 'manifest_level': 0, 'max_tags_per_repo': 30}}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--d', required=True, type=Path)
    parser.add_argument('--output', default='website/content/packages.json', type=Path)
    parser.add_argument('--repo', action='append', default=[])
    parser.add_argument('--max-repos', type=int, default=100)
    parser.add_argument('--api-base', default='https://api.github.com')
    args = parser.parse_args()
    if not 1 <= args.max_repos <= 1000: parser.error('--max-repos must be 1..1000')
    catalog = collect(Api(args.api_base), args.d.resolve(), args.repo, args.max_repos)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix('.json.tmp')
    temporary.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    temporary.replace(args.output)
    print(f"Package Store: {len(catalog['packages'])} validated repositories, {len(catalog['rejected'])} rejected")
if __name__ == '__main__': main()
