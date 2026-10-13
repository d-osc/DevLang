"""Real HTTP requests, handler recovery and process RSS sampling (source runtime)."""
import argparse
import concurrent.futures
import ctypes
import http.client
import json
import os
from pathlib import Path
import socket
import struct
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parent.parent

def rss(pid):
    if os.name != 'nt':
        return int(Path(f'/proc/{pid}/statm').read_text().split()[1]) * os.sysconf('SC_PAGE_SIZE')
    from ctypes import wintypes
    class Counters(ctypes.Structure):
        _fields_ = [('cb', wintypes.DWORD), ('faults', wintypes.DWORD)] + [(name, ctypes.c_size_t) for name in
            ['peak', 'working', 'peak_paged', 'paged', 'peak_nonpaged', 'nonpaged', 'pagefile', 'peak_pagefile']]
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    psapi = ctypes.WinDLL('psapi', use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
    handle = kernel.OpenProcess(0x1000, False, pid)
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        info = Counters(); info.cb = ctypes.sizeof(info)
        if not psapi.GetProcessMemoryInfo(handle, ctypes.byref(info), info.cb):
            raise ctypes.WinError(ctypes.get_last_error())
        return info.working
    finally:
        kernel.CloseHandle(handle)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--bin-dir', default='target/debug')
    parser.add_argument('--requests', type=int, default=5000)
    parser.add_argument('--duration-seconds', type=int, default=0)
    parser.add_argument('--concurrency', type=int, default=8)
    parser.add_argument('--max-growth-mib', type=int, default=32)
    parser.add_argument('--report', type=Path, default=Path('out/http-soak.json'))
    args = parser.parse_args()
    if args.duration_seconds < 0 or args.requests < 100 or not 1 <= args.concurrency <= 64 or args.max_growth_mib < 1:
        parser.error('requests >= 100, concurrency 1..64, positive max-growth-mib required')
    binary = (ROOT/args.bin_dir/('devrun.exe' if os.name == 'nt' else 'devrun')).resolve()
    reports = []
    for engine in ['auto', 'ast']:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with socket.socket() as reserve:
                reserve.bind(('127.0.0.1', 0)); port = reserve.getsockname()[1]
            source = root/'main.dev'
            source.write_text('''use "std/http"
fn main(){
    let server=http.createServer(fn(req:http.IncomingMessage,res:http.ServerResponse){res.end("initial")})
    server.on("request",fn(req:http.IncomingMessage,res:http.ServerResponse){
        if req.url=="/stop" {res.end("stopped");server.close();return}
        if req.url=="/fail" {let zero=0;print(1/zero);return}
        if req.url=="/invalid" {res.setHeader("X-Test","bad\\r\\nInjected:yes");return}
        if req.url=="/missing" {return}
        if req.url=="/large" {for i in 0..2048 {res.write(req.body)};res.end("");return}
        let cycle=new Object();cycle.self=cycle
        let values=new Array(cycle);cycle.values=values
        res.end("ok")
        if req.url=="/after-end" {let zero=0;print(1/zero)}
    })
    server.listenOn(PORT,"127.0.0.1")
}
main()
'''.replace('PORT',str(port)), encoding='utf-8')
            with (root/'stderr.log').open('wb') as errors:
                process = subprocess.Popen([str(binary),str(source),'--engine',engine],stdout=subprocess.DEVNULL,stderr=errors)
                def request(route='/health'):
                    connection = http.client.HTTPConnection('127.0.0.1',port,timeout=10)
                    try:
                        connection.request('POST',route,body=b'x'*512,headers={'Connection':'close'})
                        response=connection.getresponse(); return response.status,response.read()
                    finally:
                        connection.close()
                try:
                    for _ in range(200):
                        if process.poll() is not None: raise AssertionError('server exited during startup')
                        try:
                            assert request()==(200,b'ok'); break
                        except OSError: time.sleep(.025)
                    else: raise AssertionError('startup timeout')
                    for route in ['/fail','/invalid','/missing']:
                        assert request(route)[0]==500,route
                        assert request()==(200,b'ok'),route
                    # Reset a connection after response headers, then verify recovery.
                    with socket.create_connection(('127.0.0.1',port),timeout=10) as peer:
                        peer.sendall(b'POST /large HTTP/1.1\r\nHost: localhost\r\nContent-Length: 512\r\nConnection: close\r\n\r\n'+b'x'*512)
                        assert peer.recv(256).startswith(b'HTTP/1.1 200')
                        peer.setsockopt(socket.SOL_SOCKET,socket.SO_LINGER,struct.pack('hh' if os.name=='nt' else 'ii',1,0))
                    assert request()==(200,b'ok')
                    assert request('/after-end')==(200,b'ok')
                    assert request()==(200,b'ok')
                    with concurrent.futures.ThreadPoolExecutor(max_workers=args.concurrency) as pool:
                        for result in pool.map(lambda _:request(),range(200)):
                            assert result==(200,b'ok'),result
                        samples=[rss(process.pid)]; started=time.monotonic(); completed=0
                        batch=max(1,args.requests//10)
                        while completed<args.requests or time.monotonic()-started<args.duration_seconds:
                            count=min(batch,args.requests-completed) if completed<args.requests else batch
                            for result in pool.map(lambda _:request(),range(count)):
                                assert result==(200,b'ok'),result
                            completed+=count; samples.append(rss(process.pid))
                    elapsed=time.monotonic()-started
                    growth=max(samples)-samples[0]
                    assert growth<=args.max_growth_mib*1024*1024,(engine,'RSS growth',samples)
                    assert request('/stop')==(200,b'stopped')
                    assert process.wait(timeout=10)==0,'shutdown failed'
                    errors.flush()
                    log=(root/'stderr.log').read_text(encoding='utf-8')
                    assert 'http handler error:' in log and 'invalid response header' in log,log
                    reports.append({'engine':engine,'requests':completed,'concurrency':args.concurrency,
                        'seconds':round(elapsed,3),'requests_per_second':round(completed/elapsed,1),
                        'rss_samples_bytes':samples,'growth_bytes':growth,'memory_limit_bytes':args.max_growth_mib*1024*1024})
                finally:
                    if process.poll() is None: process.kill();process.wait(timeout=10)
    args.report.parent.mkdir(parents=True,exist_ok=True)
    args.report.write_text(json.dumps({'scope':'localhost source HTTP, bounded concurrency; not public-ingress certification','results':reports},indent=2)+'\n',encoding='utf-8')
    print(f'PASS HTTP error recovery, clean shutdown and RSS: minimum {args.requests} requests, duration {args.duration_seconds}s per engine')

if __name__=='__main__':main()
