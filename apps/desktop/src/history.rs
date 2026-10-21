//! Capture history: the exact PNG bytes the editor copied or saved (its rendered output,
//! Rust-generated; pins and history re-copies are not recorded) kept in `%LOCALAPPDATA%\Anshot\history`. Original screen pixels
//! and unredacted sources are never written here. `SHAREREADY_HISTORY_DIR` overrides
//! the folder (tests).
#![forbid(unsafe_code)]
use shareready_core::MAX_INPUT_BYTES;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Only files named like this are ours: `sr-<unix ms>-<seq>.png`.
const PREFIX: &str = "sr-";
/// Never list (or decode thumbnails for) more than the largest history limit.
const MAX_LISTED: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry { pub path: PathBuf, pub millis: u64 }

pub struct History { dir: PathBuf }

impl History {
    pub fn at(dir: PathBuf) -> Self { Self { dir } }
    pub fn default_dir() -> Option<PathBuf> {
        std::env::var_os("SHAREREADY_HISTORY_DIR").map(PathBuf::from)
            .or_else(|| std::env::var_os("LOCALAPPDATA").map(|a| PathBuf::from(a).join("Anshot").join("history")))
    }

    /// Store a delivered PNG, then keep only the newest `limit` entries.
    pub fn add(&self, png: &[u8], limit: usize) -> Result<PathBuf, String> {
        if png.len() > MAX_INPUT_BYTES { return Err("图片过大，未加入历史".into()); }
        std::fs::create_dir_all(&self.dir).map_err(|e| crate::messages::write_file(e.kind()))?;
        let ms = now_millis();
        let path = (0..1000).map(|n| self.dir.join(format!("{PREFIX}{ms:013}-{n:03}.png"))).find(|p| !p.exists())
            .ok_or("历史文件名冲突")?;
        shareready_platform_windows::save_new(&path, png).map_err(|e| crate::messages::write_file(e.kind()))?;
        self.prune(limit);
        Ok(path)
    }

    /// Our entries, newest first. Anything else in the folder is ignored and left alone.
    pub fn list(&self) -> Vec<Entry> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut v: Vec<Entry> = rd.filter_map(Result::ok).filter_map(|d| {
            let name = d.file_name().into_string().ok()?;
            let ms = name.strip_prefix(PREFIX)?.strip_suffix(".png")?.split('-').next()?.parse().ok()?;
            d.file_type().ok()?.is_file().then(|| Entry { path: d.path(), millis: ms })
        }).collect();
        v.sort_by(|a, b| b.millis.cmp(&a.millis).then_with(|| b.path.cmp(&a.path)));
        v.truncate(MAX_LISTED);
        v
    }

    pub fn prune(&self, limit: usize) {
        // Repeat so entries beyond the listing cap are pruned too.
        loop {
            let list = self.list();
            let extra: Vec<Entry> = list.into_iter().skip(limit).collect();
            if extra.is_empty() { break; }
            for e in &extra { let _ = std::fs::remove_file(&e.path); }
            if limit >= MAX_LISTED { break; }
        }
    }
    pub fn clear(&self) { self.prune(0); }
    pub fn remove(&self, e: &Entry) { if self.list().contains(e) { let _ = std::fs::remove_file(&e.path); } }

    /// Bounded read; the bytes still go through Core intake before use.
    pub fn read(&self, e: &Entry) -> Result<Vec<u8>, String> {
        crate::platform::read_image_file(&e.path)
    }
}

/// Relative age for the history list ("刚刚", "5 分钟前", "3 天前"); no time zone needed.
pub fn label(millis: u64, now: u64) -> String {
    let s = now.saturating_sub(millis) / 1000;
    match s {
        0..=59 => "刚刚".into(),
        60..=3599 => format!("{} 分钟前", s / 60),
        3600..=86_399 => format!("{} 小时前", s / 3600),
        _ => format!("{} 天前", s / 86_400),
    }
}

pub fn now_millis() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64) }

#[cfg(test)]
mod tests {
    use super::*;
    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sr-history-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }
    #[test]
    fn keeps_newest_within_limit_and_ignores_foreign_files() {
        let dir = tmp("limit");
        let h = History::at(dir.clone());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("keep-me.png"), b"not ours").unwrap();
        for i in 0..5u8 { h.add(&[i; 8], 3).unwrap(); }
        let list = h.list();
        assert_eq!(list.len(), 3);
        assert_eq!(std::fs::read(&list[0].path).unwrap(), vec![4u8; 8], "newest first");
        h.remove(&list[0]);
        assert_eq!(h.list().len(), 2);
        h.clear();
        assert!(h.list().is_empty());
        assert!(dir.join("keep-me.png").exists(), "never touches files it did not write");
        std::fs::remove_dir_all(&dir).unwrap();
    }
    #[test]
    fn missing_folder_is_empty_and_labels_are_stable() {
        assert!(History::at(tmp("none")).list().is_empty());
        let now = 10_000_000_000;
        assert_eq!(label(now - 5_000, now), "刚刚");
        assert_eq!(label(now - 300_000, now), "5 分钟前");
        assert_eq!(label(now - 3 * 86_400_000, now), "3 天前");
        assert_eq!(label(now + 1, now), "刚刚");
    }
}
