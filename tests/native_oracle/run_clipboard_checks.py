"""Explicit isolated OS clipboard probe; never accesses interactive content."""
from pathlib import Path
import datetime,hashlib,json,os,shutil,subprocess,sys

ROOT=Path(__file__).resolve().parents[2]
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
    isolation_only=sys.argv[1:]==['--isolation-only']
    if not isolation_only:raise RuntimeError('Full clipboard mode is not implemented')
    stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    out=ROOT/'target/p03-g-evidence'/('clipboard-'+stamp);out.mkdir(parents=True,exist_ok=False)
    files=['poc/native-egui/examples/clipboard_probe.rs','poc/native-egui/Cargo.toml',
           'poc/native-egui/Cargo.lock','tests/native_oracle/run_clipboard_checks.py']
    source={p:digest(ROOT/p) for p in files}
    review_path=ROOT/'target/p03-g-evidence/PRE_EXECUTION_REVIEW.json'
    review=json.loads(review_path.read_text(encoding='utf-8'))
    assert review['approved_for_isolation_probe'] and review['reviewer_is_implementer'] is False
    for p,sha in source.items():assert review['source_files'][p]==sha,(p,'Pre-execution review is stale')
    report={'mode':'isolation-only','source_files':source,'pre_execution_review_sha256':digest(review_path),'checks':[]}
    env=dict(os.environ);cargo=Path(env['RUSTC']).with_name('cargo.exe')
    def run(label,args,timeout=300):
        p=subprocess.run([str(a) for a in args],cwd=ROOT,env=env,capture_output=True,timeout=timeout)
        (out/(label+'.stdout.txt')).write_bytes(p.stdout);(out/(label+'.stderr.txt')).write_bytes(p.stderr)
        report['checks'].append({'name':label,'exit_code':p.returncode,'stdout_sha256':hashlib.sha256(p.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr).hexdigest()})
        assert p.returncode==0,(label,p.returncode)
        return p.stdout.decode('utf-8',errors='replace')
    try:
        run('build',[cargo,'build','--manifest-path','poc/native-egui/Cargo.toml','--example','clipboard_probe','--locked',
                     '--target-dir',ROOT/'poc/native-egui/target'])
        exe=out/'clipboard_probe.exe';shutil.copyfile(ROOT/'poc/native-egui/target/debug/examples/clipboard_probe.exe',exe)
        report['exe_sha256']=digest(exe)
        assert source=={p:digest(ROOT/p) for p in files},'Source changed during build; refusing to execute'
        text=run('isolation',[exe,'--isolation-only'],30)
        assert 'noninteractive=true fresh_clipboard=true' in text and 'context_restored=true original_sequence_equal=true' in text
        assert source=={p:digest(ROOT/p) for p in files}
        assert digest(exe)==report['exe_sha256'],'Test executable changed'
        report['ok']=True
    except (OSError,ValueError,AssertionError,subprocess.TimeoutExpired) as error:
        report.update(ok=False,error=str(error))
    (out/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'ok':report['ok'],'report':(out/'report.json').relative_to(ROOT).as_posix()}))
    return 0 if report['ok'] else 1
if __name__=='__main__':sys.exit(main())
