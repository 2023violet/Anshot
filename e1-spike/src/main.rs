use shareready_e1::{redact,Region};
use std::{env,fs,process};

fn run() -> Result<(),String> {
    let args:Vec<String>=env::args().skip(1).collect();
    if args.len()<3 {
        return Err("usage: shareready-e1 INPUT_PNG_OR_JPEG OUTPUT_PNG x0,y0,x1,y1 [...]".into());
    }
    let source=fs::read(&args[0]).map_err(|e|format!("cannot read source: {e}"))?;
    let mut regions=Vec::new();
    for a in &args[2..] {
        let fields=a.split(',').map(|s|s.parse::<i64>()).collect::<Result<Vec<_>,_>>()
            .map_err(|e|format!("invalid region coordinates: {e}"))?;
        if fields.len()!=4 { return Err("region must have exactly 4 numbers".into()); }
        regions.push(Region::new(fields[0],fields[1],fields[2],fields[3]));
    }
    let output=redact(&source,&regions)?;
    fs::write(&args[1], &output).map_err(|e|format!("cannot write output: {e}"))?;
    println!("E1_SYNTHETIC_EXPORT_OK bytes={} masks={}",output.len(),regions.len());
    Ok(())
}
fn main() {
    if let Err(error)=run() {
        eprintln!("E1_ERROR {error}");
        process::exit(2)
    }
}
