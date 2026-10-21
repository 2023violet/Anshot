//! User settings: capture shortcut, what happens after copy, default save folder,
//! recent custom colours.
//! Stored as `key=value` lines in `%APPDATA%\ShareReady\settings.txt`
//! (`SHAREREADY_SETTINGS_DIR` overrides the folder, used by tests).
#![forbid(unsafe_code)]
use shareready_platform_windows::Hotkey;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    pub hotkey: Hotkey,
    /// 02 U07: hide the editor after a successful copy.
    pub hide_after_copy: bool,
    pub save_dir: Option<PathBuf>,
    /// Most recent custom colours first (at most `RECENT_COLORS`).
    pub recent_colors: Vec<[u8; 4]>,
    /// How many delivered images to keep in the history (0 = history off).
    pub history_limit: usize,
}

/// Choices offered in settings for `history_limit`.
pub const HISTORY_LIMITS: [usize; 4] = [0, 20, 50, 200];

pub const RECENT_COLORS: usize = 6;

impl Default for Settings {
    fn default() -> Self { Self { hotkey: Hotkey::CtrlShiftA, hide_after_copy: true, save_dir: None, recent_colors: Vec::new(), history_limit: 50 } }
}

impl Settings {
    pub fn parse(text: &str) -> Self {
        let mut s = Self::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match (k.trim(), v.trim()) {
                ("hotkey", v) => if let Some(h) = Hotkey::ALL.iter().find(|h| h.label() == v) { s.hotkey = *h; },
                ("hide_after_copy", "true") => s.hide_after_copy = true,
                ("hide_after_copy", "false") => s.hide_after_copy = false,
                ("save_dir", "") => s.save_dir = None,
                ("save_dir", v) => s.save_dir = Some(PathBuf::from(v)),
                ("history_limit", v) => if let Some(n) = v.parse().ok().filter(|n| HISTORY_LIMITS.contains(n)) { s.history_limit = n; },
                ("recent_colors", v) => s.recent_colors = v.split(',').filter_map(crate::color::parse_hex).take(RECENT_COLORS).collect(),
                _ => {}
            }
        }
        s
    }
    pub fn render(&self) -> String {
        let recent: Vec<String> = self.recent_colors.iter().map(|c| crate::color::to_hex(*c)).collect();
        format!("hotkey={}\nhide_after_copy={}\nsave_dir={}\nrecent_colors={}\nhistory_limit={}\n", self.hotkey.label(), self.hide_after_copy,
            self.save_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_default(), recent.join(","), self.history_limit)
    }
    fn file() -> Option<PathBuf> {
        let dir = std::env::var_os("SHAREREADY_SETTINGS_DIR").map(PathBuf::from)
            .or_else(|| std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("ShareReady")))?;
        Some(dir.join("settings.txt"))
    }
    /// Remember a custom colour (most recent first, no duplicates).
    pub fn remember_color(&mut self, c: [u8; 4]) {
        self.recent_colors.retain(|x| *x != c);
        self.recent_colors.insert(0, c);
        self.recent_colors.truncate(RECENT_COLORS);
    }
    pub fn load() -> Self {
        Self::file().and_then(|f| std::fs::read_to_string(f).ok()).map(|t| Self::parse(&t)).unwrap_or_default()
    }
    pub fn save(&self) -> Result<(), String> {
        let f = Self::file().ok_or("找不到设置目录")?;
        if let Some(d) = f.parent() { std::fs::create_dir_all(d).map_err(|e| crate::messages::write_file(e.kind()))?; }
        std::fs::write(f, self.render()).map_err(|e| crate::messages::write_file(e.kind()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trips_and_ignores_unknown_or_bad_values() {
        let mut s = Settings { hotkey: Hotkey::PrintScreen, hide_after_copy: false, save_dir: Some(PathBuf::from(r"D:\截图")), recent_colors: Vec::new(), history_limit: 200 };
        for c in [[1, 2, 3, 255], [4, 5, 6, 255], [1, 2, 3, 255]] { s.remember_color(c); }
        assert_eq!(s.recent_colors, vec![[1, 2, 3, 255], [4, 5, 6, 255]]);
        assert_eq!(Settings::parse(&s.render()), s);
        let odd = Settings::parse("hotkey=Ctrl+Q\nhide_after_copy=maybe\nnoise\nfoo=bar\nhistory_limit=7\n");
        assert_eq!(odd, Settings::default());
    }
}
