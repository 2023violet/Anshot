"""Exact-source unit, boundary and ordinary-window regression for P03-I fixes."""
from pathlib import Path
import datetime,hashlib,json,os,re,subprocess,sys
from run_component_baseline import ROOT,source_map,sha

EXPECTED_HEAD='94f7f419aee91faf6f846349d40a897d0d72d7d0'
PROTECTED={
    'Cargo.lock':'574836ca69c075b9844928fcc320d89ce2742001ae4f0596b67558b6ff1f5266',
    'poc/native-egui/Cargo.lock':'b62d2d0993d34cc363317faa89d0cc56fcdd3610f3ffef57b16ae3e910a35038',
    'docs/V2/PRODUCT_MODIFICATION_PLAN_2026-10-17.md':'3f4d601a35f528f7ee41c82270888fcbde3c4be8570929a44a22037755433456',
    'docs/V2/deliverables/ShareReady_V2_Product_Modification_Plan_2026-10-17.pdf':'471b0f2948214e1da83a4f86142433a2811e20d38bbd6f35ff9394d6ed912d70'}

def main():
    out=ROOT/'target/p03-i-evidence'/('fix-'+datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ'))
    out.mkdir(parents=True,exist_ok=False);source=source_map()
    report={'schema':1,'source_files':source,'checks':[],'scope':'CODE_AND_ORDINARY_SYNTHETIC_WINDOW_REGRESSION_NOT_FULL_PRODUCT_ACCEPTANCE'}
    env=dict(os.environ,PYTHONUTF8='1',PYTHONIOENCODING='utf-8');cargo=Path(env['RUSTC']).with_name('cargo.exe')
    def run(label,args,minimum=None):
        p=subprocess.run([str(a) for a in args],cwd=ROOT,env=env,capture_output=True,timeout=900)
        (out/(label+'.stdout.txt')).write_bytes(p.stdout);(out/(label+'.stderr.txt')).write_bytes(p.stderr)
        row={'name':label,'exit_code':p.returncode,'stdout_sha256':hashlib.sha256(p.stdout).hexdigest(),'stderr_sha256':hashlib.sha256(p.stderr).hexdigest()}
        report['checks'].append(row)
        if p.returncode:raise RuntimeError(label+' failed; inspect raw logs')
        if minimum is not None:
            row['rust_pass_counts']=[int(n) for n in re.findall(rb'test result: ok\. (\d+) passed;',p.stdout)]
            if sum(row['rust_pass_counts'])<minimum:raise RuntimeError(label+' ran fewer required tests')
        return p.stdout.decode('utf-8')
    try:
        if run('head',['git','rev-parse','HEAD']).strip()!=EXPECTED_HEAD:raise RuntimeError('unexpected HEAD')
        if run('branch',['git','branch','--show-current']).strip()!='main':raise RuntimeError('main-only policy')
        for path,digest in PROTECTED.items():
            if sha(ROOT/path)!=digest:raise RuntimeError('protected artifact changed: '+path)
        run('protected-v1-fixture-diff',['git','diff','--exit-code','HEAD','--','e1-spike/src','web-app','desktop-windows','docs/research/fixtures','tests/golden/v1_3_baseline.json'])
        report['protected_sha256']=PROTECTED
        report['rustc']=run('rustc',[env['RUSTC'],'--version']).strip()
        run('root-tests',[cargo,'test','--workspace','--locked'],58)
        run('native-tests',[cargo,'test','--workspace','--locked','--manifest-path','poc/native-egui/Cargo.toml'],52)
        run('native-feature-tests',[cargo,'test','--workspace','--locked','--manifest-path','poc/native-egui/Cargo.toml','--features','performance-evidence'],57)
        run('performance-oracle-tests',[sys.executable,'-m','unittest','discover','-s','tests/performance','-v'])
        text=run('ordinary-window-oracle',[sys.executable,'tests/native_oracle/run_native_checks.py'])
        summary=json.loads(text.strip().splitlines()[-1]);report['ordinary_window_summary']=summary
        if not summary.get('ok'):raise RuntimeError('ordinary native window/oracle report failed')
        if source_map()!=source:raise RuntimeError('source changed during regression')
        if subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()!=EXPECTED_HEAD:raise RuntimeError('HEAD changed')
        report['ok']=True
    except (OSError,ValueError,RuntimeError,AssertionError,subprocess.TimeoutExpired) as error:report.update(ok=False,error=str(error))
    (out/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'ok':report['ok'],'report':(out/'report.json').relative_to(ROOT).as_posix(),'error':report.get('error')}))
    return 0 if report['ok'] else 1

if __name__=='__main__':raise SystemExit(main())
