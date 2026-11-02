//! Output beautify choices as the panel shows them (ADR 0.5-BEAUTIFY). The frame itself is
//! drawn by `anshot_render`; this only maps panel choices to a `Frame` and back.
#![forbid(unsafe_code)]
use anshot_render::{Background, Chrome, Frame, Ratio, Step, PRESETS};

/// Index of "automatic" among the background choices (after the presets).
pub const AUTO: usize = PRESETS.len();

/// Panel choices by index. The chrome text is never part of it: it is not remembered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style { pub background: usize, pub padding: usize, pub radius: usize, pub shadow: usize, pub ratio: usize, pub chrome: usize }

impl Default for Style {
    fn default() -> Self { Style { background: 0, padding: 2, radius: 2, shadow: 2, ratio: 0, chrome: 0 } }
}

impl Style {
    pub fn frame(self, text: &str) -> Frame {
        let step = |i: usize| Step::ALL[i.min(3)];
        Frame {
            background: PRESETS.get(self.background).copied().unwrap_or(Background::Auto),
            padding: step(self.padding), radius: step(self.radius), shadow: step(self.shadow),
            ratio: Ratio::ALL[self.ratio.min(Ratio::ALL.len() - 1)],
            chrome: Chrome::ALL[self.chrome.min(3)],
            text: text.into(),
        }
    }
    pub fn of(f: &Frame) -> Style {
        let at = |s: Step| Step::ALL.iter().position(|x| *x == s).unwrap_or(0);
        Style {
            background: PRESETS.iter().position(|b| *b == f.background).unwrap_or(AUTO),
            padding: at(f.padding), radius: at(f.radius), shadow: at(f.shadow),
            ratio: Ratio::ALL.iter().position(|r| *r == f.ratio).unwrap_or(0),
            chrome: Chrome::ALL.iter().position(|c| *c == f.chrome).unwrap_or(0),
        }
    }
    /// `background,padding,radius,shadow,ratio,chrome` for the settings file.
    pub fn render(self) -> String {
        format!("{},{},{},{},{},{}", self.background, self.padding, self.radius, self.shadow, self.ratio, self.chrome)
    }
    /// `None` unless every field is a valid index.
    pub fn parse(v: &str) -> Option<Style> {
        let n: Vec<usize> = v.split(',').map(|p| p.trim().parse().ok()).collect::<Option<_>>()?;
        let [background, padding, radius, shadow, ratio, chrome] = n[..] else { return None };
        let ok = background <= AUTO && padding < 4 && radius < 4 && shadow < 4 && ratio < Ratio::ALL.len() && chrome < 4;
        ok.then_some(Style { background, padding, radius, shadow, ratio, chrome })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn style_round_trips_through_frame_and_settings() {
        for s in [Style::default(), Style { background: AUTO, padding: 0, radius: 3, shadow: 1, ratio: 5, chrome: 3 }] {
            assert_eq!(Style::of(&s.frame("x")), s);
            assert_eq!(Style::parse(&s.render()), Some(s));
        }
        assert_eq!(Style::default().frame("").background, PRESETS[0]);
        assert_eq!(Style { background: AUTO, ..Style::default() }.frame("").background, Background::Auto);
        for bad in ["", "1,2,3", "9,0,0,0,0,0", "0,4,0,0,0,0", "0,0,0,0,6,0", "0,0,0,0,0,4", "a,0,0,0,0,0", "0,0,0,0,0,0,0"] {
            assert_eq!(Style::parse(bad), None, "{bad}");
        }
    }
}
