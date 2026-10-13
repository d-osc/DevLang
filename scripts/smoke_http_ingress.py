"""Run the shipped NGINX profile against a real source HTTP server."""
import argparse
import http.client
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent

def port():
    with socket.socket() as s:
        s.bind(('127.0.0.1',0)); return s.getsockname()[1]

def main():
    parser=argparse.ArgumentParser()
    parser.add_argument('--nginx',default='nginx')
    parser.add_argument('--bin-dir',default='target/release')
    args=parser.parse_args()
    nginx=shutil.which(args.nginx) or str(Path(args.nginx).resolve())
    binary=(ROOT/args.bin_dir/('devrun.exe' if os.name=='nt' else 'devrun')).resolve()
    backend,edge=port(),port()
    while edge==backend: edge=port()
    with tempfile.TemporaryDirectory(prefix='dev-ingress-') as directory:
        root=Path(directory).resolve()
        for name in ['logs','temp/client','temp/proxy']: (root/name).mkdir(parents=True,exist_ok=True)
        config=(ROOT/'deploy/http/nginx.conf').read_text(encoding='utf-8')
        config=config.replace('127.0.0.1:8080',f'127.0.0.1:{edge}').replace('127.0.0.1:3000',f'127.0.0.1:{backend}')
        (root/'nginx.conf').write_text(config,encoding='utf-8')
        source=root/'main.dev'
        source.write_text('''use "std/http"
fn main(){
 let server=http.createServer(fn(req:http.IncomingMessage,res:http.ServerResponse){
  print(req.url)
  if req.url=="/fail" {let zero=0;print(1/zero);return}
  res.end(req.body)
 })
 server.listenOn(PORT,"127.0.0.1")
}
main()
'''.replace('PORT',str(backend)),encoding='utf-8')
        prefix=str(root).replace('\\','/')+'/'
        command=[nginx,'-p',prefix,'-c','nginx.conf']
        flags={'creationflags':subprocess.CREATE_NO_WINDOW} if os.name=='nt' else {'start_new_session':True}
        checked=subprocess.run(command+['-t'],capture_output=True,timeout=15,**flags)
        assert checked.returncode==0,checked.stderr
        with (root/'backend.log').open('wb') as log, (root/'backend-errors.log').open('wb') as err, (root/'nginx-output.log').open('wb') as output:
            runtime=subprocess.Popen([str(binary),str(source)],stdout=log,stderr=err,**flags)
            proxy=None
            def request(path='/health',body=b'ok'):
                c=http.client.HTTPConnection('127.0.0.1',edge,timeout=15)
                try:
                    c.request('POST',path,body,{'Connection':'close'})
                    r=c.getresponse();return r.status,r.read()
                finally: c.close()
            def healthy():
                for _ in range(100):
                    assert runtime.poll() is None,'runtime exited'
                    assert proxy.poll() is None,'NGINX exited'
                    try:
                        if request()==(200,b'ok'): return
                    except OSError: pass
                    time.sleep(.03)
                raise AssertionError('ingress did not recover')
            try:
                proxy=subprocess.Popen(command+['-g','daemon off;'],stdout=output,stderr=output,**flags)
                healthy()
                assert request('/fail')[0]==500
                healthy()
                assert request('/oversize',b'x'*65537)[0]==413
                healthy()
                with socket.create_connection(('127.0.0.1',edge),timeout=20) as slow:
                    started=time.monotonic()
                    slow.sendall(b'POST /slow-body HTTP/1.1\r\nHost: localhost\r\nContent-Length: 20\r\n\r\nx')
                    healthy()  # Partial upload must never block the interpreter.
                    reply=slow.recv(1024)
                    assert not reply or b'408' in reply,reply
                    assert 8<=time.monotonic()-started<=15,'body timeout outside configured budget'
                with socket.create_connection(('127.0.0.1',edge),timeout=15) as slow:
                    started=time.monotonic()
                    slow.sendall(b'GET /slow-header HTTP/1.1\r\nHost: ')
                    healthy()
                    reply=slow.recv(1024)
                    assert not reply or b'408' in reply,reply
                    assert 3.5<=time.monotonic()-started<=10,'header timeout outside configured budget'
                with socket.create_connection(('127.0.0.1',edge),timeout=15) as huge:
                    huge.sendall(b'GET /huge-header HTTP/1.1\r\nHost: localhost\r\nX-Large: '+b'x'*40000+b'\r\n\r\n')
                    reply=huge.recv(1024)
                    assert b'400' in reply or b'431' in reply,reply
                held=[]
                try:
                    for _ in range(16):
                        s=socket.create_connection(('127.0.0.1',edge),timeout=15);held.append(s)
                        s.sendall(b'POST /held HTTP/1.1\r\nHost: localhost\r\nContent-Length: 20\r\n\r\nx')
                    time.sleep(.2)
                    assert request('/overflow')[0]==503
                finally:
                    for s in held:s.close()
                healthy()
                log.flush()
                seen=(root/'backend.log').read_text(encoding='utf-8')
                for denied in ['/slow-body','/slow-header','/huge-header','/oversize','/held','/overflow']:
                    assert denied not in seen,(denied,seen)
            finally:
                try:
                    if proxy is not None and proxy.poll() is None:
                        try:
                            subprocess.run(command+['-s','quit'],capture_output=True,timeout=15,**flags)
                            proxy.wait(timeout=15)
                        except subprocess.TimeoutExpired:
                            if os.name=='nt':subprocess.run(['taskkill','/PID',str(proxy.pid),'/T','/F'],capture_output=True,timeout=15)
                            else:os.killpg(proxy.pid,signal.SIGKILL)
                            proxy.wait(timeout=10)
                finally:
                    if runtime.poll() is None:runtime.kill();runtime.wait(timeout=10)
    print('PASS live ingress: recovery, body/header limits, slow uploads/headers and overload shedding')

if __name__=='__main__':main()
