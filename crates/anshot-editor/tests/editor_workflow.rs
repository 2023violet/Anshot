//! Cross-crate proof: Editor owns masks and passes exactly those pixels to Core.
use anshot_editor::{Editor, GestureKind, Handle, Viewport, Point, OperationKind};
use anshot_core::{Region,redact};
const T04:&[u8]=include_bytes!("../../../docs/research/fixtures/T04_EDGES.png");
const T05:&[u8]=include_bytes!("../../../docs/research/fixtures/T05_JPEG_EXIF.jpg");

#[test]
fn edge_mask_drag_resize_review_matches_core_bytes() {
    let mut editor=Editor::from_source(T04.to_vec()).unwrap();
    assert_eq!(editor.dimensions(),(2400,1600));
    let frame=Viewport::new(2400,1600,80.5,52.25,1200.0,800.0).unwrap();
    let rect=frame.pixel_region(Point{x:88.0,y:58.25},Point{x:555.5,y:97.25}).unwrap();
    assert_eq!(rect,Region::new(15,12,950,90));
    let id=editor.add_mask(rect).unwrap();
    let gesture=editor.begin_gesture(id,GestureKind::Move).unwrap();
    let final_preview=editor.preview_gesture(gesture,4,5).unwrap();
    assert_eq!(final_preview,Region::new(19,17,954,95));
    editor.commit_gesture(gesture).unwrap();
    editor.resize_mask(id,Handle::Se,-1,-1).unwrap();
    assert_eq!(editor.masks()[0].region,Region::new(19,17,953,94));
    let t=editor.begin(OperationKind::Review).unwrap();
    let prepared=editor.bytes_for(t).unwrap().to_vec();
    assert_eq!(prepared,redact(T04,&[editor.masks()[0].region]).unwrap());
    editor.finish(t,true);
    assert_eq!(editor.output_png(),Some(prepared.as_slice()));
    editor.undo().unwrap();
    assert!(editor.output_png().is_none());
    let new_review=editor.begin(OperationKind::Review).unwrap();
    assert_ne!(editor.bytes_for(new_review).unwrap(),prepared.as_slice());
}

#[test]
fn source_swap_changes_document_even_if_dimensions_or_region_ids_recur() {
    let mut editor=Editor::from_source(T05.to_vec()).unwrap();
    let first=editor.add_mask(Region::new(78,539,627,588)).unwrap();
    let old=editor.begin(OperationKind::Review).unwrap();
    let rev=editor.stamp();
    editor.replace_source(T05.to_vec()).unwrap();
    assert_ne!(rev.document_id,editor.stamp().document_id);
    assert_eq!(editor.stamp().revision,0);
    assert!(editor.bytes_for(old).is_none());
    assert!(!editor.finish(old,true));
    let second=editor.add_mask(Region::new(78,612,675,664)).unwrap();
    assert_ne!(first,second);
    let current=editor.begin(OperationKind::Review).unwrap();
    let bytes=editor.bytes_for(current).unwrap().to_vec();
    assert_eq!(bytes,redact(T05,&[Region::new(78,612,675,664)]).unwrap());
    editor.finish(current,true);
    let save=editor.begin(OperationKind::Save).unwrap();
    assert_eq!(editor.bytes_for(save),Some(bytes.as_slice()));
    editor.add_mask(Region::new(1,1,2,2)).unwrap();
    assert!(editor.bytes_for(save).is_none());
    assert!(!editor.finish(save,true));
}
