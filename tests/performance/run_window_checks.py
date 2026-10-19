"""Real NativeApp in a locked, opt-in synthetic release diagnostic build."""
from pathlib import Path
import datetime,hashlib,json,math,os,shutil,subprocess,sys
from PIL import Image
from measure_process import run_sampled
from run_component_baseline import ROOT,source_map,sha
sys.path.insert(0,str(ROOT/'tests/native_oracle'))
from png_oracle import check_files

def native_pixels(source,screenshot,metric,ppp,regions,reviewed,require_cover=None):
    if require_cover is None:require_cover=reviewed
    if require_cover and not reviewed:raise AssertionError('a required review cannot be an edit view')
    if metric['zoom']==0:return {'scope':'Fit downsample preview, no native-pixel assertion'}
    rect=metric['rect'];clip=metric['clip'];scale=metric['zoom'];tested=0;bad=[];covered=0;uncovered=0
    with Image.open(source) as original,Image.open(screenshot) as image:
        original=original.convert('RGBA');image=image.convert('RGBA')
        l=max(math.ceil(max(rect[0],clip[0])*ppp)+3,0);r=min(math.floor(min(rect[2],clip[2])*ppp)-3,image.width)
        t=max(math.ceil(max(rect[1],clip[1])*ppp)+3,0);b=min(math.floor(min(rect[3],clip[3])*ppp)-3,image.height)
        original_pixels=original.load();image_pixels=image.load()
        for y in range(t,b):
            for x in range(l,r):
                sx=math.floor((x+.5-rect[0]*ppp)/scale);sy=math.floor((y+.5-rect[1]*ppp)/scale)
                if not (0<=sx<original.width and 0<=sy<original.height):continue
                inside=any(min(a,c)-12<=sx<max(a,c)+12 and min(d,f)-12<=sy<max(d,f)+12 for a,d,c,f in regions)
                if inside and not reviewed:continue # edit overlays/handles intentionally differ from source
                cover=reviewed and any(min(a,c)<=sx<max(a,c) and min(d,f)<=sy<max(d,f) for a,d,c,f in regions)
                want=(17,24,39,255) if cover else original_pixels[sx,sy]
                covered+=int(cover);uncovered+=int(not cover)
                got=image_pixels[x,y];tested+=1
                if got!=want and len(bad)<10:bad.append({'screen':[x,y],'source':[sx,sy],'want':want,'got':got})
    if tested<500 or bad or (reviewed and require_cover and (covered<500 or uncovered<500)):
        raise AssertionError({'native_pixel_samples':tested,'covered_samples':covered,'uncovered_samples':uncovered,'mismatches':bad})
    return {'scope':'every physical pixel in visible image interior, excluding 3px clip edge and edit mask/handle margin; includes 1px lines and tile seams','samples':tested,'covered_samples':covered,'uncovered_samples':uncovered,'mismatches':bad}

def main():
    pilot='--pilot' in sys.argv[1:]
    tile_pilot='--tile-pilot' in sys.argv[1:]
    if pilot and tile_pilot:raise ValueError('choose one pilot')
    stamp=datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
    out=ROOT/'target/p03-i-evidence'/('window-'+stamp);out.mkdir(parents=True,exist_ok=False)
    source=source_map();build=hashlib.sha256(json.dumps(source,sort_keys=True).encode()).hexdigest()
    env=dict(os.environ,SR_BUILD_ID=build,WGPU_BACKEND='dx12',PYTHONUTF8='1',PYTHONIOENCODING='utf-8')
    cargo=Path(env['RUSTC']).with_name('cargo.exe');report={'schema':1,'scope':'SYNTHETIC_NATIVEAPP_RELEASE_GPU_DIAGNOSTIC','pilot':pilot,'tile_pilot':tile_pilot,'build_id':build,'source_files':source,'cases':[]}
    try:
        p=subprocess.run([str(cargo),'build','--release','--locked','--manifest-path','poc/native-egui/Cargo.toml','--target-dir','poc/native-egui/target','--features','performance-evidence','--example','performance_window'],cwd=ROOT,env=env,capture_output=True)
        (out/'build.stdout.txt').write_bytes(p.stdout);(out/'build.stderr.txt').write_bytes(p.stderr)
        if p.returncode:raise RuntimeError('release diagnostic build failed')
        exe=out/'performance_window.exe';shutil.copyfile(ROOT/'poc/native-egui/target/release/examples/performance_window.exe',exe);report['exe_sha256']=sha(exe)
        if source_map()!=source:raise RuntimeError('source changed before launch')
        cases=[('pilot','grid','native',1)] if pilot else [(f'start-{n}','grid','native',1) for n in range(5)]+[('repeat-20','grid','native',20),('wide-native','wide','native',1),('wide-tile-512','wide','512',1)]
        if tile_pilot:cases=[('tile-pilot','wide','512',1)]
        for name,fixture,cap,rounds in cases:
            directory=out/name
            measured=run_sampled([exe,directory,fixture,cap,str(rounds)],env,ROOT,out/(name+'-process'))
            streams={kind:sha((out/(name+'-process')).with_suffix('.'+kind+'.txt')) for kind in ['stdout','stderr']}
            stdout=(out/(name+'-process')).with_suffix('.stdout.txt').read_text(encoding='utf-8')
            adapters=[line for line in stdout.splitlines() if line.startswith('SR_V2_03_GPU_ADAPTER')]
            case={'name':name,'fixture':fixture,'effective_cap':cap,'rounds':rounds,'process':measured,'stream_sha256':streams,'adapter_lines':adapters};report['cases'].append(case)
            if not measured['ok']:raise RuntimeError(f'{name} process failed')
            if len(adapters)!=1 or 'backend=Dx12' not in adapters[0]:raise RuntimeError('actual DX12 adapter identity absent')
            events=[json.loads(line) for line in (directory/'performance.jsonl').read_text(encoding='utf-8').splitlines()]
            case['events']=events
            if events[0]['build']!=build or events[0]['debug_assertions'] or events[-1]['type']!='complete':raise RuntimeError(f'{name} identity/completion mismatch')
            if len([e for e in events if e['type']=='reimport'])!=rounds:raise RuntimeError('repeat count mismatch')
            traces=[json.loads(line) for line in (directory/'events.jsonl').read_text(encoding='utf-8').splitlines()]
            shots=[e for e in traces if e['type']=='gpu_screenshot']
            if [s['state'] for s in shots]!=[f'stage-{i}' for i in range(5)]:raise RuntimeError('GPU screenshot sequence absent')
            file=directory/f'source-{fixture}-24mp.png'
            if file.read_bytes()!=(ROOT/'poc/native-egui/assets/performance'/f'{fixture}-24mp.png').read_bytes():raise RuntimeError('fixed source mismatch')
            case['screenshots']=[]
            for i,shot in enumerate(shots):
                frames=[e for e in events if e['type']=='frame' and e['stage']==i and e['stage_frame']==5 and not e['will_discard']]
                if len(frames)!=1:raise RuntimeError('screenshot request frame metric missing')
                frame=frames[0];path=directory/shot['png']
                if shot['review_view']!=(i>=3) or shot['reviewed']!=(i>=3):raise RuntimeError('screenshot edit/review state differs from required stage')
                if not isinstance(frame['display'],dict) or (i>0 and frame['display']['crop'] is None):raise RuntimeError('actual visible image metric absent')
                if i in (2,3) and (frame['display']['crop'][2:]!=frame['display']['source_size']):raise RuntimeError('right/bottom edge source pixels not reached')
                if frame['display']['zoom']!=[0,1,2,1,1][i]:raise RuntimeError('actual screenshot zoom differs from requested stage')
                device_cap=frame['display']['device_texture_cap'];expected_cap=min(512,device_cap) if cap=='512' else device_cap
                if frame['display']['effective_display_cap']!=expected_cap:raise RuntimeError('actual image cap differs from declared simulation')
                if i==4 and not(frame['display']['crop'][1]<=frame['display']['source_size'][1]//2<frame['display']['crop'][3]):raise RuntimeError('one-pixel horizontal fiducial not visible')
                verified=native_pixels(file,path,frame['display'],shot['pixels_per_point'],shot['regions'],shot['review_view'],require_cover=i==4)
                case['screenshots'].append({'shot':shot,'sha256':sha(path),'request_frame':frame,'pixel_check':verified})
            full=[]
            for output in sorted(directory.glob('round-*.png')):full.append({'png':output.name,'sha256':sha(output),'oracle':check_files(file,output,output.with_suffix('.regions.json'))})
            case['full_output_checks']=full
            interaction=[e for e in events if e['type']=='frame' and e['stage']==5 and not e['will_discard']]
            if len(interaction)!=30 or len({e['cumulative_frame_nr'] for e in interaction})!=30:raise RuntimeError('actual independent interaction frame count mismatch')
            case['interactive_sample_count']=len(interaction)
            if cap=='512' and not any(e.get('display',{}).get('tile_count',0)>1 for e in interaction):raise RuntimeError('forced-cap path never generated multiple tiles')
            if cap=='512' and not any(s['request_frame']['display']['tile_count']>1 and s['pixel_check'].get('samples',0)>500 for s in case['screenshots']):raise RuntimeError('no strictly checked GPU screenshot contains actual tile seams')
            if source_map()!=source or sha(exe)!=report['exe_sha256']:raise RuntimeError('source/artifact changed during execution')
        report['ok']=True
    except (OSError,ValueError,RuntimeError,AssertionError) as error:report.update(ok=False,error=str(error))
    (out/'report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'ok':report['ok'],'report':(out/'report.json').relative_to(ROOT).as_posix(),'build_id':build,'exe_sha256':report.get('exe_sha256'),'error':report.get('error')}))
    return 0 if report['ok'] else 1

if __name__=='__main__':raise SystemExit(main())
