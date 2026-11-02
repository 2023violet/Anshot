//! Format rules over recognised text: finds what looks like personal data and where it is.
//! Pure logic on words someone else recognised. The rules tolerate misreads because OCR
//! splits digit runs and reads `0` as `e`; a tolerant hit is `Possible`, never `Confirmed`.
//! Measured limits: `spike/ocr/RESULT.md`. Found text may contain misreads.

/// One recognised word and its box in image pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct TextWord { pub text: String, pub x: f32, pub y: f32, pub w: f32, pub h: f32 }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SensitiveKind { Phone, IdCard, BankCard, Email, Ipv4, ApiKey, SameText }

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Certainty { Possible, Confirmed }

/// `bounds` is `(x0, y0, x1, y1)`: the union of the words that hold the text.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding { pub kind: SensitiveKind, pub certainty: Certainty, pub text: String, pub bounds: (f32, f32, f32, f32) }

const KEY_PREFIXES: [&str; 10] = ["sk-", "ghp_", "gho_", "ghs_", "xoxb-", "xoxp-", "xoxs-", "AKIA", "AKlA", "AK1A"];
const KEY_MIN_TAIL: usize = 8;
/// Characters the engine reads in place of a digit.
const DIGIT_LOOKALIKES: &str = "OolI|eScgBZ";
/// Units that follow numbers in ordinary text; never a misread dot.
const NUMBER_UNITS: &str = "天时分秒月日年号室厅卫平元万亿个条次人岁点";
/// Words this far apart (in line heights) are columns, not one value.
const COLUMN_GAP: f32 = 2.0;

/// Full-width forms to ASCII; everything else stays.
fn fold(c: char) -> char {
    match c as u32 { 0xFF01..=0xFF5E => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c), _ => c }
}
fn is_mail_char(c: char) -> bool { c.is_ascii_alphanumeric() || "._%+-".contains(c) }
fn is_domain_char(c: char) -> bool { c.is_ascii_alphanumeric() || ".-".contains(c) }
fn is_key_char(c: char) -> bool { c.is_ascii_alphanumeric() || "_-".contains(c) }

/// Characters of a line (words joined without spaces, as OCR splits at random) and the word
/// each came from.
struct Line<'a> { chars: Vec<char>, owner: Vec<usize>, words: &'a [TextWord] }
impl<'a> Line<'a> {
    fn new(words: &'a [TextWord]) -> Self {
        let (mut chars, mut owner) = (Vec::new(), Vec::new());
        for (i, w) in words.iter().enumerate() {
            if let Some(p) = i.checked_sub(1).map(|p| &words[p]) {
                if w.x - (p.x + p.w) > COLUMN_GAP * w.h.max(p.h) { chars.push('\t'); owner.push(i); }
            }
            for c in w.text.chars().map(fold).filter(|c| !c.is_whitespace()) { chars.push(c); owner.push(i); }
        }
        Self { chars, owner, words }
    }
    fn finding(&self, kind: SensitiveKind, certainty: Certainty, s: usize, e: usize) -> Finding {
        let mut b = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for w in self.owner[s..e].iter().map(|&i| &self.words[i]) {
            b = (b.0.min(w.x), b.1.min(w.y), b.2.max(w.x + w.w), b.3.max(w.y + w.h));
        }
        Finding { kind, certainty, text: self.chars[s..e].iter().collect(), bounds: b }
    }
    fn digits(&self, s: usize, e: usize) -> bool { self.chars[s..e].iter().all(char::is_ascii_digit) }
}

fn luhn(digits: &[char]) -> bool {
    let sum: u32 = digits.iter().rev().enumerate().map(|(i, c)| {
        let d = c.to_digit(10).unwrap_or(0);
        if i % 2 == 1 { if d > 4 { d * 2 - 9 } else { d * 2 } } else { d }
    }).sum();
    sum % 10 == 0
}
fn id_checksum(chars: &[char]) -> bool {
    const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
    const CHECK: [char; 11] = ['1', '0', 'X', '9', '8', '7', '6', '5', '4', '3', '2'];
    if chars.len() != 18 || !chars[..17].iter().all(char::is_ascii_digit) { return false; }
    let sum: u32 = chars[..17].iter().zip(WEIGHTS).map(|(c, w)| c.to_digit(10).unwrap_or(0) * w).sum();
    chars[17].to_ascii_uppercase() == CHECK[(sum % 11) as usize]
}

/// Digit runs with at most 2 misread characters inside; both ends are real digits.
fn numbers(l: &Line, out: &mut Vec<Finding>) {
    let (c, mut i) = (&l.chars, 0);
    while i < c.len() {
        if !c[i].is_ascii_digit() { i += 1; continue; }
        let (mut j, mut bad, mut end, mut real) = (i, 0, i, 0);
        while j < c.len() {
            if c[j].is_ascii_digit() { real += 1; end = j; }
            else if DIGIT_LOOKALIKES.contains(c[j]) || !c[j].is_ascii() { bad += 1; if bad > 2 { break; } }
            else { break; }
            j += 1;
        }
        let mut e = end + 1;
        // An ID's check character can be X.
        if e - i == 17 && c.get(e).is_some_and(|x| matches!(x, 'X' | 'x')) { e += 1; }
        let before = out.len();
        classify_number(l, i, e, real, out);
        // Nothing fits the run as a whole: the characters inside were ordinary text, not
        // misreads, so read the numbers on either side on their own.
        if out.len() == before {
            let mut s = i;
            while s < e {
                let n = (s..e).find(|&x| !c[x].is_ascii()).unwrap_or(e);
                if n > s { classify_number(l, s, n, n - s, out); }
                s = n + 1;
            }
        }
        i = e;
    }
}
fn classify_number(l: &Line, s: usize, e: usize, real: usize, out: &mut Vec<Finding>) {
    use {Certainty::*, SensitiveKind::*};
    let (len, clean) = (e - s, l.digits(s, e));
    let run = &l.chars[s..e];
    if clean && len == 13 && run[..2] == ['8', '6'] && run[2] == '1' {
        out.push(l.finding(Phone, Confirmed, s, e));
    } else if real >= 9 && (11..=13).contains(&len) && run[0] == '1' && (!clean || len == 11) {
        let exact = clean && len == 11 && ('3'..='9').contains(&run[1]);
        out.push(l.finding(Phone, if exact { Confirmed } else { Possible }, s, e));
    } else if id_checksum(run) {
        out.push(l.finding(IdCard, Confirmed, s, e));
    } else if clean && (16..=19).contains(&len) && luhn(run) {
        out.push(l.finding(BankCard, Confirmed, s, e));
    } else if real >= 13 && (15..=20).contains(&len) {
        out.push(l.finding(if len == 18 { IdCard } else { BankCard }, Possible, s, e));
    }
}

/// `138-0013-8000`: hyphens anywhere else are dates and part numbers.
fn dashed_phone(l: &Line, out: &mut Vec<Finding>) {
    let c = &l.chars;
    for s in 0..c.len().saturating_sub(12) {
        let shape = (0..13).all(|i| if i == 3 || i == 8 { c[s + i] == '-' } else { c[s + i].is_ascii_digit() });
        let edge = |i: Option<&char>| i.is_some_and(|x| x.is_ascii_digit() || *x == '-');
        if shape && c[s] == '1' && !edge(s.checked_sub(1).map(|p| &c[p])) && !edge(c.get(s + 13)) {
            out.push(l.finding(SensitiveKind::Phone, Certainty::Confirmed, s, s + 13));
        }
    }
}

/// `a.b.c.d`, where the engine may put a non-Latin mark in place of a dot.
fn ipv4(l: &Line, out: &mut Vec<Finding>) {
    let c = &l.chars;
    let octet = |mut i: usize| { let s = i; while i < c.len() && i - s < 3 && c[i].is_ascii_digit() { i += 1; } (i > s).then_some(i) };
    let sep = |i: usize| c.get(i).is_some_and(|x| *x == '.' || *x == ',' || (!x.is_ascii() && !NUMBER_UNITS.contains(*x)));
    for s in 0..c.len() {
        if !c[s].is_ascii_digit() || (s > 0 && (c[s - 1].is_ascii_digit() || c[s - 1] == '.')) { continue; }
        let Some(mut e) = octet(s) else { continue };
        let (mut parts, mut exact) = (1, true);
        while parts < 4 && sep(e) {
            let gap = if sep(e + 1) { 2 } else { 1 };
            exact &= gap == 1 && c[e] == '.';
            let Some(n) = octet(e + gap) else { break };
            e = n; parts += 1;
        }
        if parts < 4 || c.get(e).is_some_and(char::is_ascii_digit) { continue; }
        let in_range = l.chars[s..e].split(|x| !x.is_ascii_digit()).all(|o| o.iter().collect::<String>().parse::<u16>().is_ok_and(|n| n <= 255));
        if !in_range { continue; }
        out.push(l.finding(SensitiveKind::Ipv4, if exact { Certainty::Confirmed } else { Certainty::Possible }, s, e));
    }
}

fn email(l: &Line, out: &mut Vec<Finding>) {
    let c = &l.chars;
    for at in (0..c.len()).filter(|&i| c[i] == '@') {
        let mut s = at;
        while s > 0 && is_mail_char(c[s - 1]) { s -= 1; }
        let mut e = at + 1;
        while e < c.len() && is_domain_char(c[e]) { e += 1; }
        if at - s < 2 || e - at - 1 < 3 { continue; }
        let dotted = c[at + 1..e].contains(&'.');
        out.push(l.finding(SensitiveKind::Email, if dotted { Certainty::Confirmed } else { Certainty::Possible }, s, e));
    }
}

fn api_key(l: &Line, out: &mut Vec<Finding>) {
    let c = &l.chars;
    for p in KEY_PREFIXES {
        let p: Vec<char> = p.chars().collect();
        for s in 0..c.len().saturating_sub(p.len()) {
            if c[s..s + p.len()] != p[..] { continue; }
            // "sk-" also ends ordinary words ("task-…", "sk-learn"); the other prefixes do not.
            let wordy = p[0] == 's';
            if wordy && s > 0 && c[s - 1].is_ascii_alphabetic() { continue; }
            let mut e = s + p.len();
            while e < c.len() && is_key_char(c[e]) { e += 1; }
            // A misread can split a key: reach over up to 2 non-Latin characters to cover the rest.
            let mut bridged = false;
            loop {
                let gap = (e..c.len().min(e + 2)).take_while(|&i| !c[i].is_ascii()).count();
                if gap == 0 || !c.get(e + gap).copied().is_some_and(is_key_char) { break; }
                e += gap;
                while e < c.len() && is_key_char(c[e]) { e += 1; }
                bridged = true;
            }
            let tail = &c[s + p.len()..e];
            if tail.len() < KEY_MIN_TAIL || (wordy && !tail.iter().any(char::is_ascii_digit)) { continue; }
            out.push(l.finding(SensitiveKind::ApiKey, if bridged { Certainty::Possible } else { Certainty::Confirmed }, s, e));
        }
    }
}

/// Everything that looks sensitive on one recognised line.
pub fn scan_line(words: &[TextWord]) -> Vec<Finding> {
    let l = Line::new(words);
    let mut out = Vec::new();
    numbers(&l, &mut out);
    dashed_phone(&l, &mut out);
    ipv4(&l, &mut out);
    email(&l, &mut out);
    api_key(&l, &mut out);
    out
}

/// The text inside `area`: the longest run on one line among words centred in it.
fn text_inside(lines: &[Line], area: (f32, f32, f32, f32)) -> Vec<char> {
    let inside = |w: &TextWord| { let (x, y) = (w.x + w.w / 2.0, w.y + w.h / 2.0); x >= area.0 && x <= area.2 && y >= area.1 && y <= area.3 };
    lines.iter().map(|l| l.chars.iter().zip(&l.owner).filter(|(_, &o)| inside(&l.words[o])).map(|(c, _)| *c).collect::<Vec<char>>())
        .max_by_key(Vec::len).unwrap_or_default()
}
/// The text in `area`, if there is enough of it to search for: 1 character would match all
/// over the image.
pub fn text_in(lines: &[Vec<TextWord>], area: (f32, f32, f32, f32)) -> Option<Vec<char>> {
    let lines: Vec<Line> = lines.iter().map(|w| Line::new(w)).collect();
    Some(text_inside(&lines, area)).filter(|p| p.len() >= 2)
}
/// Whether a word reaches over the edge of `area`: part of it is outside the cover, so what is
/// left may still be read even though the whole word is not found.
pub fn straddles(lines: &[Vec<TextWord>], area: (f32, f32, f32, f32)) -> bool {
    const SLACK: f32 = 2.0;
    lines.iter().flatten().any(|w| {
        let b = (w.x, w.y, w.x + w.w, w.y + w.h);
        overlaps(b, area) && (b.0 < area.0 - SLACK || b.1 < area.1 - SLACK || b.2 > area.2 + SLACK || b.3 > area.3 + SLACK)
    })
}
/// Every place `phrase` can be read. Exact match after folding: a misread copy is not found.
pub fn find_text(lines: &[Vec<TextWord>], phrase: &[char]) -> Vec<Finding> {
    let mut out = Vec::new();
    for words in lines {
        let l = Line::new(words);
        for s in 0..(l.chars.len() + 1).saturating_sub(phrase.len()) {
            if l.chars[s..s + phrase.len()] == *phrase { out.push(l.finding(SensitiveKind::SameText, Certainty::Confirmed, s, s + phrase.len())); }
        }
    }
    out
}
/// Other places with the same text as `area`; `None` when `area` holds too little to search.
pub fn same_text(lines: &[Vec<TextWord>], area: (f32, f32, f32, f32)) -> Option<Vec<Finding>> {
    let phrase = text_in(lines, area)?;
    Some(find_text(lines, &phrase).into_iter().filter(|f| !overlaps(f.bounds, area)).collect())
}

fn overlaps(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool { a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1 }

/// Merge findings that overlap (several OCR passes, tile seams, two rules on one value): one
/// entry per place, covering all of them, with the strongest reading's kind and text.
pub fn merge_findings(findings: impl IntoIterator<Item = Finding>) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    for mut f in findings {
        let mut i = 0;
        while i < out.len() {
            if !overlaps(out[i].bounds, f.bounds) { i += 1; continue; }
            let o = out.swap_remove(i);
            let b = (o.bounds.0.min(f.bounds.0), o.bounds.1.min(f.bounds.1), o.bounds.2.max(f.bounds.2), o.bounds.3.max(f.bounds.3));
            if o.certainty > f.certainty { f = o; }
            f.bounds = b;
            i = 0;
        }
        out.push(f);
    }
    out.sort_by(|a, b| a.bounds.1.total_cmp(&b.bounds.1).then(a.bounds.0.total_cmp(&b.bounds.0)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use Certainty::*;
    use SensitiveKind::*;

    fn line(words: &[&str]) -> Vec<TextWord> {
        words.iter().enumerate().map(|(i, t)| TextWord { text: (*t).into(), x: i as f32 * 100.0, y: 0.0, w: 90.0, h: 20.0 }).collect()
    }
    fn kinds(words: &[&str]) -> Vec<(SensitiveKind, Certainty)> {
        let mut v: Vec<_> = scan_line(&line(words)).iter().map(|f| (f.kind, f.certainty)).collect();
        v.dedup();
        v
    }

    #[test] fn phones() {
        assert_eq!(kinds(&["电话", "13800138000"]), [(Phone, Confirmed)]);
        assert_eq!(kinds(&["138", "0013", "8000"]), [(Phone, Confirmed)]);
        assert_eq!(kinds(&["1345e422312"]), [(Phone, Possible)]);
        assert_eq!(kinds(&["１３８００１３８０００"]), [(Phone, Confirmed)]);
        assert!(kinds(&["共", "1024", "条"]).is_empty());
        assert!(kinds(&["23800138000"]).is_empty());
    }
    #[test] fn phones_next_to_other_numbers_and_in_other_forms() {
        assert_eq!(kinds(&["1", "张", "三", "13800138000"]), [(Phone, Confirmed)]);
        assert_eq!(kinds(&["13800138000", "张", "三", "5"]), [(Phone, Confirmed)]);
        assert_eq!(kinds(&["+86", "13800138000"]), [(Phone, Confirmed)]);
        assert_eq!(kinds(&["138-0013-8000"]), [(Phone, Confirmed)]);
        assert!(kinds(&["2024-1015-1430"]).is_empty());
        assert!(kinds(&["1730000000000"]).is_empty(), "a millisecond timestamp");
    }
    #[test] fn columns_are_not_one_number() {
        let cell = |t: &str, x: f32| TextWord { text: t.into(), x, y: 0.0, w: 30.0, h: 16.0 };
        // Joined, these would read as the phone number 15300881024.
        assert!(scan_line(&[cell("15", 0.0), cell("300", 80.0), cell("88", 160.0), cell("1024", 240.0)]).is_empty());
        assert_eq!(scan_line(&[cell("153", 0.0), cell("0088", 34.0), cell("1024", 68.0)]).len(), 1, "a number spaced as 3-4-4 is still one");
    }
    #[test] fn ids_use_the_check_digit() {
        assert_eq!(kinds(&["11010519491231002X"]), [(IdCard, Confirmed)]);
        assert_eq!(kinds(&["110105194912310021"]), [(IdCard, Possible)]);
    }
    #[test] fn cards_use_luhn() {
        assert_eq!(kinds(&["6222020000000000000"]), [(BankCard, Confirmed)]);
        assert_eq!(kinds(&["4111111111111111"]), [(BankCard, Confirmed)]);
        assert_eq!(kinds(&["4111111111111112"]), [(BankCard, Possible)]);
    }
    #[test] fn ipv4_tolerates_marks_but_not_ordinary_numbers() {
        assert_eq!(kinds(&["192.0.2.10"]), [(Ipv4, Confirmed)]);
        assert_eq!(kinds(&["198", "。", "51", "。", "100", "。", "63"]), [(Ipv4, Possible)]);
        for ordinary in ["999.1.1.1", "1.18.1", "19045.3803", "14:30:25", "2024-10-15", "¥1,299.00", "1天2小时3分4秒", "10月15日14时30分", "3室2厅1卫80平", "1,234,567,890"] { assert!(kinds(&[ordinary]).is_empty(), "{ordinary}"); }
    }
    #[test] fn mail_and_split_words() {
        assert_eq!(kinds(&["wpauv40@exa", "mple.co", "m"]), [(Email, Confirmed)]);
        assert_eq!(scan_line(&line(&["邮箱", "a.b@example.com"]))[0].text, "a.b@example.com");
        assert!(kinds(&["@me"]).is_empty());
    }
    #[test] fn keys_need_a_boundary_and_a_digit() {
        assert_eq!(kinds(&["token:", "ghp_2NiPnAn76ooJUsqA5qxv5r5"]), [(ApiKey, Confirmed)]);
        assert!(kinds(&["task-management-board-12"]).is_empty());
        assert!(kinds(&["sk-learn-toolkit-guide"]).is_empty());
        let split = scan_line(&line(&["AKlAbJ9Ki5FVoDPNz3KIRW", "错", "srgc", "mcPR"]));
        assert_eq!((split[0].kind, split[0].certainty), (ApiKey, Possible));
        assert_eq!(split[0].bounds.2, 390.0, "the box reaches the part after the misread");
    }
    #[test] fn same_text_finds_exact_copies_elsewhere() {
        let at = |t: &[&str], y: f32| -> Vec<TextWord> { line(t).into_iter().map(|w| TextWord { y, ..w }).collect() };
        let lines = [at(&["客", "户", "张", "三"], 0.0), at(&["联", "系", "人", "张", "三"], 50.0), at(&["张", "五"], 100.0)];
        let found = same_text(&lines, (200.0, 0.0, 400.0, 20.0)).unwrap();
        assert_eq!(found.len(), 1, "the selected place itself is not listed, and 张五 is not 张三");
        assert_eq!((found[0].text.as_str(), found[0].bounds), ("张三", (300.0, 50.0, 490.0, 70.0)));
    }
    #[test] fn same_text_needs_at_least_two_characters() {
        assert_eq!(same_text(&[line(&["张", "三"])], (0.0, 0.0, 90.0, 20.0)), None);
        assert_eq!(same_text(&[line(&["张", "三"])], (500.0, 0.0, 600.0, 20.0)), None);
        assert_eq!(same_text(&[], (0.0, 0.0, 90.0, 20.0)), None);
    }
    #[test] fn bounds_are_the_owning_words() {
        let f = &scan_line(&line(&["手机", "138", "0013", "8000", "备注"]))[0];
        assert_eq!(f.bounds, (100.0, 0.0, 390.0, 20.0));
    }
    #[test] fn merging_keeps_the_strongest_reading_and_the_widest_box() {
        let f = |k, c, t: &str, x0, x1| Finding { kind: k, certainty: c, text: t.into(), bounds: (x0, 0.0, x1, 20.0) };
        let got = merge_findings([f(Phone, Possible, "1345e422312", 10.0, 100.0), f(Phone, Confirmed, "13450422312", 5.0, 90.0), f(Email, Confirmed, "a@b.c", 200.0, 300.0)]);
        assert_eq!(got.len(), 2);
        assert_eq!((got[0].text.as_str(), got[0].bounds), ("13450422312", (5.0, 0.0, 100.0, 20.0)));
    }
    #[test] fn merging_joins_a_box_that_bridges_two_others() {
        let f = |x0, x1| Finding { kind: Phone, certainty: Possible, text: String::new(), bounds: (x0, 0.0, x1, 20.0) };
        assert_eq!(merge_findings([f(0.0, 10.0), f(20.0, 30.0), f(5.0, 25.0)]).len(), 1);
    }

    /// Replays recorded OCR output (invented values, held-out fonts; `spike/ocr/RESULT.md`), so
    /// rule changes show their effect on recall without an OCR engine.
    #[test] fn recorded_ocr_meets_the_gate() {
        type Box4 = (f32, f32, f32, f32);
        let mut truth: HashMap<&str, (&str, Box4)> = HashMap::new();
        for row in include_str!("../testdata/ocr-truth.tsv").lines() {
            let c: Vec<&str> = row.split('\t').collect();
            let n = |i: usize| c[i].parse::<f32>().unwrap();
            truth.insert(c[0], (c[4], (n(6), n(7), n(8), n(9))));
        }
        let mut runs: HashMap<&str, Vec<Finding>> = HashMap::new();
        for run in [include_str!("../testdata/ocr-words-1.tsv"), include_str!("../testdata/ocr-words-1.5.tsv"), include_str!("../testdata/ocr-words-2.tsv")] {
            let mut lines: HashMap<(&str, &str), Vec<TextWord>> = HashMap::new();
            for row in run.lines() {
                let c: Vec<&str> = row.splitn(7, '\t').collect();
                let n = |i: usize| c[i].parse::<f32>().unwrap();
                lines.entry((c[0].rsplit('/').next().unwrap(), c[1])).or_default().push(TextWord { text: c[6].into(), x: n(2), y: n(3), w: n(4), h: n(5) });
            }
            for ((name, _), words) in &lines { runs.entry(name).or_default().extend(scan_line(words)); }
        }
        let (mut values, mut found, mut covered, mut ordinary, mut flagged) = (0, 0, 0, 0, 0);
        for (name, (kind, b)) in &truth {
            let got = merge_findings(runs.remove(name).unwrap_or_default());
            if *kind == "none" { ordinary += 1; flagged += usize::from(!got.is_empty()); continue; }
            values += 1;
            let near: Vec<_> = got.iter().filter(|f| overlaps(f.bounds, *b)).collect();
            found += usize::from(!near.is_empty());
            covered += usize::from(near.iter().any(|f| f.bounds.0 - 4.0 <= b.0 && f.bounds.1 - 4.0 <= b.1 && f.bounds.2 + 4.0 >= b.2 && f.bounds.3 + 4.0 >= b.3));
        }
        let (recall, cover) = (found as f32 / values as f32, covered as f32 / found as f32);
        eprintln!("recall {recall:.3} cover {cover:.3} ordinary lines flagged {flagged}/{ordinary}");
        assert_eq!((values, ordinary), (540, 360));
        assert!(recall >= 0.90 && cover >= 0.95, "recall {recall} cover {cover}");
        assert!(flagged <= 14, "{flagged} ordinary lines flagged");
    }
}
