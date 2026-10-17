//! All edit/export decisions are delegated to shareready-editor/core.
use eframe::egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, Vec2};
use shareready_core::{decode_source, Region};
use shareready_editor::{Editor, GestureKind, GestureTicket, Handle, OperationKind, Point, Viewport};
use std::borrow::Cow;
use std::path::Path;

enum DragMode { Draw, Existing(GestureTicket) }
struct Drag { start:Pos2, mode:DragMode }
pub struct NativeApp {
    editor:Editor,
    texture:Option<TextureHandle>,
    review_texture:Option<TextureHandle>,
    source_label:String,
    zoom:f32,
    drag:Option<Drag>,
    preview:Option<Region>,
    status:String,
    review_view:bool,
    smoke:bool,
    frames:usize,
}

const DEMO: &[u8]=include_bytes!("../../../docs/research/fixtures/T04_EDGES.png");

fn display_texture(ctx:&egui::Context,bytes:&[u8],label:&str)->Result<TextureHandle,String>{
    let img=decode_source(bytes)?;
    let (w,h)=(img.width() as usize,img.height() as usize);
    // Do not upload 24 MP as one GPU texture: a bounded downsampled DISPLAY image
    // is separate from the full-resolution trusted Core export.
    let ratio=(w.max(h) as f64/2048.0).max(1.0);
    let sw=(w as f64/ratio).round().max(1.0) as usize;
    let sh=(h as f64/ratio).round().max(1.0) as usize;
    let mut pixels=Vec::with_capacity(sw*sh*4);
    for y in 0..sh {
        for x in 0..sw {
            let sx=((x as f64*ratio).floor() as u32).min(img.width()-1);
            let sy=((y as f64*ratio).floor() as u32).min(img.height()-1);
            pixels.extend_from_slice(&img.get_pixel(sx,sy).0);
        }
    }
    let color=egui::ColorImage::from_rgba_unmultiplied([sw,sh],&pixels);
    Ok(ctx.load_texture(label,color,egui::TextureOptions::LINEAR))
}

impl NativeApp {
    pub fn new(cc:&eframe::CreationContext<'_>,smoke:bool)->Self {
        let editor=Editor::from_source(DEMO.to_vec()).expect("bundled synthetic PNG must decode");
        let texture=display_texture(&cc.egui_ctx,DEMO,"synthetic-source").ok();
        Self {
            editor,texture,review_texture:None,
            source_label:"Synthetic T04_EDGES.png · no private images".into(),
            zoom:0.36,drag:None,preview:None,
            status:"Rust native POC: import an image or draw a rectangle.".into(),
            review_view:false,smoke,frames:0,
        }
    }
    fn load_source(&mut self,ctx:&egui::Context,path:&Path){
        match std::fs::read(path) {
            Ok(bytes)=>{
                // Prepare decoder and GPU display first. A failure never erases a good edit.
                match (Editor::from_source(bytes.clone()),display_texture(ctx,&bytes,"imported-source")) {
                    (Ok(_),Ok(texture))=>{
                        match self.editor.replace_source(bytes){
                            Ok(())=>{
                                self.texture=Some(texture);
                                self.review_texture=None;self.review_view=false;
                                self.drag=None;self.preview=None;
                                self.source_label=path.file_name()
                                    .map(|p|p.to_string_lossy().into_owned())
                                    .unwrap_or_else(||"Imported image".into());
                                self.zoom=0.5;
                                self.status="Loaded locally. Original bytes never uploaded.".into();
                            }
                            Err(e)=>self.status=format!("Image rejected: {e:?}"),
                        }
                    }
                    (Err(e),_)=>self.status=format!("Image rejected: {e:?}"),
                    (_,Err(e))=>self.status=format!("Preview rejected: {e}"),
                }
            }
            Err(e)=>self.status=format!("File read failed: {e}"),
        }
    }
    fn mark_edit(&mut self){
        self.review_view=false;
        self.review_texture=None;
    }
    fn start_review(&mut self,ctx:&egui::Context){
        let ticket=match self.editor.begin(OperationKind::Review){
            Ok(t)=>t,
            Err(e)=>{self.status=format!("Cannot review: {e:?}");return;}
        };
        let bytes=match self.editor.bytes_for(ticket) {
            Some(p)=>p.to_vec(),
            None=>{self.editor.finish(ticket,false);self.status="Missing encoded PNG".into();return;}
        };
        match display_texture(ctx,&bytes,"encoded-png-review") {
            Ok(texture)=>{
                if self.editor.finish(ticket,true) {
                    self.review_texture=Some(texture);
                    self.review_view=true;
                    self.status="Rust encoded PNG shown. Inspect all unmasked details before copying.".into();
                }else{self.status="Review superseded by an edit.".into();}
            }
            Err(e)=>{self.editor.finish(ticket,false);self.status=format!("PNG readback failed: {e}");}
        }
    }
    fn save(&mut self){
        let ticket=match self.editor.begin(OperationKind::Save){
            Ok(t)=>t,Err(e)=>{self.status=format!("Save unavailable: {e:?}");return;}
        };
        let Some(bytes)=self.editor.bytes_for(ticket).map(ToOwned::to_owned) else{
            self.editor.finish(ticket,false);self.status="Stale save".into();return;
        };
        let picked=rfd::FileDialog::new().add_filter("PNG",&["png"])
            .set_file_name("share_ready.png").save_file();
        let Some(path)=picked else{
            self.editor.finish(ticket,false);self.status="Save cancelled (nothing written).".into();return;
        };
        // Deliberate POC policy: no overwrite; never silently destroy an existing file.
        let result=std::fs::OpenOptions::new().write(true).create_new(true).open(&path)
            .and_then(|mut file|{use std::io::Write;file.write_all(&bytes)?;file.sync_all()});
        let ok=result.is_ok() && self.editor.finish(ticket,true);
        if !ok {self.editor.finish(ticket,false);}
        self.status=if ok {
            "PNG saved as a NEW file. Verify output in an external viewer.".into()
        }else {
            format!("Save failed (existing files are never overwritten): {}",result.err().map(|e|e.to_string()).unwrap_or_else(||"stale result".into()))
        };
    }
    fn copy(&mut self){
        let ticket=match self.editor.begin(OperationKind::Copy){
            Ok(t)=>t,Err(e)=>{self.status=format!("Copy unavailable: {e:?}");return;}
        };
        let result=(||->Result<(),String>{
            let png=self.editor.bytes_for(ticket).ok_or("Stale copy ticket")?;
            let pixels=decode_source(png)?;
            let data=arboard::ImageData{
                width:pixels.width() as usize,height:pixels.height() as usize,
                bytes:Cow::Owned(pixels.into_raw()),
            };
            let mut clipboard=arboard::Clipboard::new().map_err(|e|format!("{e}"))?;
            clipboard.set_image(data).map_err(|e|format!("{e}"))
        })();
        let accepted=self.editor.finish(ticket,result.is_ok());
        self.status=match(result,accepted) {
            (Ok(()),true)=>"OS clipboard accepted image. Cross-app readback is NOT verified.".into(),
            (Err(e),_)=>format!("Copy failed: {e}"),
            _=>"Old copy result rejected.".into(),
        };
    }
    fn frame_view(w:u32,h:u32,rect:Rect)->Option<Viewport>{
        Viewport::new(w,h,f64::from(rect.min.x),f64::from(rect.min.y),
            f64::from(rect.width()),f64::from(rect.height())).ok()
    }
    fn to_screen(rect:Region,view:Viewport)->Option<Rect>{
        let (l,t,r,b)=rect.clipped(view.image_width,view.image_height)?;
        let a=view.source_to_screen(Point{x:f64::from(l),y:f64::from(t)}).ok()?;
        let z=view.source_to_screen(Point{x:f64::from(r),y:f64::from(b)}).ok()?;
        Some(Rect::from_min_max(Pos2::new(a.x as f32,a.y as f32),
                                Pos2::new(z.x as f32,z.y as f32)))
    }
    fn hit_mask(&self,p:Pos2,view:Viewport)->Option<(u64,GestureKind)>{
        for mask in self.editor.masks().iter().rev(){
            let Some(rect)=Self::to_screen(mask.region,view) else{continue};
            let around=rect.expand(9.0);
            if !around.contains(p){continue;}
            let near_l=(p.x-rect.left()).abs()<=9.0;
            let near_r=(p.x-rect.right()).abs()<=9.0;
            let near_t=(p.y-rect.top()).abs()<=9.0;
            let near_b=(p.y-rect.bottom()).abs()<=9.0;
            let handle=match(near_t,near_b,near_l,near_r){
                (true,_,true,_)=>Some(Handle::Nw),
                (true,_,_,true)=>Some(Handle::Ne),
                (_,true,true,_)=>Some(Handle::Sw),
                (_,true,_,true)=>Some(Handle::Se),
                (true,_,_,_)=>Some(Handle::N),
                (_,true,_,_)=>Some(Handle::S),
                (_,_,true,_)=>Some(Handle::W),
                (_,_,_,true)=>Some(Handle::E),
                _=>None,
            };
            if let Some(handle)=handle{return Some((mask.id,GestureKind::Resize(handle)));}
            if rect.contains(p){return Some((mask.id,GestureKind::Move));}
        }
        None
    }
    fn update_pointer(&mut self,response:&egui::Response,view:Viewport){
        let hovered=response.interact_pointer_pos();
        if response.drag_started(){
            if let Some(p)=hovered {
                self.review_view=false;
                self.review_texture=None;
                if let Some((id,kind))=self.hit_mask(p,view){
                    if let Ok(ticket)=self.editor.begin_gesture(id,kind){
                        self.drag=Some(Drag{start:p,mode:DragMode::Existing(ticket)});
                    }
                }else{
                    self.drag=Some(Drag{start:p,mode:DragMode::Draw});
                }
            }
        }
        if let Some(drag)=self.drag.as_ref(){
            if let Some(p)=hovered{
                match drag.mode {
                    DragMode::Draw=>{
                        self.preview=view.pixel_region(
                            Point{x:f64::from(drag.start.x),y:f64::from(drag.start.y)},
                            Point{x:f64::from(p.x),y:f64::from(p.y)}
                        ).ok();
                    }
                    DragMode::Existing(ticket)=>{
                        let dx=((p.x-drag.start.x)/response.rect.width()
                            *view.image_width as f32).round() as i64;
                        let dy=((p.y-drag.start.y)/response.rect.height()
                            *view.image_height as f32).round() as i64;
                        self.preview=self.editor.preview_gesture(ticket,dx,dy).ok();
                    }
                }
            }
        }
        if response.drag_stopped(){
            if let Some(drag)=self.drag.take(){
                let result=match drag.mode {
                    DragMode::Draw=>self.preview.and_then(|region|self.editor.add_mask(region).ok().map(|_|true)),
                    DragMode::Existing(t)=>self.editor.commit_gesture(t).ok(),
                };
                if result==Some(true){self.mark_edit();self.status="Mask updated by Rust Editor.".into();}
                self.preview=None;
            }
        }
        if response.clicked(){
            if let Some(p)=hovered{
                let selection=self.hit_mask(p,view).map(|(id,_)|id);
                let _=self.editor.select(selection);
            }
        }
    }
    fn render_canvas(&mut self,ui:&mut egui::Ui){
        let Some(texture)=self.texture.as_ref() else{ui.label("No image loaded");return};
        let (w,h)=self.editor.dimensions();
        let shown=self.review_view && self.editor.is_reviewed();
        let tex=if shown{self.review_texture.as_ref().unwrap_or(texture)}else{texture};
        let size=Vec2::new(w as f32*self.zoom,h as f32*self.zoom);
        egui::ScrollArea::both().auto_shrink([false,false]).show(ui,|ui|{
            let response=ui.add(egui::Image::new((tex.id(),tex.size_vec2()))
                .fit_to_exact_size(size).sense(Sense::click_and_drag()));
            if let Some(view)=Self::frame_view(w,h,response.rect){
                if !shown {self.update_pointer(&response,view);}
                if !shown {
                    let painter=ui.painter();
                    for mask in self.editor.masks(){
                        if let Some(rect)=Self::to_screen(mask.region,view){
                            let selected=self.editor.selected()==Some(mask.id);
                            painter.rect_filled(rect,0.0,
                                if selected{Color32::from_rgba_unmultiplied(17,24,39,185)}
                                else{Color32::from_rgba_unmultiplied(17,24,39,135)});
                        }
                    }
                    if let Some(preview)=self.preview.and_then(|r|Self::to_screen(r,view)){
                        painter.rect_filled(preview,0.0,Color32::from_rgba_unmultiplied(217,112,58,95));
                    }
                }
            }
        });
    }
    fn keyboard(&mut self,ctx:&egui::Context){
        let(undo,redo,delete)=ctx.input(|i|(
            i.modifiers.command && i.key_pressed(egui::Key::Z),
            i.modifiers.command && i.key_pressed(egui::Key::Y),
            i.key_pressed(egui::Key::Delete)
        ));
        if undo {if self.editor.undo().is_ok(){self.mark_edit();}}
        if redo {if self.editor.redo().is_ok(){self.mark_edit();}}
        if delete {
            if let Some(id)=self.editor.selected(){
                if self.editor.remove_mask(id).is_ok(){self.mark_edit();}
            }
        }
    }
}

impl eframe::App for NativeApp {
    fn ui(&mut self,ui:&mut egui::Ui,_frame:&mut eframe::Frame){
        let ctx=ui.ctx().clone();
        if self.smoke {
            self.frames+=1;
            if self.frames>7 {println!("SR_V2_03_NATIVE_WINDOW_LIFECYCLE_PASS");ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
            else {ctx.request_repaint();}
        }
        self.keyboard(&ctx);
        for path in ctx.input(|i|i.raw.dropped_files.iter().filter_map(|f|f.path.clone()).collect::<Vec<_>>()){
            self.load_source(&ctx,&path);
        }
        egui::TopBottomPanel::top("toolbar").show_inside(ui,|ui|{
            ui.horizontal_wrapped(|ui|{
                ui.heading("ShareReady · Rust Native");
                ui.separator();
                if ui.button("Open image").clicked(){
                    if let Some(path)=rfd::FileDialog::new().add_filter("Images",&["png","jpg","jpeg"]).pick_file(){
                        self.load_source(&ctx,&path);
                    }
                }
                if ui.button("Undo").clicked() && self.editor.undo().is_ok(){self.mark_edit();}
                if ui.button("Redo").clicked() && self.editor.redo().is_ok(){self.mark_edit();}
                if ui.button("Review PNG").clicked(){self.start_review(&ctx);}
                if ui.add_enabled(self.editor.is_reviewed(),egui::Button::new("Copy image")).clicked(){self.copy();}
                if ui.add_enabled(self.editor.is_reviewed(),egui::Button::new("Save new PNG")).clicked(){self.save();}
                ui.add(egui::Slider::new(&mut self.zoom,0.1..=2.0).text("zoom"));
            });
            ui.label(&self.status);
        });
        egui::SidePanel::right("inspect").resizable(true).min_width(200.0).show_inside(ui,|ui|{
            ui.heading("Inspector");
            ui.label(&self.source_label);
            ui.label(format!("Source: {}×{}",self.editor.dimensions().0,self.editor.dimensions().1));
            ui.label(format!("Revision: {}",self.editor.stamp().revision));
            ui.label(format!("Masks: {}",self.editor.masks().len()));
            if self.review_view{ui.colored_label(Color32::LIGHT_GREEN,"Reviewing actual encoded PNG");}
            ui.separator();
            let masks=self.editor.masks().to_vec();
            for m in masks {
                let picked=self.editor.selected()==Some(m.id);
                if ui.selectable_label(picked,format!("#{} [{},{},{},{}]",m.id,
                    m.region.x0,m.region.y0,m.region.x1,m.region.y1)).clicked(){
                    let _=self.editor.select(Some(m.id));
                }
            }
            ui.separator();
            if let Some(id)=self.editor.selected(){
                ui.label("Selected mask · resize by 1 source pixel");
                ui.horizontal_wrapped(|ui|{
                    for (label,h,dx,dy) in [
                        ("NW",Handle::Nw,-1,-1),("N",Handle::N,0,-1),("NE",Handle::Ne,1,-1),
                        ("W",Handle::W,-1,0),("E",Handle::E,1,0),
                        ("SW",Handle::Sw,-1,1),("S",Handle::S,0,1),("SE",Handle::Se,1,1)
                    ] {
                        if ui.button(label).clicked() {
                            if self.editor.resize_mask(id,h,dx,dy).unwrap_or(false) {self.mark_edit();}
                        }
                    }
                });
                if ui.button("Delete selected").clicked(){
                    if self.editor.remove_mask(id).is_ok(){self.mark_edit();}
                }
            }
            if ui.button("Clear masks").clicked() {
                if self.editor.clear_masks().unwrap_or(false){self.mark_edit();}
            }
            ui.separator();
            ui.small("This is a POC. Native clipboard and multi-DPI behavior require external Windows verification.");
        });
        egui::CentralPanel::default().show_inside(ui,|ui|{self.render_canvas(ui);});
    }
}
