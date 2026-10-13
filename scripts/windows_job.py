"""Windows worker containment. Start suspended, assign limits, then resume."""
import ctypes
from ctypes import wintypes as W
import os

class BasicLimits(ctypes.Structure):
    _fields_=[('process_time',ctypes.c_longlong),('job_time',ctypes.c_longlong),('flags',W.DWORD),
              ('min_working',ctypes.c_size_t),('max_working',ctypes.c_size_t),('active',W.DWORD),
              ('affinity',ctypes.c_size_t),('priority',W.DWORD),('scheduling',W.DWORD)]
class IoCounters(ctypes.Structure):
    _fields_=[(name,ctypes.c_ulonglong) for name in ['read_ops','write_ops','other_ops','read_bytes','write_bytes','other_bytes']]
class ExtendedLimits(ctypes.Structure):
    _fields_=[('basic',BasicLimits),('io',IoCounters)]+[(name,ctypes.c_size_t) for name in ['process_memory','job_memory','peak_process','peak_job']]
class CpuLimits(ctypes.Structure):
    _fields_=[('flags',W.DWORD),('rate',W.DWORD)]
class ThreadEntry(ctypes.Structure):
    _fields_=[('size',W.DWORD),('usage',W.DWORD),('tid',W.DWORD),('pid',W.DWORD),('base',W.LONG),('delta',W.LONG),('flags',W.DWORD)]

class WindowsJob:
    def __init__(self,memory_mib=384,processes=8,cpu_percent=50):
        if os.name!='nt':raise OSError('Windows Job Objects require Windows')
        if not 1<=memory_mib<=ctypes.c_size_t(-1).value//1048576 or not 1<=processes<=0xffffffff or not 1<=cpu_percent<=100:
            raise ValueError('invalid Windows Job limits')
        self.api=ctypes.WinDLL('kernel32',use_last_error=True)
        signatures={
            'CreateJobObjectW':([ctypes.c_void_p,W.LPCWSTR],W.HANDLE),
            'SetInformationJobObject':([W.HANDLE,ctypes.c_int,ctypes.c_void_p,W.DWORD],W.BOOL),
            'AssignProcessToJobObject':([W.HANDLE,W.HANDLE],W.BOOL),
            'CloseHandle':([W.HANDLE],W.BOOL),
            'OpenProcess':([W.DWORD,W.BOOL,W.DWORD],W.HANDLE),
            'CreateToolhelp32Snapshot':([W.DWORD,W.DWORD],W.HANDLE),
            'Thread32First':([W.HANDLE,ctypes.POINTER(ThreadEntry)],W.BOOL),
            'Thread32Next':([W.HANDLE,ctypes.POINTER(ThreadEntry)],W.BOOL),
            'OpenThread':([W.DWORD,W.BOOL,W.DWORD],W.HANDLE),
            'ResumeThread':([W.HANDLE],W.DWORD),
        }
        for name,(params,result) in signatures.items():
            fn=getattr(self.api,name);fn.argtypes=params;fn.restype=result
        self.handle=self.api.CreateJobObjectW(None,None)
        if not self.handle:raise ctypes.WinError(ctypes.get_last_error())
        try:
            limits=ExtendedLimits()
            limits.basic.flags=0x2000|0x200|0x8 # kill on close, job commit limit, process count
            limits.basic.active=processes;limits.job_memory=memory_mib*1048576
            self._set(9,limits)
            self._set(15,CpuLimits(0x1|0x4,cpu_percent*100)) # enable + hard CPU cap
        except BaseException:self.close();raise
    def _set(self,kind,value):
        if not self.api.SetInformationJobObject(self.handle,kind,ctypes.byref(value),ctypes.sizeof(value)):
            raise ctypes.WinError(ctypes.get_last_error())
    def assign_and_resume(self,worker):
        process=self.api.OpenProcess(0x100|0x1,False,worker.pid)
        if not process:raise ctypes.WinError(ctypes.get_last_error())
        try:
            if not self.api.AssignProcessToJobObject(self.handle,process):raise ctypes.WinError(ctypes.get_last_error())
        finally:self.api.CloseHandle(process)
        snapshot=self.api.CreateToolhelp32Snapshot(0x4,0) # snapshot thread IDs
        if snapshot==ctypes.c_void_p(-1).value:raise ctypes.WinError(ctypes.get_last_error())
        try:
            entry=ThreadEntry();entry.size=ctypes.sizeof(entry)
            found=self.api.Thread32First(snapshot,ctypes.byref(entry))
            while found:
                if entry.pid==worker.pid:
                    thread=self.api.OpenThread(0x2,False,entry.tid)
                    if not thread:raise ctypes.WinError(ctypes.get_last_error())
                    try:
                        if self.api.ResumeThread(thread)==0xffffffff:raise ctypes.WinError(ctypes.get_last_error())
                        return
                    finally:self.api.CloseHandle(thread)
                found=self.api.Thread32Next(snapshot,ctypes.byref(entry))
            raise OSError('cannot locate suspended worker thread')
        finally:self.api.CloseHandle(snapshot)
    def close(self):
        if self.handle:self.api.CloseHandle(self.handle);self.handle=None
