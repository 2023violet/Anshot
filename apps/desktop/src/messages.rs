//! User-facing wording for failures. Raw error text (Rust Debug output, OS error codes,
//! decoder internals) never reaches the screen.
#![forbid(unsafe_code)]
use anshot_editor::EditError;
use std::io::ErrorKind;

/// Why an image could not be taken in (open, paste, drop, capture).
pub fn intake(e: &EditError) -> String {
    match e {
        EditError::CoreFailure(m) if m.contains("oversized") => "文件超过 25 MB".into(),
        EditError::CoreFailure(m) if m.contains("bounds") || m.contains("dimension") => "图片超过 2400 万像素".into(),
        EditError::CoreFailure(m) if m.contains("magic") => "格式不支持（仅 PNG / JPEG）".into(),
        EditError::InvalidImageDimensions => "图片尺寸无效".into(),
        _ => "不是有效的 PNG / JPEG 图片，或文件已损坏".into(),
    }
}

/// Core's decoder reports plain strings; map them the same way.
pub fn decode(m: &str) -> String { intake(&EditError::CoreFailure(m.to_string())) }

pub fn read_file(kind: ErrorKind) -> String {
    match kind {
        ErrorKind::NotFound => "找不到这个文件，可能已被移动或删除".into(),
        ErrorKind::PermissionDenied => "没有读取这个文件的权限".into(),
        _ => "无法读取这个文件".into(),
    }
}

pub fn write_file(kind: ErrorKind) -> String {
    match kind {
        ErrorKind::AlreadyExists => "同名文件已存在，请换个名字（不会覆盖原文件）".into(),
        ErrorKind::PermissionDenied => "没有写入这个文件夹的权限，请换个位置".into(),
        ErrorKind::NotFound => "保存位置不存在，请重新选择".into(),
        _ => "无法写入这个位置，请换个文件夹再试".into(),
    }
}

/// Failures while preparing or delivering an edit.
pub fn edit(e: &EditError) -> String {
    match e {
        EditError::NoEffectiveMask => "还没有遮挡".into(),
        EditError::OperationPending => "请等上一次复制或保存完成".into(),
        EditError::GestureInProgress => "请先松开鼠标或按 Esc".into(),
        EditError::MaskLimit => "遮挡数量已达上限".into(),
        EditError::MissingSource => "还没有打开图片".into(),
        EditError::StaleReview => "内容已变化，请重新复制或保存".into(),
        _ => "内部状态异常，请重试；如反复出现请重新打开图片".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clean(s: &str) { for bad in ["CoreFailure", "os error", "{", "Error", "chunk"] { assert!(!s.contains(bad), "{s}"); } }
    #[test]
    fn no_raw_error_text_reaches_the_user() {
        let raw = EditError::CoreFailure("invalid image header: Format error decoding Png: ChunkType { type: real } chunk appeared before IHDR chunk".into());
        assert_eq!(intake(&raw), "不是有效的 PNG / JPEG 图片，或文件已损坏");
        assert_eq!(decode("invalid or oversized compressed input"), "文件超过 25 MB");
        for e in [raw, EditError::StaleReview, EditError::IdentityOverflow, EditError::WrongGesture] { clean(&intake(&e)); clean(&edit(&e)); }
        for k in [ErrorKind::NotFound, ErrorKind::PermissionDenied, ErrorKind::Other, ErrorKind::AlreadyExists] { clean(&read_file(k)); clean(&write_file(k)); }
    }
}
