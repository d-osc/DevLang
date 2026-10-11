"""Real CLI regressions for package policy, version search, scripts and offline downloads."""
import argparse, hashlib, json, os, shutil, subprocess, sys, tempfile
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from threading import Thread
from pathlib import Path

parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--d',required=True)
d=Path(parser.parse_args().d).resolve();checks=0
with tempfile.TemporaryDirectory(prefix='devlang package features ') as temporary:
 root=Path(temporary);env=os.environ.copy();env.update(DEVLANG_HOME=str(root/'home'),DEVLANG_NO_PATH_UPDATE='1',GIT_CONFIG_GLOBAL=str(root/'gitconfig'),GIT_CONFIG_NOSYSTEM='1',GIT_TERMINAL_PROMPT='0')
 def run(*args,cwd=root,error=None,code=None,extra=None):
  global checks
  result=subprocess.run([str(d),'-C',str(cwd),*map(str,args)],env={**env,**(extra or {})},capture_output=True,text=True,encoding='utf-8',timeout=90)
  assert (result.returncode==code if code is not None else result.returncode!=0 if error else result.returncode==0),(args,result.stdout,result.stderr)
  if error: assert error in result.stderr,(args,result.stdout,result.stderr)
  checks+=1;return result.stdout.strip()
 def write(at,**fields):
  at.mkdir(parents=True,exist_ok=True);m={'version':'1.0.0','package':{'name':at.name},**fields};(at/'package.don').write_text(json.dumps(m),encoding='utf-8')
  (at/'src').mkdir(exist_ok=True);(at/'src/main.dev').write_text('fn main() { print(42) }\nmain()\n',encoding='utf-8');(at/'src/lib.dev').write_text('fn value() i64 { return 42 }\n',encoding='utf-8');return m
 def change(at,**fields):
  m=json.loads((at/'package.don').read_text(encoding='utf-8'));m.update(fields);(at/'package.don').write_text(json.dumps(m),encoding='utf-8')
 def lock(at): return json.loads(run('don','to-json',at/'package-lock.don'))
 def git(at,*args):
  r=subprocess.run(['git','-C',str(at),*args],env=env,capture_output=True,text=True,encoding='utf-8',timeout=45);assert r.returncode==0,r.stderr;return r.stdout.strip()
 def commit(at,tag):git(at,'add','.');git(at,'commit','-m',tag);git(at,'tag',tag);return git(at,'rev-parse','HEAD')
 # Omitted optional dependencies are frozen and cannot be injected into a lock.
 app=root/'app';write(app,optionalDependencies={'missing':{'path':'../missing'}})
 assert 'omitted' in run('pkg','install',cwd=app)
 run('pkg','ci',cwd=app);run('run',cwd=app)
 saved=(app/'package-lock.don').read_bytes();bad=lock(app);bad['omitted']['stale']={'path':'../stale','optional':True};(app/'package-lock.don').write_text(json.dumps(bad),encoding='utf-8')
 run('run',cwd=app,error='stale');(app/'package-lock.don').write_bytes(saved)
 # Scripts execute explicitly, forward literal arguments, and preserve exit codes.
 helper=app/'args.py';helper.write_text('import sys\nprint(repr(sys.argv[1:]))\n',encoding='utf-8')
 change(app,scripts={'args':[sys.executable,'args.py'],'run':['d','run'],'fail':[sys.executable,'-c','raise SystemExit(7)'],'shell':'echo script-ok'})
 assert 'args' in run('task',cwd=app)
 assert '$(inert)' in run('task','args','--','a b','$(inert)',cwd=app)
 assert run('task','run',cwd=app)=='42';run('task','fail',cwd=app,code=7)
 assert run('task','shell',cwd=app)=='script-ok';run('task','shell','--','x',cwd=app,error='array script')
 # Platform policy and consumer-level overrides rebase through child manifests.
 base=root/'base';write(base);replacement=root/'replacement';write(replacement,version='2.0.0')
 child=root/'child';write(child,dependencies={'base':{'path':'../base'}})
 change(app,dependencies={'child':{'path':'../child'}},optionalDependencies={},overrides={'base':{'path':'../replacement'}})
 run('pkg','install',cwd=app);assert '../replacement' in lock(app)['packages']['base']['root']
 assert 'app -> child -> base' in run('pkg','tree',cwd=app);assert 'child -> base' in run('pkg','why','base',cwd=app)
 run('pkg','why','absent',cwd=app,error='not found');run('pkg','dedupe',cwd=app);run('pkg','ci',cwd=app)
 change(child,dependencies={'base':{'path':'../base','version':'^1'}});change(app,bin={'package-test':'src/main.dev'});run('pkg','install',cwd=app)
 run('pkg','install','--global',cwd=app)
 launcher=root/'home/bin'/('package-test.cmd' if os.name=='nt' else 'package-test')
 global_run=subprocess.run([str(launcher)],env=env,cwd=root,shell=os.name=='nt',capture_output=True,text=True,encoding='utf-8',timeout=30)
 assert global_run.returncode==0 and global_run.stdout.strip()=='42',(global_run.stdout,global_run.stderr)
 run('pkg','uninstall','--global','app')
 change(app,os=['!'+('win32' if os.name=='nt' else 'linux')]);run('run',cwd=app,error='not supported');change(app,os=[])
 change(app,overrides={'base':{'path':'../base'}});run('run',cwd=app,error='changed');run('pkg','install',cwd=app)
 # Backtrack across real Git tags when the newest version has incompatible peers.
 source=root/'search';write(source);git(source,'init','-b','main');git(source,'config','user.name','Fixture');git(source,'config','user.email','fixture@example.invalid')
 sha1=commit(source,'v1.0.0');change(source,version='1.1.0',peerDependencies={'base':'^2'});sha2=commit(source,'v1.1.0')
 solver=root/'solver';write(solver,dependencies={'search':{'git':source.as_uri(),'version':'1.0.0 - 1.9.9 || ^2'},'base':{'path':'../base'}})
 run('pkg','install',cwd=solver,extra={'DEVLANG_RESOLUTION_ATTEMPTS':'0'},error='integer from')
 run('pkg','install',cwd=solver,extra={'DEVLANG_RESOLUTION_ATTEMPTS':'1'},error='1-attempt budget')
 run('pkg','install',cwd=solver,extra={'DEVLANG_RESOLUTION_ATTEMPTS':'16'});assert lock(solver)['packages']['search']['resolved_version']=='1.0.0'
 run('pkg','ci',cwd=solver);run('pkg','install','--offline','--locked',cwd=solver)
 # A peer uses its nearest ancestor's provider in an isolated graph.
 plugin=root/'plugin';write(plugin,peerDependencies={'base':'^1'})
 left=root/'left';write(left,dependencies={'base':{'path':'../base'},'plugin':{'path':'../plugin'}})
 right=root/'right';write(right,dependencies={'base':{'path':'../replacement'}})
 scope=root/'scope';write(scope,dependencies={'left':{'path':'../left'},'right':{'path':'../right'},'base':{'path':'../replacement'}})
 run('pkg','install',cwd=scope);assert lock(scope)['version']==2;run('run',cwd=scope);run('pkg','ci',cwd=scope)
 # Missing peers are explicit errors until auto-install is opted into.
 git(base,'init','-b','main');git(base,'config','user.name','Fixture');git(base,'config','user.email','fixture@example.invalid');base_sha=commit(base,'v1.0.0')
 git(root,'config','--global','url.'+base.as_uri()+'.insteadOf','https://github.com/owner/base.git')
 catalog=root/'catalog.json';catalog.write_text(json.dumps({'schema_version':1,'packages':[{'id':'owner/base','name':'base','repository':'https://github.com/owner/base','versions':[{'version':'1.0.0','tag':'v1.0.0','commit':base_sha}]}]}),encoding='utf-8')
 auto=root/'auto';write(auto,dependencies={'plugin':{'path':'../plugin'}});before=(auto/'package.don').read_bytes()
 run('pkg','install',cwd=auto,error="requires peer 'base'");assert (auto/'package.don').read_bytes()==before
 run('pkg','install','--auto-peers',cwd=auto,extra={'DEVLANG_PEER_STORE':catalog.as_uri()});run('pkg','ci',cwd=auto);assert 'base' in json.loads(run('don','to-json',auto/'package.don'))['dependencies']
 change(plugin,peerDependencies={'base':'^1','zmissing':'^1'});rollback=root/'rollback';write(rollback,dependencies={'plugin':{'path':'../plugin'}});original=(rollback/'package.don').read_bytes()
 run('pkg','install','--auto-peers',cwd=rollback,extra={'DEVLANG_PEER_STORE':catalog.as_uri()},error='not found');assert (rollback/'package.don').read_bytes()==original
 change(plugin,peerDependencies={'base':'^1'})
 # Static registry alternatives must retain the original declaration and pin actual URL/hash.
 registry=root/'registry';change(source,version='1.0.0',peerDependencies={});run('pkg','publish','--registry',registry,cwd=source)
 change(source,version='1.1.0',peerDependencies={'base':'^2'});run('pkg','publish','--registry',registry,cwd=source)
 regapp=root/'regapp';write(regapp,dependencies={'base':{'path':'../base'}})
 run('pkg','add','search','--registry',(registry/'index.don').as_uri(),'--version','^1',cwd=regapp)
 locked=lock(regapp)['packages']['search'];assert locked['resolved_version']=='1.0.0' and locked['resolved_source']['version']=='=1.0.0',locked
 run('pkg','ci',cwd=regapp);run('run',cwd=regapp)
 receipt=root/'receipt.json';run('pkg','publish','--github','owner/search','--dry-run','--receipt',receipt,cwd=source)
 publication=json.loads(receipt.read_text(encoding='utf-8'));assert publication['uploaded'] is False and hashlib.sha256(Path(publication['asset']).read_bytes()).hexdigest()==publication['sha256']
 # Explicit local security feed reports both findings and absent coverage honestly.
 feed=root/'advisories.don';feed.write_text(json.dumps({'schema_version':1,'coverage':['base'],'advisories':[{'id':'TEST-1','package':'base','affected':'>= 1 <2'}]}),encoding='utf-8')
 assert 'TEST-1' in run('pkg','audit','--database',feed.as_uri(),cwd=app,code=1)
 feed.write_text(json.dumps({'schema_version':1,'coverage':[],'advisories':[]}),encoding='utf-8');assert 'UNASSESSED' in run('pkg','audit','--database',feed.as_uri(),cwd=app,code=2)
 run('pkg','audit','--offline',cwd=app,error='requires --database')
 run('pkg','verify-tag','v1.1.0','--key','A'*40,cwd=source,error='signature')
 # Shared HTTP cache restores archives after deleting only the fixture checkout, without networking.
 class Handler(SimpleHTTPRequestHandler):
  def __init__(self,*args,**kwargs):super().__init__(*args,directory=str(registry),**kwargs)
  def log_message(self,*args):pass
 server=ThreadingHTTPServer(('127.0.0.1',0),Handler);Thread(target=server.serve_forever,daemon=True).start()
 offline=root/'offline';write(offline)
 try:run('pkg','add','search','--registry',f'http://127.0.0.1:{server.server_port}/index.don','--version','=1.0.0',cwd=offline)
 finally:server.shutdown();server.server_close()
 shutil.rmtree(offline/'.dev/packages');run('pkg','install','--locked','--offline',cwd=offline);assert run('run',cwd=offline)=='42'
 cache=root/'home/cache/downloads';assert 'files' in run('pkg','cache','list')
 record=next(p for p in cache.glob('*.don') if 'tar.gz' in p.read_text(encoding='utf-8'));blob=cache/(json.loads(run('don','to-json',record))['sha256']+'.blob');blob.write_bytes(b'corrupted')
 shutil.rmtree(offline/'.dev/packages');run('pkg','install','--locked','--offline',cwd=offline,error='integrity')
 run('pkg','cache','clean');assert not list(cache.glob('*.blob')) and (cache/'.devlang-cache').exists()
 print(f'Extended package features: {checks} real CLI checks passed')
