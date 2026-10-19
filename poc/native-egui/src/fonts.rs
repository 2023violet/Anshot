//! Bundled, licensed fallback for the Chinese workbench.
use eframe::egui;

pub fn install(ctx:&egui::Context) {
    let mut fonts=egui::FontDefinitions::default();
    let name="Noto Sans CJK SC".to_owned();
    fonts.font_data.insert(name.clone(),std::sync::Arc::new(egui::FontData::from_static(
        include_bytes!("../assets/fonts/NotoSansCJKsc-Regular.otf"))));
    for family in [egui::FontFamily::Proportional,egui::FontFamily::Monospace]{
        fonts.families.entry(family).or_default().push(name.clone());
    }
    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn workbench_glyphs_exist_in_both_ui_font_families(){
        let ctx=egui::Context::default();install(&ctx);
        let mut out=ctx.run_ui(Default::default(),|_|{
            ctx.fonts_mut(|fonts|for family in [egui::FontFamily::Proportional,egui::FontFamily::Monospace]{
                let id=egui::FontId::new(14.0,family);
                let absent=fonts.layout_no_wrap("\u{10ffff}".into(),id.clone(),egui::Color32::WHITE);
                let replacement=absent.rows[0].glyphs[0].uv_rect;
                for ch in "打开图片粘贴遮挡撤销重做复核返回编辑复制另存为保留放弃修改失败重试原图像素检查器合成示例未验证中文文件名：，。×".chars(){
                    // In epaint 0.36.2 has_glyph compares font faces, producing
                    // false negatives for valid symbols in the replacement face.
                    // Compare actual rasterized atlas glyphs against missing-glyph.
                    let glyph=fonts.layout_no_wrap(ch.to_string(),id.clone(),egui::Color32::WHITE).rows[0].glyphs[0].uv_rect;
                    assert!(!glyph.is_nothing() && (glyph.min,glyph.max)!=(replacement.min,replacement.max),"missing {ch}");
                }
            });
        });
        out.textures_delta.clear();
    }
}
