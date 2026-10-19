"""Read counters for a newly launched owned child, never arbitrary process memory.

Microsoft PROCESS_MEMORY_COUNTERS_EX units: bytes. PrivateUsage is commit,
not resident/private working set. No VM_READ, memory read, UI, or elevation.
"""
from pathlib import Path
import ctypes,datetime,json,os,subprocess,time
from ctypes import wintypes

class Counters(ctypes.Structure):
    _fields_=[('cb',wintypes.DWORD),('PageFaultCount',wintypes.DWORD)]+[
        (name,ctypes.c_size_t) for name in ['PeakWorkingSetSize','WorkingSetSize','QuotaPeakPagedPoolUsage',
        'QuotaPagedPoolUsage','QuotaPeakNonPagedPoolUsage','QuotaNonPagedPoolUsage','PagefileUsage','PeakPagefileUsage','PrivateUsage']]

def run_sampled(args,env,cwd,prefix,*,sample_seconds=.02,stall_seconds=120,memory_limit=2*1024**3):
    if os.name!='nt':raise RuntimeError('Windows counter monitor required')
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    kernel.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD];kernel.OpenProcess.restype=wintypes.HANDLE
    kernel.CloseHandle.argtypes=[wintypes.HANDLE];kernel.CloseHandle.restype=wintypes.BOOL
    kernel.K32GetProcessMemoryInfo.argtypes=[wintypes.HANDLE,ctypes.POINTER(Counters),wintypes.DWORD]
    kernel.K32GetProcessMemoryInfo.restype=wintypes.BOOL
    prefix=Path(prefix);prefix.parent.mkdir(parents=True,exist_ok=True)
    stdout=prefix.with_suffix('.stdout.txt');stderr=prefix.with_suffix('.stderr.txt');samples=prefix.with_suffix('.memory.jsonl')
    start=time.perf_counter();start_unix_ns=time.time_ns();utc=datetime.datetime.now(datetime.timezone.utc).isoformat()
    rows=[];query_errors=[];stopped=None;handle=None;child=None;last_progress=start;last_size=0
    with stdout.open('xb') as sout,stderr.open('xb') as serr,samples.open('x',encoding='utf-8') as raw:
        try:
            child=subprocess.Popen([str(a) for a in args],cwd=cwd,env=env,stdout=sout,stderr=serr,creationflags=subprocess.CREATE_NO_WINDOW)
            handle=kernel.OpenProcess(0x1000,False,child.pid) # PROCESS_QUERY_LIMITED_INFORMATION, owned child only
            if not handle:raise ctypes.WinError(ctypes.get_last_error())
            while True:
                now=time.perf_counter();data=Counters();data.cb=ctypes.sizeof(Counters)
                if kernel.K32GetProcessMemoryInfo(handle,ctypes.byref(data),data.cb):
                    row={'seconds':now-start,'unix_ns':time.time_ns(),'working_set_bytes':data.WorkingSetSize,'os_peak_working_set_bytes':data.PeakWorkingSetSize,
                         'private_commit_bytes':data.PrivateUsage,'os_peak_commit_bytes':data.PeakPagefileUsage}
                    rows.append(row);raw.write(json.dumps(row)+'\n');raw.flush()
                    if data.WorkingSetSize>memory_limit:
                        stopped={'reason':'working_set_guard','observed_bytes':data.WorkingSetSize,'limit_bytes':memory_limit,'overshoot_bytes':data.WorkingSetSize-memory_limit};break
                else:
                    query_errors.append({'seconds':now-start,'error':ctypes.get_last_error(),'terminal':child.poll() is not None})
                if child.poll() is not None:break
                size=stdout.stat().st_size
                if size!=last_size:last_progress=now;last_size=size
                if now-last_progress>stall_seconds:
                    stopped={'reason':'no_stdout_progress_guard','seconds':now-last_progress,'limit_seconds':stall_seconds};break
                time.sleep(sample_seconds)
        except OSError as error:
            stopped={'reason':'monitor_error','error':str(error)}
        finally:
            if child is not None and child.poll() is None:
                child.terminate()
                try:child.wait(timeout=5)
                except subprocess.TimeoutExpired:child.kill();child.wait(timeout=5)
            if handle:kernel.CloseHandle(handle)
    gaps=[b['seconds']-a['seconds'] for a,b in zip(rows,rows[1:])]
    result={'schema':1,'pid':child.pid if child else None,'started_utc':utc,'start_unix_ns':start_unix_ns,'exit_code':child.returncode if child else None,
            'wall_seconds':time.perf_counter()-start,'sample_count':len(rows),'requested_sample_seconds':sample_seconds,
            'max_observed_sample_gap_seconds':max(gaps,default=None),'sampled_max_working_set_bytes':max((r['working_set_bytes'] for r in rows),default=0),
            'os_peak_working_set_bytes':max((r['os_peak_working_set_bytes'] for r in rows),default=0),
            'sampled_max_private_commit_bytes':max((r['private_commit_bytes'] for r in rows),default=0),
            'query_errors':query_errors,'stop':stopped,'memory_limit_bytes':memory_limit,'stall_limit_seconds':stall_seconds,
            'memory_scope':'owned child process working set/OS lifetime peak/private commit; excludes separate GPU memory; sampled values can miss peaks',
            'ok':bool(child is not None and child.returncode==0 and rows and stopped is None and not any(not e['terminal'] for e in query_errors))}
    prefix.with_suffix('.monitor.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
    return result
