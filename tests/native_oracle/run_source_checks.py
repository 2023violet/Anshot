"""Fixed synthetic source-worker/window evidence, never physical acceptance."""
from pathlib import Path
import datetime, hashlib, json, math, os, shutil, subprocess, sys
from PIL import Image
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tests/performance'))
from run_component_baseline import source_map, sha
from run_fix_checks import PROTECTED, EXPECTED_HEAD
from run_window_checks import native_pixels
from measure_process import run_sampled
from png_oracle import check_files, bounded_read


def source_jobs(events):
    tasks = {}
    def finite(value):
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
            raise ValueError('nonfinite or negative captured time')
        return value
    for row in events:
        if row['type'] == 'source_event':
            request = row['request']
            if type(request) is not int or request <= 0:
                raise ValueError('invalid diagnostic request identity')
            finite(row['ms']); tasks.setdefault(request, []).append(row)
    if len(tasks) != 3:
        raise ValueError('expected first, cancelled and retry source tasks')
    result = []
    for index, (request, rows) in enumerate(sorted(tasks.items(), key=lambda item: min(r['ms'] for r in item[1]))):
        phases = {r['name']: r for r in rows}
        required = ['trigger', 'trigger_return', 'worker_start', 'read_complete', 'decoded', 'cpu_prepared',
                    'send_invoked', 'received', 'finished_observed']
        if len(phases) != len(rows) or any(n not in phases for n in required):
            raise ValueError('duplicate or missing actual source phase')
        times = [phases[n]['ms'] for n in ['trigger', 'worker_start', 'read_complete', 'decoded', 'cpu_prepared', 'send_invoked', 'received']]
        if times != sorted(times) or phases['trigger_return']['ms'] < times[0] or phases['finished_observed']['ms'] < phases['send_invoked']['ms']:
            raise ValueError('source chronology mismatch')
        settled = max(phases[n]['ms'] for n in ['received', 'finished_observed'])
        if index == 1:
            if not {'cancel', 'cancel_return', 'discarded_stale'} <= phases.keys() or {'install_return', 'accepted_observed'} & phases.keys():
                raise ValueError('cancelled source must be discarded without installation or acceptance')
            if not phases['read_complete']['ms'] <= phases['cancel']['ms'] <= phases['cancel_return']['ms'] <= phases['cpu_prepared']['ms'] or phases['discarded_stale']['ms'] < settled:
                raise ValueError('cancel did not return during actual CPU work')
        elif not {'install_return', 'accepted_observed'} <= phases.keys() or not settled <= phases['install_return']['ms'] <= phases['accepted_observed']['ms']:
            raise ValueError('valid source lacks settled installation and later observed acceptance')
        busy = []
        for row in events:
            if row['type'] != 'frame':
                continue
            for name in ['callback_begin_ms', 'begin_ms', 'end_ms', 'callback_end_ms', 'ui_cpu_ms', 'controller_cpu_ms', 'callback_to_app_return_cpu_ms']:
                finite(row[name])
            if not row['callback_begin_ms'] <= row['begin_ms'] <= row['end_ms'] <= row['callback_end_ms']:
                raise ValueError('captured callback chronology mismatch')
            if row['fresh'] and not row['discard'] and row['source_busy'] and row['source_request'] == request and phases['read_complete']['ms'] <= row['callback_begin_ms'] <= row['callback_end_ms'] <= phases['cpu_prepared']['ms']:
                busy.append(row)
        if len({r['frame'] for r in busy}) != len(busy) or (index != 1 and len(busy) < 2):
            raise ValueError('actual CPU interval lacks two distinct complete instrumented callbacks')
        result.append(dict(request=request, events=rows, busy_distinct_frames=len(busy), busy_frames=busy,
                           trigger_return_ms=phases['trigger_return']['ms']-phases['trigger']['ms'],
                           maximum_busy_ui_cpu_ms=max((r['ui_cpu_ms'] for r in busy), default=None),
                           maximum_busy_callback_span_ms=max((r['callback_end_ms']-r['callback_begin_ms'] for r in busy), default=None)))
    return result


def source_shot(req, shot, job):
    if req['request'] != job['request']:
        raise RuntimeError('GPU request not bound to actual source key')
    if any(req[field] != shot[field] for field in ['document', 'revision', 'reviewed', 'review_view']):
        raise RuntimeError('GPU receipt differs from requested document or view')
    phases = {e['name']: e for e in job['events']}
    if req['name'] == 'busy':
        if not req['source_busy'] or not phases['read_complete']['ms'] <= req['ms'] <= phases['cpu_prepared']['ms'] or req['review_view']:
            raise RuntimeError('busy GPU request outside actual source CPU or old editing view')
    elif req['source_busy'] or not req['reviewed'] or not req['review_view'] or req['ms'] < phases['accepted_observed']['ms'] or req['document'] != phases['accepted_observed']['document']:
        raise RuntimeError('review GPU request not current accepted document')


def main():
    if sys.argv[1:] not in ([], ['--pilot']):
        raise ValueError('Only optional --pilot: grid only, not full acceptance')
    pilot = bool(sys.argv[1:])
    out = ROOT / 'target/p03-k-evidence' / ('window-' + datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ'))
    out.mkdir(parents=True, exist_ok=False)
    source = source_map(); build = hashlib.sha256(json.dumps(source, sort_keys=True).encode()).hexdigest()
    env = dict(os.environ, SR_BUILD_ID=build, WGPU_BACKEND='dx12', PYTHONUTF8='1', PYTHONIOENCODING='utf-8')
    cargo = Path(env['RUSTC']).with_name('cargo.exe')
    report = dict(schema=1, pilot=pilot, scope='SYNTHETIC_SOURCE_WORKER_AND_GPU_NOT_PHYSICAL_OR_PRODUCT_ACCEPTANCE',
                  source_files=source, build_id=build, cases=[], commands=[])

    def run(label, args, timeout=1800):
        p = subprocess.run([str(a) for a in args], cwd=ROOT, env=env, capture_output=True, timeout=timeout)
        for name, data in [('stdout', p.stdout), ('stderr', p.stderr)]:
            (out / f'{label}.{name}.txt').write_bytes(data)
        report['commands'].append(dict(name=label, exit_code=p.returncode, stdout_sha256=sha(out / f'{label}.stdout.txt'), stderr_sha256=sha(out / f'{label}.stderr.txt')))
        if p.returncode:
            raise RuntimeError(label + ' failed; retained raw logs')
        return p.stdout.decode('utf-8', errors='replace')

    def unchanged():
        if source_map() != source or subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() != EXPECTED_HEAD:
            raise RuntimeError('source or HEAD changed during exact-artifact execution')
        for path, digest in PROTECTED.items():
            if sha(ROOT / path) != digest:
                raise RuntimeError('protected artifact changed: ' + path)

    try:
        unchanged()
        if run('branch', ['git', 'branch', '--show-current']).strip() != 'main':
            raise RuntimeError('main-only policy')
        run('protected-v1', ['git', 'diff', '--exit-code', 'HEAD', '--', 'e1-spike/src', 'web-app', 'desktop-windows', 'docs/research/fixtures', 'tests/golden/v1_3_baseline.json'])
        report['protected_sha256'] = PROTECTED
        report['rustc'] = run('rustc', [env['RUSTC'], '--version']).strip()
        run('source-method-tests', [sys.executable, '-m', 'unittest', 'discover', '-s', 'tests/performance', '-p', 'test_source_method.py', '-v'])
        run('build', [cargo, 'build', '--release', '--locked', '--manifest-path', 'poc/native-egui/Cargo.toml', '--target-dir', 'poc/native-egui/target', '--features', 'performance-evidence', '--example', 'source_window'])
        exe = out / 'source_window.exe'; shutil.copyfile(ROOT / 'poc/native-egui/target/release/examples/source_window.exe', exe)
        report['exe_sha256'] = sha(exe)
        for fixture in (['grid'] if pilot else ['grid', 'noise']):
            unchanged(); files = out / fixture
            measured = run_sampled([exe, files, fixture], env, ROOT, out / (fixture + '-process'))
            case = dict(fixture=fixture, process=measured); report['cases'].append(case)
            if not measured['ok']:
                raise RuntimeError(fixture + ' owned process/monitor failed')
            stdout = (out / (fixture + '-process.stdout.txt')).read_text(encoding='utf-8')
            case['adapters'] = [line for line in stdout.splitlines() if line.startswith('SR_V2_03_GPU_ADAPTER')]
            if len(case['adapters']) != 1 or 'backend=Dx12' not in case['adapters'][0]:
                raise RuntimeError('actual DX12 adapter absent')
            events = [json.loads(line) for line in (files / 'source.jsonl').read_text(encoding='utf-8').splitlines()]
            case['events'] = events
            if events[0]['build'] != build or events[0]['debug_assertions'] or events[-1]['type'] != 'complete':
                raise RuntimeError('source build/profile/completion absent: ' + str(events[-1]))
            case['tasks'] = source_jobs(events)
            traces = [json.loads(line) for line in (files / 'events.jsonl').read_text(encoding='utf-8').splitlines()]
            shots = [e for e in traces if e['type'] == 'gpu_screenshot']
            if [s['state'] for s in shots] != ['gpu-busy', 'gpu-first', 'gpu-retry']:
                raise RuntimeError('busy/current/retry GPU evidence absent')
            raw = files / ('source.png' if fixture == 'grid' else 'source.jpg')
            embedded = ROOT / ('poc/native-egui/assets/performance/grid-24mp.png' if fixture == 'grid' else 'tests/performance/fixtures/synthetic-noise-9mp.jpg')
            if raw.read_bytes() != embedded.read_bytes() or (files / 'old-source.png').read_bytes() != (ROOT / 'docs/research/fixtures/T04_EDGES.png').read_bytes():
                raise RuntimeError('fixed synthetic source changed')
            normalized = files / 'normalized-source.png'; case['outputs'] = []
            for name in ['old', 'first', 'retry']:
                png = files / (name + '.png'); source_png = files / 'old-source.png' if name == 'old' else (raw if fixture == 'grid' else normalized)
                oracle = check_files(source_png, png, files / (name + '.regions.json'), normalized_source=fixture == 'noise' and name != 'old')
                copied = bounded_read(files / (name + '.rgba'), 24_000_000 * 4)
                with Image.open(png) as image:
                    if copied != image.convert('RGBA').tobytes():
                        raise RuntimeError('entire controlled Copy RGBA differs from current PNG')
                case['outputs'].append(dict(name=name, png_sha256=sha(png), oracle=oracle, copy_rgba_sha256=hashlib.sha256(copied).hexdigest()))
            if case['outputs'][1]['copy_rgba_sha256'] == case['outputs'][2]['copy_rgba_sha256']:
                raise RuntimeError('retry mask movement did not change pixels')
            if fixture == 'noise' and not raw.stat().st_size < 25 * 1024**2 < (files / 'first.png').stat().st_size <= 128 * 1024**2:
                raise RuntimeError('large accepted-source/generated-PNG boundary lost')
            case['screenshots'] = []
            for shot in shots:
                name = shot['state'].removeprefix('gpu-')
                requests = [e for e in events if e['type'] == 'screenshot_request' and e['name'] == name]
                if len(requests) != 1:
                    raise RuntimeError('GPU request identity not unique')
                req = requests[0]; job = case['tasks'][0 if name in ['busy', 'first'] else 2]
                source_shot(req, shot, job)
                receipts = [e for e in events if e['type'] == 'screenshot_received' and e['name'] == name]
                if len(receipts) != 1 or receipts[0]['request'] != req['request'] or receipts[0]['ms'] < req['ms'] or receipts[0]['frame'] < req['frame']:
                    raise RuntimeError('GPU receipt not bound to the requested source key and later callback')
                frames = [e for e in events if e['type'] == 'frame' and e['frame'] == req['frame'] and not e['discard']]
                if len(frames) != 1:
                    raise RuntimeError('GPU request final-pass metric absent')
                pixel = native_pixels(files / 'old-source.png' if name == 'busy' else normalized, files / shot['png'], frames[0]['display'], shot['pixels_per_point'], shot['regions'], shot['review_view'])
                case['screenshots'].append(dict(shot=shot, request=req, frame=frames[0], png_sha256=sha(files / shot['png']), pixels=pixel))
            case['files'] = {p.name: dict(bytes=p.stat().st_size, sha256=sha(p)) for p in sorted(files.iterdir()) if p.is_file()}
            unchanged()
            if sha(exe) != report['exe_sha256']:
                raise RuntimeError('owned source executable changed')
        if not pilot:
            text = run('j-full-regression', [sys.executable, 'tests/native_oracle/run_async_checks.py'])
            summary = json.loads(text.strip().splitlines()[-1]); report['j_full_regression'] = summary
            if not summary.get('ok'):
                raise RuntimeError('current J/full regression failed')
            checked = json.loads((ROOT / summary['report']).read_text(encoding='utf-8'))
            counts = checked['current_rust_pass_counts']; report['current_rust_pass_counts'] = counts
            if counts.get('root-tests', 0) < 72 or counts.get('native-tests', 0) < 86 or counts.get('native-feature-tests', 0) < 93:
                raise RuntimeError('current complete Rust suites absent')
        unchanged()
        report['limitations'] = ['fixed synthetic ordinary worker and GPU readback; no human, physical DPI, Narrator, or OS clipboard acceptance',
            'callback window runs from entry to final timing sample before buffered frame-row formatting/return; not physical latency',
            'all captured frames retained; CPU coverage starts at read_complete and excludes I/O, discarded/repeated frames and callbacks outside decode/display preparation',
            'OS dialogs and bounded OS clipboard read, GPU upload/visible tile work can still block UI; no full G04 budget acceptance',
            'cancel revokes ownership, CPU continues and slot remains until actual thread exit',
            '9MP JPEG uses Rust-normalized source and independent composition; not an independent JPEG decoder validation',
            'pilot excludes noise and all regression commands; independent complete review and product acceptance remain open']
        report['ok'] = True
    except (OSError, ValueError, RuntimeError, AssertionError, KeyError, subprocess.TimeoutExpired) as error:
        report.update(ok=False, error=str(error))
    (out / 'report.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(dict(ok=report['ok'], pilot=pilot, report=str((out / 'report.json').relative_to(ROOT)), build_id=build, error=report.get('error'))))
    return 0 if report['ok'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
