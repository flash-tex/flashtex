//! lipsum placeholder paragraphs (`\lipsum[<range>]`).
//!
//! Real lipsum ships only its sources (`lipsum.dtx` + `lipsum.ins`): the
//! batch file generates both `lipsum.sty` and the paragraph data
//! (`lipsum.ltd.tex`) with the docstrip utility, which
//! `flashtex-docstrip` already interprets — but with no TeX Live and no
//! network here those sources are unreachable, so the data is bundled
//! directly. Paragraphs 1-7 below are transcribed from the published
//! lipsum text (the `consectetuer` spelling is lipsum's own); they have
//! NOT been verified byte-for-byte against a live pdflatex run.
//! Paragraphs 8-150 are not bundled: selecting one is diagnosed.

/// lipsum's bundled paragraphs, 1-based as `\lipsum` numbers them.
/// Single spaces throughout: `lipsum.ltd.tex` wraps lines that TeX
/// collapses to spaces, so the canonical text has one space per gap.
pub const PARAGRAPHS: &[&str] = &[
    "Lorem ipsum dolor sit amet, consectetuer adipiscing elit. Ut purus elit, vestibulum ut, placerat ac, adipiscing vitae, felis. Curabitur dictum gravida mauris. Nam arcu libero, nonummy eget, consectetuer id, vulputate a, magna. Donec vehicula augue eu neque. Pellentesque habitant morbi tristique senectus et netus et malesuada fames ac turpis egestas. Mauris ut leo. Cras viverra metus rhoncus sem. Nulla et lectus vestibulum urna fringilla ultrices. Phasellus eu tellus sit amet tortor gravida placerat. Integer sapien est, iaculis in, pretium quis, viverra ac, nunc. Praesent eget sem vel leo ultrices bibendum. Aenean faucibus. Morbi dolor nulla, malesuada eu, pulvinar at, mollis ac, nulla. Curabitur auctor semper nulla. Donec varius orci eget risus. Duis nibh mi, congue eu, accumsan eleifend, sagittis quis, diam. Duis eget orci sit amet orci dignissim rutrum.",
    "Nam dui ligula, fringilla a, euismod sodales, sollicitudin vel, wisi. Morbi auctor lorem vitae tortor. Quisque ullamcorper placerat ipsum. Cras nibh. Morbi vel justo vitae lacus tincidunt ultrices. Lorem ipsum dolor sit amet, consectetuer adipiscing elit. In hac habitasse platea dictumst. Integer tempus convallis augue. Etiam facilisis. Nunc elementum fermentum wisi. Aenean placerat. Ut imperdiet, enim sed gravida sollicitudin, felis odio placerat quam, ac pulvinar elit purus eget enim. Nunc vitae tortor. Proin tempus nibh sit amet nisl. Vivamus quis tortor vitae risus porta vehicula.",
    "Fusce mauris. Vestibulum luctus nibh at lectus. Sed bibendum, nulla a faucibus semper, leo velit ultricies tellus, ac venenatis arcu wisi vel nisl. Vestibulum diam. Aliquam pellentesque, augue quis sagittis posuere, turpis lacus congue quam, in hendrerit risus eros eget felis. Maecenas eget erat in sapien mattis porttitor. Vestibulum porttitor. Nulla facilisi. Sed a turpis eu lacus commodo facilisis. Morbi fringilla, wisi in dignissim interdum, justo lectus sagittis dui, et vehicula libero dui cursus dui. Mauris tempor ligula sed lacus. Duis cursus enim ut augue. Cras ac magna. Cras nulla. Nulla egestas. Curabitur a leo. Quisque egestas wisi eget nunc. Nam feugiat lacus vel est. Curabitur consectetuer.",
    "Suspendisse vel felis. Ut lorem lorem, interdum eu, tincidunt sit amet, laoreet vitae, arcu. Aenean faucibus pede eu ante. Praesent enim elit, rutrum at, molestie non, nonummy vel, nisl. Ut lectus eros, malesuada sit amet, fermentum eu, sodales cursus, magna. Donec eu purus. Quisque vehicula, urna sed ultricies auctor, pede lorem egestas nisl, in laoreet eros nisl non est. Donec lectus. Nulla at nisl. Aliquam erat volutpat. Nam dui mi, tincidunt quis, accumsan porttitor, facilisis luctus, metus.",
    "Phasellus hendrerit. Pellentesque aliquet nibh nec urna. In nisi neque, aliquet vel, dapibus id, mattis vel, nisi. Sed pretium, ligula sollicitudin laoreet viverra, tortor libero sodales leo, eget blandit nunc tortor eu nibh. Nullam mollis. Ut justo. Suspendisse potenti.",
    "Sed egestas, ante et vulputate volutpat, eros pede semper est, vitae luctus metus libero eu augue. Morbi purus libero, faucibus adipiscing, commodo quis, gravida id, est. Sed lectus. Praesent elementum hendrerit tortor. Sed semper lorem at felis. Vestibulum volutpat, lacus a ultrices sagittis, mi neque euismod dui, eu pulvinar nunc sapien ornare nisl. Phasellus pede arcu, dapibus eu, fermentum et, dapibus sed, urna.",
    "Morbi interdum mollis sapien. Sed ac risus. Phasellus lacinia, magna a ullamcorper laoreet, lectus arcu pulvinar risus, vitae facilisis libero dolor a purus. Sed vel lacus. Mauris nibh felis, adipiscing varius, adipiscing in, lacinia vel, tellus. Suspendisse ac urna. Etiam pellentesque mauris ut lectus. Nunc tellus ante, mattis eget, gravida vitae, ultricies ac, leo. Integer leo pede, ornare a, lacinia eu, vulputate vel, nisl.",
];

/// The paragraphs a bare `\lipsum` sets (lipsum.sty's documented default;
/// unverified against a live run): 1 through 7, inclusive, 1-based.
pub const DEFAULT_RANGE: (usize, usize) = (1, 7);

/// What went wrong parsing a `\lipsum[<range>]` spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeError {
    /// The `[...]` held only whitespace.
    Empty,
    /// An item that is neither `n` nor `n-m` (`String` is the item).
    Malformed(String),
    /// A 1-based paragraph number outside the bundled table.
    OutOfRange(usize),
}

impl RangeError {
    /// The diagnostic message (always names `\lipsum`).
    pub fn message(&self) -> String {
        match self {
            RangeError::Empty => "\\lipsum range is empty".to_string(),
            RangeError::Malformed(item) => format!(
                "\\lipsum could not parse range item `{item}` (use `n` or `n-m`, separated by commas)"
            ),
            RangeError::OutOfRange(n) => format!(
                "\\lipsum paragraph {n} is out of range (paragraphs 1-{} are bundled)",
                PARAGRAPHS.len()
            ),
        }
    }
}

fn number(word: &str) -> Option<usize> {
    if word.is_empty() || !word.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    word.parse::<usize>().ok()
}

/// The 0-based [`PARAGRAPHS`] indices a `\lipsum[<range>]` spec selects:
/// comma-separated `n` / `n-m` items (lipsum.sty's range syntax), each
/// kept in order, duplicates included.
pub fn select(spec: &str) -> Result<Vec<usize>, RangeError> {
    if spec.trim().is_empty() {
        return Err(RangeError::Empty);
    }
    let mut out = Vec::new();
    for item in spec.split(',') {
        let item = item.trim();
        let (start, end) = match item.split_once('-') {
            Some((a, b)) => (a.trim(), Some(b.trim())),
            None => (item, None),
        };
        let end = end.unwrap_or(start);
        let (Some(a), Some(b)) = (number(start), number(end)) else {
            return Err(RangeError::Malformed(item.to_string()));
        };
        if item.contains('-') && a > b {
            return Err(RangeError::Malformed(item.to_string()));
        }
        for n in a..=b {
            if !(1..=PARAGRAPHS.len()).contains(&n) {
                return Err(RangeError::OutOfRange(n));
            }
            out.push(n - 1);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_range_is_bundled() {
        assert!(DEFAULT_RANGE.1 <= PARAGRAPHS.len());
        let selected: Vec<usize> =
            (DEFAULT_RANGE.0..=DEFAULT_RANGE.1).map(|n| n - 1).collect();
        assert_eq!(selected.len(), 7);
    }

    #[test]
    fn range_shapes() {
        assert_eq!(select("1"), Ok(vec![0]));
        assert_eq!(select("1-3"), Ok(vec![0, 1, 2]));
        assert_eq!(select("2, 4-5"), Ok(vec![1, 3, 4]));
        assert_eq!(select(" 3 "), Ok(vec![2]));
        assert!(matches!(select(""), Err(RangeError::Empty)));
        assert!(matches!(select("   "), Err(RangeError::Empty)));
        assert!(matches!(select("a"), Err(RangeError::Malformed(_))));
        assert!(matches!(select("1-"), Err(RangeError::Malformed(_))));
        assert!(matches!(select("3-1"), Err(RangeError::Malformed(_))));
        assert!(matches!(select("0"), Err(RangeError::OutOfRange(0))));
        assert_eq!(
            select("7-8"),
            Err(RangeError::OutOfRange(8)),
            "the first unbundled number reports, after selecting 7"
        );
    }
}
