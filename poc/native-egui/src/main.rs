//! ShareReady native GUI experimental app — never bypass Rust Editor/ Core output rules.
#![forbid(unsafe_code)]
mod app;
mod display;
mod trace;
mod input;
mod fonts;
mod visual;
use shareready_core::{redact, Region};
use shareready_editor::{Editor, OperationKind};

const SAMPLE: &[u8] = include_bytes!("../../../docs/research/fixtures/T04_EDGES.png");

fn self_test(output:Option<&std::path::Path>) -> Result<(),String>{
    let mut ed=Editor::from_source(SAMPLE.to_vec()).map_err(|e|format!("{e:?}"))?;
    if ed.dimensions()!=(2400,1600){return Err("unexpected synthetic fixture size".into());}
    let rect=Region::new(15,12,950,90);
    ed.add_mask(rect).map_err(|e|format!("{e:?}"))?;
    let rev=ed.stamp();
    let ticket=ed.begin(OperationKind::Review).map_err(|e|format!("{e:?}"))?;
    let generated=ed.bytes_for(ticket).ok_or("missing prepared PNG")?.to_vec();
    let expected=redact(SAMPLE,&[rect]).map_err(|e|e.to_string())?;
    if generated!=expected{return Err("Rust editor diverged from Rust Core".into());}
    if let Some(dir)=output {
        use std::io::Write;
        std::fs::create_dir_all(dir).map_err(|e|e.to_string())?;
        for (name,bytes) in [("source-T04.png",SAMPLE),("regions.json",b"[[15,12,950,90]]".as_slice())] {
            let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(dir.join(name)).map_err(|e|e.to_string())?;
            file.write_all(bytes).and_then(|_|file.sync_all()).map_err(|e|e.to_string())?;
        }
    }
    let decoded=shareready_core::decode_generated_png(&generated,(2400,1600)).map_err(|e|e.to_string())?;
    if decoded.dimensions()!=(2400,1600) {return Err("readback size mismatch".into());}
    if !ed.finish(ticket,true){return Err("review ticket not accepted".into());}
    if ed.output_png()!=Some(expected.as_slice()){return Err("review artifact missing".into());}
    if let Some(dir)=output {
        let save=ed.begin(OperationKind::Save).map_err(|e|format!("{e:?}"))?;
        let png=ed.bytes_for(save).ok_or("missing save bytes")?;
        let result=shareready_platform_windows::save_new(&dir.join("output.png"),png);
        let accepted=ed.finish(save,result.is_ok());
        result.map_err(|e|e.to_string())?;
        if !accepted{return Err("save ticket not accepted".into());}
        println!("SR_V2_03_SAVE_NEW_HANDLE_BOUND_PASS fixture=T04");
    }
    ed.undo().map_err(|e|format!("{e:?}"))?;
    if ed.stamp().revision<=rev.revision||ed.output_png().is_some(){
        return Err("undo restored stale artifact".into());
    }
    println!("SR_V2_03_NATIVE_POC_SELFTEST_PASS fixture=T04 document_version_guard=PASS same_core_consistency=PASS independent_file_oracle=NOT_RUN");
    Ok(())
}

fn main()->eframe::Result {
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args.iter().any(|a|a=="--self-test"){
        let out=args.iter().position(|a|a=="--self-test-output").map(|i|{
            args.get(i+1).map(std::path::Path::new).unwrap_or_else(||{eprintln!("--self-test-output requires a directory");std::process::exit(2)})
        });
        match self_test(out){
            Ok(())=>return Ok(()),
            Err(e)=>{eprintln!("SR_V2_03_SELFTEST_FAIL {e}");std::process::exit(2);}
        }
    }
    let smoke=args.iter().any(|a|a=="--smoke-window");
    let visual=args.iter().any(|a|a=="--visual-evidence");
    if visual && (smoke || args.iter().any(|a|a=="--synthetic-trace") || args.iter().filter(|a|*a=="--visual-evidence").count()!=1){
        eprintln!("--visual-evidence cannot be duplicated or combined with smoke/trace");std::process::exit(2);
    }
    let trace_flag=if visual{"--visual-evidence"}else{"--synthetic-trace"};
    let trace=match args.iter().position(|a|a==trace_flag) {
        Some(i)=>{
            let Some(dir)=args.get(i+1).filter(|s|!s.starts_with("--")) else{eprintln!("{trace_flag} requires a new local evidence directory");std::process::exit(2)};
            match trace::Trace::create(std::path::Path::new(dir),SAMPLE){
                Ok(t)=>Some(t),Err(e)=>{eprintln!("Synthetic trace unavailable: {e}");std::process::exit(2)}
            }
        },None=>None,
    };
    let selected=match args.iter().position(|a|a=="--renderer") {
        Some(i)=>match args.get(i+1).map(String::as_str){
            Some("glow")=>eframe::Renderer::Glow,
            Some("wgpu")=>eframe::Renderer::Wgpu,
            _=>{eprintln!("Usage: --renderer glow|wgpu");std::process::exit(2)}
        },
        None=>eframe::Renderer::Wgpu,
    };
    // WGPU_BACKEND=dx12 selects Direct3D 12 for the wgpu probe. It is not
    // assumed to exist in a Windows CI VM; failure is reported, never waived.
    let renderer_name=match selected {
        eframe::Renderer::Glow=>"glow",
        eframe::Renderer::Wgpu=>"wgpu",
    };
    println!("SR_V2_03_RENDERER_SELECTED={renderer_name} WGPU_BACKEND={}",
        std::env::var("WGPU_BACKEND").unwrap_or_else(|_|"auto".into()));
    let paste_bridge=shareready_native_input_bridge::PasteBridge::default();
    let hook_bridge=paste_bridge.clone();
    let native=eframe::NativeOptions{
        renderer:selected,
        dithering:false,
        event_loop_builder:Some(Box::new(move |builder|hook_bridge.install(builder))),
        viewport:eframe::egui::ViewportBuilder::default().with_inner_size([1120.0,760.0])
            .with_min_inner_size([700.0,480.0]).with_title("ShareReady · 图片遮挡（开发样机）"),
        ..Default::default()
    };
    eframe::run_native("ShareReady Rust Native POC",native,
        Box::new(move |cc| Ok(Box::new(app::NativeApp::new(cc,smoke,trace,paste_bridge,visual)))))
}
