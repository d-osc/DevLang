"""Validate real worker hang/exit recovery, restart budgets, RSS and log rotation."""
import argparse
import http.client
import os
from pathlib import Path
import socket
import signal
import subprocess
import sys
import tempfile
import time

ROOT=Path(__file__).resolve().parent.parent

def main():
    p=argparse.ArgumentParser();p.add_argument('--bin-dir',default='target/release');a=p.parse_args()
    binary=(ROOT/a.bin_dir/('devrun.exe' if os.name=='nt' else 'devrun')).resolve()
    with tempfile.TemporaryDirectory(prefix='dev-supervisor-') as directory:
        root=Path(directory)
        with socket.socket() as reserve:reserve.bind(('127.0.0.1',0));port=reserve.getsockname()[1]
        source=root/'main.dev'
        source.write_text('''use "std/http"
fn main(){
 print("LOGTEXT")
 let server=http.createServer(fn(req:http.IncomingMessage,res:http.ServerResponse){res.end("initial")})
 server.on("request",fn(req:http.IncomingMessage,res:http.ServerResponse){
  if req.url=="/hang" {while true {}}
  res.end("ok")
  if req.url=="/exit" {server.close()}
 })
 server.listenOn(PORT,"127.0.0.1")
}
main()
'''.replace('PORT',str(port)).replace('LOGTEXT','x'*30000),encoding='utf-8')
        base=[sys.executable,str(ROOT/'scripts/serve_http.py'),'--runtime',str(binary),'--file',str(source),
              '--health',f'http://127.0.0.1:{port}/healthz','--interval','.1','--timeout','.2',
              '--startup-grace','.3','--failures','2','--restart-window','60','--log',str(root/'worker.log'),
              '--log-bytes','8192','--run-seconds','20']
        def request(route,method='GET'):
            c=http.client.HTTPConnection('127.0.0.1',port,timeout=.3)
            try:c.request(method,route);r=c.getresponse();return r.status,r.read()
            finally:c.close()
        def healthy(process):
            for _ in range(100):
                assert process.poll() is None,(root/'events').read_text(encoding='utf-8')
                try:
                    if request('/healthz')==(200,b'ok'):return
                except OSError:pass
                time.sleep(.05)
            raise AssertionError('health recovery timed out')
        with (root/'events').open('wb') as events:
            service=subprocess.Popen(base+['--max-restarts','2'],stdout=events,stderr=events,**({'creationflags':subprocess.CREATE_NO_WINDOW} if os.name=='nt' else {'start_new_session':True}))
            try:
                healthy(service)
                try:request('/hang')
                except OSError:pass
                healthy(service)
                assert request('/exit')==(200,b'ok')
                # Wait for the old listener to finish and the replacement to bind.
                time.sleep(.2);healthy(service)
                try:request('/hang')
                except OSError:pass
                assert service.wait(timeout=10)==1,'restart budget must fail closed'
            finally:
                if service.poll() is None:
                    if os.name=='nt':subprocess.run(['taskkill','/PID',str(service.pid),'/T','/F'],capture_output=True,timeout=10,creationflags=subprocess.CREATE_NO_WINDOW)
                    else:os.killpg(service.pid,signal.SIGTERM)
                    service.wait(timeout=10)
        text=(root/'events').read_text(encoding='utf-8')
        assert text.count('started:')==3 and 'health failures' in text and 'exit 0' in text and 'restart budget exhausted' in text,text
        logs=list(root.glob('worker.log*'))
        assert 2<=len(logs)<=4,logs
        assert all(f.stat().st_size<=8192+5000 for f in logs),[(f,f.stat().st_size) for f in logs]
        memory=subprocess.run(base+['--max-rss-mib','1','--max-restarts','0'],capture_output=True,text=True,timeout=10)
        assert memory.returncode==1 and 'RSS budget exceeded' in memory.stdout,memory
        normal=subprocess.run(base+['--runtime',os.path.relpath(binary),'--run-seconds','1.5'],capture_output=True,text=True,timeout=10)
        assert normal.returncode==0 and normal.stdout.count('started:')==1,normal
        for extra in [['--health','http://example.com/'],['--interval','nan'],['--health',f'http://127.0.0.1:0/'],['--runtime',str(root/'missing')]]:
            bad=subprocess.run(base+extra,capture_output=True,text=True,timeout=10)
            assert bad.returncode!=0,bad
        # A separate data directory must be writable while source stays elsewhere.
        data=root/'data';data.mkdir()
        original=source.read_text(encoding='utf-8')
        source.write_text('use "std/fs"\nfs.writeFileSync("marker.txt","ok")\n'+original,encoding='utf-8')
        located=subprocess.run(base+['--cwd',str(data),'--run-seconds','1.5'],capture_output=True,text=True,timeout=10)
        assert located.returncode==0 and (data/'marker.txt').read_text()=='ok',located
        assert not (root/'marker.txt').exists()
        # Validate the shipped starter on an isolated port using its real source.
        starter=(ROOT/'examples/production-http/main.dev').read_text(encoding='utf-8').replace('listenOn(3000,','listenOn('+str(port)+',')
        source.write_text(starter,encoding='utf-8')
        with (root/'starter-events').open('wb') as events:
            starter_service=subprocess.Popen(base+['--run-seconds','2'],stdout=events,stderr=events)
            try:
                healthy(starter_service)
                assert request('/')==(200,b'DevLang HTTP service')
                assert request('/missing')[0]==404
                assert request('/healthz','POST')[0]==405
            finally:
                assert starter_service.wait(timeout=10)==0
        with socket.socket() as probe:
            assert probe.connect_ex(('127.0.0.1',port))!=0,'worker was left behind'
    print('PASS supervisor: hang/exit recovery, restart cap, RSS budget, rotating logs and clean stop')

if __name__=='__main__':main()
