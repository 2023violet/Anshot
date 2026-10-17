//! Preserve the shipped V1 CLI/WASM ABI; image behavior is implemented only in shareready-core.
pub use shareready_core::{decode_source, jpeg_orientation, orient_rgba, paint_rgba, preview_png, redact, Region, COVER, MAX_INPUT_BYTES, MAX_PIXELS};

#[cfg(feature = "web")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "web")]
#[wasm_bindgen]
pub fn preview_web(input: &[u8]) -> Result<Vec<u8>, JsValue> {
    preview_png(input).map_err(|e| JsValue::from_str(&e))
}
#[cfg(feature = "web")]
#[wasm_bindgen]
pub fn redact_web(input: &[u8], rectangles: &[i32]) -> Result<Vec<u8>, JsValue> {
    if rectangles.len() % 4 != 0 { return Err(JsValue::from_str("invalid region shape")); }
    let regions: Vec<Region> = rectangles.chunks_exact(4).map(|s| {
        Region::new(s[0] as i64,s[1] as i64,s[2] as i64,s[3] as i64)
    }).collect();
    redact(input, &regions).map_err(|s| JsValue::from_str(&s))
}

