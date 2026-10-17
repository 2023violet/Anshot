//! ShareReady native GUI experimental app — never bypass Rust Editor/ Core output rules.
#![forbid(unsafe_code)]
mod app;
use shareready_core::{decode_source, redact, Region};
use shareready_editor::{Editor, OperationKind};

const SAMPLE: &[u8] = include_bytes!("../../../docs/research/fixtures/T04_EDGES.png");

fn self_test() -> Result<(),String>{
    let mut ed=Editor::from_source(SAMPLE.to_vec()).map_err(|e|format!("{e:?}"))?;
    if ed.dimensions()!=(2400,1600){return Err("unexpected synthetic fixture size".into());}
    let rect=Region::new(15,12,950,90);
    ed.add_mask(rect).map_err(|e|format!("{e:?}"))?;
    let rev=ed.stamp();
    let ticket=ed.begin(OperationKind::Review).map_err(|e|format!("{e:?}"))?;
    let generated=ed.bytes_for(ticket).ok_or("missing prepared PNG")?.to_vec();
    let expected=redact(SAMPLE,&[rect]).map_err(|e|e.to_string())?;
    if generated!=expected{return Err("Rust editor diverged from Rust Core".into());}
    let decoded=decode_source(&generated).map_err(|e|e.to_string())?;
    if decoded.dimensions()!=(2400,1600) {return Err("readback size mismatch".into());}
    if !ed.finish(ticket,true){return Err("review ticket not accepted".into());}
    if ed.output_png()!=Some(expected.as_slice()){return Err("review artifact missing".into());}
    ed.undo().map_err(|e|format!("{e:?}"))?;
    if ed.stamp().revision<=rev.revision||ed.output_png().is_some(){
        return Err("undo restored stale artifact".into());
    }
    println!("SR_V2_03_NATIVE_POC_SELFTEST_PASS fixture=T04 document_version_guard=PASS pixel_oracle=PASS");
    Ok(())
}

fn main()->eframe::Result {
    let args:Vec<String>=std::env::args().skip(1).collect();
    if args.iter().any(|a|a=="--self-test"){
        match self_test(){
            Ok(())=>return Ok(()),
            Err(e)=>{eprintln!("SR_V2_03_SELFTEST_FAIL {e}");std::process::exit(2);}
        }
    }
    let smoke=args.iter().any(|a|a=="--smoke-window");
    let native=eframe::NativeOptions{
        viewport:eframe::egui::ViewportBuilder::default().with_inner_size([1120.0,760.0])
            .with_min_inner_size([700.0,480.0]).with_title("ShareReady · Rust Native POC"),
        ..Default::default()
    };
    eframe::run_native("ShareReady Rust Native POC",native,
        Box::new(move |cc| Ok(Box::new(app::NativeApp::new(cc,smoke)))))
}
