//! Display geometry only. Export pixels remain owned by Rust Core.
use eframe::egui::{self, ColorImage, Pos2, Rect, TextureHandle, Vec2};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Zoom { Fit, Pixels(f32) }

pub(crate) fn texture_limit(ctx:&egui::Context)->usize{
    let device=ctx.input(|i|i.max_texture_side).max(1);
    #[cfg(feature="performance-evidence")]
    if ctx.data(|d|d.get_temp::<bool>(egui::Id::new("sr-performance-active")))==Some(true){
        return ctx.data(|d|d.get_temp::<usize>(egui::Id::new("sr-performance-image-cap"))).unwrap_or(device).clamp(1,device);
    }
    device
}

#[cfg(feature="performance-evidence")]
fn record_display(ui:&egui::Ui,rect:Rect,zoom:Zoom,size:[usize;2],crop:Option<[usize;4]>,tiles:usize,bytes:usize){
    if ui.ctx().data(|d|d.get_temp::<bool>(egui::Id::new("sr-performance-active")))!=Some(true){return;}
    let c=ui.clip_rect();let crop=crop.map(|r|format!("[{},{},{},{}]",r[0],r[1],r[2],r[3])).unwrap_or("null".into());
    ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("sr-performance-clip"),c));
    let zoom=match zoom{Zoom::Fit=>0.0,Zoom::Pixels(x)=>x};
    let row=format!("{{\"rect\":[{},{},{},{}],\"clip\":[{},{},{},{}],\"source_size\":[{},{}],\"zoom\":{zoom},\"crop\":{crop},\"tile_count\":{tiles},\"new_tile_cpu_bytes\":{bytes},\"effective_display_cap\":{},\"device_texture_cap\":{}}}",rect.min.x,rect.min.y,rect.max.x,rect.max.y,c.min.x,c.min.y,c.max.x,c.max.y,size[0],size[1],texture_limit(ui.ctx()),ui.ctx().input(|i|i.max_texture_side));
    ui.ctx().data_mut(|d|d.insert_temp(egui::Id::new("sr-performance-metric"),row));
}

pub fn image_size(source:[usize;2],zoom:Zoom,available:Vec2,ppp:f32)->Vec2 {
    let native=Vec2::new(source[0] as f32,source[1] as f32);
    match zoom {
        Zoom::Fit=>native*(available.x/native.x).min(available.y/native.y).max(0.001),
        Zoom::Pixels(factor)=>native*factor/ppp,
    }
}

/// Outward rounding keeps every intersecting source pixel in the visible tile.
pub fn visible_pixels(image:Rect,clip:Rect,source:[usize;2])->Option<[usize;4]> {
    let visible=image.intersect(clip);
    if !visible.is_positive(){return None;}
    let scale=Vec2::new(source[0] as f32/image.width(),source[1] as f32/image.height());
    let a=(visible.min-image.min)*scale;
    let b=(visible.max-image.min)*scale;
    Some([a.x.floor().max(0.0) as usize,a.y.floor().max(0.0) as usize,
        (b.x.ceil() as usize).min(source[0]),(b.y.ceil() as usize).min(source[1])])
}

pub fn tiles(rect:[usize;4],limit:usize)->Vec<[usize;4]> {
    let mut result=Vec::new();
    for y in (rect[1]..rect[3]).step_by(limit.max(1)) {
        for x in (rect[0]..rect[2]).step_by(limit.max(1)) {
            result.push([x,y,(x+limit.max(1)).min(rect[2]),(y+limit.max(1)).min(rect[3])]);
        }
    }
    result
}

/// CPU-only data prepared by the same job that owns the validated image.
pub(crate) struct PreparedDisplay {
    full:ColorImage,
    thumbnail:ColorImage,
    cap:usize,
}
impl PreparedDisplay {
    pub(crate) fn source(bytes:&[u8],cap:usize)->Result<Self,String>{
        let img=shareready_core::decode_source(bytes)?;
        Ok(Self::from_color(ColorImage::from_rgba_unmultiplied(
            [img.width() as usize,img.height() as usize],img.as_raw()),cap))
    }
    pub(crate) fn generated(bytes:&[u8],expected:(u32,u32),cap:usize)->Result<Self,String>{
        let img=shareready_core::decode_generated_png(bytes,expected)?;
        Ok(Self::from_color(ColorImage::from_rgba_unmultiplied(
            [img.width() as usize,img.height() as usize],img.as_raw()),cap))
    }
    fn from_color(full:ColorImage,cap:usize)->Self{
        let cap=cap.max(1);let thumbnail_cap=cap.min(2048);
        let ratio=(full.size[0].max(full.size[1]) as f64/thumbnail_cap as f64).max(1.0);
        let size=[(full.size[0] as f64/ratio).ceil() as usize,(full.size[1] as f64/ratio).ceil() as usize];
        let mut pixels=Vec::with_capacity(size[0]*size[1]);
        for y in 0..size[1] {for x in 0..size[0] {
            let sx=((x as f64*ratio) as usize).min(full.size[0]-1);
            let sy=((y as f64*ratio) as usize).min(full.size[1]-1);
            pixels.push(full.pixels[sy*full.size[0]+sx]);
        }}
        Self{full,thumbnail:ColorImage::new(size,pixels),cap}
    }
}

pub struct DisplayImage {
    full:ColorImage,
    thumbnail:TextureHandle,
    visible:Vec<([usize;4],TextureHandle)>,
}
impl DisplayImage {
    pub fn decode(ctx:&egui::Context,bytes:&[u8])->Result<Self,String>{
        let img=shareready_core::decode_source(bytes)?;
        Ok(Self::from_color(ctx,ColorImage::from_rgba_unmultiplied(
            [img.width() as usize,img.height() as usize],img.as_raw())))
    }
    pub fn decode_generated(ctx:&egui::Context,bytes:&[u8],expected:(u32,u32))->Result<Self,String>{
        Self::from_prepared(ctx,PreparedDisplay::generated(bytes,expected,texture_limit(ctx))?)
    }
    pub fn from_color(ctx:&egui::Context,full:ColorImage)->Self {
        Self::upload(ctx,PreparedDisplay::from_color(full,texture_limit(ctx)))
    }
    pub(crate) fn from_prepared(ctx:&egui::Context,prepared:PreparedDisplay)->Result<Self,String>{
        if prepared.cap!=texture_limit(ctx){return Err("显示能力已变化，请重试。".into());}
        Ok(Self::upload(ctx,prepared))
    }
    fn upload(ctx:&egui::Context,prepared:PreparedDisplay)->Self{
        let thumbnail=ctx.load_texture("bounded-fit-preview",prepared.thumbnail,egui::TextureOptions::LINEAR);
        Self{full:prepared.full,thumbnail,visible:Vec::new()}
    }
    pub(crate) fn identity(&self)->egui::TextureId{
        self.thumbnail.id()
    }
    pub fn paint(&mut self,ui:&egui::Ui,rect:Rect,zoom:Zoom){
        let uv=Rect::from_min_max(Pos2::ZERO,Pos2::new(1.0,1.0));
        if zoom==Zoom::Fit {
            self.visible.clear();
            ui.painter().image(self.thumbnail.id(),rect,uv,egui::Color32::WHITE);
            #[cfg(feature="performance-evidence")]
            record_display(ui,rect,zoom,self.full.size,None,1,0);
            return;
        }
        let Some(crop)=visible_pixels(rect,ui.clip_rect(),self.full.size) else{
            #[cfg(feature="performance-evidence")]
            record_display(ui,rect,zoom,self.full.size,None,0,0);
            return;
        };
        let required=tiles(crop,texture_limit(ui.ctx()));
        let changed=self.visible.iter().map(|t|t.0).ne(required.iter().copied());
        if changed {
            self.visible=required.iter().map(|&r|{
                let mut pixels=Vec::with_capacity((r[2]-r[0])*(r[3]-r[1]));
                for y in r[1]..r[3] {pixels.extend_from_slice(&self.full.pixels[y*self.full.size[0]+r[0]..y*self.full.size[0]+r[2]]);}
                let image=ColorImage::new([r[2]-r[0],r[3]-r[1]],pixels);
                (r,ui.ctx().load_texture("visible-original-png-pixels",image,egui::TextureOptions::NEAREST))
            }).collect();
        }
        #[cfg(feature="performance-evidence")]
        record_display(ui,rect,zoom,self.full.size,Some(crop),self.visible.len(),
            if changed{required.iter().map(|r|(r[2]-r[0])*(r[3]-r[1])*4).sum()}else{0});
        let scale=rect.size()/Vec2::new(self.full.size[0] as f32,self.full.size[1] as f32);
        for (r,texture) in &self.visible {
            let a=rect.min+Vec2::new(r[0] as f32,r[1] as f32)*scale;
            let b=rect.min+Vec2::new(r[2] as f32,r[3] as f32)*scale;
            ui.painter().image(texture.id(),Rect::from_min_max(a,b),uv,egui::Color32::WHITE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;
    #[cfg(feature="performance-evidence")]
    #[test] fn simulated_image_limit_does_not_shrink_the_font_atlas(){
        let ctx=egui::Context::default();ctx.data_mut(|d|{
            d.insert_temp(egui::Id::new("sr-performance-active"),true);
            d.insert_temp(egui::Id::new("sr-performance-image-cap"),2usize);
        });
        let mut display=DisplayImage::from_color(&ctx,ColorImage::filled([4,2],egui::Color32::WHITE));
        let input=egui::RawInput{max_texture_side:Some(8192),..Default::default()};
        let mut output=ctx.run_ui(input,|ui|{
            ui.label("Font atlas keeps real device limit");
            display.paint(ui,Rect::from_min_size(pos2(0.0,0.0),Vec2::new(4.0,2.0)),Zoom::Pixels(1.0));
        });
        output.textures_delta.clear();assert_eq!(ctx.input(|i|i.max_texture_side),8192);assert_eq!(display.visible.len(),2);
    }
    #[test] fn physical_zoom_recomputes_logical_size_after_dpi_change(){
        assert_eq!(image_size([2400,1600],Zoom::Pixels(1.0),Vec2::ZERO,1.0),Vec2::new(2400.0,1600.0));
        assert_eq!(image_size([2400,1600],Zoom::Pixels(1.0),Vec2::ZERO,2.0),Vec2::new(1200.0,800.0));
        assert_eq!(image_size([2400,1600],Zoom::Pixels(2.0),Vec2::ZERO,1.5),Vec2::new(3200.0,6400.0/3.0));
        assert_eq!(image_size([2400,1600],Zoom::Fit,Vec2::new(900.0,500.0),2.0),Vec2::new(750.0,500.0));
    }
    #[test] fn scroll_and_fractional_clip_use_source_pixels_with_outward_rounding(){
        let image=Rect::from_min_size(pos2(-20.0,10.0),Vec2::new(1200.0,800.0));
        assert_eq!(visible_pixels(image,Rect::from_min_max(pos2(0.25,20.25),pos2(100.25,60.25)),[2400,1600]),Some([40,20,241,101]));
        assert_eq!(visible_pixels(image,Rect::from_min_max(pos2(2000.0,0.0),pos2(2100.0,100.0)),[2400,1600]),None);
    }
    #[test] fn device_bounded_tiles_cover_visible_pixels_once(){
        let result=tiles([5,7,14,14],4);
        assert_eq!(result,vec![[5,7,9,11],[9,7,13,11],[13,7,14,11],[5,11,9,14],[9,11,13,14],[13,11,14,14]]);
        assert_eq!(result.iter().map(|r|(r[2]-r[0])*(r[3]-r[1])).sum::<usize>(),63);
    }
    #[test] fn physical_detail_uploads_original_pixels_without_downsampling(){
        let ctx=egui::Context::default();
        let pixels=(0..4100).map(|i|egui::Color32::from_rgba_unmultiplied(i as u8,255-i as u8,7,255)).collect();
        let mut display=DisplayImage::from_color(&ctx,ColorImage::new([2050,2],pixels));
        let mut input=egui::RawInput::default();
        input.max_texture_side=Some(1024);
        input.screen_rect=Some(Rect::from_min_size(Pos2::ZERO,Vec2::new(4096.0,500.0)));
        let mut output=ctx.run_ui(input,|ui|{
            display.paint(ui,Rect::from_min_size(Pos2::new(0.0,0.0),Vec2::new(2050.0,2.0)),Zoom::Pixels(1.0));
        });
        let uploads=display.visible.iter().map(|(r,t)|(*r,output.textures_delta.set.get(&t.id()).unwrap().last().unwrap().clone())).collect::<Vec<_>>();
        output.textures_delta.clear();
        assert_eq!(uploads.len(),3);
        for (r,delta) in &uploads {
            let egui::ImageData::Color(image)=&delta.image;
            assert!(image.size[0]<=1024 && image.size[1]<=1024);
            for y in 0..image.size[1] {for x in 0..image.size[0] {
                assert_eq!(image.pixels[y*image.size[0]+x],display.full.pixels[(r[1]+y)*2050+r[0]+x]);
            }}
            assert_eq!(delta.options,egui::TextureOptions::NEAREST);
        }
    }
    #[test] fn prepared_display_preserves_original_and_thumbnail_pixels(){
        let pixels=(0..4100).map(|i|egui::Color32::from_rgb(i as u8,255-i as u8,7)).collect::<Vec<_>>();
        let prepared=PreparedDisplay::from_color(ColorImage::new([2050,2],pixels.clone()),1024);
        assert_eq!(prepared.full.size,[2050,2]);assert_eq!(prepared.full.pixels,pixels);
        assert_eq!(prepared.thumbnail.size,[1024,1]);
        for x in 0..1024 {assert_eq!(prepared.thumbnail.pixels[x],pixels[(x as f64*(2050.0/1024.0)) as usize]);}
    }
    #[test] fn prepared_generated_rejects_changed_image_cap_before_upload(){
        let ctx=egui::Context::default();
        ctx.run_ui(egui::RawInput{max_texture_side:Some(4096),..Default::default()},|_|{}).drop_without_applying_deltas();
        let png=shareready_core::redact(include_bytes!("../../../docs/research/fixtures/T01_CHAT.png"),&[shareready_core::Region::new(1,1,3,3)]).unwrap();
        let prepared=PreparedDisplay::generated(&png,(1200,800),texture_limit(&ctx)).unwrap();
        ctx.run_ui(egui::RawInput{max_texture_side:Some(1024),..Default::default()},|_|{}).drop_without_applying_deltas();
        assert!(DisplayImage::from_prepared(&ctx,prepared).is_err());
    }
}
