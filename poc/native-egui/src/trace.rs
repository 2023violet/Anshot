//! Explicit synthetic-only evidence. No logging is created in the normal app.
use std::{fs::{File,OpenOptions},io::{self,Write},path::{Path,PathBuf}};
use eframe::egui::{Event,Pos2,Rect};
use shareready_editor::{Editor,Ticket};

pub struct Trace {file:File,dir:PathBuf,last:String,exports:u64,fixture:&'static[u8],label:&'static str,
    #[cfg(feature="performance-evidence")] frame_logging_paused:bool}
fn new_file(path:&Path,bytes:&[u8])->io::Result<()> {
    let mut f=OpenOptions::new().write(true).create_new(true).open(path)?;
    f.write_all(bytes)?;f.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn non_fixture_is_rejected_before_creating_any_evidence(){
        let dir=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/trace-tests").join(format!("reject-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        assert!(Trace::create(&dir,b"not the frozen synthetic fixture").is_err());
        assert!(!dir.exists());
    }
    #[cfg(feature="performance-evidence")]
    #[test] fn performance_trace_accepts_only_the_two_embedded_24mp_sources(){
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/trace-tests").join(format!("perf-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        for (name,source) in [("grid",include_bytes!("../assets/performance/grid-24mp.png").as_slice()),("wide",include_bytes!("../assets/performance/wide-24mp.png").as_slice())]{
            assert!(Trace::create_performance(&root.join(name),source).is_ok(),"known embedded synthetic fixture must be supported");
        }
        assert!(Trace::create_performance(&root.join("private"),b"not a fixed performance source").is_err());
        assert!(!root.join("private").exists());
        assert!(Trace::create_performance(&root.join("other-valid-png"),include_bytes!("../../../docs/research/fixtures/T04_EDGES.png")).is_err());
        assert!(!root.join("other-valid-png").exists());
    }
    #[cfg(feature="performance-evidence")]
    #[test] fn async_trace_accepts_only_its_two_fixed_sources(){
        let root=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/trace-tests").join(format!("async-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        for (name,source) in [("grid",include_bytes!("../assets/performance/grid-24mp.png").as_slice()),("noise",include_bytes!("../../../tests/performance/fixtures/synthetic-noise-9mp.jpg").as_slice())]{
            assert!(Trace::create_async(&root.join(name),source).is_ok());
        }
        assert!(Trace::create_async(&root.join("other"),include_bytes!("../../../docs/research/fixtures/T04_EDGES.png")).is_err());
        assert!(!root.join("other").exists());
    }
}
pub fn regions(editor:&Editor)->String {
    let masks=editor.masks().iter().map(|m|format!("[{},{},{},{}]",m.region.x0,m.region.y0,m.region.x1,m.region.y1)).collect::<Vec<_>>().join(",");
    format!("[{masks}]")
}
impl Trace {
    #[cfg(feature="performance-evidence")]
    pub fn pause_frame_logging(&mut self){self.frame_logging_paused=true;}
    #[cfg(feature="performance-evidence")]
    pub fn create_performance(dir:&Path,fixture:&[u8])->io::Result<Self>{
        const GRID:&[u8]=include_bytes!("../assets/performance/grid-24mp.png");
        const WIDE:&[u8]=include_bytes!("../assets/performance/wide-24mp.png");
        if fixture==GRID{Self::create_fixed(dir,GRID,"PERF_GRID_24MP","grid-24mp.png","source-grid-24mp.png")}
        else if fixture==WIDE{Self::create_fixed(dir,WIDE,"PERF_WIDE_24MP","wide-24mp.png","source-wide-24mp.png")}
        else{Err(io::Error::new(io::ErrorKind::InvalidInput,"performance trace accepts only the two embedded 24MP synthetic fixtures"))}
    }
    #[cfg(feature="performance-evidence")]
    pub fn create_async(dir:&Path,fixture:&[u8])->io::Result<Self>{
        const GRID:&[u8]=include_bytes!("../assets/performance/grid-24mp.png");
        const NOISE:&[u8]=include_bytes!("../../../tests/performance/fixtures/synthetic-noise-9mp.jpg");
        if fixture==GRID{Self::create_fixed(dir,GRID,"ASYNC_GRID_24MP","grid-24mp.png","source.png")}
        else if fixture==NOISE{Self::create_fixed(dir,NOISE,"ASYNC_NOISE_9MP","synthetic-noise-9mp.jpg","source.jpg")}
        else{Err(io::Error::new(io::ErrorKind::InvalidInput,"async trace accepts only embedded grid24MP/noise9MP"))}
    }
    pub fn fixture(&self)->&'static[u8]{self.fixture}
    pub fn label(&self)->&'static str{self.label}
    pub fn screenshot(&mut self,name:&str,image:&eframe::egui::ColorImage,editor:&Editor,review_view:bool,ppp:f32)->io::Result<()> {
        let (w,h)=(image.size[0],image.size[1]);
        if w==0 || h==0 || w as u64*h as u64>shareready_core::MAX_PIXELS || image.pixels.len()!=w*h {
            return Err(io::Error::new(io::ErrorKind::InvalidData,"invalid GPU readback size"));
        }
        let mut dib=vec![0u8;124];
        for (at,value) in [(0,124u32),(4,w as u32),(8,(-(h as i32)) as u32),(16,3),
            (40,0x00ff0000),(44,0x0000ff00),(48,0x000000ff),(52,0xff000000),(56,0x73524742)]{
            dib[at..at+4].copy_from_slice(&value.to_le_bytes());
        }
        dib[12]=1;dib[14]=32;
        for pixel in &image.pixels{let [r,g,b,a]=pixel.to_srgba_unmultiplied();dib.extend([b,g,r,a]);}
        let png=shareready_core::dib_to_png(&dib).map_err(io::Error::other)?;
        new_file(&self.dir.join(format!("{name}.png")),&png)?;
        writeln!(self.file,"{{\"type\":\"gpu_screenshot\",\"state\":\"{name}\",\"png\":\"{name}.png\",\"size\":[{w},{h}],\"document\":{},\"revision\":{},\"regions\":{},\"review_view\":{review_view},\"reviewed\":{},\"pixels_per_point\":{ppp}}}",editor.stamp().document_id,editor.stamp().revision,regions(editor),editor.is_reviewed())?;
        self.file.flush()
    }
    pub fn create(dir:&Path,fixture:&[u8])->io::Result<Self>{
        if fixture!=include_bytes!("../../../docs/research/fixtures/T04_EDGES.png"){
            return Err(io::Error::new(io::ErrorKind::InvalidInput,"trace accepts only the bundled T04 synthetic fixture"));
        }
        Self::create_fixed(dir,include_bytes!("../../../docs/research/fixtures/T04_EDGES.png"),"T04_EDGES","T04_EDGES.png","source-T04.png")
    }
    fn create_fixed(dir:&Path,fixture:&'static[u8],name:&str,label:&'static str,source_name:&str)->io::Result<Self>{
        std::fs::create_dir_all(dir)?;
        let file=OpenOptions::new().write(true).create_new(true).open(dir.join("events.jsonl"))?;
        new_file(&dir.join(source_name),fixture)?;
        let mut trace=Self {file,dir:dir.into(),last:String::new(),exports:0,fixture,label,
            #[cfg(feature="performance-evidence")] frame_logging_paused:false};
        writeln!(trace.file,"{{\"type\":\"synthetic_trace\",\"fixture\":\"{name}\",\"build\":\"{}\",\"private_import\":false}}",option_env!("SR_BUILD_ID").unwrap_or("unidentified-local-build"))?;
        Ok(trace)
    }
    pub fn frame(&mut self,editor:&Editor,image:Rect,ppp:f32,pointer:Option<Pos2>,preview:Option<shareready_core::Region>,time:f64,events:&[Event])->io::Result<()> {
        #[cfg(feature="performance-evidence")]
        if self.frame_logging_paused{return Ok(());}
        let point=pointer.map(|p|format!("[{},{}]",p.x,p.y)).unwrap_or("null".into());
        let source=pointer.map(|p|format!("[{},{}]",(p.x-image.min.x)/image.width()*editor.dimensions().0 as f32,
            (p.y-image.min.y)/image.height()*editor.dimensions().1 as f32)).unwrap_or("null".into());
        let preview=preview.map(|r|format!("[{},{},{},{}]",r.x0,r.y0,r.x1,r.y1)).unwrap_or("null".into());
        let input=events.iter().filter_map(|event|match event {
            Event::PointerMoved(p)=>Some(format!("{{\"kind\":\"move\",\"point\":[{},{}]}}",p.x,p.y)),
            Event::PointerButton{pos,button,pressed,..}=>Some(format!("{{\"kind\":\"button\",\"point\":[{},{}],\"button\":\"{:?}\",\"pressed\":{}}}",pos.x,pos.y,button,pressed)),
            Event::PointerGone=>Some("{\"kind\":\"gone\"}".into()),_=>None,
        }).collect::<Vec<_>>().join(",");
        let row=format!("\"type\":\"frame\",\"document\":{},\"revision\":{},\"dimensions\":[{},{}],\"image_rect_points\":[{},{},{},{}],\"pixels_per_point\":{},\"pointer_points\":{},\"pointer_source\":{},\"pointer_events\":[{}],\"preview\":{},\"regions\":{},\"reviewed\":{}",
            editor.stamp().document_id,editor.stamp().revision,editor.dimensions().0,editor.dimensions().1,image.min.x,image.min.y,image.max.x,image.max.y,ppp,point,source,input,preview,regions(editor),editor.is_reviewed());
        if row!=self.last || !input.is_empty(){writeln!(self.file,"{{\"time_seconds\":{time},{row}}}")?;self.file.flush()?;self.last=row;}
        Ok(())
    }
    pub fn review(&mut self,editor:&Editor,png:&[u8],ticket:Ticket)->io::Result<()> {
        self.exports+=1;
        let base=format!("review-{}-r{}",self.exports,editor.stamp().revision);
        new_file(&self.dir.join(format!("{base}.png")),png)?;
        new_file(&self.dir.join(format!("{base}.regions.json")),regions(editor).as_bytes())?;
        writeln!(self.file,"{{\"type\":\"review\",\"document\":{},\"revision\":{},\"operation\":{},\"png\":\"{base}.png\",\"regions\":{}}}",ticket.stamp.document_id,ticket.stamp.revision,ticket.operation_id,regions(editor))?;
        self.file.flush()
    }
}
