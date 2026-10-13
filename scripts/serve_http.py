"""Supervise one source HTTP worker with health/RSS checks and bounded restarts."""
import argparse
from collections import deque
import http.client
import logging
import math
from logging.handlers import RotatingFileHandler
import os
from pathlib import Path
import signal
import shutil
import subprocess
import threading
import time
from urllib.parse import urlsplit
from soak_http_server import rss
from windows_job import WindowsJob

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--runtime',required=True)
    p.add_argument('--file',type=Path,required=True)
    p.add_argument('--cwd',type=Path,help='worker data/working directory (defaults to source directory)')
    p.add_argument('--health',default='http://127.0.0.1:3000/healthz')
    p.add_argument('--interval',type=float,default=2)
    p.add_argument('--timeout',type=float,default=1)
    p.add_argument('--startup-grace',type=float,default=15)
    p.add_argument('--failures',type=int,default=3)
    p.add_argument('--max-restarts',type=int,default=5)
    p.add_argument('--restart-window',type=float,default=60)
    p.add_argument('--max-rss-mib',type=int,default=256)
    p.add_argument('--job-memory-mib',type=int,default=384,help='Windows: OS committed-memory ceiling for worker tree')
    p.add_argument('--job-processes',type=int,default=8,help='Windows: worker tree process count')
    p.add_argument('--cpu-percent',type=int,default=50,help='Windows: worker tree hard CPU rate')
    p.add_argument('--log',type=Path,default=Path('out/http-worker.log'))
    p.add_argument('--log-bytes',type=int,default=1048576)
    p.add_argument('--run-seconds',type=float,default=0,help='0 runs until stopped')
    a=p.parse_args()
    try:u=urlsplit(a.health)
    except ValueError:p.error('invalid health URL')
    if u.scheme!='http' or u.hostname not in ['localhost','127.0.0.1','::1'] or u.username or u.password or u.fragment:
        p.error('--health must be a local HTTP URL without credentials/fragment')
    if all(math.isfinite(x) for x in [a.interval,a.timeout,a.restart_window,a.startup_grace,a.run_seconds]) and min(a.interval,a.timeout,a.restart_window)>0 and a.startup_grace>=0 and a.run_seconds>=0 and min(a.failures,a.max_rss_mib,a.log_bytes)>=1 and a.max_restarts>=0:
        pass
    else:p.error('invalid supervision limits')
    if min(a.job_memory_mib,a.job_processes)<1 or not 1<=a.cpu_percent<=100:p.error('invalid job limits')
    try: health_port=u.port if u.port is not None else 80
    except ValueError:p.error('invalid health port')
    if not 1<=health_port<=65535:p.error('invalid health port')
    candidate=Path(a.runtime).expanduser()
    runtime=str(candidate.resolve()) if candidate.is_file() else shutil.which(a.runtime)
    if not runtime:p.error('runtime executable not found')
    source=a.file.resolve()
    if not source.is_file():p.error('--file must exist')
    working=(a.cwd.resolve() if a.cwd else source.parent)
    if not working.is_dir():p.error('--cwd must be an existing directory')
    a.log.parent.mkdir(parents=True,exist_ok=True)
    logger=logging.getLogger('http-worker');logger.setLevel(logging.INFO)
    handler=RotatingFileHandler(a.log,maxBytes=a.log_bytes,backupCount=3,encoding='utf-8')
    logger.addHandler(handler)
    stop=threading.Event()
    def stopped(*_):stop.set()
    signal.signal(signal.SIGINT,stopped);signal.signal(signal.SIGTERM,stopped)
    if hasattr(signal,'SIGBREAK'):signal.signal(signal.SIGBREAK,stopped)
    def event(name,detail):
        message=f'{name}: {detail}';logger.info(message);print(message,flush=True)
    def healthy():
        c=http.client.HTTPConnection(u.hostname,health_port,timeout=a.timeout)
        try:
            c.request('GET',(u.path or '/')+('?' + u.query if u.query else ''),headers={'Connection':'close'})
            r=c.getresponse()
            # The health route must have a small, finite response. Never follow redirects.
            return r.status==200 and len(r.read(4097))<=4096
        except (OSError,http.client.HTTPException):return False
        finally:c.close()
    def drain(pipe,label):
        try:
            while data:=pipe.read(4096):logger.info('%s: %s',label,data.decode('utf-8',errors='replace').rstrip())
        finally:pipe.close()
    def kill(worker):
        if worker.poll() is None:
            if os.name=='nt':
                subprocess.run(['taskkill','/PID',str(worker.pid),'/T','/F'],capture_output=True,timeout=15,creationflags=subprocess.CREATE_NO_WINDOW)
            else:os.killpg(worker.pid,signal.SIGTERM)
            try:worker.wait(timeout=5)
            except subprocess.TimeoutExpired:
                if os.name!='nt':os.killpg(worker.pid,signal.SIGKILL)
                else:worker.kill()
                worker.wait(timeout=5)
    started=time.monotonic();history=deque();worker=None;readers=[];code=0;job=None
    try:
        while not stop.is_set():
            if a.run_seconds and time.monotonic()-started>=a.run_seconds:break
            flags={'creationflags':subprocess.CREATE_NO_WINDOW|0x4} if os.name=='nt' else {'start_new_session':True}
            if os.name=='nt':job=WindowsJob(a.job_memory_mib,a.job_processes,a.cpu_percent)
            worker=subprocess.Popen([runtime,str(source)],cwd=working,stdout=subprocess.PIPE,stderr=subprocess.PIPE,**flags)
            if job is not None:job.assign_and_resume(worker)
            readers=[threading.Thread(target=drain,args=(pipe,label),daemon=True) for pipe,label in [(worker.stdout,'stdout'),(worker.stderr,'stderr')]]
            for reader in readers:reader.start()
            event('started',worker.pid);launched=time.monotonic();failed=0;reason=''
            while not stop.wait(a.interval):
                if a.run_seconds and time.monotonic()-started>=a.run_seconds:stop.set();break
                if worker.poll() is not None:reason=f'exit {worker.returncode}';break
                try: memory=rss(worker.pid)
                except OSError:
                    if worker.poll() is not None:reason=f'exit {worker.returncode}';break
                    raise
                if memory>a.max_rss_mib*1048576:reason='RSS budget exceeded';break
                if time.monotonic()-launched<a.startup_grace:continue
                failed=0 if healthy() else failed+1
                if failed>=a.failures:reason='health failures';break
            if job is not None:job.close();job=None
            kill(worker)
            for reader in readers:reader.join(timeout=5)
            if stop.is_set():break
            now=time.monotonic()
            while history and now-history[0]>=a.restart_window:history.popleft()
            if len(history)>=a.max_restarts:
                event('restart budget exhausted',reason);code=1;break
            history.append(now);event('restart',reason)
            stop.wait(min(4,0.25*2**min(len(history)-1,4)))
    except (OSError,ValueError) as error:
        event('supervisor error',str(error));code=1
    finally:
        if job is not None:job.close()
        if worker is not None:kill(worker)
        for reader in readers:reader.join(timeout=5)
        handler.close();logger.removeHandler(handler)
    return code

if __name__=='__main__':raise SystemExit(main())
