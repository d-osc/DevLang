"""Exercise kernel-enforced Windows worker limits and owner-death cleanup."""
import ctypes
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import subprocess
import sys
from windows_job import WindowsJob

def main():
    if os.name!='nt':raise SystemExit('Windows-only test')
    for settings in [{'memory_mib':0},{'memory_mib':2**64},{'processes':0},{'cpu_percent':101}]:
        try:WindowsJob(**settings);raise AssertionError('invalid limits accepted')
        except ValueError:pass
    job=WindowsJob(memory_mib=64,processes=1,cpu_percent=25)
    worker=None
    try:
        code='''import subprocess,sys
blocks=[]
try:
 while True:blocks.append(bytearray(1048576))
except MemoryError:print('memory blocked',len(blocks),flush=True)
try:subprocess.Popen([sys.executable,'-c','pass']);raise AssertionError('process escaped limit')
except OSError:print('process blocked',flush=True)
'''
        worker=subprocess.Popen([sys.executable,'-c',code],stdout=subprocess.PIPE,stderr=subprocess.PIPE,
                                creationflags=subprocess.CREATE_NO_WINDOW|0x4)
        job.assign_and_resume(worker)
        out,err=worker.communicate(timeout=20)
        assert worker.returncode==0 and b'memory blocked' in out and b'process blocked' in out,(out,err,worker.returncode)
    finally:
        job.close()
        if worker is not None and worker.poll() is None:worker.wait(timeout=10)
    # Keep process handles open so PID reuse cannot make the death check ambiguous.
    api=ctypes.WinDLL('kernel32',use_last_error=True)
    api.OpenProcess.argtypes=[W.DWORD,W.BOOL,W.DWORD];api.OpenProcess.restype=W.HANDLE
    api.WaitForSingleObject.argtypes=[W.HANDLE,W.DWORD];api.WaitForSingleObject.restype=W.DWORD
    api.CloseHandle.argtypes=[W.HANDLE]
    child="import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(60)']);print(p.pid,flush=True);time.sleep(60)"
    owner_code=f'''import subprocess,sys,json,time
from windows_job import WindowsJob
j=WindowsJob()
p=subprocess.Popen([sys.executable,'-c',{child!r}],stdout=subprocess.PIPE,creationflags=subprocess.CREATE_NO_WINDOW|0x4)
j.assign_and_resume(p)
print(json.dumps([p.pid,int(p.stdout.readline())]),flush=True)
time.sleep(60)
'''
    owner=subprocess.Popen([sys.executable,'-c',owner_code],cwd=Path(__file__).resolve().parent,
                           stdout=subprocess.PIPE,stderr=subprocess.PIPE,creationflags=subprocess.CREATE_NO_WINDOW)
    handles=[]
    try:
        pids=json.loads(owner.stdout.readline())
        for pid in pids:
            handle=api.OpenProcess(0x100000,False,pid)
            assert handle,ctypes.WinError(ctypes.get_last_error());handles.append(handle)
            assert api.WaitForSingleObject(handle,0)==258,'process exited before owner death'
        owner.terminate();owner.wait(timeout=10)
        for handle in handles:assert api.WaitForSingleObject(handle,10000)==0,'worker descendant outlived supervisor'
    finally:
        if owner.poll() is None:owner.terminate();owner.wait(timeout=10)
        for handle in handles:api.CloseHandle(handle)
        owner.communicate(timeout=10)
    print('PASS Windows Job: hard commit/process caps and abrupt owner death kill worker + descendant')

if __name__=='__main__':main()
