"""Exercise Store discovery through a localhost GitHub API fixture and the real d parser."""
import argparse
import base64
from copy import deepcopy
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import subprocess
import tempfile
from threading import Thread
from urllib.error import HTTPError
from urllib.parse import urlparse
from build_package_store import Api, collect, inspect, version_key, quality, package_documents

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--d', required=True)
d = Path(parser.parse_args().d).resolve()
repo = Path(__file__).resolve().parents[1]
checks = 0
manifest = {'version':'1.0.0','package':{'name':'math','modules':'src'}}
snapshots = {'head':deepcopy(manifest),'stable':deepcopy(manifest),'preview':dict(manifest,version='2.0.0-beta.1'),'old':dict(manifest,version='0.9.0'),'badtag':deepcopy(manifest)}
metadata = {'full_name':'owner/math','default_branch':'main','topics':['devlang-package'],'private':False,'fork':False,'archived':False,'description':'<script>not executed</script>','license':{'spdx_id':'MIT'}}
files = [{'path':'package.don','type':'blob','mode':'100644'},{'path':'src/lib.dev','type':'blob','mode':'100644'}]
response_overrides = {}
class Handler(BaseHTTPRequestHandler):
    def log_message(self,*args): pass
    def do_GET(self):
        path = urlparse(self.path).path
        status = 200
        if path in response_overrides:
            status, value = response_overrides[path]
        elif path == '/search/repositories': value = {'total_count':1,'items':[{'full_name':'owner/math'}]}
        elif path == '/repos/owner/math': value = metadata
        elif path == '/repos/owner/math/branches/main': value = {'commit':{'sha':'head'}}
        elif path.startswith('/repos/owner/math/git/trees/'): value = {'tree':files,'truncated':False}
        elif path == '/repos/owner/math/contents/package.don':
            from urllib.parse import parse_qs
            ref = parse_qs(urlparse(self.path).query)['ref'][0]
            contents = json.dumps(snapshots[ref]).encode()
            value = {'type':'file','encoding':'base64','size':len(contents),'content':base64.b64encode(contents).decode()}
        elif path == '/repos/owner/math/tags': value = [{'name':v,'commit':{'sha':k}} for k,v in [('preview','v2.0.0-beta.1'),('badtag','v3.0.0'),('stable','v1.0.0'),('old','0.9.0')]]
        else: status,value = 404,{'message':'Not found'}
        data = json.dumps(value).encode()
        self.send_response(status);self.send_header('Content-Type','application/json');self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
server = ThreadingHTTPServer(('127.0.0.1',0),Handler)
Thread(target=server.serve_forever,daemon=True).start()
api = Api(f'http://127.0.0.1:{server.server_port}')
def check(condition, message):
    global checks
    assert condition,message
    checks += 1
def reject(reason):
    result = collect(api,d)
    check(not result['packages'] and reason in result['rejected'][0]['reason'],result)
try:
    result = collect(api,d)
    p = result['packages'][0]
    check(p['id']=='owner/math' and p['latest']['tag']=='v1.0.0',p)
    check([v['version'] for v in p['versions']]==['2.0.0-beta.1','1.0.0','0.9.0'],p)
    check(p['ignored_tags'][0]['tag']=='v3.0.0',p)
    check(version_key('1.0.0-beta.2') < version_key('1.0.0-beta.10') < version_key('1.0.0'), 'SemVer order')
    for prop in ('private','fork','archived'):
        metadata[prop]=True;reject('public, active');metadata[prop]=False
    metadata['topics']=[];reject('topic');metadata['topics']=['devlang-package']
    files[0]['path']='nested/package.don';reject('level 0');files[0]['path']='package.don'
    files[0]['mode']='120000';reject('regular file');files[0]['mode']='100644'
    files[1]['mode']='120000';reject('.dev source');files[1]['mode']='100644'
    snapshots['head']['unsupported']='no';reject('unknown field');del snapshots['head']['unsupported']
    snapshots['head']['version']='garbage';reject('invalid package version');snapshots['head']['version']='1.0.0'
    snapshots['head']['bin']={'mycli':'src/missing.dev'};reject('bin entry');del snapshots['head']['bin']
    snapshots['head']['dependencies']={'broken':{'git':'https://github.com/owner/other.git','tag':'v1','branch':'main'}};reject('cannot be combined');del snapshots['head']['dependencies']
    snapshots['head']['dependencies']={'bad-name':{'path':'../unused'}};reject('invalid dependency namespace');del snapshots['head']['dependencies']
    response_overrides['/repos/owner/math/tags']=(200,[])
    check(collect(api,d)['packages'][0]['latest'] is None,'source-only repository')
    del response_overrides['/repos/owner/math/tags']
    response_overrides['/repos/owner/math/git/trees/head']=(200,{'tree':files,'truncated':True});reject('truncated');del response_overrides['/repos/owner/math/git/trees/head']
    response_overrides['/repos/owner/math/contents/package.don']=(200,{'type':'file','encoding':'base64','size':65537});reject('64 KiB');del response_overrides['/repos/owner/math/contents/package.don']
    response_overrides['/search/repositories']=(403,{'message':'API rate limit exceeded'})
    with tempfile.TemporaryDirectory(prefix='store output ') as temporary:
        output=Path(temporary)/'packages.json';output.write_text('preserved',encoding='utf-8')
        run = subprocess.run(['python',str(repo/'scripts/build_package_store.py'),'--d',str(d),'--api-base',api.base,'--output',str(output)],capture_output=True,text=True,encoding='utf-8',timeout=30)
        check(run.returncode != 0 and output.read_text()=='preserved','API failure must preserve catalog')
    del response_overrides['/search/repositories']
    with tempfile.TemporaryDirectory(prefix='store cache ') as cache:
        caching = Api(api.base, Path(cache))
        immutable_path='/repos/owner/math/git/trees/'+'a'*40
        first=caching.get(immutable_path, recursive='1')
        second=caching.get(immutable_path, recursive='1')
        check(first==second and caching.calls==1 and caching.cache_hits==1,'immutable snapshot cache')
        limited=collect(Api(api.base,max_requests=2),d)
        check(limited['limited'] and limited['deferred'][0]['repository']=='owner/math','request budget reports deferred packages')
    checks_path='/repos/owner/math/commits/head/check-runs'
    for conclusion, expected in [('success','passed'),('failure','failed'),('skipped','skipped')]:
        response_overrides[checks_path]=(200,{'total_count':1,'check_runs':[{'id':1,'name':'test','status':'completed','conclusion':conclusion}]})
        check(quality(api,'owner/math','head')['state']==expected,'CI '+expected)
    response_overrides[checks_path]=(200,{'total_count':1,'check_runs':[{'id':1,'name':'test','status':'in_progress'}]})
    check(quality(api,'owner/math','head')['state']=='pending','CI pending')
    response_overrides[checks_path]=(200,{'total_count':2,'check_runs':[{'id':1,'name':'test','status':'completed','conclusion':'failure'},{'id':2,'name':'test','status':'completed','conclusion':'success'}]})
    check(quality(api,'owner/math','head')['state']=='passed','CI latest rerun supersedes failure')
    del response_overrides[checks_path]
    body=b'# API\n<script>alert(1)</script>\n[bad](javascript:alert(1))'
    files.append({'path':'README.md','type':'blob','mode':'100644'})
    response_overrides['/repos/owner/math/contents/README.md']=(200,{'encoding':'base64','size':len(body),'content':base64.b64encode(body).decode()})
    check(package_documents(api,'owner/math','head')==[{'path':'README.md','markdown':body.decode()}],'document snapshot preserves source for safe rendering')
    response_overrides['/repos/owner/math/contents/README.md']=(200,{'encoding':'base64','size':65537})
    check(not package_documents(api,'owner/math','head'),'document size limit')
    del response_overrides['/repos/owner/math/contents/README.md'];files.pop()
    for args, expected in [(('pkg','inspect','--json'),None),(('examples/main.dev',),'42'),(('tests/main.dev',),''),(('run','tests/main.dev','--engine','ast'),'')]:
        run = subprocess.run([str(d),'-C',str(repo/'templates/devlang-package'),*args],capture_output=True,text=True,encoding='utf-8',timeout=30)
        check(run.returncode==0 and (expected is None or run.stdout.strip()==expected),(args,run.stdout,run.stderr))
    check(not (repo/'templates/devlang-package/.dev').exists(),'inspect/template must not install')
    print(f'Package Store: {checks} checks passed (real parser, mocked HTTP, release tags, failures, template runtimes)')
finally:
    server.shutdown();server.server_close()
