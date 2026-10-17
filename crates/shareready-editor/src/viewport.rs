//! Screen-to-source pixel geometry; the actual rendered frame rectangle is authoritative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {pub x: f64,pub y: f64}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub image_width: u32,
    pub image_height: u32,
    pub frame_x: f64,
    pub frame_y: f64,
    pub frame_width: f64,
    pub frame_height: f64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportError { InvalidDimension, InvalidFrame, InvalidPoint }

impl Viewport {
    pub fn new(w:u32,h:u32,x:f64,y:f64,fw:f64,fh:f64)->Result<Self,ViewportError>{
        if w==0 || h==0 {return Err(ViewportError::InvalidDimension);}
        if ![x,y,fw,fh].iter().all(|v|v.is_finite()) || fw<=0.0 || fh<=0.0 {
            return Err(ViewportError::InvalidFrame);
        }
        Ok(Self {image_width:w,image_height:h,frame_x:x,frame_y:y,frame_width:fw,frame_height:fh})
    }
    /// Returns a *continuous* source coordinate. Pointer events may be outside image;
    /// only the committing Rust geometry layer clamps pixel boundaries.
    pub fn screen_to_source(self,screen:Point)->Result<Point,ViewportError> {
        if !screen.x.is_finite() || !screen.y.is_finite() {return Err(ViewportError::InvalidPoint);}
        let x=(screen.x-self.frame_x)/self.frame_width*f64::from(self.image_width);
        let y=(screen.y-self.frame_y)/self.frame_height*f64::from(self.image_height);
        if !x.is_finite() || !y.is_finite() {return Err(ViewportError::InvalidPoint);}
        Ok(Point{x,y})
    }
    pub fn source_to_screen(self,source:Point)->Result<Point,ViewportError> {
        if !source.x.is_finite() || !source.y.is_finite(){return Err(ViewportError::InvalidPoint);}
        let x=self.frame_x+source.x/f64::from(self.image_width)*self.frame_width;
        let y=self.frame_y+source.y/f64::from(self.image_height)*self.frame_height;
        if !x.is_finite() || !y.is_finite() {return Err(ViewportError::InvalidPoint);}
        Ok(Point{x,y})
    }
    /// Outward rounding ensures fractional pointer coverage does not leak a pixel at a corner.
    pub fn pixel_region(self,a:Point,b:Point)->Result<shareready_core::Region,ViewportError>{
        let a=self.screen_to_source(a)?;
        let b=self.screen_to_source(b)?;
        fn bounded_floor(v:f64,max:u32)->i64 {v.floor().clamp(0.0,f64::from(max)) as i64}
        fn bounded_ceil(v:f64,max:u32)->i64 {v.ceil().clamp(0.0,f64::from(max)) as i64}
        Ok(shareready_core::Region::new(
            bounded_floor(a.x.min(b.x),self.image_width),
            bounded_floor(a.y.min(b.y),self.image_height),
            bounded_ceil(a.x.max(b.x),self.image_width),
            bounded_ceil(a.y.max(b.y),self.image_height)
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fractional_frame_and_mixed_display_scale_do_not_shift_source_pixels(){
        for scale in [0.25,0.75,1.0,1.25,1.5,2.0,3.2] {
            let v=Viewport::new(2400,1600,23.4,11.8,2400.0*scale,1600.0*scale).unwrap();
            for (x,y) in [(0.0,0.0),(2399.0,1599.0),(15.0,12.0),(2000.0,1200.0)] {
                let screen=v.source_to_screen(Point{x,y}).unwrap();
                let recovered=v.screen_to_source(screen).unwrap();
                assert!((recovered.x-x).abs()<1e-9 && (recovered.y-y).abs()<1e-9);
            }
        }
    }
    #[test]
    fn outward_rounding_negative_drag_1px_and_edge_clip(){
        let v=Viewport::new(2400,1600,100.0,50.0,1200.0,800.0).unwrap();
        let r=v.pixel_region(Point{x:101.1,y:50.1},Point{x:100.1,y:50.1}).unwrap();
        assert_eq!(r,shareready_core::Region::new(0,0,3,1));
        let edge=v.pixel_region(Point{x:-100.0,y:-100.0},Point{x:1500.0,y:900.0}).unwrap();
        assert_eq!(edge,shareready_core::Region::new(0,0,2400,1600));
    }
    #[test]
    fn nan_and_zero_scale_are_rejected(){
        assert_eq!(Viewport::new(2,3,0.0,0.0,0.0,1.0),Err(ViewportError::InvalidFrame));
        let v=Viewport::new(2,3,0.0,0.0,1.0,1.0).unwrap();
        assert_eq!(v.screen_to_source(Point{x:f64::NAN,y:1.0}),Err(ViewportError::InvalidPoint));
    }
}
