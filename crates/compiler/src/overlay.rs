//! beamer overlay specifications (`beamerbasedecode.sty`, issue #944 Tier
//! 2): the `<...>` argument of `\item`, `\only`, `\uncover`, `\onslide`,
//! `\alert`, `\visible`, `\invisible` and the overlay environments.
//!
//! `\beamer@masterdecode` splits the specification at `|` into
//! `mode:spec` entries (an entry with no `:` is `beamer:`), keeps the ones
//! for the current mode (`beamer`, `presentation`, `all` under pdflatex;
//! `handout:`, `article:`, `trans:`, `second:` are other modes and never
//! match), replaces `+` and `.` by the `beamerpauses` counter (`+` steps it
//! once per specification, after the decode) and reads the rest as a comma
//! list of `*`, `n`, `n-`, `-n` and `n-m`. A number larger than the slide
//! being set (`\beamer@anotherslide`) makes the frame's loop
//! (`beamerbaseframe.sty` `\beamer@slideinframe`) set one more slide, so
//! the frame's slide count is the largest number any specification names.

/// One `n`, `n-`, `-n` or `n-m` of a specification's comma list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayRange {
    /// `n`.
    One(u32),
    /// `n-`: from slide `n` on.
    From(u32),
    /// `-n`: up to slide `n`.
    Until(u32),
    /// `n-m`.
    Between(u32, u32),
}

impl OverlayRange {
    fn contains(self, slide: u32) -> bool {
        match self {
            OverlayRange::One(n) => slide == n,
            OverlayRange::From(n) => slide >= n,
            OverlayRange::Until(n) => slide <= n,
            OverlayRange::Between(a, b) => a <= slide && slide <= b,
        }
    }

    /// The largest slide the range names (`\beamer@anotherslide` is set
    /// for it). `Until(n)` names `n`; `From(n)` names `n`.
    fn max_slide(self) -> u32 {
        match self {
            OverlayRange::One(n) | OverlayRange::From(n) | OverlayRange::Until(n) => n,
            OverlayRange::Between(a, b) => a.max(b),
        }
    }
}

/// A decoded overlay specification: the slides it selects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct OverlaySpec {
    /// `*` (or an empty specification): every slide.
    pub all: bool,
    /// The ranges of the beamer-mode entries, in source order.
    pub ranges: Vec<OverlayRange>,
}

impl OverlaySpec {
    /// The specification selecting every slide (`<*>`, `\beamer@emptyospec`).
    pub fn all() -> OverlaySpec {
        OverlaySpec { all: true, ranges: Vec::new() }
    }

    /// `<n->`.
    pub fn from(n: u32) -> OverlaySpec {
        OverlaySpec { all: false, ranges: vec![OverlayRange::From(n)] }
    }

    /// Decodes the text between `<` and `>`. `pauses` is the frame's
    /// `beamerpauses` counter (1 at the start of every slide): `+` reads it
    /// and `.` reads one less, and a `+` anywhere in the specification steps
    /// it once the decode is done (`\beamer@decodefind`'s
    /// `\ifbeamer@plusencountered\stepcounter{beamerpauses}\fi`), so
    /// `\item<+->` in sequence uncovers item by item.
    pub fn parse(raw: &str, pauses: &mut u32) -> OverlaySpec {
        let mut spec = OverlaySpec::default();
        let mut plus = false;
        let mut any_entry = false;
        for entry in raw.split('|') {
            let entry: String = entry.chars().filter(|c| !c.is_whitespace()).collect();
            // `action@spec`: an action specification (`alert@2`); the
            // slides of the default action are the ones without `@`.
            if entry.contains('@') {
                continue;
            }
            let (mode, body) = match entry.split_once(':') {
                Some((mode, body)) => (mode, body),
                None => ("beamer", entry.as_str()),
            };
            if !matches!(mode, "beamer" | "presentation" | "all" | "") {
                // A bare keyword (`<handout>`) is that mode's `1-`.
                continue;
            }
            any_entry = true;
            if body.is_empty() {
                spec.all = true;
                continue;
            }
            // A body opening with a letter is a mode keyword (`<beamer>`,
            // `<all>`): `\beamer@checkcat` reads it as `1-` for that mode.
            if body.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
                spec.all = true;
                continue;
            }
            for part in body.split(',') {
                if part.is_empty() {
                    continue;
                }
                if part == "*" {
                    spec.all = true;
                    continue;
                }
                let (lo, hi) = match part.split_once('-') {
                    Some((lo, hi)) => (lo, Some(hi)),
                    None => (part, None),
                };
                let lo_n = relative_number(lo, *pauses, &mut plus);
                match hi {
                    None => {
                        if let Some(n) = lo_n {
                            spec.ranges.push(OverlayRange::One(n));
                        }
                    }
                    Some("") => {
                        if lo.is_empty() {
                            spec.all = true;
                        } else if let Some(n) = lo_n {
                            spec.ranges.push(OverlayRange::From(n));
                        }
                    }
                    Some(hi) => {
                        let hi_n = relative_number(hi, *pauses, &mut plus);
                        match (lo.is_empty(), lo_n, hi_n) {
                            (true, _, Some(m)) => spec.ranges.push(OverlayRange::Until(m)),
                            (false, Some(n), Some(m)) => spec.ranges.push(OverlayRange::Between(n, m)),
                            (false, Some(n), None) => spec.ranges.push(OverlayRange::From(n)),
                            _ => {}
                        }
                    }
                }
            }
        }
        if plus {
            *pauses += 1;
        }
        // Only other modes named (`<handout:2>`): nothing is decoded for
        // beamer and `\beamer@decodefound` keeps its initial `*`, so the
        // material shows on every slide -- as does a specification that
        // decoded to no range at all.
        if !any_entry || (!spec.all && spec.ranges.is_empty()) {
            spec.all = true;
        }
        spec
    }

    /// Whether `slide` (1-based) is selected.
    pub fn contains(&self, slide: u32) -> bool {
        self.all || self.ranges.iter().any(|r| r.contains(slide))
    }

    /// The largest slide named (0 for `*` alone): the frame sets slides up
    /// to the largest such number over all its specifications.
    pub fn max_slide(&self) -> u32 {
        self.ranges.iter().map(|r| r.max_slide()).max().unwrap_or(0)
    }
}

/// `+`, `+(n)`, `.`, `.(n)` or a plain number, against the `beamerpauses`
/// counter (`\beamer@relnumber`, `\beamer@relnumberdot`). `None` for text
/// that is no number.
fn relative_number(text: &str, pauses: u32, plus: &mut bool) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    let (base, rest) = if let Some(rest) = text.strip_prefix('+') {
        *plus = true;
        (pauses as i64, rest)
    } else if let Some(rest) = text.strip_prefix('.') {
        (pauses as i64 - 1, rest)
    } else {
        return text.parse::<u32>().ok();
    };
    let offset = match rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) {
        Some(inner) => inner.parse::<i64>().ok()?,
        None if rest.is_empty() => 0,
        None => return None,
    };
    Some((base + offset).max(0) as u32)
}

/// What an overlay-aware command does with its argument on the slides its
/// specification does not select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayKind {
    /// `\uncover`, `\onslide<spec>{...}`, `\item<spec>`, `uncoverenv`: the
    /// material keeps its space and is not painted (`\setbeamercovered
    /// {invisible}`, the default).
    Cover,
    /// `\only`, `onlyenv`: the material is omitted, so the page reflows.
    Only,
    /// `\alert<spec>`, `alertenv`: the alert colour on the selected slides,
    /// the surrounding colour otherwise.
    Alert,
    /// `\visible`, `visibleenv`: like `Cover` (beamer's `\visible` ignores
    /// `\setbeamercovered{transparent}`, which is not modelled).
    Visible,
    /// `\invisible`, `invisibleenv`: covered on the selected slides,
    /// painted on the others.
    Invisible,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(raw: &str) -> OverlaySpec {
        let mut pauses = 1;
        OverlaySpec::parse(raw, &mut pauses)
    }

    #[test]
    fn ranges() {
        let s = parse("2-");
        assert!(!s.contains(1) && s.contains(2) && s.contains(9));
        assert_eq!(s.max_slide(), 2);
        let s = parse("2-3");
        assert!(!s.contains(1) && s.contains(2) && s.contains(3) && !s.contains(4));
        assert_eq!(s.max_slide(), 3);
        let s = parse("1,3");
        assert!(s.contains(1) && !s.contains(2) && s.contains(3));
        let s = parse("-2");
        assert!(s.contains(1) && s.contains(2) && !s.contains(3));
        assert_eq!(s.max_slide(), 2);
        let s = parse("*");
        assert!(s.all && s.contains(7));
        assert_eq!(s.max_slide(), 0);
        assert!(parse("").contains(3));
    }

    #[test]
    fn plus_and_dot_read_the_pause_counter() {
        let mut pauses = 1;
        let a = OverlaySpec::parse("+-", &mut pauses);
        assert_eq!(pauses, 2);
        assert!(a.contains(1) && a.contains(2));
        let b = OverlaySpec::parse("+-", &mut pauses);
        assert_eq!(pauses, 3);
        assert!(!b.contains(1) && b.contains(2));
        let c = OverlaySpec::parse(".-", &mut pauses);
        assert_eq!(pauses, 3, "`.` does not step");
        assert!(!c.contains(1) && c.contains(2));
        let d = OverlaySpec::parse("+(1)-", &mut pauses);
        assert_eq!(pauses, 4);
        assert!(!d.contains(3) && d.contains(4));
        let e = OverlaySpec::parse("+-+(1)", &mut pauses);
        assert_eq!(pauses, 5, "one step per specification");
        assert_eq!(e.ranges, vec![OverlayRange::Between(4, 5)]);
    }

    #[test]
    fn modes_and_actions() {
        let s = parse("2-|handout:1");
        assert!(!s.contains(1) && s.contains(2));
        assert_eq!(s.max_slide(), 2);
        let s = parse("handout:2");
        assert!(s.all && s.contains(1), "other modes alone decode to `*`");
        let s = parse("beamer:2|all:3");
        assert!(s.contains(2) && s.contains(3) && !s.contains(1));
        let s = parse("1-|alert@2");
        assert!(s.contains(1) && s.contains(2));
        assert_eq!(s.max_slide(), 1);
        let s = parse("2, 4");
        assert!(s.contains(2) && s.contains(4) && !s.contains(3));
    }
}
