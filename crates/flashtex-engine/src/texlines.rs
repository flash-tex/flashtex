//! A source file's lines as TeX numbers them: `system::read_tex_line`'s
//! rule (texmfmp.c's `input_line`), a line ends at a LF, at a CR not
//! followed by a LF, and at a CR LF (once). Everything outside the engine
//! that turns a line number into a place in the file or back -- the display
//! list's spans as an edit moves them (`host::resident::line_change`), the
//! diagnostics' positions (`host::diag`) -- goes through here, so that they
//! cannot count lines differently from TeX or from each other.

/// The lines of `b`: (start, end) byte offsets, the end before the line's
/// terminator. Line 1 starts at 0; the last line is what follows the last
/// terminator (empty when `b` ends with one).
pub fn lines(b: &[u8]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let (mut i, mut start) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'\n' => {
                out.push((start, i));
                start = i + 1;
            }
            b'\r' => {
                out.push((start, i));
                if b.get(i + 1) == Some(&b'\n') {
                    i += 1;
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push((start, b.len()));
    out
}

/// The byte offset where each line starts (line `k` is element `k - 1`).
pub fn starts(b: &[u8]) -> Vec<usize> {
    lines(b).into_iter().map(|(s, _)| s).collect()
}

/// The line (1-based) holding byte `off`, and that line's start. A
/// terminator belongs to the line it ends.
pub fn line_of(b: &[u8], off: usize) -> (usize, usize) {
    let s = starts(b);
    let k = s.partition_point(|&x| x <= off).max(1);
    (k, s[k - 1])
}

/// Whether `c` ends a line (a LF or a CR; a CR LF is one end).
pub fn is_end(c: u8) -> bool {
    c == b'\n' || c == b'\r'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_texts(n: usize) -> Vec<Vec<u8>> {
        let mut seed = 0x51f1_5eed_0bad_cafeu64;
        let mut rnd = |m: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed % m
        };
        (0..n)
            .map(|_| {
                let len = rnd(16) as usize;
                (0..len).map(|_| b"ab\n\r"[rnd(4) as usize]).collect()
            })
            .collect()
    }

    /// The engine's own reader (`system::read_tex_line`) gives these lines:
    /// the same contents, in the same order (the reader stops at the end of
    /// the file, so a final terminator's empty last line is not read).
    #[test]
    fn lines_are_the_engines() {
        for b in random_texts(20_000) {
            let mut r = std::io::Cursor::new(b.clone());
            let mut got: Vec<Vec<u8>> = vec![];
            let mut line = vec![];
            while crate::system::read_tex_line(&mut r, &mut line) {
                got.push(line.clone());
            }
            let mut want: Vec<Vec<u8>> = lines(&b).iter().map(|&(s, e)| b[s..e].to_vec()).collect();
            if b.is_empty() || is_end(*b.last().unwrap()) {
                want.pop();
            }
            assert_eq!(got, want, "{:?}", String::from_utf8_lossy(&b));
        }
    }

    /// The line shift counts line ends by the same rule
    /// (`lineshift::line_ends`).
    #[test]
    fn line_ends_count_these_lines() {
        for b in random_texts(20_000) {
            assert_eq!(
                crate::lineshift::line_ends(&b, 0, b.len()) + 1,
                lines(&b).len(),
                "{:?}",
                String::from_utf8_lossy(&b)
            );
        }
    }

    #[test]
    fn line_of_finds_the_line() {
        for b in random_texts(5_000) {
            let ls = lines(&b);
            for off in 0..b.len() {
                let (k, s) = line_of(&b, off);
                assert_eq!(s, ls[k - 1].0);
                // the byte is in the line or in its terminator
                let next = ls.get(k).map_or(b.len(), |l| l.0);
                assert!(s <= off && off < next, "{:?} at {off}", b);
            }
        }
    }
}
