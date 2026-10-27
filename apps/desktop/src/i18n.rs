//! UI language. Chinese is the source text everywhere (Slint `@tr("…")`, Rust `t("…")`);
//! English comes from one gettext file, `lang/en_US/LC_MESSAGES/anshot-desktop.po`, that
//! Slint bundles at build time and this module reads for Rust-side strings. Placeholders
//! are `{}` in both, filled in order.
#![forbid(unsafe_code)]
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

const EN_PO: &str = include_str!("../lang/en_US/LC_MESSAGES/anshot-desktop.po");
/// The bundled Slint translation's language name.
pub const SLINT_EN: &str = "en_US";

static ENGLISH: AtomicBool = AtomicBool::new(false);
static EN: LazyLock<HashMap<String, String>> = LazyLock::new(|| parse_po(EN_PO));

/// Settings value → whether the UI is English. "auto" follows the Windows UI language:
/// Chinese stays Chinese, everything else gets English.
pub fn wants_english(setting: &str, system_is_chinese: bool) -> bool {
    match setting { "en" => true, "zh" => false, _ => !system_is_chinese }
}
pub fn set_english(on: bool) { ENGLISH.store(on, Ordering::Relaxed); }
pub fn is_english() -> bool { ENGLISH.load(Ordering::Relaxed) }

/// The UI text for a Chinese source string.
pub fn t(zh: &'static str) -> &'static str { lookup(zh, is_english()) }
fn lookup(zh: &'static str, english: bool) -> &'static str {
    if !english { return zh; }
    EN.get(zh).map_or(zh, |s| s.as_str())
}
/// Like [`t`], with `{}` placeholders filled in order.
pub fn tf(zh: &'static str, args: &[&dyn Display]) -> String { fill(lookup(zh, is_english()), args) }
fn fill(template: &str, args: &[&dyn Display]) -> String {
    let mut out = String::new();
    let mut rest = template;
    for a in args {
        let Some(i) = rest.find("{}") else { break };
        out.push_str(&rest[..i]);
        out.push_str(&a.to_string());
        rest = &rest[i + 2..];
    }
    out.push_str(rest);
    out
}

fn unquote(s: &str) -> Option<String> {
    let s = s.trim().strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' { out.push(c); continue; }
        match it.next()? { 'n' => out.push('\n'), 't' => out.push('\t'), o => out.push(o) }
    }
    Some(out)
}

/// Minimal reader for the single-line `msgid "…"` / `msgstr "…"` pairs this project writes.
fn parse_po(po: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut id: Option<String> = None;
    for line in po.lines() {
        if let Some(v) = line.strip_prefix("msgid ") { id = unquote(v); }
        else if let Some(v) = line.strip_prefix("msgstr ") {
            if let (Some(k), Some(v)) = (id.take(), unquote(v)) { if !k.is_empty() && !v.is_empty() { map.insert(k, v); } }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// Every translatable string in the sources (Slint `@tr("…")`, Rust `t("…")` / `tf("…")`).
    fn source_ids() -> BTreeSet<String> {
        let root = env!("CARGO_MANIFEST_DIR");
        let mut ids = BTreeSet::new();
        let mut files: Vec<std::path::PathBuf> = Vec::new();
        for dir in ["ui", "src", "src/controller"] {
            for e in std::fs::read_dir(format!("{root}/{dir}")).unwrap().flatten() {
                let p = e.path();
                if p.extension().is_some_and(|x| x == "slint" || x == "rs") && !p.ends_with("i18n.rs") { files.push(p); }
            }
        }
        for p in files {
            let text = std::fs::read_to_string(&p).unwrap();
            for marker in ["@tr(\"", "t(\"", "tf(\""] {
                let mut rest = text.as_str();
                while let Some(i) = rest.find(marker) {
                    // `t("` must be the function, not the end of another name.
                    let before = rest[..i].chars().last();
                    rest = &rest[i + marker.len()..];
                    if marker == "t(\"" && before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') { continue; }
                    let end = rest.find('"').unwrap();
                    ids.insert(rest[..end].replace("\\\\", "\\"));
                }
            }
        }
        ids
    }

    #[test]
    fn every_string_has_english_and_none_is_stale() {
        let po: BTreeSet<String> = parse_po(EN_PO).into_keys().collect();
        let src = source_ids();
        let missing: Vec<_> = src.difference(&po).collect();
        let stale: Vec<_> = po.difference(&src).collect();
        assert!(missing.is_empty(), "no English for: {missing:?}");
        assert!(stale.is_empty(), "unused .po entries: {stale:?}");
        // Placeholders line up.
        for (k, v) in parse_po(EN_PO) { assert_eq!(k.matches("{}").count(), v.matches("{}").count(), "{k} -> {v}"); }
    }

    #[test]
    fn lookup_and_placeholders() {
        // (No global switch here: other tests run in parallel and expect Chinese.)
        assert_eq!(lookup("复制", true), "Copy");
        assert_eq!(fill(lookup("{} 分钟前", true), &[&5]), "5 min ago");
        assert_eq!(lookup("复制", false), "复制");
        assert_eq!(fill(lookup("{} 分钟前", false), &[&5]), "5 分钟前");
        assert!(wants_english("auto", false) && !wants_english("auto", true) && wants_english("en", true) && !wants_english("zh", false));
    }
}
