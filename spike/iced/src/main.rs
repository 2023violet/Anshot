//! Gate 03 spike: iced renders the 03_VISUAL_SAMPLE editor skeleton.
//! Chrome uses iced widgets + styles from 04_DESIGN tokens; icons and the
//! canvas overlay use iced's canvas; content is the shared synthetic shot.
use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineDash, LineJoin, Path, Stroke};
use iced::widget::{button, column, container, image, row, space, stack, text, Space};
use iced::{alignment, mouse, Background, Border, Color, Element, Font, Length, Point, Rectangle, Renderer, Size, Theme};

const fn c(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}
const BG_CANVAS: Color = c(0xf7f7f4); const BG_SOFT: Color = c(0xfafaf7); const BG_CARD: Color = c(0xffffff);
const BG_WORK: Color = c(0xecebe5); const BG_HOVER: Color = c(0xefeee8); const BG_PRESSED: Color = c(0xe6e5e0);
const LINE: Color = c(0xe6e5e0); const LINE_STRONG: Color = c(0xcfcdc4);
const INK: Color = c(0x26251e); const BODY: Color = c(0x5a5852); const MUTE: Color = c(0x6b6960); const DIS: Color = c(0xa09c92);
const SEL_FG: Color = c(0xf7f7f4); const BRAND: Color = c(0xf54e00); const PRIMARY: Color = c(0xd04200);
const SECURE: Color = c(0x1f8a65); const ERROR: Color = c(0xcf2d56);
const PILL_SECURE: Color = c(0x9fc9a2); const PILL_DONE: Color = c(0xc08532);
const RED: Color = c(0xe5342b); const BLACK: Color = c(0x1a1915);
const MOSAIC: [Color; 4] = [c(0x3a3934), c(0x4b4a44), c(0x5d5b54), c(0x6f6d65)];
const UI: Font = Font::with_name("Noto Sans CJK SC");
const MONO: Font = Font::with_name("Consolas");

type Line = &'static [(f32, f32)];
const ICONS: [&[Line]; 11] = [
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
];

#[derive(Debug, Clone)]
enum Msg { Noop }

#[derive(Default)]
struct Spike;

fn stroke(color: Color, width: f32) -> Stroke<'static> {
    Stroke::default().with_color(color).with_width(width).with_line_cap(LineCap::Round).with_line_join(LineJoin::Round)
}

fn shield(frame: &mut Frame, c0: Point, s: f32, color: Color) {
    let pts = [(0.0, -1.0), (0.9, -0.65), (0.9, 0.05), (0.55, 0.65), (0.0, 1.0), (-0.55, 0.65), (-0.9, 0.05), (-0.9, -0.65)];
    let path = Path::new(|b| {
        for (i, (x, y)) in pts.iter().enumerate() {
            let p = Point::new(c0.x + x * s, c0.y + y * s);
            if i == 0 { b.move_to(p) } else { b.line_to(p) }
        }
        b.close();
    });
    frame.fill(&path, color);
}

struct Icon { idx: usize, color: Color, badge: bool }
impl canvas::Program<Msg> for Icon {
    type State = ();
    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, bounds.size());
        let s = bounds.width / 24.0;
        for line in ICONS[self.idx] {
            let path = Path::new(|b| for (i, (x, y)) in line.iter().enumerate() {
                let p = Point::new(x * s, y * s);
                if i == 0 { b.move_to(p) } else { b.line_to(p) }
            });
            f.stroke(&path, stroke(self.color, 1.5));
        }
        if self.idx == 4 { f.stroke(&Path::circle(Point::new(12. * s, 12. * s), 8. * s), stroke(self.color, 1.5)); }
        if self.badge { shield(&mut f, Point::new(bounds.width - 2.0, bounds.height - 2.0), 4.0, SECURE); }
        vec![f.into_geometry()]
    }
}

fn icon<'a>(idx: usize, color: Color, badge: bool) -> Element<'a, Msg> {
    canvas(Icon { idx, color, badge }).width(18).height(18).into()
}

fn canvas<P: canvas::Program<Msg>>(p: P) -> canvas::Canvas<P, Msg> { iced::widget::canvas(p) }

fn label<'a>(t: &'a str, size: f32, color: Color) -> Element<'a, Msg> { text(t).size(size).font(UI).color(color).into() }
fn mono<'a>(t: &'a str, size: f32, color: Color) -> Element<'a, Msg> { text(t).size(size).font(MONO).color(color).into() }

fn boxed(bg: Color, border: Color, bw: f32, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(bg)), border: Border { color: border, width: bw, radius: radius.into() }, ..Default::default() }
}

fn keycap<'a>(t: &'a str, on_primary: bool) -> Element<'a, Msg> {
    let (bg, fg, bw) = if on_primary { (Color { a: 0.18, ..Color::WHITE }, Color::WHITE, 0.0) } else { (BG_CARD, MUTE, 1.0) };
    container(mono(t, 11.0, fg)).padding([0, 5]).height(18).align_y(alignment::Vertical::Center)
        .style(boxed(bg, LINE_STRONG, bw, 4.0)).into()
}

fn tool<'a>(idx: usize, selected: bool, focused: bool) -> Element<'a, Msg> {
    let fg = if selected { SEL_FG } else { BODY };
    let b = button(container(icon(idx, fg, idx == 7)).center(32)).padding(0).width(32).height(32).on_press(Msg::Noop)
        .style(move |_, status| button::Style {
            background: Some(Background::Color(if selected { INK } else if matches!(status, button::Status::Hovered) { BG_HOVER } else { Color::TRANSPARENT })),
            border: Border { radius: 6.0.into(), ..Default::default() }, text_color: fg, ..Default::default()
        });
    if focused {
        container(b).padding(2).style(boxed(Color::TRANSPARENT, BRAND, 2.0, 8.0)).into()
    } else { b.into() }
}

fn text_button<'a>(t: &'a str, key: &'a str, kind: u8) -> Element<'a, Msg> {
    let fg = match kind { 1 => Color::WHITE, 2 => ERROR, _ => INK };
    button(row![label(t, 13.0, fg), keycap(key, kind == 1)].spacing(8).align_y(alignment::Vertical::Center))
        .padding([0, 12]).height(32).on_press(Msg::Noop)
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered);
            let (bg, bw) = match kind { 1 => (if hovered { c(0xbf3d00) } else { PRIMARY }, 0.0), 2 => (Color::TRANSPARENT, 0.0), _ => (if hovered { BG_HOVER } else { BG_CARD }, 1.0) };
            button::Style { background: Some(Background::Color(bg)), text_color: fg, border: Border { color: LINE_STRONG, width: bw, radius: 8.0.into() }, ..Default::default() }
        }).into()
}

fn vsep<'a>() -> Element<'a, Msg> { container(Space::new()).width(1).height(22).style(boxed(LINE_STRONG, LINE_STRONG, 0.0, 0.0)).into() }

fn bar<'a>(content: impl Into<Element<'a, Msg>>, h: f32, bg: Color) -> Element<'a, Msg> {
    container(content).width(Length::Fill).height(h).padding([0, 10]).align_y(alignment::Vertical::Center)
        .style(move |_| container::Style { background: Some(Background::Color(bg)), border: Border { color: LINE, width: 0.0, radius: 0.0.into() }, ..Default::default() }).into()
}

fn hline<'a>() -> Element<'a, Msg> { container(Space::new()).width(Length::Fill).height(1).style(boxed(LINE, LINE, 0.0, 0.0)).into() }

fn pill<'a>(t: &'a str, bg: Color, secure: bool) -> Element<'a, Msg> {
    let mut r = row![].spacing(5).align_y(alignment::Vertical::Center);
    if secure { r = r.push(canvas(ShieldMark).width(10).height(11)); }
    container(r.push(label(t, 11.0, INK))).padding([0, 8]).height(20).align_y(alignment::Vertical::Center)
        .style(boxed(bg, bg, 0.0, 10.0)).into()
}

struct ShieldMark;
impl canvas::Program<Msg> for ShieldMark {
    type State = ();
    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, b: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        shield(&mut f, Point::new(b.width / 2.0, b.height / 2.0), 5.0, SECURE);
        vec![f.into_geometry()]
    }
}

struct Overlay;
impl canvas::Program<Msg> for Overlay {
    type State = ();
    fn draw(&self, _: &(), renderer: &Renderer, _: &Theme, b: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let mut f = Frame::new(renderer, b.size());
        // safe mosaic: generated palette, independent of source pixels
        let (mx, my, mw, mh, cell) = (374.0, 143.0, 102.0, 20.0, 6.0);
        let (mut yy, mut y) = (0usize, 0.0);
        while y < mh {
            let (mut xx, mut x) = (0usize, 0.0);
            while x < mw {
                f.fill_rectangle(Point::new(mx + x, my + y), Size::new((mw - x).min(cell), (mh - y).min(cell)), MOSAIC[(xx * 7 + yy * 3) % 4]);
                xx += 1; x += cell;
            }
            yy += 1; y += cell;
        }
        let dash = Stroke { line_dash: LineDash { segments: &[3.0, 2.0], offset: 0 }, ..stroke(SECURE, 1.0) };
        f.stroke(&Path::rectangle(Point::new(mx + 0.5, my + 0.5), Size::new(mw - 1.0, mh - 1.0)), dash);
        f.fill(&Path::rounded_rectangle(Point::new(mx - 6.0, my - 6.0), Size::new(12.0, 12.0), 3.0.into()), SECURE);
        shield(&mut f, Point::new(mx, my), 4.0, Color::WHITE);
        // solid + selection (white inner, ink outer) + handles
        let (sx, sy, sw, sh) = (516.0, 143.0, 80.0, 20.0);
        f.fill_rectangle(Point::new(sx, sy), Size::new(sw, sh), BLACK);
        f.stroke(&Path::rectangle(Point::new(sx - 0.5, sy - 0.5), Size::new(sw + 1.0, sh + 1.0)), stroke(Color::WHITE, 1.0));
        f.stroke(&Path::rectangle(Point::new(sx - 1.5, sy - 1.5), Size::new(sw + 3.0, sh + 3.0)), stroke(INK, 1.0));
        for hx in [sx, sx + sw / 2.0, sx + sw] { for hy in [sy, sy + sh / 2.0, sy + sh] {
            if hx == sx + sw / 2.0 && hy == sy + sh / 2.0 { continue; }
            let p = Path::rounded_rectangle(Point::new(hx - 4.0, hy - 4.0), Size::new(8.0, 8.0), 2.0.into());
            f.fill(&p, Color::WHITE); f.stroke(&p, stroke(INK, 1.5));
        }}
        // annotations
        f.stroke(&Path::rounded_rectangle(Point::new(11.5, 265.5), Size::new(337.0, 43.0), 2.0.into()), stroke(RED, 3.0));
        f.fill(&Path::circle(Point::new(350.0, 264.0), 13.0), RED);
        f.fill_text(canvas::Text { content: "1".into(), position: Point::new(350.0, 264.0), color: Color::WHITE, size: 14.0.into(), font: UI,
            align_x: iced::alignment::Horizontal::Center.into(), align_y: alignment::Vertical::Center, ..Default::default() });
        let (a, bb) = (Point::new(398.0, 362.0), Point::new(356.0, 306.0));
        f.stroke(&Path::line(a, bb), stroke(RED, 4.0));
        let (dx, dy) = (bb.x - a.x, bb.y - a.y); let l = (dx * dx + dy * dy).sqrt(); let (ux, uy) = (dx / l, dy / l);
        f.fill(&Path::new(|p| { p.move_to(Point::new(bb.x + ux * 4.0, bb.y + uy * 4.0)); p.line_to(Point::new(bb.x - ux * 14.0 - uy * 8.0, bb.y - uy * 14.0 + ux * 8.0)); p.line_to(Point::new(bb.x - ux * 14.0 + uy * 8.0, bb.y - uy * 14.0 - ux * 8.0)); p.close(); }), RED);
        f.fill_text(canvas::Text { content: "这里看凭证".into(), position: Point::new(404.0, 352.0), color: RED, size: 18.0.into(), font: UI, ..Default::default() });
        vec![f.into_geometry()]
    }
}

impl Spike {
    fn view(&self) -> Element<'_, Msg> {
        let mark = row![container(Space::new()).width(12).height(12).style(boxed(BRAND, BRAND, 0.0, 3.0)), label("ShareReady", 13.0, INK)]
            .spacing(8).align_y(alignment::Vertical::Center);
        let toolbar = row![
            mark, Space::new().width(10),
            tool(0, false, false), tool(1, false, false), tool(2, false, false), tool(3, false, true), tool(4, false, false), tool(5, false, false),
            vsep(), tool(6, true, false), tool(7, false, false), vsep(), tool(8, false, false),
            space::horizontal(),
            tool(9, false, false), container(icon(10, DIS, false)).center(32), vsep(),
            text_button("保存", "Ctrl S", 0), text_button("复制", "Ctrl C", 1),
        ].spacing(4).align_y(alignment::Vertical::Center);

        let seg = container(row![
            container(label("实心", 12.0, INK)).padding([0, 10]).height(24).align_y(alignment::Vertical::Center).style(boxed(BG_CARD, LINE_STRONG, 1.0, 6.0)),
            container(label("安全马赛克", 12.0, BODY)).padding([0, 10]).height(24).align_y(alignment::Vertical::Center),
        ].spacing(2)).padding(2).style(boxed(BG_PRESSED, BG_PRESSED, 0.0, 8.0));
        let swatch = |col: Color, sel: bool| -> Element<'_, Msg> {
            let s = container(Space::new()).width(20).height(20).style(boxed(col, LINE_STRONG, if sel { 0.0 } else { 1.0 }, 6.0));
            if sel { container(container(s).padding(2).style(boxed(BG_SOFT, BG_SOFT, 0.0, 8.0))).padding(0).style(boxed(INK, INK, 0.0, 9.0)).padding(2).into() } else { s.into() }
        };
        let field = |v: &'static str| -> Element<'static, Msg> {
            container(mono(v, 12.0, INK)).padding([0, 8]).width(48).height(28).align_y(alignment::Vertical::Center).style(boxed(BG_CARD, LINE_STRONG, 1.0, 8.0)).into()
        };
        let propbar = row![
            label("遮挡样式", 12.0, MUTE), seg, label("颜色", 12.0, MUTE), swatch(BLACK, true), swatch(Color::WHITE, false), vsep(),
            label("X", 12.0, MUTE), field("412"), label("Y", 12.0, MUTE), field("142"), label("W", 12.0, MUTE), field("176"), label("H", 12.0, MUTE), field("22"),
            space::horizontal(), text_button("删除", "Del", 2),
        ].spacing(8).align_y(alignment::Vertical::Center);

        let shot = stack![
            image(image::Handle::from_bytes(include_bytes!("../../assets/shot.png").as_slice())).width(620).height(430).filter_method(image::FilterMethod::Nearest),
            canvas(Overlay).width(620).height(430),
        ];
        let framed = container(shot).padding(1).style(boxed(LINE_STRONG, LINE_STRONG, 0.0, 0.0));
        let toast = container(row![
            pill("✓ 已复制", PILL_DONE, false),
            column![label("已复制到剪贴板", 13.0, INK), label("620 × 430 · 已遮挡 2 处", 12.0, MUTE)],
        ].spacing(10).align_y(alignment::Vertical::Center)).padding([10, 14]).width(280).style(boxed(BG_CARD, LINE_STRONG, 1.0, 12.0));
        let work = stack![
            container(framed).center(Length::Fill).style(boxed(BG_WORK, BG_WORK, 0.0, 0.0)),
            container(toast).align_right(Length::Fill).align_bottom(Length::Fill).padding(16),
        ];

        let zoom = row![
            label("适应", 12.0, MUTE), keycap("Ctrl 0", false),
            container(mono("100%", 12.0, INK)).padding([0, 8]).height(22).align_y(alignment::Vertical::Center).style(boxed(BG_PRESSED, BG_PRESSED, 0.0, 6.0)),
            label("−", 12.0, MUTE), label("+", 12.0, MUTE),
        ].spacing(8).align_y(alignment::Vertical::Center);
        let status = row![mono("620 × 430", 12.0, MUTE), pill("已遮挡 2 处", PILL_SECURE, true), space::horizontal(), zoom]
            .spacing(12).align_y(alignment::Vertical::Center);

        column![
            bar(toolbar, 51.0, BG_CANVAS), hline(),
            bar(propbar, 39.0, BG_SOFT), hline(),
            container(work).height(Length::Fill),
            hline(), bar(status, 29.0, BG_CANVAS),
        ].into()
    }
}

fn main() -> iced::Result {
    let scale: f32 = std::env::var("SPIKE_SCALE").ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let consola: &'static [u8] = Box::leak(std::fs::read(r"C:\Windows\Fonts\consola.ttf").unwrap_or_default().into_boxed_slice());
    iced::application(Spike::default, |_: &mut Spike, _: Msg| {}, Spike::view)
        .title("spike-iced")
        .window_size(Size::new(1180.0, 700.0))
        .font(include_bytes!("../../../poc/native-egui/assets/fonts/NotoSansCJKsc-Regular.otf").as_slice())
        .font(consola)
        .default_font(UI)
        .theme(|_: &Spike| Theme::Light)
        .style(|_, _| iced::theme::Style { background_color: BG_CANVAS, text_color: INK })
        .scale_factor(move |_| scale)
        .run()
}
