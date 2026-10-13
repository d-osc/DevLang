"""Validate the Web API example with real HTTP requests in both engines."""
import http.client
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import time

ROOT=Path(__file__).resolve().parent.parent
binary=ROOT/'target/release'/('devrun.exe' if os.name=='nt' else 'devrun')
for engine in ['auto','ast']:
    with tempfile.TemporaryDirectory() as directory:
        with socket.socket() as reserve:reserve.bind(('127.0.0.1',0));port=reserve.getsockname()[1]
        source=Path(directory)/'main.dev'
        source.write_text((ROOT/'examples/web-api/main.dev').read_text(encoding='utf-8').replace('listenOn(3000,',f'listenOn({port},'),encoding='utf-8')
        worker=subprocess.Popen([str(binary),str(source),'--engine',engine,'--','serve'],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
        def request(method,path,body=None):
            c=http.client.HTTPConnection('127.0.0.1',port,timeout=3)
            try:
                c.request(method,path,body.encode('utf-8') if isinstance(body,str) else body,{'Content-Type':'application/json'})
                r=c.getresponse();headers={k.lower():v for k,v in r.getheaders()};data=json.loads(r.read())
                assert headers['content-type'].startswith('application/json')
                return r.status,data,headers
            finally:c.close()
        try:
            for _ in range(100):
                assert worker.poll() is None,worker.stderr.read().decode()
                try:
                    assert request('GET','/healthz')[:2]==(200,{'status':'ok'});break
                except OSError:time.sleep(.03)
            else:raise AssertionError('startup timeout')
            status,users,_=request('GET','/api/users')
            assert status==200 and users['count']==2 and users['data'][0]['name']=='Dev'
            assert request('POST','/api/greet',json.dumps({'name':'โลก'},ensure_ascii=False))[:2]==(200,{'message':'Hello โลก'})
            for invalid in ['{','[]','{}','{"name":42}','{"name":""}',json.dumps({'name':'x'*101})]:
                assert request('POST','/api/greet',invalid)[0]==400,invalid
            assert request('GET','/missing')[0]==404
            assert request('GET','/api/greet')[2]['allow']=='POST'
            assert request('POST','/api/users','{}')[0]==405
            assert request('GET','/healthz')[0]==200
        finally:
            if worker.poll() is None:worker.kill()
            _,err=worker.communicate(timeout=5)
            assert not err,err.decode()
print('PASS Web API routes, JSON/UTF-8, validation, 404/405 and recovery (auto/ast)')
