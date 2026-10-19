import os,sys,time,unittest
from pathlib import Path
from measure_process import run_sampled

class Calibration(unittest.TestCase):
    def test_memory_guard_stops_only_the_owned_child_before_completion(self):
        root=Path(__file__).resolve().parents[2];out=root/'target/p03-i-evidence'/('guard-'+str(time.time_ns()))
        code="import time; a=bytearray(128*1024*1024); a[::4096]=b'x'*32768; print('allocated',flush=True); time.sleep(5); print('unexpected completion',flush=True)"
        result=run_sampled([sys.executable,'-c',code],dict(os.environ),root,out,memory_limit=64*1024*1024)
        self.assertEqual(result['stop']['reason'],'working_set_guard')
        self.assertNotEqual(result['exit_code'],0)
        self.assertNotIn('unexpected completion',out.with_suffix('.stdout.txt').read_text())
    def test_no_progress_guard_cleans_up_the_owned_child(self):
        root=Path(__file__).resolve().parents[2];out=root/'target/p03-i-evidence'/('stall-'+str(time.time_ns()))
        result=run_sampled([sys.executable,'-c',"import time; time.sleep(5)"],dict(os.environ),root,out,stall_seconds=.15)
        self.assertEqual(result['stop']['reason'],'no_stdout_progress_guard')
        self.assertNotEqual(result['exit_code'],0)
    def test_os_peak_captures_touched_allocation_in_owned_child(self):
        root=Path(__file__).resolve().parents[2]
        out=root/'target/p03-i-evidence'/('calibration-'+str(time.time_ns()))
        code="import time,gc; print('baseline',flush=True); time.sleep(.2); a=bytearray(128*1024*1024); a[::4096]=b'x'*32768; print('allocated',flush=True); time.sleep(.4); del a; gc.collect(); print('released',flush=True); time.sleep(.2)"
        result=run_sampled([sys.executable,'-c',code],dict(os.environ),root,out)
        self.assertGreaterEqual(result.get('os_peak_working_set_bytes',0),96*1024*1024)
        self.assertEqual(result['exit_code'],0)
        self.assertGreater(result['sample_count'],5)
        self.assertGreaterEqual(result['os_peak_working_set_bytes'],result['sampled_max_working_set_bytes'])
        self.assertIn('released',out.with_suffix('.stdout.txt').read_text())

if __name__=='__main__':unittest.main()
