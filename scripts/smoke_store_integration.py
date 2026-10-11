"""Run real Store/Git installs, isolated-version runtime/native checks and publication preparation."""
import argparse,json,os,subprocess,tempfile
from pathlib import Path
from http.server import BaseHTTPRequestHandler,ThreadingHTTPServer
from threading import Thread

args=argparse.ArgumentParser(description=__doc__);args.add_argument('--d',required=True);d=Path(args.parse_args().d).resolve()
checks=0
with tempfile.TemporaryDirectory(prefix='dev store integration ') as tmp:
 root=Path(tmp).resolve();env=os.environ.copy();env['DEVLANG_HOME']=str(root/'global');env['DEVLANG_NO_PATH_UPDATE']='1';env['GIT_CONFIG_GLOBAL']=str(root/'gitconfig');env['GIT_CONFIG_NOSYSTEM']='1';env['GIT_TERMINAL_PROMPT']='0'
 def run(*arguments,cwd=root,error=None):
  global checks
  r=subprocess.run([str(d),'-C',str(cwd),*map(str,arguments)],env=env,capture_output=True,text=True,encoding='utf-8',timeout=90)
  assert (r.returncode!=0 and error in r.stderr) if error else r.returncode==0,(arguments,r.stdout,r.stderr)
  checks+=1;return r.stdout.strip()
 def git(cwd,*arguments):
  r=subprocess.run(['git','-C',str(cwd),*arguments],env=env,capture_output=True,text=True,encoding='utf-8',timeout=45);assert r.returncode==0,r.stderr;return r.stdout.strip()
 def package(at,name,version='1.0.0',deps=None,body=None,bin=None):
  (at/'src').mkdir(parents=True,exist_ok=True);(at/'package.don').write_text(json.dumps({'version':version,'package':{'name':name},'dependencies':deps or {},'bin':bin or {}}),encoding='utf-8')
  (at/'src/lib.dev').write_text(body or 'fn value() i64 { return 10 }\n',encoding='utf-8')
 def app(at,body,deps=None,bin=None):
  package(at,at.name,deps=deps,bin=bin);(at/'src/main.dev').write_text(body,encoding='utf-8')
 def init(at):
  git(at,'init','-b','main');git(at,'config','user.name','Store Test');git(at,'config','user.email','store@example.invalid')
 def commit(at,version):
  git(at,'add','.');git(at,'commit','-m',version);git(at,'tag','v'+version);return git(at,'rev-parse','HEAD')
 source=root/'gitmath';package(source,'math');init(source);sha1=commit(source,'1.0.0')
 catalog={'schema_version':1,'packages':[{'id':'owner/math','name':'math','repository':'https://github.com/owner/math','description':'integration fixture','versions':[{'version':'1.0.0','tag':'v1.0.0','commit':sha1}]}]}
 git(root,'config','--global','url.'+source.as_uri()+'.insteadOf','https://github.com/owner/math.git')
 class Handler(BaseHTTPRequestHandler):
  def log_message(self,*args):pass
  def do_GET(self):
   data=json.dumps(catalog).encode();self.send_response(200);self.send_header('Content-Length',str(len(data)));self.end_headers();self.wfile.write(data)
 server=ThreadingHTTPServer(('127.0.0.1',0),Handler);Thread(target=server.serve_forever,daemon=True).start();url=f'http://127.0.0.1:{server.server_port}/packages.json'
 try:
  assert '@owner/math' in run('pkg','search','math','--store',url)
  assert sha1 in run('pkg','info','@owner/math','--store',url)
  consumer=root/'consumer';app(consumer,'use "numbers/lib" as numbers\nfn main() { print(numbers.value()) }\nmain()\n')
  run('pkg','add','@owner/math','--as','numbers','--store',url,'--version','^1',cwd=consumer)
  assert run('run',cwd=consumer)=='10'
  lock=json.loads(run('don','to-json',consumer/'package-lock.don'));assert lock['packages']['numbers']['source']['package']=='@owner/math'
  run('pkg','install','--locked',cwd=consumer)
  package(source,'math','1.1.0',body='fn value() i64 { return 11 }\n');sha11=commit(source,'1.1.0')
  package(source,'math','2.0.0',body='fn value() i64 { return 20 }\n');sha2=commit(source,'2.0.0')
  catalog['packages'][0]['versions'] += [{'version':'1.1.0','tag':'v1.1.0','commit':sha11},{'version':'2.0.0','tag':'v2.0.0','commit':sha2}]
  assert 'numbers\t1.0.0\t1.1.0\t2.0.0' in run('pkg','outdated',cwd=consumer)
  before=(consumer/'package.don').read_bytes();run('pkg','upgrade','--dry-run',cwd=consumer);assert (consumer/'package.don').read_bytes()==before
  run('pkg','upgrade','numbers',cwd=consumer);assert run('run',cwd=consumer)=='11'
  run('pkg','upgrade','numbers','--latest',cwd=consumer);assert run('run',cwd=consumer)=='20'
  catalog['packages'][0]['versions'][-1]['commit']='0'*40
  run('pkg','upgrade',cwd=consumer,error='tag moved');catalog['packages'][0]['versions'][-1]['commit']=sha2
  catalog['packages'].append({**catalog['packages'][0],'id':'other/math','repository':'https://github.com/other/math'})
  run('pkg','add','math','--store',url,cwd=consumer,error='ambiguous');catalog['packages'].pop()
  mono=root/'mono';mono.mkdir();package(mono/'packages/math','math');init(mono);commit(mono,'1.0.0')
  subapp=root/'subapp';app(subapp,'use "submath/lib" as math\nfn main() { print(math.value()) }\nmain()\n')
  run('pkg','add','submath','--git',mono.as_uri(),'--subdir','packages/math','--version','^1',cwd=subapp)
  assert run('run',cwd=subapp)=='10';run('pkg','install','--locked',cwd=subapp)
  run('pkg','add','bad','--git',mono.as_uri(),'--subdir','../math',cwd=subapp,error="without '..'")
  for name,value in [('math1',10),('math2',20)]:package(root/name,'math','1.0.0' if value==10 else '2.0.0',body=f'fn value() i64 {{ return {value} }}\n')
  for name,path in [('left','math1'),('right','math2')]:package(root/name,name,deps={'math':{'path':'../'+path}},body='use "math/lib" as math\nfn value() i64 { return math.value() }\n')
  right_manifest=json.loads((root/'right/package.don').read_text());right_manifest['peerDependencies']={'graph':'^1'};(root/'right/package.don').write_text(json.dumps(right_manifest),encoding='utf-8')
  (root/'right/src/lib.dev').write_text('use "math/lib" as math\nuse "graph/host" as host\nfn value() i64 { return math.value() + host.zero() }\n',encoding='utf-8')
  graph=root/'graph';app(graph,'use "left/lib" as left\nuse "right/lib" as right\nuse "math/lib" as math\nfn main() { print(left.value() + right.value() + math.value()) }\nmain()\n',deps={'left':{'path':'../left'},'right':{'path':'../right'},'math':{'path':'../math1'}},bin={'answer':'src/main.dev'})
  (graph/'src/host.dev').write_text('fn zero() i64 { return 0 }\n',encoding='utf-8')
  run('pkg','install',cwd=graph)
  parsed=json.loads(run('don','to-json',graph/'package-lock.don'));assert parsed['version']==2 and len(parsed['packages'])==4
  assert run('run',cwd=graph)=='40';assert run('run','--engine','ast',cwd=graph)=='40'
  run('check','src/main.dev',cwd=graph)
  native=root/('native-graph.exe' if os.name=='nt' else 'native-graph')
  run('build','src/main.dev','--output',native,cwd=graph)
  execution=subprocess.run([str(native)],capture_output=True,text=True,encoding='utf-8',timeout=30);assert execution.returncode==0 and execution.stdout.strip()=='40',(execution.stdout,execution.stderr)

  run('pkg','install','--locked',cwd=graph)
  saved=(graph/'package-lock.don').read_bytes();parsed['edges']['root']['math']='bad';(graph/'package-lock.don').write_text(json.dumps(parsed),encoding='utf-8')
  run('run',cwd=graph,error='graph changed');(graph/'package-lock.don').write_bytes(saved)
  # Lock graph identity must survive relocation of an otherwise identical local workspace.
  import shutil
  relocated=root/'relocated';relocated.mkdir()
  for name in ('graph','math1','math2','left','right'): shutil.copytree(root/name,relocated/name,ignore=shutil.ignore_patterns('.dev'))
  assert run('run',cwd=relocated/'graph')=='40';run('pkg','install','--locked',cwd=relocated/'graph')
  original=(root/'math1/src/lib.dev').read_text();(root/'math1/src/lib.dev').write_text(original+'\n');run('run',cwd=graph,error='content changed');(root/'math1/src/lib.dev').write_text(original)
  run('pkg','install','--global',cwd=graph)
  listing=run('pkg','list','--global');assert 'answer' in listing
  run('pkg','uninstall','--global','graph')
  # A publication dry run prepares verified assets, without invoking gh or making uploads.
  assert 'prepared' in run('pkg','publish','--github','owner/math','--dry-run',cwd=source)
  incompatible=root/'incompatible';app(incompatible,'fn main() { print(1) }\nmain()\n');m=json.loads((incompatible/'package.don').read_text());m['devlang']='>=999.0.0';(incompatible/'package.don').write_text(json.dumps(m),encoding='utf-8')
  run('pkg','inspect','--json',cwd=incompatible);assert json.loads(run('pkg','compatibility','--json',cwd=incompatible))['compatible'] is False
  run('run',cwd=incompatible,error='requires DevLang CLI')
  print(f'Store integration: {checks} CLI checks passed; real Git, SemVer upgrades, tag pinning, subdir, isolated graph, peers/global snapshot and compatibility')
 finally:server.shutdown();server.server_close()
