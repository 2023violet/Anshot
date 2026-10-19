"""Fixed-fixture real worker/window evidence; no OS input/clipboard or publishing."""
from pathlib import Path
import datetime, hashlib, json, os, shutil, subprocess, sys
from PIL import Image
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tests/performance'))
from run_component_baseline import source_map, sha
from run_window_checks import native_pixels
from measure_process import run_sampled
from png_oracle import check_files, bounded_read


def main():
    if sys.argv[1:] not in ([], ['--pilot']):
        raise ValueError('Only optional --pilot (grid window only, not full acceptance)')
    pilot = bool(sys.argv[1:])
    stamp = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    out = ROOT / 'target/p03-j-evidence' / ('window-' + stamp)
    out.mkdir(parents=True, exist_ok=False)
    source = source_map()
    build = hashlib.sha256(json.dumps(source, sort_keys=True).encode()).hexdigest()
    env = dict(os.environ, SR_BUILD_ID=build, WGPU_BACKEND='dx12', PYTHONUTF8='1', PYTHONIOENCODING='utf-8')
    cargo = Path(env['RUSTC']).with_name('cargo.exe')
    report = dict(schema=1, pilot=pilot, scope='SYNTHETIC_REAL_WORKER_AND_GPU_NOT_PHYSICAL_OR_PRODUCT_ACCEPTANCE',
                  source_files=source, build_id=build, cases=[], commands=[])

    def run(label, args, timeout=900):
        p = subprocess.run([str(a) for a in args], cwd=ROOT, env=env, capture_output=True, timeout=timeout)
        (out / (label + '.stdout.txt')).write_bytes(p.stdout)
        (out / (label + '.stderr.txt')).write_bytes(p.stderr)
        report['commands'].append(dict(name=label, exit_code=p.returncode,
                                      stdout_sha256=sha(out / (label + '.stdout.txt')),
                                      stderr_sha256=sha(out / (label + '.stderr.txt'))))
        if p.returncode:
            raise RuntimeError(label + ' failed; inspect retained logs')
        return p.stdout.decode('utf-8', errors='replace')

    def unchanged():
        if source_map() != source:
            raise RuntimeError('source changed during this exact-artifact run')
        if subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() != '94f7f419aee91faf6f846349d40a897d0d72d7d0':
            raise RuntimeError('unexpected HEAD')

    try:
        unchanged()
        if subprocess.check_output(['git', 'branch', '--show-current'], cwd=ROOT, text=True).strip() != 'main':
            raise RuntimeError('main-only policy')
        report['rustc'] = run('rustc', [env['RUSTC'], '--version']).strip()
        run('build', [cargo, 'build', '--release', '--locked', '--manifest-path', 'poc/native-egui/Cargo.toml',
                      '--target-dir', 'poc/native-egui/target', '--features', 'performance-evidence', '--example', 'async_window'])
        exe = out / 'async_window.exe'
        shutil.copyfile(ROOT / 'poc/native-egui/target/release/examples/async_window.exe', exe)
        report['exe_sha256'] = sha(exe)
        for fixture in (['grid'] if pilot else ['grid', 'noise']):
            unchanged()
            files = out / fixture
            measured = run_sampled([exe, files, fixture], env, ROOT, out / (fixture + '-process'))
            case = dict(fixture=fixture, process=measured)
            report['cases'].append(case)
            if not measured['ok']:
                raise RuntimeError(fixture + ' process/monitor failed')
            stdout = (out / (fixture + '-process.stdout.txt')).read_text(encoding='utf-8')
            adapters = [line for line in stdout.splitlines() if line.startswith('SR_V2_03_GPU_ADAPTER')]
            case['adapters'] = adapters
            if len(adapters) != 1 or 'backend=Dx12' not in adapters[0]:
                raise RuntimeError('actual DX12 adapter absent')
            events = [json.loads(line) for line in (files / 'async.jsonl').read_text(encoding='utf-8').splitlines()]
            case['events'] = events
            if events[0]['build'] != build or events[0]['debug_assertions'] or events[-1]['type'] != 'complete':
                raise RuntimeError('async build/profile/completion absent: ' + str(events[-1]))
            tasks = {}
            for event in events:
                if event['type'] == 'review_event':
                    key = tuple(event[p] for p in ('document', 'revision', 'operation'))
                    tasks.setdefault(key, []).append(event)
            if len(tasks) != 3:
                raise RuntimeError('expected first / cancelled / retry real workers')
            case['tasks'] = []
            for index, (key, rows) in enumerate(tasks.items()):
                phases = {e['name']: e for e in rows if e['name'] != 'draw'}
                required = ['trigger', 'trigger_return', 'worker_start', 'encoded', 'cpu_prepared', 'send_invoked', 'received', 'finished_observed']
                if any(name not in phases for name in required):
                    raise RuntimeError('missing actual worker/controller event')
                ordered = [phases[n]['ms'] for n in ['trigger', 'worker_start', 'encoded', 'cpu_prepared', 'send_invoked', 'received']]
                if ordered != sorted(ordered) or phases['finished_observed']['ms'] < phases['send_invoked']['ms']:
                    raise RuntimeError('worker phase chronology mismatch')
                busy = [e for e in events if e['type'] == 'frame' and e['fresh'] and not e['discard']
                        and phases['worker_start']['ms'] <= e['callback_begin_ms'] <= e['end_ms'] <= phases['cpu_prepared']['ms']]
                frames = {e['frame'] for e in busy}
                if len(frames) != len(busy) or (index != 1 and len(frames) < 30):
                    raise RuntimeError(f'real worker independent busy frame coverage insufficient: {len(frames)}')
                if index == 1:
                    if not {'cancel', 'discarded_stale'} <= phases.keys() or 'promoted' in phases:
                        raise RuntimeError('cancelled result was not discarded without promotion')
                    if not phases['worker_start']['ms'] <= phases['cancel']['ms'] <= phases['cancel_return']['ms'] <= phases['cpu_prepared']['ms']:
                        raise RuntimeError('cancel was outside actual CPU work or did not return while work continued')
                else:
                    drawn = [e for e in rows if e['name'] == 'draw']
                    if 'promoted' not in phases or not drawn or max(e['frame'] for e in drawn) >= phases['promoted']['frame']:
                        raise RuntimeError('promotion lacks preceding completed drawing frame')
                case['tasks'].append(dict(key=key, events=rows, busy_distinct_frames=len(frames), busy_frames=busy,
                                          trigger_return_ms=phases['trigger_return']['ms'] - phases['trigger']['ms']))
            traces = [json.loads(line) for line in (files / 'events.jsonl').read_text(encoding='utf-8').splitlines()]
            shots = [e for e in traces if e['type'] == 'gpu_screenshot']
            if [s['state'] for s in shots] != ['gpu-busy', 'gpu-first', 'gpu-retry']:
                raise RuntimeError('busy/current/retry GPU images absent')
            fixture_file = ROOT / ('poc/native-egui/assets/performance/grid-24mp.png' if fixture == 'grid' else 'tests/performance/fixtures/synthetic-noise-9mp.jpg')
            raw_source = files / ('source.png' if fixture == 'grid' else 'source.jpg')
            if raw_source.read_bytes() != fixture_file.read_bytes():
                raise RuntimeError('fixed synthetic source changed')
            case['outputs'] = []
            normalized = files / 'normalized-source.png'
            for name in ['first', 'retry']:
                png = files / (name + '.png')
                oracle = check_files(raw_source if fixture == 'grid' else normalized, png,
                                     files / (name + '.regions.json'), normalized_source=fixture == 'noise')
                copied = bounded_read(files / (name + '.rgba'), 24_000_000 * 4)
                with Image.open(png) as image:
                    if copied != image.convert('RGBA').tobytes():
                        raise RuntimeError('controlled Copy RGBA differs from entire current PNG')
                case['outputs'].append(dict(name=name, png_sha256=sha(png), oracle=oracle,
                                            copy_rgba_sha256=hashlib.sha256(copied).hexdigest()))
            if case['outputs'][0]['copy_rgba_sha256'] == case['outputs'][1]['copy_rgba_sha256']:
                raise RuntimeError('post-cancel real mask movement did not change output pixels')
            if fixture == 'noise' and not raw_source.stat().st_size < 25 * 1024**2 < (files / 'first.png').stat().st_size <= 128 * 1024**2:
                raise RuntimeError('large accepted source/generated output counterexample lost')
            case['screenshots'] = []
            for shot in shots:
                name = shot['state'].removeprefix('gpu-')
                requests = [e for e in events if e['type'] == 'screenshot_request' and e['name'] == name]
                if len(requests) != 1:
                    raise RuntimeError('screenshot request identity is not unique')
                req = requests[0]
                key = tuple(req[p] for p in ('document', 'revision', 'operation'))
                if key not in tasks:
                    raise RuntimeError('GPU request is not tied to an observed current worker')
                worker = {e['name']: e for e in tasks[key] if e['name'] != 'draw'}
                if name == 'busy' and not worker['worker_start']['ms'] <= req['ms'] <= worker['cpu_prepared']['ms']:
                    raise RuntimeError('busy GPU request was outside actual CPU work interval')
                if name != 'busy' and ('promoted' not in worker or req['ms'] < worker['promoted']['ms']):
                    raise RuntimeError('current PNG GPU request preceded draw promotion')
                if shot['reviewed'] != (name != 'busy') or req['reviewed'] != shot['reviewed']:
                    raise RuntimeError('GPU busy/review state mismatch')
                frames = [e for e in events if e['type'] == 'frame' and e['frame'] == req['frame'] and not e['discard']]
                if len(frames) != 1:
                    raise RuntimeError('screenshot final-pass frame metric absent')
                pixel = native_pixels(normalized, files / shot['png'], frames[0]['display'], shot['pixels_per_point'], shot['regions'], shot['reviewed'])
                case['screenshots'].append(dict(shot=shot, request=req, frame=frames[0], png_sha256=sha(files / shot['png']), pixels=pixel))
            case['files'] = {p.name: dict(bytes=p.stat().st_size, sha256=sha(p)) for p in sorted(files.iterdir()) if p.is_file()}
            unchanged()
            if sha(exe) != report['exe_sha256']:
                raise RuntimeError('owned executable changed')
        if not pilot:
            for label, script in [('ordinary-and-unit', 'tests/performance/run_fix_checks.py'),
                                  ('large-controller', 'tests/performance/run_large_artifact_checks.py'),
                                  ('window-regression', 'tests/performance/run_window_checks.py')]:
                text = run(label, [sys.executable, script])
                summary = json.loads(text.strip().splitlines()[-1])
                report[label] = summary
                if not summary.get('ok'):
                    raise RuntimeError(label + ' report not accepted')
                if label == 'ordinary-and-unit':
                    checked = json.loads((ROOT / summary['report']).read_text(encoding='utf-8'))
                    counts = {c['name']: sum(c.get('rust_pass_counts', [])) for c in checked['checks']}
                    for name, minimum in [('root-tests', 66), ('native-tests', 68), ('native-feature-tests', 74)]:
                        if counts.get(name, 0) < minimum:
                            raise RuntimeError(name + ' did not run the required current acceptance tests')
                    for name in ['native-tests', 'native-feature-tests']:
                        raw = (ROOT / summary['report']).parent.joinpath(name + '.stdout.txt').read_text(encoding='utf-8')
                        for required in ['final_result_sent_after_empty_receive_is_not_lost_when_worker_exits',
                                         'async_confirmed_close_revokes_running_and_awaiting_results_before_late_frames',
                                         'async_confirmed_close_is_terminal_across_repeated_viewport_close_events',
                                         'async_discarded_pass_does_not_authorize_an_undrawn_final_pass',
                                         'async_waiting_frame_blocks_delivery_across_all_passes']:
                            if required + ' ... ok' not in raw:
                                raise RuntimeError('required Native boundary test absent: ' + required)
                    report['current_rust_pass_counts'] = counts
            run('v1-build', [cargo, 'build', '--release', '--locked', '--manifest-path', 'e1-spike/Cargo.toml', '--target-dir', 'target', '--bins', '--examples'])
            report['v1_executables'] = {str(p.relative_to(ROOT)): sha(p) for p in [ROOT / 'target/release/shareready-e1.exe', ROOT / 'target/release/examples/v1_baseline_preview.exe']}
            run('five-goldens', [sys.executable, 'tests/golden/verify_v1_baseline.py', '--report', out / 'goldens.json'])
            if any(sha(ROOT / p) != value for p, value in report['v1_executables'].items()):
                raise RuntimeError('V1 executable changed during frozen golden validation')
        unchanged()
        report['limitations'] = ['synthetic diagnostic app GPU readback; no human, physical DPI, Narrator or OS clipboard acceptance',
                                 'UI draw/promotion is egui submission evidence, not proof a human saw the pixels',
                                 'worker busy frame count uses captured real worker_start through cpu_prepared; excludes later waiting/readback',
                                 'ordinary source import, texture upload/visible tile preparation and OS dialogs can still block UI',
                                 'UI CPU times can include explicit synthetic trace I/O at promotion; GPU time is not measured',
                                 '9MP JPEG uses Rust-normalized source and independent pixel composition, not independent JPEG decoder validation',
                                 'independent Native review is still required; machine success is not product completion']
        report['ok'] = True
    except (OSError, ValueError, RuntimeError, AssertionError, KeyError, subprocess.TimeoutExpired) as error:
        report.update(ok=False, error=str(error))
    (out / 'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(dict(ok=report['ok'], report=str((out / 'report.json').relative_to(ROOT)), build_id=build, error=report.get('error'))))
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
