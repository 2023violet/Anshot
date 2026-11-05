//! Capture history: the exact PNG bytes the editor copied or saved — the image as it was
//! shared, with whatever was or was not covered — kept unencrypted in
//! `%LOCALAPPDATA%\Anshot\history`. Frozen screens, source files and editing state are
//! never written here; pins and history re-copies are not recorded.
//! `ANSHOT_HISTORY_DIR` overrides the folder (tests).
#![forbid(unsafe_code)]
use crate::i18n::{t, tf};
use anshot_core::MAX_INPUT_BYTES;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Only files named like this are ours: `anshot-<unix ms>-<seq>.png` (0.2.0 wrote `sr-…`).
const PREFIX: &str = "anshot-";
const PREFIX_020: &str = "sr-";
/// Never list (or decode thumbnails for) more than the largest history limit.
const MAX_LISTED: usize = 200;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry { pub path: PathBuf, pub millis: u64 }

pub struct History { dir: PathBuf }

impl History {
    pub fn at(dir: PathBuf) -> Self { Self { dir } }
    pub fn default_dir() -> Option<PathBuf> {
        std::env::var_os("ANSHOT_HISTORY_DIR").map(PathBuf::from)
            .or_else(|| std::env::var_os("LOCALAPPDATA").map(|a| PathBuf::from(a).join("Anshot").join("history")))
    }

    /// Store a delivered PNG, then keep only the newest `limit` entries.
    pub fn add(&self, png: &[u8], limit: usize) -> Result<PathBuf, String> {
        if png.len() > MAX_INPUT_BYTES { return Err(t("图片过大，未加入历史").into()); }
        std::fs::create_dir_all(&self.dir).map_err(|e| crate::messages::write_file(e.kind()))?;
        let ms = now_millis();
        let path = (0..1000).map(|n| self.dir.join(format!("{PREFIX}{ms:013}-{n:03}.png"))).find(|p| !p.exists())
            .ok_or(t("历史文件名冲突"))?;
        anshot_platform_windows::save_new(&path, png).map_err(|e| crate::messages::write_file(e.kind()))?;
        self.prune(limit);
        Ok(path)
    }

    /// Our entries for the history dialog, newest first, at most `MAX_LISTED`. Anything else
    /// in the folder is ignored and left alone.
    pub fn list(&self) -> Vec<Entry> {
        let mut v = self.all();
        v.truncate(MAX_LISTED);
        v
    }
    /// Every entry of ours, newest first. The limits apply to all of them: more than the
    /// dialog shows can be on disk (a second instance, copied files, 0.2.0 leftovers).
    fn all(&self) -> Vec<Entry> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut v: Vec<Entry> = rd.filter_map(Result::ok).filter_map(|d| {
            let name = d.file_name().into_string().ok()?;
            let ms = entry_millis(&name)?;
            d.file_type().ok()?.is_file().then(|| Entry { path: d.path(), millis: ms })
        }).collect();
        v.sort_by(|a, b| b.millis.cmp(&a.millis).then_with(|| b.path.cmp(&a.path)));
        v
    }

    /// Keep only the newest `limit` entries. One pass: a locked file is skipped, not retried.
    pub fn prune(&self, limit: usize) {
        for e in self.all().into_iter().skip(limit) { let _ = std::fs::remove_file(&e.path); }
    }
    pub fn clear(&self) { self.prune(0); }
    /// Remove entries older than `days` days (0 = keep), judged by the time in the name.
    pub fn prune_age(&self, days: u32, now_ms: u64) {
        if days == 0 { return; }
        let cutoff = now_ms.saturating_sub(u64::from(days) * 86_400_000);
        for e in self.all().into_iter().filter(|e| e.millis < cutoff) { let _ = std::fs::remove_file(&e.path); }
    }
    pub fn remove(&self, e: &Entry) { if self.all().contains(e) { let _ = std::fs::remove_file(&e.path); } }

    /// Bounded read; the bytes still go through Core intake before use.
    pub fn read(&self, e: &Entry) -> Result<Vec<u8>, String> {
        crate::platform::read_image_file(&e.path)
    }
}

/// The time in a name Anshot wrote: `anshot-` or `sr-`, 13 digits, `-`, 3 digits, `.png`.
/// Anything else (say `sr-2024-trip.png` put there by hand) is not ours and never removed.
fn entry_millis(name: &str) -> Option<u64> {
    let rest = name.strip_prefix(PREFIX).or_else(|| name.strip_prefix(PREFIX_020))?.strip_suffix(".png")?;
    let (ms, seq) = rest.split_once('-')?;
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    (digits(ms, 13) && digits(seq, 3)).then(|| ms.parse().ok()).flatten()
}

/// Relative age for the history list ("刚刚", "5 分钟前", "3 天前"); no time zone needed.
pub fn label(millis: u64, now: u64) -> String {
    let s = now.saturating_sub(millis) / 1000;
    match s {
        0..=59 => t("刚刚").into(),
        60..=3599 => tf("{} 分钟前", &[&(s / 60)]),
        3600..=86_399 => tf("{} 小时前", &[&(s / 3600)]),
        _ => tf("{} 天前", &[&(s / 86_400)]),
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
    fn entries_older_than_the_age_limit_are_removed() {
        let dir = tmp("age");
        let h = History::at(dir.clone());
        std::fs::create_dir_all(&dir).unwrap();
        let now = 1_800_000_000_000u64;
        for (i, days) in [0u64, 2, 8, 40].iter().enumerate() {
            std::fs::write(dir.join(format!("anshot-{:013}-{i:03}.png", now - days * 86_400_000 - 1)), [i as u8]).unwrap();
        }
        h.prune_age(0, now);
        assert_eq!(h.list().len(), 4, "0 = keep");
        h.prune_age(30, now);
        assert_eq!(h.list().len(), 3);
        h.prune_age(7, now);
        assert_eq!(h.list().len(), 2);
        h.prune_age(1, now);
        assert_eq!(h.list().len(), 1);
        h.clear();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// `n` entries one minute apart ending at `newest`, alternating the 0.2.0 and current
    /// name forms, plus a file that is not ours.
    fn filled(tag: &str, n: u64, newest: u64) -> (PathBuf, History) {
        let dir = tmp(tag);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("keep-me.png"), b"not ours").unwrap();
        for i in 0..n {
            let ms = newest - i * 60_000;
            let name = if i % 2 == 0 { format!("anshot-{ms:013}-000.png") } else { format!("sr-{ms:013}-000.png") };
            std::fs::write(dir.join(name), [0u8]).unwrap();
        }
        (dir.clone(), History::at(dir))
    }
    fn ours(dir: &std::path::Path) -> Vec<u64> {
        let mut v: Vec<u64> = std::fs::read_dir(dir).unwrap().filter_map(|d| {
            let n = d.ok()?.file_name().into_string().ok()?;
            n.strip_prefix(PREFIX).or_else(|| n.strip_prefix(PREFIX_020))?.split('-').next()?.parse().ok()
        }).collect();
        v.sort_unstable_by(|a, b| b.cmp(a));
        v
    }

    #[test]
    fn the_count_limit_holds_past_the_listing_cap() {
        let now = 1_800_000_000_000u64;
        for n in [201u64, 250, 500] {
            let (dir, h) = filled(&format!("cap{n}"), n, now);
            h.prune(200);
            let left = ours(&dir);
            assert_eq!(left.len(), 200, "{n} entries, limit 200");
            assert_eq!((left[0], left[199]), (now, now - 199 * 60_000), "the newest are kept");
            h.prune(50);
            assert_eq!(ours(&dir).len(), 50);
            assert!(dir.join("keep-me.png").exists());
            std::fs::remove_dir_all(&dir).unwrap();
        }
    }

    #[test]
    fn the_age_limit_reaches_entries_past_the_listing_cap() {
        // 500 entries, one minute apart: the oldest 300 are past a 1-day limit only if the
        // clock says so; put "now" so that exactly the newest 10 are inside the last day.
        let newest = 1_800_000_000_000u64;
        let (dir, h) = filled("age500", 500, newest);
        let now = newest - 9 * 60_000 + 86_400_000 - 1;
        h.prune_age(1, now);
        assert_eq!(ours(&dir).len(), 10);
        assert!(dir.join("keep-me.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
        // 201 entries: only the very oldest (the 201st) is past the limit.
        let (dir, h) = filled("age201", 201, newest);
        h.prune_age(1, newest - 199 * 60_000 + 86_400_000 - 1);
        let left = ours(&dir);
        assert_eq!(left.len(), 200);
        assert_eq!(*left.last().unwrap(), newest - 199 * 60_000);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn clear_and_the_listing_still_work_past_the_cap() {
        let (dir, h) = filled("clear500", 500, 1_800_000_000_000);
        assert_eq!(h.list().len(), MAX_LISTED, "the dialog still lists at most the cap");
        h.clear();
        assert!(ours(&dir).is_empty());
        assert!(dir.join("keep-me.png").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn only_names_anshot_wrote_are_entries() {
        for ours in ["anshot-1800000000000-000.png", "sr-1700000000000-999.png"] { assert!(entry_millis(ours).is_some(), "{ours}"); }
        for not in ["sr-2024-trip.png", "anshot-1800000000000.png", "anshot-180000000000-000.png", "anshot-1800000000000-00.png",
                    "anshot-1800000000000-000.jpg", "anshot-18000000000x0-000.png", "copy of anshot-1800000000000-000.png"] {
            assert_eq!(entry_millis(not), None, "{not}");
        }
        let dir = tmp("names");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sr-2024-trip.png"), b"a holiday").unwrap();
        let h = History::at(dir.clone());
        h.prune_age(1, 1_800_000_000_000);
        h.clear();
        assert!(dir.join("sr-2024-trip.png").exists(), "a file that only looks like ours is left alone");
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
