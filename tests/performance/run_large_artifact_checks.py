"""Fixed synthetic release Native controller and full generated-PNG regression.

No window/GPU/OS clipboard. JPEG truth is anchored to Rust-normalized pixels;
independent Python composes the expected mask and compares all output/Copy bytes.
"""
from pathlib import Path
import datetime,hashlib,json,os,shutil,subprocess,sys
from PIL import Image
from measure_process import run_sampled
from run_component_baseline import ROOT,source_map,sha
sys.path.insert(0,str(ROOT/'tests/native_oracle'))
from png_oracle import check_files,bounded_read

def main():
    stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    out=ROOT/'target/p03-i-evidence'/('large-'+stamp);out.mkdir(parents=True,exist_ok=False)
    source=source_map();build=hashlib.sha256(json.dumps(source,sort_keys=True).encode()).hexdigest()
    env=dict(os.environ,SR_BUILD_ID=build,PYTHONUTF8='1',PYTHONIOENCODING='utf-8')
    cargo=Path(env['RUSTC']).with_name('cargo.exe')
    report={'schema':1,'scope':'SYNTHETIC_RELEASE_NATIVE_CONTROLLER_NO_WINDOW_NO_GPU_NO_OS_CLIPBOARD',
            'build_id':build,'source_files':source,'head':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()}
    try:
        p=subprocess.run([str(cargo),'build','--release','--locked','--manifest-path','poc/native-egui/Cargo.toml','--target-dir','poc/native-egui/target',
                          '--features','performance-evidence','--example','generated_artifact_check'],cwd=ROOT,env=env,capture_output=True)
        (out/'build.stdout.txt').write_bytes(p.stdout);(out/'build.stderr.txt').write_bytes(p.stderr)
        if p.returncode:raise RuntimeError('large artifact diagnostic build failed')
        exe=out/'generated_artifact_check.exe';shutil.copyfile(ROOT/'poc/native-egui/target/release/examples/generated_artifact_check.exe',exe)
        report['exe_sha256']=sha(exe)
        if source_map()!=source:raise RuntimeError('source changed before release child launch')
        report['process']=run_sampled([exe,out/'files'],env,ROOT,out/'process')
        if not report['process']['ok']:raise RuntimeError('large artifact owned process/monitor failed')
        lines=(out/'process.stdout.txt').read_text(encoding='utf-8').splitlines()
        result=json.loads(lines[-1]);report['native_result']=result
        if result['type']!='large_artifact_complete' or result['build']!=build or result['debug_assertions'] or result['os_clipboard'] or result['window']:
            raise RuntimeError('scope/build/profile/completion mismatch')
        if result['source_intake_cap']!=25*1024**2 or result['generated_cap']!=128*1024**2 or result['dimensions']!=[3000,3000]:
            raise RuntimeError('input/artifact contract changed')
        files=out/'files';fixture=ROOT/'tests/performance/fixtures/synthetic-noise-9mp.jpg'
        if (files/'source.jpg').read_bytes()!=fixture.read_bytes():raise RuntimeError('non-fixed synthetic source')
        if not (files/'source.jpg').stat().st_size<25*1024**2<(files/'output.png').stat().st_size<=128*1024**2:
            raise RuntimeError('required accepted-source/large-artifact counterexample absent')
        try:check_files(files/'normalized-source.png',files/'output.png',files/'regions.json')
        except ValueError as error:
            if str(error)!='file exceeds oracle resource limit':raise
            report['default_oracle_source_cap_preserved']=True
        else:raise RuntimeError('default source oracle unexpectedly accepts oversized normalized PNG')
        report['full_png_oracle']=check_files(files/'normalized-source.png',files/'output.png',files/'regions.json',normalized_source=True)
        copied=bounded_read(files/'controlled-copy.rgba',24_000_000*4)
        with Image.open(files/'output.png') as image:
            if len(copied)!=3000*3000*4 or copied!=image.convert('RGBA').tobytes():raise RuntimeError('controlled Copy RGBA differs from actual current PNG')
        report['full_copy_rgba_sha256']=hashlib.sha256(copied).hexdigest()
        if report['full_copy_rgba_sha256']!=report['full_png_oracle']['rgba_sha256']:raise RuntimeError('Copy/full PNG oracle identity differs')
        report['files']={p.name:{'bytes':p.stat().st_size,'sha256':sha(p)} for p in sorted(files.iterdir()) if p.is_file()}
        report['limitations']=['headless actual Native controller and CPU Display preparation, not a GPU/human/OS clipboard test',
            'mask composition and PNG/Copy bytes independently checked against current Rust-normalized JPEG pixels; not independent JPEG decoder validation',
            '9MP synthetic high-entropy counterexample, not all legal24MP/high-entropy input coverage']
        if source_map()!=source or sha(exe)!=report['exe_sha256']:raise RuntimeError('source/artifact changed during checks')
        report['ok']=True
    except (OSError,ValueError,RuntimeError,AssertionError) as error:report.update(ok=False,error=str(error))
    (out/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'ok':report['ok'],'report':(out/'report.json').relative_to(ROOT).as_posix(),'build_id':build,'error':report.get('error')}))
    return 0 if report['ok'] else 1

if __name__=='__main__':raise SystemExit(main())
