"""Discover public DevLang GitHub packages and validate root manifests/tag versions."""
import argparse
import base64
from datetime import datetime, timezone, date, timedelta
import json
import hashlib
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
class BudgetExceeded(Exception): pass
class Api:
    def __init__(self, base='https://api.github.com', cache_dir=None, max_requests=900):
        parsed = urlparse(base)
        if base != 'https://api.github.com' and not (parsed.scheme == 'http' and parsed.hostname in ('127.0.0.1', 'localhost')):
            raise ValueError('API base must be GitHub or a localhost test server')
        self.cache_dir = Path(cache_dir) if cache_dir else None
        self.calls = self.cache_hits = 0
        self.max_requests = max_requests
        if self.cache_dir: self.cache_dir.mkdir(parents=True, exist_ok=True)
        self.base = base.rstrip('/')
        self.token = (os.environ.get('GITHUB_TOKEN') or os.environ.get('GH_TOKEN')) if base == 'https://api.github.com' else None
        self.opener = build_opener(NoRedirect())
    def get(self, path, **query):
        url = self.base + path + ('?' + urlencode(query) if query else '')
        cached = None
        cache_path = self.cache_dir / (hashlib.sha256(url.encode()).hexdigest() + '.json') if self.cache_dir else None
        if cache_path and cache_path.is_file() and cache_path.stat().st_size <= 12 * 1024 * 1024:
            try:
                saved = json.loads(cache_path.read_text(encoding='utf-8'))
                if saved.get('url') == url: cached = saved
            except (ValueError, OSError): pass
        immutable = bool(re.search(r'/git/trees/[0-9a-f]{40,64}(?:\?|$)', url) or re.search(r'(?:[?&])ref=[0-9a-f]{40,64}(?:&|$)', url))
        if cached and immutable:
            self.cache_hits += 1
            return cached['data']
        if self.calls >= self.max_requests: raise BudgetExceeded('GitHub API request budget reached')
        self.calls += 1
        headers = {'Accept': 'application/vnd.github+json', 'User-Agent': 'DevLang-Package-Store', 'X-GitHub-Api-Version': '2026-03-10'}
        if self.token: headers['Authorization'] = 'Bearer ' + self.token
        if cached and isinstance(cached.get('etag'), str) and not any(c in cached['etag'] for c in '\r\n'):
            headers['If-None-Match'] = cached['etag']
        try:
            with self.opener.open(Request(url, headers=headers), timeout=30) as response:
                data = response.read(8 * 1024 * 1024 + 1)
                etag = response.headers.get('ETag')
        except HTTPError as error:
            if error.code == 304 and cached:
                self.cache_hits += 1
                return cached['data']
            raise
        if len(data) > 8 * 1024 * 1024: raise ValueError('GitHub response exceeds 8 MiB')
        result = json.loads(data)
        if cache_path:
            temporary = cache_path.with_suffix('.tmp')
            temporary.write_text(json.dumps({'url': url, 'etag': etag, 'data': result}), encoding='utf-8')
            temporary.replace(cache_path)
        return result

def inspect(api, full_name, ref, d, subdir=None, workspace_root=False):
    entries = api.get('/repos/' + full_name + '/git/trees/' + quote(ref, safe=''), recursive='1')
    if entries.get('truncated'): raise ValueError('source tree is truncated; cannot validate package')
    files = {x['path']: x for x in entries.get('tree', [])}
    prefix = subdir.rstrip('/') + '/' if subdir else ''
    if subdir and (subdir.startswith('/') or '\\' in subdir or any(p in ('', '.', '..') for p in subdir.split('/'))): raise ValueError('invalid package subdirectory')
    root = files.get(prefix + 'package.don')
    if not root or root.get('type') != 'blob' or root.get('mode') not in ('100644', '100755'):
        raise ValueError('package.don must be a regular file at repository level 0')
    data = api.get('/repos/' + full_name + '/contents/' + quote(prefix + 'package.don', safe='/'), ref=ref)
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
        compatibility = subprocess.run([str(d), '-C', temporary, 'pkg', 'compatibility', '--json'], text=True, encoding='utf-8', capture_output=True, timeout=15)
        if compatibility.returncode: raise ValueError(compatibility.stderr.strip())
        manifest['_compatibility'] = json.loads(compatibility.stdout)
    version = manifest.get('version', '').removeprefix('v')
    if workspace_root and manifest.get('workspace'): return manifest, version
    if not VERSION.fullmatch(version): raise ValueError('package needs a SemVer version')
    modules = manifest['package'].get('modules', 'src').rstrip('/')
    if '\\' in modules: raise ValueError('module directory must use forward slashes')
    source_prefix = prefix + modules + '/'
    if not any(path.startswith(source_prefix) and path.endswith('.dev') and item.get('type') == 'blob' and item.get('mode') in ('100644', '100755') for path, item in files.items()):
        raise ValueError('module directory must contain a regular .dev source file')
    for entry in manifest.get('bin', {}).values():
        if files.get(prefix + entry, {}).get('mode') not in ('100644', '100755'): raise ValueError('package bin entry is missing')
    return manifest, version

def quality(api, name, commit):
    try:
        data = api.get('/repos/' + name + '/commits/' + commit + '/check-runs', per_page=100)
        runs = data.get('check_runs', [])
        if not runs: return {'state': 'unknown', 'checks': 0, 'commit': commit}
        # Latest check per workflow/job name, so reruns do not retain obsolete failures.
        latest = {}
        for run in sorted(runs, key=lambda r:r.get('id', 0)): latest[run['name']] = run
        state = 'pending' if any(r['status'] != 'completed' for r in latest.values()) else 'passed' if all(r.get('conclusion') in ('success', 'neutral', 'skipped') for r in latest.values()) else 'failed'
        if state == 'passed' and not any(r.get('conclusion') == 'success' for r in latest.values()): state = 'skipped'
        if data.get('total_count', 0) > len(runs): state = 'partial'
        return {'state': state, 'checks': len(latest), 'commit': commit}
    except HTTPError as error:
        if error.code not in (403, 404, 422): raise
        return {'state': 'unavailable', 'checks': 0, 'commit': commit}

def package_documents(api, name, commit, subdir=None):
    tree = api.get('/repos/' + name + '/git/trees/' + commit, recursive='1')
    prefix = subdir.rstrip('/') + '/' if subdir else ''
    candidates = [dict(x, display=x['path'][len(prefix):]) for x in tree.get('tree', []) if x['path'].startswith(prefix) if x.get('type') == 'blob' and x.get('mode') in ('100644', '100755') and (x['path'][len(prefix):].lower() in ('readme.md', 'changelog.md', 'api.md') or x['path'][len(prefix):].startswith('docs/') and x['path'].endswith('.md'))]
    candidates.sort(key=lambda x: (x['path'].lower() not in ('readme.md','changelog.md','api.md'), x['path']))
    documents = []
    for item in candidates[:10]:
        if item.get('size', 0) > 65536: continue
        data = api.get('/repos/' + name + '/contents/' + quote(item['path'], safe='/'), ref=commit)
        if data.get('encoding') != 'base64' or data.get('size', 65537) > 65536: continue
        body = base64.b64decode(''.join(data['content'].split()), validate=True)
        try: text = body.decode('utf-8')
        except UnicodeDecodeError: continue
        if len(body) <= 65536: documents.append({'path': item['path'], 'markdown': text})
    return documents

def version_key(version):
    major, minor, patch, pre = VERSION.fullmatch(version).groups()
    prerelease = tuple((0, int(x)) if x.isdigit() else (1, x) for x in (pre or '').split('.'))
    return int(major), int(minor), int(patch), pre is None, prerelease

def discover(api, explicit, limit):
    names=list(explicit); total=0; incomplete=False
    query='topic:devlang-package fork:false archived:false is:public'
    ranges=[(None,None)]
    while ranges and len(dict.fromkeys(names)) < limit:
        low,high=ranges.pop()
        q=query if low is None else query+f' created:{low.isoformat()}..{high.isoformat()}'
        response=api.get('/search/repositories',q=q,sort='updated',per_page=min(100,limit),page=1)
        count=response.get('total_count',0)
        if low is None: total=count
        incomplete=incomplete or response.get('incomplete_results',False)
        if count>1000 and limit>1000:
            if low is None: low,high=date(2008,1,1),datetime.now(timezone.utc).date()
            if low<high:
                mid=low+(high-low)//2;ranges.extend([(low,mid),(mid+timedelta(days=1),high)]);continue
            incomplete=True # More than 1,000 repos created on one day: explicitly report partial coverage.
        items=response.get('items',[]);names.extend(x['full_name'] for x in items)
        for page in range(2,min(10,(count+99)//100)+1):
            if len(items)<min(100,limit) or len(dict.fromkeys(names))>=limit: break
            response=api.get('/search/repositories',q=q,sort='updated',per_page=min(100,limit),page=page)
            incomplete=incomplete or response.get('incomplete_results',False)
            items=response.get('items',[]);names.extend(x['full_name'] for x in items)
    return list({name.lower():name for name in names}.values())[:limit],total,incomplete or bool(ranges)

def collect(api, d, explicit=(), limit=100, max_tags=400, max_version_docs=10):
    names,total,incomplete=discover(api,explicit,limit)
    packages,rejected,deferred=[],[],[]
    for position,name in enumerate(names):
        if not REPOSITORY.fullmatch(name): raise ValueError('invalid repository identity')
        try:
            metadata=api.get('/repos/'+name)
            if metadata.get('private') or metadata.get('fork') or metadata.get('archived'): raise ValueError('repository must be public, active and not a fork')
            if 'devlang-package' not in metadata.get('topics',[]): raise ValueError('repository needs the devlang-package topic')
            full_name=metadata['full_name']
            if not REPOSITORY.fullmatch(full_name): raise ValueError('invalid repository identity')
            branch=api.get('/repos/'+full_name+'/branches/'+quote(metadata['default_branch'],safe=''))
            commit=branch['commit']['sha']
            root,current=inspect(api,full_name,commit,d,workspace_root=True)
            descriptors=[]
            try: descriptors.append((None,*inspect(api,full_name,commit,d)))
            except ValueError:
                if not root.get('workspace'): raise
            members=root.get('workspace',{}).get('members',[])
            if len(members)>128: raise ValueError('workspace exceeds 128 members')
            paths=[];namespaces=set()
            for member in members:
                if not isinstance(member,str) or member.startswith('/') or '\\' in member or any(x in ('','.','..') for x in member.split('/')): raise ValueError('unsafe workspace member')
                if any(member==p or member.startswith(p+'/') or p.startswith(member+'/') for p in paths): raise ValueError('overlapping workspace members')
                paths.append(member)
                child,version=inspect(api,full_name,commit,d,member)
                if child.get('workspace') or child['package']['name'] in namespaces or child['package']['name']==root['package']['name']: raise ValueError('workspace names must be unique and non-nested')
                namespaces.add(child['package']['name']);descriptors.append((member,child,version))
            tags=[]
            for page in range(1,(max_tags+99)//100+1):
                batch=api.get('/repos/'+full_name+'/tags',per_page=100,page=page);tags.extend(batch[:max_tags-len(tags)])
                if len(batch)<100: break
            ci=quality(api,full_name,commit)
            for subdir,manifest,current_version in descriptors:
                if len(packages)>=20000: raise BudgetExceeded('Store package budget reached (20,000)')
                namespace=manifest['package']['name'];identity=full_name+('/'+namespace if subdir else '')
                versions,ignored=[],[]
                for tag in tags:
                    label=tag['name']
                    if subdir:
                        prefix=namespace+'/'
                        if not label.startswith(prefix): continue
                        label=label[len(prefix):]
                    version=label.removeprefix('v')
                    if not VERSION.fullmatch(version): continue
                    try:
                        tagged,checked=inspect(api,full_name,tag['commit']['sha'],d,subdir)
                        if checked!=version or tagged['package']['name']!=namespace: raise ValueError('tag and package version/name differ')
                        versions.append({'version':checked,'tag':tag['name'],'commit':tag['commit']['sha']})
                    except (ValueError,HTTPError) as error:
                        if isinstance(error,HTTPError) and error.code not in (404,422): raise
                        ignored.append({'tag':tag['name'],'reason':str(error)[:300]})
                versions.sort(key=lambda x:version_key(x['version']),reverse=True)
                stable=[v for v in versions if '-' not in v['version'].split('+')[0]]
                latest=(stable or versions or [None])[0]
                documents=package_documents(api,full_name,commit,subdir)
                for v in versions[:max_version_docs]:
                    try: v['documents']=package_documents(api,full_name,v['commit'],subdir)
                    except BudgetExceeded: v['documents_deferred']=True
                license=metadata.get('license') or {}
                package={'id':identity,'name':namespace,'owner':full_name.split('/')[0],'repository':'https://github.com/'+full_name,'subdir':subdir,
                    'description':metadata.get('description') or '', 'topics':metadata.get('topics',[]),'stars':metadata.get('stargazers_count',0),
                    'license':license.get('spdx_id') or '', 'updated_at':metadata.get('pushed_at'),'checked_commit':commit,'source_version':current_version,
                    'modules':manifest['package']['modules'],'bin':manifest.get('bin',{}),'dependencies':list(manifest.get('dependencies',{})),'peers':manifest.get('peerDependencies',{}),
                    'versions':versions,'latest':latest,'tag_limited':len(tags)>=max_tags,'ignored_tags':ignored,
                    'devlang':manifest.get('devlang'),'compatibility':manifest['_compatibility'],'quality':ci,'documents':documents}
                packages.append(package)
        except BudgetExceeded as error:
            deferred.extend({'repository':pending,'reason':str(error)} for pending in names[position:]);break
        except (ValueError,HTTPError) as error:
            if isinstance(error,HTTPError) and error.code not in (404,422): raise
            rejected.append({'repository':name,'reason':str(error)[:300]})
    return {'schema_version':1,'generated_at':datetime.now(timezone.utc).isoformat(),'discovery_total':total,'limited':incomplete or total>limit or bool(deferred),
        'packages':packages,'rejected':rejected,'deferred':deferred,'requests':api.calls,'cache_hits':api.cache_hits,
        'policy':{'topic':'devlang-package','manifest':'package.don','manifest_level':0,'max_tags_per_repo':max_tags,'max_version_documents':max_version_docs,'search_sharding':'created-date','max_repos':limit}}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--d', required=True, type=Path)
    parser.add_argument('--output', default='website/content/packages.json', type=Path)
    parser.add_argument('--repo', action='append', default=[])
    parser.add_argument('--max-repos', type=int, default=10000)
    parser.add_argument('--max-tags',type=int,default=400)
    parser.add_argument('--max-version-docs',type=int,default=10)
    parser.add_argument('--cache-dir', type=Path)
    parser.add_argument('--max-requests', type=int, default=900)
    parser.add_argument('--api-base', default='https://api.github.com')
    args = parser.parse_args()
    if not 1 <= args.max_requests <= 5000: parser.error('--max-requests must be 1..5000')
    if not 1 <= args.max_repos <= 10000: parser.error('--max-repos must be 1..10000')
    if not 1 <= args.max_tags <= 10000: parser.error('--max-tags must be 1..10000')
    if not 0 <= args.max_version_docs <= 400: parser.error('--max-version-docs must be 0..400')
    catalog = collect(Api(args.api_base, args.cache_dir, args.max_requests), args.d.resolve(), args.repo, args.max_repos,args.max_tags,args.max_version_docs)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.output.with_suffix('.json.tmp')
    details = args.output.parent / 'package-details'
    details.mkdir(exist_ok=True)
    for package in catalog['packages']:
        for version in package['versions']:
            if 'documents' not in version: continue
            docs=version.pop('documents');version_file=hashlib.sha256((package['id']+'@'+version['commit']).encode()).hexdigest()+'.json'
            version['details']='package-details/'+version_file
            (details/version_file).write_text(json.dumps({'id':package['id'],'commit':version['commit'],'documents':docs},ensure_ascii=False),encoding='utf-8')
        documents = package.pop('documents', [])
        filename = hashlib.sha256(package['id'].encode()).hexdigest() + '.json'
        package['details'] = 'package-details/' + filename
        (details / filename).write_text(json.dumps({'id': package['id'], 'commit': package['checked_commit'], 'documents': documents}, ensure_ascii=False), encoding='utf-8')
    temporary.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    temporary.replace(args.output)
    print(f"Package Store: {len(catalog['packages'])} validated repositories, {len(catalog['rejected'])} rejected")
if __name__ == '__main__': main()
