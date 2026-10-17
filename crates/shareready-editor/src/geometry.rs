//! Exact half-open source-pixel geometry: no floating-point pixels cross into PNG export.
use shareready_core::Region;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle { N, Ne, E, Se, S, Sw, W, Nw }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect { pub left: i64, pub top: i64, pub right: i64, pub bottom: i64 }

impl PixelRect {
    pub fn from_region(r: Region, width: u32, height: u32) -> Option<Self> {
        let (l,t,x,b) = r.clipped(width,height)?;
        Some(Self {left: l.into(),top: t.into(),right: x.into(),bottom: b.into()})
    }
    pub fn region(self) -> Region { Region::new(self.left,self.top,self.right,self.bottom) }
    pub fn width(self) -> i64 { self.right-self.left }
    pub fn height(self) -> i64 { self.bottom-self.top }
    pub fn translate(self, dx: i64, dy: i64, w: u32, h: u32) -> Self {
        // Clamp translations without narrowing; preserves the entire original mask size.
        let x = dx.clamp(-self.left, i64::from(w)-self.right);
        let y = dy.clamp(-self.top, i64::from(h)-self.bottom);
        Self {left:self.left+x,right:self.right+x,top:self.top+y,bottom:self.bottom+y}
    }
    pub fn resize(self, handle: Handle, dx: i64, dy: i64, w: u32, h: u32) -> Self {
        let mut r=self;
        let on_left=matches!(handle,Handle::Nw|Handle::W|Handle::Sw);
        let on_right=matches!(handle,Handle::Ne|Handle::E|Handle::Se);
        let on_top=matches!(handle,Handle::Nw|Handle::N|Handle::Ne);
        let on_bottom=matches!(handle,Handle::Sw|Handle::S|Handle::Se);
        if on_left {r.left=self.left.saturating_add(dx).clamp(0,self.right-1);}
        if on_right {r.right=self.right.saturating_add(dx).clamp(self.left+1,i64::from(w));}
        if on_top {r.top=self.top.saturating_add(dy).clamp(0,self.bottom-1);}
        if on_bottom {r.bottom=self.bottom.saturating_add(dy).clamp(self.top+1,i64::from(h));}
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_eight_handles_keep_opposite_edges_and_at_least_one_pixel() {
        let src=PixelRect::from_region(Region::new(10,20,30,40),100,80).unwrap();
        for h in [Handle::N,Handle::Ne,Handle::E,Handle::Se,Handle::S,Handle::Sw,Handle::W,Handle::Nw] {
            let r=src.resize(h,10000,-10000,100,80);
            assert!(r.left<r.right && r.top<r.bottom,"{h:?}");
            assert!((0..=100).contains(&r.left) && r.right<=100);
            assert!((0..=80).contains(&r.top) && r.bottom<=80);
            if !matches!(h,Handle::Nw|Handle::W|Handle::Sw) {assert_eq!(r.left,src.left);}
            if !matches!(h,Handle::Ne|Handle::E|Handle::Se) {assert_eq!(r.right,src.right);}
            if !matches!(h,Handle::Nw|Handle::N|Handle::Ne) {assert_eq!(r.top,src.top);}
            if !matches!(h,Handle::Sw|Handle::S|Handle::Se) {assert_eq!(r.bottom,src.bottom);}
        }
        assert_eq!(src.resize(Handle::Nw,1000,1000,100,80).width(),1);
        assert_eq!(src.resize(Handle::Nw,1000,1000,100,80).height(),1);
    }
    #[test]
    fn saturated_translation_keeps_full_shape_even_at_overflow() {
        let src=PixelRect::from_region(Region::new(10,20,30,40),100,80).unwrap();
        let far=src.translate(i64::MAX,i64::MIN,100,80);
        assert_eq!(far,(PixelRect{left:80,top:0,right:100,bottom:20}));
        let back=src.translate(i64::MIN,i64::MAX,100,80);
        assert_eq!(back,(PixelRect{left:0,top:60,right:20,bottom:80}));
    }
}
