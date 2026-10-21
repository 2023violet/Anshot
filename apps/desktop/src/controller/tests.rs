use super::*;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use std::io::Cursor;

fn png(w: u32, h: u32) -> Vec<u8> {
    let img = RgbaImage::from_fn(w, h, |x, y| Rgba([(x % 256) as u8, (y % 256) as u8, 200, 255]));
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(img).write_to(&mut out, ImageFormat::Png).unwrap();
    out.into_inner()
}
fn opened(w: u32, h: u32) -> Controller {
    let mut c = Controller::default();
    c.set_area(1000.0, 600.0);
    c.accept_source(SourceCandidate::decode(png(w, h)).unwrap(), Some("t.png".into())).unwrap();
    c
}
fn at(c: &Controller, sx: f64, sy: f64) -> Point {
    let v = c.view().unwrap();
    Point { x: v.x + sx * v.scale, y: v.y + sy * v.scale }
}
fn drag(c: &mut Controller, a: (f64, f64), b: (f64, f64)) {
    c.pointer_down(at(c, a.0, a.1));
    c.pointer_move(at(c, b.0, b.1));
    c.pointer_up();
}
fn editor(c: &Controller) -> &Editor { c.editor.as_ref().unwrap() }

#[test]
fn draw_move_resize_undo_through_pointer_intents() {
    let mut c = opened(400, 200); // scale 1
    c.pointer_down(at(&c, 10.0, 20.0));
    c.pointer_move(at(&c, 60.0, 50.0));
    assert_eq!(c.display_scene().redactions[0].region, Region::new(10, 20, 60, 50));
    c.pointer_up();
    assert_eq!(editor(&c).masks()[0].region, Region::new(10, 20, 60, 50));
    assert_eq!(editor(&c).masks()[0].style, RedactStyle::Solid(SOLIDS[0]));
    assert!(c.has_unsaved_edits());
    drag(&mut c, (30.0, 30.0), (40.0, 35.0));
    assert_eq!(editor(&c).masks()[0].region, Region::new(20, 25, 70, 55));
    // Resize from the Se handle, grabbing 6px off the corner.
    drag(&mut c, (76.0, 61.0), (86.0, 71.0));
    assert_eq!(editor(&c).masks()[0].region, Region::new(20, 25, 80, 65));
    assert!(c.undo());
    assert_eq!(editor(&c).masks()[0].region, Region::new(20, 25, 70, 55));
    assert!(c.redo());
    assert!(c.nudge(1, 0));
    assert_eq!(editor(&c).masks()[0].region, Region::new(21, 25, 81, 65));
    assert!(c.delete_selected());
    assert!(editor(&c).masks().is_empty());
}

#[test]
fn every_annotation_tool_creates_one_object_and_select_tool_never_draws() {
    let mut c = opened(400, 300);
    c.pointer_down(at(&c, 10.0, 10.0));
    c.pointer_up();
    assert!(editor(&c).masks().is_empty(), "click without drag adds nothing");
    for (tool, kind) in [(Tool::Arrow, "arrow"), (Tool::Rect, "rect"), (Tool::Highlight, "hl"), (Tool::Mosaic, "mosaic")] {
        c.tool = tool;
        let y = 20.0 + 60.0 * editor(&c).annotations().len() as f64;
        drag(&mut c, (200.0, y), (260.0, y + 30.0));
        let _ = kind;
    }
    assert_eq!(editor(&c).annotations().len(), 3);
    assert_eq!(editor(&c).masks()[0].style, RedactStyle::SafeMosaic);
    c.tool = Tool::Number;
    c.pointer_down(at(&c, 20.0, 250.0)); c.pointer_up();
    c.pointer_down(at(&c, 60.0, 250.0)); c.pointer_up();
    let numbers: Vec<u32> = editor(&c).annotations().iter().filter_map(|a| match a.annotation.shape { Shape::Number { n, .. } => Some(n), _ => None }).collect();
    assert_eq!(numbers, vec![1, 2]);
    c.tool = Tool::Select;
    let before = editor(&c).annotations().len();
    drag(&mut c, (5.0, 150.0), (100.0, 190.0));
    assert_eq!(editor(&c).annotations().len(), before);
}

#[test]
fn annotations_move_resize_and_restyle_with_single_undo_entries() {
    let mut c = opened(400, 300);
    c.tool = Tool::Rect;
    drag(&mut c, (50.0, 50.0), (150.0, 120.0));
    let id = editor(&c).selected().unwrap();
    let rev = editor(&c).stamp().revision;
    // Body drag moves the rectangle; the live preview never touches history.
    c.pointer_down(at(&c, 100.0, 85.0)); // interior, away from the 16px grips
    c.pointer_move(at(&c, 120.0, 95.0));
    assert_eq!(editor(&c).stamp().revision, rev);
    c.pointer_up();
    match &editor(&c).annotations()[0].annotation.shape { Shape::Rect { region, .. } => assert_eq!(*region, Region::new(70, 60, 170, 130)), s => panic!("{s:?}") }
    // Se grip resizes.
    drag(&mut c, (170.0, 130.0), (190.0, 140.0));
    match &editor(&c).annotations()[0].annotation.shape { Shape::Rect { region, .. } => assert_eq!(*region, Region::new(70, 60, 190, 140)), s => panic!("{s:?}") }
    // Restyle applies to the selection.
    assert!(c.apply_style(|s| { s.color = 3; s.width = 2; }));
    let a = &editor(&c).annotations()[0].annotation;
    assert_eq!(a.color, INKS[3]);
    assert!(matches!(a.shape, Shape::Rect { width, .. } if width == WIDTHS[2]));
    assert_eq!(editor(&c).selected(), Some(id));
    assert!(c.undo()); assert!(c.undo());
    match &editor(&c).annotations()[0].annotation.shape { Shape::Rect { region, .. } => assert_eq!(*region, Region::new(70, 60, 170, 130)), s => panic!("{s:?}") }
}

#[test]
fn arrow_endpoints_are_independent_grips() {
    let mut c = opened(400, 300);
    c.tool = Tool::Arrow;
    drag(&mut c, (50.0, 50.0), (200.0, 100.0));
    drag(&mut c, (200.0, 100.0), (220.0, 150.0)); // To grip
    match editor(&c).annotations()[0].annotation.shape { Shape::Arrow { from, to, .. } => { assert_eq!(from, (50.0, 50.0)); assert_eq!(to, (220.0, 150.0)); } _ => panic!() }
}

#[test]
fn text_commit_edit_and_empty_removal() {
    let mut c = opened(400, 300);
    c.tool = Tool::Text;
    c.pointer_down(at(&c, 40.0, 40.0));
    assert!(c.text_edit.is_some());
    assert!(!c.can_deliver(), "no delivery while typing");
    assert!(c.commit_text("你好 OK"));
    let id = editor(&c).annotations()[0].id;
    assert!(c.edit_text_at(at(&c, 45.0, 48.0)));
    assert_eq!(c.text_edit.as_ref().unwrap().initial, "你好 OK");
    assert!(c.display_scene().annotations.is_empty(), "text being edited is hidden from the canvas");
    assert!(c.commit_text("改了"));
    assert!(matches!(&editor(&c).annotations()[0].annotation.shape, Shape::Text { text, .. } if text == "改了"));
    assert!(c.edit_text_at(at(&c, 45.0, 48.0)));
    assert!(c.commit_text("  "));
    assert!(editor(&c).annotations().iter().all(|a| a.id != id));
}

#[test]
fn delivery_writes_the_rendered_png_even_without_redactions() {
    let mut c = opened(64, 48);
    c.tool = Tool::Rect;
    drag(&mut c, (4.0, 4.0), (40.0, 30.0));
    assert_eq!(c.redaction_count(), 0);
    let work = c.begin_delivery(Delivery::Copy).unwrap();
    let (_, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
    let mut red = 0;
    c.deliver(ticket, |bytes, dims| {
        let img = shareready_core::decode_generated_png(bytes, dims)?;
        red = img.pixels().filter(|p| p.0 == INKS[0]).count();
        Ok(())
    }).unwrap();
    assert!(red > 50, "rect stroke pixels {red}");
    assert!(!c.has_unsaved_edits());
}

#[test]
fn delivered_redaction_pixels_match_the_canvas_style() {
    let mut c = opened(64, 48);
    c.editor.as_mut().unwrap().add_redaction(Region::new(8, 8, 24, 16), RedactStyle::Solid(SOLIDS[0])).unwrap();
    let work = c.begin_delivery(Delivery::Copy).unwrap();
    let (_, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
    let mut seen = None;
    c.deliver(ticket, |bytes, dims| {
        let img = shareready_core::decode_generated_png(bytes, dims)?;
        seen = Some((img.get_pixel(8, 8).0, img.get_pixel(23, 15).0, img.get_pixel(24, 16).0));
        Ok(())
    }).unwrap();
    assert_eq!(seen, Some((SOLIDS[0], SOLIDS[0], [24, 16, 200, 255])));
}

#[test]
fn edit_during_render_discards_the_stale_result() {
    let mut c = opened(64, 48);
    c.editor.as_mut().unwrap().add_mask(Region::new(8, 8, 24, 16)).unwrap();
    let work = c.begin_delivery(Delivery::Save).unwrap();
    let key = work.key();
    let result = work.run();
    let id = editor(&c).masks()[0].id;
    c.editor.as_mut().unwrap().move_mask(id, 1, 0).unwrap();
    assert_eq!(c.review_finished(&key, result), Err(Notice::Changed));
    assert!(c.has_unsaved_edits() && c.can_deliver());
}

#[test]
fn an_older_worker_result_cannot_consume_or_strand_the_current_render() {
    let mut c = opened(64, 48);
    c.editor.as_mut().unwrap().add_mask(Region::new(0, 0, 4, 4)).unwrap();
    let old = c.begin_delivery(Delivery::Copy).unwrap();
    let (old_key, old_result) = (old.key(), old.run());
    c.accept_source(SourceCandidate::decode(png(32, 32)).unwrap(), None).unwrap();
    c.editor.as_mut().unwrap().add_mask(Region::new(1, 1, 5, 5)).unwrap();
    let current = c.begin_delivery(Delivery::Copy).unwrap();
    assert_eq!(c.review_finished(&old_key, old_result), Err(Notice::Ignored));
    assert_eq!(c.busy(), Some(Delivery::Copy));
    let (_, ticket) = c.review_finished(&current.key(), current.run()).unwrap();
    assert!(c.deliver(ticket, |_, dims| { assert_eq!(dims, (32, 32)); Ok(()) }).is_ok());
    assert!(c.can_deliver());
}

#[test]
fn failed_write_keeps_edits_unsaved_and_allows_retry() {
    let mut c = opened(64, 48);
    c.editor.as_mut().unwrap().add_mask(Region::new(0, 0, 4, 4)).unwrap();
    let work = c.begin_delivery(Delivery::Copy).unwrap();
    let (_, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
    assert_eq!(c.deliver(ticket, |_, _| Err("busy".into())), Err("busy".into()));
    assert!(c.has_unsaved_edits());
    let work = c.begin_delivery(Delivery::Copy).unwrap();
    let (_, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
    assert!(c.deliver(ticket, |_, _| Ok(())).is_ok());
    assert!(!c.has_unsaved_edits());
}

#[test]
fn replacing_the_source_resets_history_and_delivery_state() {
    let mut c = opened(64, 48);
    c.editor.as_mut().unwrap().add_mask(Region::new(0, 0, 4, 4)).unwrap();
    c.accept_source(SourceCandidate::decode(png(32, 32)).unwrap(), None).unwrap();
    let e = editor(&c);
    assert!(e.masks().is_empty() && e.annotations().is_empty() && !e.can_undo());
    assert_eq!(e.dimensions(), (32, 32));
    assert!(!c.has_unsaved_edits());
}

#[test]
fn handle_hit_box_is_16px_half_open_and_beats_bodies() {
    let mut c = opened(400, 200); // scale 1
    let a = c.editor.as_mut().unwrap().add_mask(Region::new(100, 50, 200, 100)).unwrap();
    c.editor.as_mut().unwrap().add_mask(Region::new(150, 80, 260, 150)).unwrap();
    c.editor.as_mut().unwrap().select(Some(a)).unwrap();
    assert_eq!(c.hit(at(&c, 200.0, 100.0)), Hit::Grip(a, Grip::Box(Handle::Se)));
    assert_eq!(c.hit(at(&c, 207.9, 107.9)), Hit::Grip(a, Grip::Box(Handle::Se)));
    assert!(matches!(c.hit(at(&c, 208.0, 100.0)), Hit::Mask(id) if id != a));
    c.deselect();
    assert!(matches!(c.hit(at(&c, 200.0, 100.0)), Hit::Mask(id) if id != a), "topmost body wins without selection");
    assert_eq!(c.hit(at(&c, 10.0, 10.0)), Hit::Empty);
}

#[test]
fn crop_drafts_apply_on_confirm_and_shape_the_export() {
    let mut c = opened(200, 100);
    c.enter_crop();
    drag(&mut c, (20.0, 10.0), (120.0, 70.0));
    assert_eq!(c.crop_box(), Some(Region::new(20, 10, 120, 70)));
    assert_eq!(editor(&c).crop(), None, "draft only");
    assert!(c.confirm_crop());
    assert_eq!(editor(&c).output_dimensions(), (100, 60));
    let work = c.begin_delivery(Delivery::Copy).unwrap();
    let (_, ticket) = c.review_finished(&work.key(), work.run()).unwrap();
    c.deliver(ticket, |bytes, dims| { assert_eq!(dims, (100, 60)); shareready_core::decode_generated_png(bytes, dims).map(|_| ()) }).unwrap();
    c.enter_crop();
    c.cancel_crop();
    assert_eq!(editor(&c).crop(), Some(Region::new(20, 10, 120, 70)));
    assert!(c.undo());
    assert_eq!(editor(&c).crop(), None);
}

#[test]
fn a_large_highlight_does_not_hide_the_redaction_beneath_it() {
    let mut c = opened(400, 300);
    let m = c.editor.as_mut().unwrap().add_mask(Region::new(100, 100, 140, 120)).unwrap();
    c.tool = Tool::Highlight;
    drag(&mut c, (0.0, 0.0), (400.0, 300.0));
    c.deselect();
    assert_eq!(c.hit(at(&c, 110.0, 110.0)), Hit::Mask(m));
    assert!(matches!(c.hit(at(&c, 300.0, 250.0)), Hit::Ann(_)));
}

#[test]
fn zoom_at_pointer_keeps_the_source_point_fixed_and_space_drag_pans() {
    let mut c = opened(2000, 1500);
    let p = at(&c, 1000.0, 750.0);
    c.zoom_by(4.0, p);
    let v = c.view().unwrap();
    let back = ((p.x - v.x) / v.scale, (p.y - v.y) / v.scale);
    assert!((back.0 - 1000.0).abs() < 1.0 && (back.1 - 750.0).abs() < 1.0, "{back:?}");
    c.space_pan = true;
    let before = c.view().unwrap();
    c.pointer_down(Point { x: 500.0, y: 300.0 });
    c.pointer_move(Point { x: 460.0, y: 280.0 });
    c.pointer_up();
    let after = c.view().unwrap();
    assert_eq!((after.x - before.x, after.y - before.y), (-40.0, -20.0));
    assert!(editor(&c).masks().is_empty(), "panning never draws");
    c.zoom_to(Zoom::Fit);
    assert!(c.view().unwrap().scale < 1.0);
}

#[test]
fn tab_cycles_through_all_objects() {
    let mut c = opened(200, 200);
    let a = c.editor.as_mut().unwrap().add_mask(Region::new(0, 0, 10, 10)).unwrap();
    let b = c.editor.as_mut().unwrap().add_annotation(Annotation { shape: Shape::Highlight { region: Region::new(20, 20, 40, 40) }, color: INKS[1] }).unwrap();
    c.deselect();
    assert!(c.cycle_selection(false)); assert_eq!(editor(&c).selected(), Some(a));
    assert!(c.cycle_selection(false)); assert_eq!(editor(&c).selected(), Some(b));
    assert!(c.cycle_selection(false)); assert_eq!(editor(&c).selected(), Some(a));
    assert!(c.cycle_selection(true)); assert_eq!(editor(&c).selected(), Some(b));
}

#[test]
fn keyboard_resize_moves_the_far_edge_of_masks_and_frames() {
    let mut c = opened(200, 200);
    let m = c.editor.as_mut().unwrap().add_mask(Region::new(10, 10, 50, 40)).unwrap();
    assert!(c.resize_selected(5, -3));
    assert_eq!(editor(&c).masks().iter().find(|x| x.id == m).unwrap().region, Region::new(10, 10, 55, 37));
    c.tool = Tool::Rect;
    drag(&mut c, (100.0, 100.0), (150.0, 140.0));
    assert!(c.resize_selected(10, 10));
    assert!(matches!(editor(&c).annotations()[0].annotation.shape, Shape::Rect { region, .. } if region == Region::new(100, 100, 160, 150)));
}

#[test]
fn shift_squares_boxes_snaps_arrows_and_locks_moves() {
    let mut c = opened(400, 200);
    c.shift = true;
    drag(&mut c, (10.0, 10.0), (60.0, 30.0)); // redaction: larger side wins
    assert_eq!(editor(&c).masks()[0].region, Region::new(10, 10, 60, 60));
    c.tool = Tool::Arrow;
    drag(&mut c, (100.0, 100.0), (150.0, 108.0)); // ~9° snaps to horizontal
    match editor(&c).annotations()[0].annotation.shape {
        Shape::Arrow { from, to, .. } => { assert_eq!(from, (100.0, 100.0)); assert!((to.1 - 100.0).abs() < 0.01 && to.0 > 150.0); }
        _ => panic!("arrow expected"),
    }
    c.tool = Tool::Select;
    drag(&mut c, (20.0, 20.0), (50.0, 26.0)); // move the redaction: horizontal only
    assert_eq!(editor(&c).masks()[0].region, Region::new(40, 10, 90, 60));
    c.shift = false;
    drag(&mut c, (50.0, 20.0), (60.0, 30.0));
    assert_eq!(editor(&c).masks()[0].region, Region::new(50, 20, 100, 70));
}

#[test]
fn constraint_helpers_keep_direction() {
    let p = |x, y| Point { x, y };
    assert_eq!(super::square(p(10.0, 10.0), p(0.0, 40.0)), p(-20.0, 40.0));
    assert_eq!(super::lock_axis(p(0.0, 0.0), p(3.0, -9.0)), p(0.0, -9.0));
    let d = super::snap45(p(0.0, 0.0), p(10.0, 9.0));
    assert!((d.x - d.y).abs() < 1e-9 && d.x > 0.0);
}

#[test]
fn crop_draft_can_be_resized_by_handle_and_moved_from_inside() {
    let mut c = opened(400, 200);
    c.enter_crop();
    drag(&mut c, (100.0, 50.0), (200.0, 150.0));
    assert_eq!(c.crop_draft, Some(Region::new(100, 50, 200, 150)));
    assert_eq!(c.grips().len(), 8, "crop shows eight handles");
    drag(&mut c, (200.0, 150.0), (240.0, 170.0)); // south-east handle
    assert_eq!(c.crop_draft, Some(Region::new(100, 50, 240, 170)));
    drag(&mut c, (150.0, 100.0), (130.0, 90.0)); // inside: move
    assert_eq!(c.crop_draft, Some(Region::new(80, 40, 220, 160)));
    drag(&mut c, (10.0, 10.0), (30.0, 30.0)); // outside: a new crop
    assert_eq!(c.crop_draft, Some(Region::new(10, 10, 30, 30)));
    assert!(c.confirm_crop());
    assert_eq!(editor(&c).output_dimensions(), (20, 20));
}

#[test]
fn ellipse_line_and_pen_tools_create_their_shapes() {
    let mut c = opened(400, 200);
    c.tool = Tool::Ellipse;
    drag(&mut c, (10.0, 10.0), (60.0, 40.0));
    assert!(matches!(editor(&c).annotations()[0].annotation.shape, Shape::Ellipse { region, .. } if region == Region::new(10, 10, 60, 40)));
    c.tool = Tool::Line;
    c.shift = true;
    drag(&mut c, (100.0, 100.0), (150.0, 104.0)); // Shift snaps the line flat
    match editor(&c).annotations()[1].annotation.shape {
        Shape::Line { from, to, .. } => assert!(from == (100.0, 100.0) && (to.1 - 100.0).abs() < 0.01),
        _ => panic!("line expected"),
    }
    c.shift = false;
    c.tool = Tool::Pen;
    c.pointer_down(at(&c, 200.0, 50.0));
    for i in 1..=20 { c.pointer_move(at(&c, 200.0 + i as f64 * 3.0, 50.0 + (i % 4) as f64)); }
    c.pointer_up();
    match &editor(&c).annotations()[2].annotation.shape {
        Shape::Pen { points, .. } => { assert!(points.len() >= 10, "{}", points.len()); assert_eq!(points[0], (200.0, 50.0)); }
        _ => panic!("pen expected"),
    }
    // The pen stroke can be hit and moved as a whole.
    c.tool = Tool::Select;
    drag(&mut c, (230.0, 51.0), (230.0, 81.0));
    match &editor(&c).annotations()[2].annotation.shape { Shape::Pen { points, .. } => assert_eq!(points[0], (200.0, 80.0)), _ => unreachable!() }
}

#[test]
fn shape_pairs_convert_and_custom_colour_applies() {
    let mut c = opened(400, 200);
    c.tool = Tool::Rect;
    drag(&mut c, (10.0, 10.0), (60.0, 40.0)); // the new frame stays selected
    assert!(c.set_shape_tool(Tool::Ellipse));
    assert!(matches!(editor(&c).annotations()[0].annotation.shape, Shape::Ellipse { .. }));
    assert!(c.set_custom_color([1, 2, 3, 255]));
    assert_eq!(editor(&c).annotations()[0].annotation.color, [1, 2, 3, 255]);
    assert_eq!((c.style.color, c.ink()), (CUSTOM_INK, [1, 2, 3, 255]));
    assert!(c.undo());
    assert_eq!(editor(&c).annotations()[0].annotation.color, INKS[0]);
    // Selecting a custom-coloured object adopts its colour.
    c.style.color = 0;
    c.undo(); c.redo(); c.redo();
    c.deselect();
    c.tool = Tool::Select;
    drag(&mut c, (11.0, 25.0), (11.0, 25.0));
    assert_eq!((c.style.color, c.custom), (CUSTOM_INK, [1, 2, 3, 255]));
}
