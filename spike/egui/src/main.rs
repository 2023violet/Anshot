//! Gate 03 spike: egui renders the 03_VISUAL_SAMPLE editor skeleton.
//! Chrome is painted from 04_DESIGN tokens; content is the shared synthetic shot.
use eframe::egui::{self, emath::GuiRounding, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Vec2};

const fn c(hex: u32) -> Color32 { Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8) }
const BG_CANVAS: Color32 = c(0xf7f7f4); const BG_SOFT: Color32 = c(0xfafaf7); const BG_CARD: Color32 = c(0xffffff);
const BG_WORK: Color32 = c(0xecebe5); const BG_HOVER: Color32 = c(0xefeee8); const BG_PRESSED: Color32 = c(0xe6e5e0);
const LINE: Color32 = c(0xe6e5e0); const LINE_STRONG: Color32 = c(0xcfcdc4);
const INK: Color32 = c(0x26251e); const BODY: Color32 = c(0x5a5852); const MUTE: Color32 = c(0x6b6960); const DIS: Color32 = c(0xa09c92);
const SEL_FG: Color32 = c(0xf7f7f4); const BRAND: Color32 = c(0xf54e00); const PRIMARY: Color32 = c(0xd04200);
const SECURE: Color32 = c(0x1f8a65); const ERROR: Color32 = c(0xcf2d56);
const PILL_SECURE: Color32 = c(0x9fc9a2); const PILL_DONE: Color32 = c(0xc08532);
const RED: Color32 = c(0xe5342b); const BLACK: Color32 = c(0x1a1915);
const MOSAIC: [Color32; 4] = [c(0x3a3934), c(0x4b4a44), c(0x5d5b54), c(0x6f6d65)];

type Line = &'static [(f32, f32)];
const ICONS: [&[Line]; 12] = [
    &[&[(4., 4.), (11., 21.), (13.5, 13.5), (21., 11.), (4., 4.)]],
    &[&[(5., 19.), (19., 5.)], &[(9., 5.), (19., 5.), (19., 15.)]],
    &[&[(4., 5.), (20., 5.), (20., 19.), (4., 19.), (4., 5.)]],
    &[&[(5., 6.), (5., 4.), (19., 4.), (19., 6.)], &[(12., 4.), (12., 20.)], &[(9., 20.), (15., 20.)]],
    &[&[(11., 9.), (12.5, 8.), (12.5, 16.)]],
    &[&[(9., 11.), (4., 16.), (4., 20.), (8., 20.), (13., 15.)], &[(9., 11.), (14., 6.), (18., 10.), (13., 15.), (9., 11.)]],
    &[&[(12., 3.), (20., 6.), (20., 12.), (17., 18.), (12., 21.), (7., 18.), (4., 12.), (4., 6.), (12., 3.)]],
    &[&[(4., 4.), (11., 4.), (11., 11.), (4., 11.), (4., 4.)], &[(13., 13.), (20., 13.), (20., 20.), (13., 20.), (13., 13.)]],
    &[&[(6., 2.), (6., 18.), (22., 18.)], &[(2., 6.), (18., 6.), (18., 22.)]],
    &[&[(9., 14.), (4., 9.), (9., 4.)], &[(4., 9.), (15., 9.), (18., 10.), (20., 14.), (18., 18.), (15., 19.), (12., 19.)]],
    &[&[(15., 14.), (20., 9.), (15., 4.)], &[(20., 9.), (9., 9.), (6., 10.), (4., 14.), (6., 18.), (9., 19.), (12., 19.)]],
    &[],
];

struct Spike { shot: Option<egui::TextureHandle>, hover_tools: bool }

fn fonts(ctx: &egui::Context) {
    let mut f = egui::FontDefinitions::default();
    f.font_data.insert("noto".into(), std::sync::Arc::new(egui::FontData::from_static(
        include_bytes!("../../../poc/native-egui/assets/fonts/NotoSansCJKsc-Regular.otf"))));
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\consola.ttf") {
        f.font_data.insert("mono".into(), std::sync::Arc::new(egui::FontData::from_owned(bytes)));
        f.families.entry(FontFamily::Monospace).or_default().insert(0, "mono".into());
    }
    f.families.entry(FontFamily::Proportional).or_default().insert(0, "noto".into());
    f.families.entry(FontFamily::Monospace).or_default().push("noto".into());
    ctx.set_fonts(f);
}

fn ui_font(size: f32) -> FontId { FontId::new(size, FontFamily::Proportional) }
fn mono(size: f32) -> FontId { FontId::new(size, FontFamily::Monospace) }

fn icon(p: &egui::Painter, center: Pos2, idx: usize, color: Color32) {
    let s = 18.0 / 24.0;
    let o = center - Vec2::splat(9.0);
    for line in ICONS[idx] {
        let pts: Vec<Pos2> = line.iter().map(|(x, y)| o + Vec2::new(x * s, y * s)).collect();
        p.add(Shape::line(pts, Stroke::new(1.5, color)));
    }
    if idx == 4 { p.circle_stroke(center, 8.0 * s, Stroke::new(1.5, color)); }
}

fn keycap(p: &egui::Painter, right_center: Pos2, label: &str, on_primary: bool) -> f32 {
    let galley = p.layout_no_wrap(label.into(), mono(11.0), if on_primary { Color32::WHITE } else { MUTE });
    let w = galley.size().x + 10.0;
    let r = Rect::from_min_size(Pos2::new(right_center.x - w, right_center.y - 9.0), Vec2::new(w, 18.0));
    if on_primary {
        p.rect_filled(r, 4.0, Color32::from_white_alpha(46));
    } else {
        p.rect_filled(r, 4.0, BG_CARD);
        p.rect_stroke(r, 4.0, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
        p.line_segment([r.left_bottom() + Vec2::new(2.0, -0.5), r.right_bottom() + Vec2::new(-2.0, -0.5)], Stroke::new(1.0, LINE_STRONG));
    }
    p.galley(Pos2::new(r.min.x + 5.0, r.center().y - galley.size().y / 2.0), galley, MUTE);
    w
}

fn text_button(ui: &mut egui::Ui, p: &egui::Painter, rect: Rect, label: &str, key: &str, kind: u8) {
    let resp = ui.interact(rect, ui.id().with(label), Sense::click());
    let (bg, fg) = match kind { 1 => (if resp.hovered() { c(0xbf3d00) } else { PRIMARY }, Color32::WHITE), 2 => (Color32::TRANSPARENT, ERROR), _ => (if resp.hovered() { BG_HOVER } else { BG_CARD }, INK) };
    p.rect_filled(rect, 8.0, bg);
    if kind == 0 { p.rect_stroke(rect, 8.0, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside); }
    p.text(Pos2::new(rect.min.x + 14.0, rect.center().y), Align2::LEFT_CENTER, label, ui_font(13.0), fg);
    keycap(p, Pos2::new(rect.max.x - 10.0, rect.center().y), key, kind == 1);
}

fn pill(p: &egui::Painter, left_center: Pos2, label: &str, bg: Color32) -> f32 {
    let g = p.layout_no_wrap(label.into(), ui_font(11.0), INK);
    let r = Rect::from_min_size(Pos2::new(left_center.x, left_center.y - 10.0), Vec2::new(g.size().x + 26.0, 20.0));
    p.rect_filled(r, 10.0, bg);
    shield(p, Pos2::new(r.min.x + 11.0, r.center().y), 5.0, SECURE);
    p.galley(Pos2::new(r.min.x + 18.0, r.center().y - g.size().y / 2.0), g, INK);
    r.width()
}

fn shield(p: &egui::Painter, c0: Pos2, s: f32, color: Color32) {
    let pts = [(0.0, -1.0), (0.9, -0.65), (0.9, 0.05), (0.55, 0.65), (0.0, 1.0), (-0.55, 0.65), (-0.9, 0.05), (-0.9, -0.65)]
        .iter().map(|(x, y)| c0 + Vec2::new(x * s, y * s)).collect();
    p.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

impl Spike {
    fn toolbar(&self, ui: &mut egui::Ui, p: &egui::Painter, r: Rect) {
        p.rect_filled(r, 0.0, BG_CANVAS);
        p.hline(r.x_range(), r.max.y - 0.5, Stroke::new(1.0, LINE));
        let cy = r.center().y;
        p.rect_filled(Rect::from_center_size(Pos2::new(r.min.x + 16.0, cy), Vec2::splat(12.0)), 3.0, BRAND);
        p.text(Pos2::new(r.min.x + 28.0, cy), Align2::LEFT_CENTER, "ShareReady", ui_font(13.0), INK);
        let mut x = r.min.x + 118.0;
        for (i, idx) in [0usize, 1, 2, 3, 4, 5, 99, 6, 7, 99, 8].iter().enumerate() {
            if *idx == 99 { p.vline(x + 6.0, (cy - 11.0)..=(cy + 11.0), Stroke::new(1.0, LINE_STRONG)); x += 13.0; continue; }
            let b = Rect::from_min_size(Pos2::new(x, cy - 16.0), Vec2::splat(32.0));
            let resp = ui.interact(b, ui.id().with(("tool", i)), Sense::click());
            let selected = *idx == 6;
            let bg = if selected { INK } else if resp.hovered() && self.hover_tools { BG_HOVER } else { Color32::TRANSPARENT };
            p.rect_filled(b, 6.0, bg);
            if *idx == 3 { p.rect_stroke(b.expand(2.0), 8.0, Stroke::new(2.0, BRAND), StrokeKind::Outside); }
            icon(p, b.center(), *idx, if selected { SEL_FG } else { BODY });
            if *idx == 7 { shield(p, b.max - Vec2::new(7.0, 7.0), 4.0, SECURE); }
            x += 36.0;
        }
        let mut rx = r.max.x - 10.0;
        let copy = Rect::from_min_max(Pos2::new(rx - 104.0, cy - 16.0), Pos2::new(rx, cy + 16.0));
        text_button(ui, p, copy, "复制", "Ctrl C", 1);
        rx -= 112.0;
        let save = Rect::from_min_max(Pos2::new(rx - 100.0, cy - 16.0), Pos2::new(rx, cy + 16.0));
        text_button(ui, p, save, "保存", "Ctrl S", 0);
        rx -= 112.0;
        p.vline(rx + 2.0, (cy - 11.0)..=(cy + 11.0), Stroke::new(1.0, LINE_STRONG));
        icon(p, Pos2::new(rx - 22.0, cy), 10, DIS);
        icon(p, Pos2::new(rx - 58.0, cy), 9, BODY);
    }

    fn propbar(&self, ui: &mut egui::Ui, p: &egui::Painter, r: Rect) {
        p.rect_filled(r, 0.0, BG_SOFT);
        p.hline(r.x_range(), r.max.y - 0.5, Stroke::new(1.0, LINE));
        let cy = r.center().y;
        let mut x = r.min.x + 12.0;
        let label = |p: &egui::Painter, x: &mut f32, t: &str| { let w = p.text(Pos2::new(*x, cy), Align2::LEFT_CENTER, t, ui_font(12.0), MUTE).width(); *x += w + 8.0; };
        label(p, &mut x, "遮挡样式");
        let seg = Rect::from_min_size(Pos2::new(x, cy - 14.0), Vec2::new(132.0, 28.0));
        p.rect_filled(seg, 8.0, BG_PRESSED);
        let on = Rect::from_min_size(seg.min + Vec2::splat(2.0), Vec2::new(44.0, 24.0));
        p.rect_filled(on, 6.0, BG_CARD);
        p.rect_stroke(on, 6.0, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
        p.text(on.center(), Align2::CENTER_CENTER, "实心", ui_font(12.0), INK);
        p.text(Pos2::new(on.max.x + 42.0, cy), Align2::CENTER_CENTER, "安全马赛克", ui_font(12.0), BODY);
        x = seg.max.x + 12.0;
        label(p, &mut x, "颜色");
        for (i, col) in [BLACK, Color32::WHITE].iter().enumerate() {
            let s = Rect::from_min_size(Pos2::new(x, cy - 10.0), Vec2::splat(20.0));
            p.rect_filled(s, 6.0, *col);
            if i == 0 { p.rect_stroke(s.expand(2.0), 8.0, Stroke::new(2.0, INK), StrokeKind::Outside); } else { p.rect_stroke(s, 6.0, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside); }
            x += 28.0;
        }
        p.vline(x + 4.0, (cy - 11.0)..=(cy + 11.0), Stroke::new(1.0, LINE_STRONG));
        x += 16.0;
        for (k, v) in [("X", "412"), ("Y", "142"), ("W", "176"), ("H", "22")] {
            label(p, &mut x, k);
            let f = Rect::from_min_size(Pos2::new(x, cy - 14.0), Vec2::new(48.0, 28.0));
            p.rect_filled(f, 8.0, BG_CARD);
            p.rect_stroke(f, 8.0, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
            p.text(Pos2::new(f.min.x + 8.0, cy), Align2::LEFT_CENTER, v, mono(12.0), INK);
            x += 58.0;
        }
        let del = Rect::from_min_max(Pos2::new(r.max.x - 102.0, cy - 16.0), Pos2::new(r.max.x - 10.0, cy + 16.0));
        text_button(ui, p, del, "删除", "Del", 2);
    }

    fn canvas(&self, p: &egui::Painter, r: Rect) {
        p.rect_filled(r, 0.0, BG_WORK);
        let Some(tex) = &self.shot else { return };
        let size = tex.size_vec2();
        let img = Rect::from_center_size(r.center(), size).round_to_pixels(p.pixels_per_point());
        p.image(tex.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        p.rect_stroke(img, 0.0, Stroke::new(1.0, LINE_STRONG), StrokeKind::Outside);
        let at = |x: f32, y: f32| img.min + Vec2::new(x, y);
        // safe mosaic: generated palette, independent of source pixels
        let m = Rect::from_min_size(at(374.0, 143.0), Vec2::new(102.0, 20.0));
        let cell = 6.0;
        let mut yy = 0;
        while (yy as f32) * cell < m.height() {
            let mut xx = 0;
            while (xx as f32) * cell < m.width() {
                let cr = Rect::from_min_size(m.min + Vec2::new(xx as f32 * cell, yy as f32 * cell), Vec2::splat(cell)).intersect(m);
                p.rect_filled(cr, 0.0, MOSAIC[(xx * 7 + yy * 3) % 4]);
                xx += 1;
            }
            yy += 1;
        }
        dashed_rect(p, m, SECURE);
        p.rect_filled(Rect::from_center_size(m.min, Vec2::splat(12.0)), 3.0, SECURE);
        shield(p, m.min, 4.0, Color32::WHITE);
        // solid redaction + selection (ink + white outer line) + handles
        let s = Rect::from_min_size(at(516.0, 143.0), Vec2::new(80.0, 20.0));
        p.rect_filled(s, 0.0, BLACK);
        p.rect_stroke(s, 0.0, Stroke::new(1.0, Color32::WHITE), StrokeKind::Outside);
        p.rect_stroke(s.expand(1.0), 0.0, Stroke::new(1.0, INK), StrokeKind::Outside);
        for hx in [s.min.x, s.center().x, s.max.x] { for hy in [s.min.y, s.center().y, s.max.y] {
            if hx == s.center().x && hy == s.center().y { continue; }
            let h = Rect::from_center_size(Pos2::new(hx, hy), Vec2::splat(8.0));
            p.rect_filled(h, 2.0, Color32::WHITE);
            p.rect_stroke(h, 2.0, Stroke::new(1.5, INK), StrokeKind::Inside);
        }}
        // annotations (content colours)
        p.rect_stroke(Rect::from_min_max(at(10.0, 264.0), at(350.0, 310.0)), 2.0, Stroke::new(3.0, RED), StrokeKind::Inside);
        p.circle_filled(at(350.0, 264.0), 13.0, RED);
        p.text(at(350.0, 264.0), Align2::CENTER_CENTER, "1", ui_font(14.0), Color32::WHITE);
        let (a, b) = (at(398.0, 362.0), at(356.0, 306.0));
        p.line_segment([a, b], Stroke::new(4.0, RED));
        let dir = (b - a).normalized();
        let n = Vec2::new(-dir.y, dir.x);
        p.add(Shape::convex_polygon(vec![b + dir * 4.0, b - dir * 14.0 + n * 8.0, b - dir * 14.0 - n * 8.0], RED, Stroke::NONE));
        p.text(at(404.0, 352.0), Align2::LEFT_TOP, "这里看凭证", ui_font(18.0), RED);
        // toast
        let t = Rect::from_min_size(Pos2::new(r.max.x - 296.0, r.max.y - 72.0), Vec2::new(280.0, 56.0));
        p.rect_filled(t, 12.0, BG_CARD);
        p.rect_stroke(t, 12.0, Stroke::new(1.0, LINE_STRONG), StrokeKind::Inside);
        let pr = Rect::from_min_size(Pos2::new(t.min.x + 14.0, t.center().y - 10.0), Vec2::new(62.0, 20.0));
        p.rect_filled(pr, 10.0, PILL_DONE);
        p.text(pr.center(), Align2::CENTER_CENTER, "✓ 已复制", ui_font(11.0), INK);
        p.text(Pos2::new(pr.max.x + 10.0, t.center().y - 9.0), Align2::LEFT_CENTER, "已复制到剪贴板", ui_font(13.0), INK);
        p.text(Pos2::new(pr.max.x + 10.0, t.center().y + 10.0), Align2::LEFT_CENTER, "620 × 430 · 已遮挡 2 处", ui_font(12.0), MUTE);
    }

    fn status(&self, p: &egui::Painter, r: Rect) {
        p.rect_filled(r, 0.0, BG_CANVAS);
        p.hline(r.x_range(), r.min.y + 0.5, Stroke::new(1.0, LINE));
        let cy = r.center().y;
        let w = p.text(Pos2::new(r.min.x + 12.0, cy), Align2::LEFT_CENTER, "620 × 430", mono(12.0), MUTE).width();
        pill(p, Pos2::new(r.min.x + 24.0 + w, cy), "已遮挡 2 处", PILL_SECURE);
        let mut x = r.max.x - 12.0;
        for (t, on) in [("+", false), ("−", false), ("100%", true), ("适应", false)] {
            let g = p.layout_no_wrap(t.into(), if on { mono(12.0) } else { ui_font(12.0) }, if on { INK } else { MUTE });
            let extra = if t == "适应" { 50.0 } else { 0.0 };
            let b = Rect::from_min_max(Pos2::new(x - g.size().x - 16.0 - extra, cy - 11.0), Pos2::new(x, cy + 11.0));
            if on { p.rect_filled(b, 6.0, BG_PRESSED); }
            p.galley(Pos2::new(b.min.x + 8.0, cy - g.size().y / 2.0), g, MUTE);
            if extra > 0.0 { keycap(p, Pos2::new(b.max.x - 6.0, cy), "Ctrl 0", false); }
            x = b.min.x - 2.0;
        }
    }
}

fn dashed_rect(p: &egui::Painter, r: Rect, color: Color32) {
    let pts = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
    p.extend(Shape::dashed_line(&pts, Stroke::new(1.0, color), 3.0, 2.0));
}

impl eframe::App for Spike {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.shot.is_none() {
            let img = image::load_from_memory(include_bytes!("../../assets/shot.png")).expect("shot").to_rgba8();
            let ci = egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], &img);
            self.shot = Some(ctx.load_texture("shot", ci, egui::TextureOptions::NEAREST));
        }
        let full = ui.max_rect();
        let p = ui.painter().clone();
        let tool = Rect::from_min_max(full.min, Pos2::new(full.max.x, full.min.y + 52.0));
        let prop = Rect::from_min_max(Pos2::new(full.min.x, tool.max.y), Pos2::new(full.max.x, tool.max.y + 40.0));
        let stat = Rect::from_min_max(Pos2::new(full.min.x, full.max.y - 30.0), full.max);
        let canv = Rect::from_min_max(Pos2::new(full.min.x, prop.max.y), Pos2::new(full.max.x, stat.min.y));
        self.canvas(&p, canv);
        self.toolbar(ui, &p, tool);
        self.propbar(ui, &p, prop);
        self.status(&p, stat);
        let _ = CornerRadius::ZERO;
    }
}

fn main() -> eframe::Result {
    let scale: Option<f32> = std::env::var("SPIKE_SCALE").ok().and_then(|s| s.parse().ok());
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1180.0, 700.0]).with_title("spike-egui"),
        ..Default::default()
    };
    eframe::run_native("spike-egui", opts, Box::new(move |cc| {
        fonts(&cc.egui_ctx);
        if let Some(s) = scale { cc.egui_ctx.set_zoom_factor(s); }
        cc.egui_ctx.set_theme(egui::Theme::Light); cc.egui_ctx.style_mut_of(egui::Theme::Light, |s| s.visuals.panel_fill = BG_CANVAS);
        Ok(Box::new(Spike { shot: None, hover_tools: true }))
    }))
}
