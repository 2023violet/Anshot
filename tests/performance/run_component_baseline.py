"""Exact release component artifact, measured owned child and independent PNGs.

This is an initial component baseline, not GUI/GPU or full G04 acceptance.
"""
from pathlib import Path
import datetime,hashlib,json,os,shutil,subprocess,sys
from measure_process import run_sampled
ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/'tests/native_oracle'))
from png_oracle import check_files

def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def source_map():
    patterns=['poc/native-egui/src/*.rs','poc/native-egui/examples/**/*.rs','poc/native-egui/input-bridge/src/*.rs',
              'poc/native-egui/assets/**/*','crates/shareready-core/src/**/*.rs','crates/shareready-editor/src/**/*.rs',
              'crates/shareready-platform-windows/src/**/*.rs','tests/performance/*.py','tests/performance/fixtures/**/*','tests/native_oracle/*.py',
              'docs/research/fixtures/T0*','docs/research/fixtures/manifest.json','docs/research/fixtures/generate.py',
              'crates/*/tests/**/*.rs','e1-spike/src/**/*.rs','e1-spike/tests/*.rs','e1-spike/examples/*.rs',
              'e1-spike/Cargo.toml','e1-spike/Cargo.lock','tests/golden/*.py','tests/golden/*.json']
    paths={p for pattern in patterns for p in ROOT.glob(pattern) if p.is_file()}
    paths.update(ROOT/p for p in ['Cargo.toml','Cargo.lock','poc/native-egui/Cargo.toml','poc/native-egui/Cargo.lock',
                 'poc/native-egui/input-bridge/Cargo.toml','crates/shareready-core/Cargo.toml',
                 'crates/shareready-editor/Cargo.toml','crates/shareready-platform-windows/Cargo.toml'])
    return {p.relative_to(ROOT).as_posix():sha(p) for p in sorted(paths)}
def main():
    stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    out=ROOT/'target/p03-i-evidence'/('component-'+stamp);out.mkdir(parents=True,exist_ok=False)
    source=source_map();build=hashlib.sha256(json.dumps(source,sort_keys=True).encode()).hexdigest()
    env=dict(os.environ,SR_BUILD_ID=build,PYTHONUTF8='1',PYTHONIOENCODING='utf-8')
    cargo=Path(env['RUSTC']).with_name('cargo.exe')
    report={'schema':1,'scope':'RELEASE_COMPONENT_BASELINE_NO_WINDOW_NO_GPU','build_id':build,'source_files':source,
            'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'checks':[]}
    def run(label,args):
        p=subprocess.run([str(a) for a in args],cwd=ROOT,env=env,capture_output=True)
        (out/(label+'.stdout.txt')).write_bytes(p.stdout);(out/(label+'.stderr.txt')).write_bytes(p.stderr)
        report['checks'].append({'name':label,'exit_code':p.returncode,'stdout_sha256':hashlib.sha256(p.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr).hexdigest()})
        if p.returncode:raise RuntimeError(f'{label} exit {p.returncode}')
        return p.stdout.decode('utf-8',errors='replace')
    try:
        run('monitor-calibration',[sys.executable,'-m','unittest','discover','-s','tests/performance','-v'])
        report['rustc']=run('rustc',[env['RUSTC'],'--version']).strip()
        run('build',[cargo,'build','--release','--locked','--manifest-path','poc/native-egui/Cargo.toml','--target-dir','poc/native-egui/target','--example','performance_baseline'])
        exe=out/'performance_baseline.exe';shutil.copyfile(ROOT/'poc/native-egui/target/release/examples/performance_baseline.exe',exe)
        report['exe_sha256']=sha(exe)
        if source_map()!=source:raise RuntimeError('source changed during build; refusing execution')
        measured=run_sampled([exe,out/'files'],env,ROOT,out/'process')
        report['process']=measured
        if not measured['ok']:raise RuntimeError('component process or memory monitor failed')
        events=[json.loads(line) for line in (out/'process.stdout.txt').read_text(encoding='utf-8').splitlines()]
        if events[0]['build']!=build or events[0]['debug_assertions'] or events[-1].get('type')!='complete':raise RuntimeError('build/profile/completion identity mismatch')
        report['events']=events;report['independent_pixels']={}
        for name in ['grid','wide']:
            file=out/'files'/f'{name}-source.png'
            embedded=ROOT/'poc/native-egui/assets/performance'/f'{name}-24mp.png'
            if file.read_bytes()!=embedded.read_bytes():raise RuntimeError('copied source differs from embedded fixture')
            report['independent_pixels'][name]=check_files(file,out/'files'/f'{name}-review.png',out/'files'/f'{name}.regions.json')
        if source_map()!=source or sha(exe)!=report['exe_sha256']:raise RuntimeError('source/artifact changed during measurement')
        report['ok']=True
    except (OSError,ValueError,RuntimeError,AssertionError) as error:report.update(ok=False,error=str(error))
    (out/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'ok':report['ok'],'report':(out/'report.json').relative_to(ROOT).as_posix(),'build_id':build,'exe_sha256':report.get('exe_sha256'),'error':report.get('error')}))
    return 0 if report['ok'] else 1

if __name__=='__main__':raise SystemExit(main())
