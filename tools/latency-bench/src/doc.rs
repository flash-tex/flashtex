//! A benchmark document on disk and the scripted edits applied to it.
//!
//! The documents come from `gen.py`: one paragraph per source line, every
//! body paragraph starting with a capital letter. Edits are byte splices sent
//! in `COMPILE.edits` (spec §6.3), so the host writes the file as the editor
//! would; this side keeps its own copy of the text to compute offsets.

use flashtex_display_list::client::Edit;

/// Where in the prose an edit goes: the first, the middle or the last tenth
/// of the body paragraphs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Region {
    Start,
    Middle,
    End,
}

impl Region {
    pub const ALL: [Region; 3] = [Region::Start, Region::Middle, Region::End];
    pub fn name(self) -> &'static str {
        match self {
            Region::Start => "start",
            Region::Middle => "middle",
            Region::End => "end",
        }
    }
}

/// An edit site: a byte offset, its 1-based line and 0-based column.
#[derive(Clone, Copy, Debug)]
pub struct Site {
    pub offset: usize,
    pub line: u32,
    pub col: u32,
}

pub struct Doc {
    pub text: String,
    /// Byte offset of each line's start.
    starts: Vec<usize>,
    /// Line indices (0-based) of the body paragraphs.
    paras: Vec<usize>,
    /// Byte offset of the end of the last `\usepackage` line (before its
    /// newline): where a preamble edit goes.
    pub preamble_end: usize,
}

/// Twenty-eight words: enough to add two or three lines to a paragraph,
/// so the paragraph grows and every later page moves.
pub const SENTENCE: &str = "Inserted text reflows every later page of the document because \
these twenty eight words add at least two full lines to the paragraph that \
holds them and so push its tail onward. ";

impl Doc {
    pub fn new(text: String) -> Result<Doc, String> {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i + 1);
            }
        }
        let lines: Vec<&str> = text.split('\n').collect();
        let body = lines
            .iter()
            .position(|l| l.starts_with("\\begin{document}"))
            .ok_or("no \\begin{document}")?;
        let paras: Vec<usize> = (body + 1..lines.len())
            .filter(|&i| {
                lines[i]
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase)
                    && lines[i].len() > 200
            })
            .collect();
        if paras.len() < 10 {
            return Err(format!("only {} body paragraphs", paras.len()));
        }
        let pre = (0..body)
            .rev()
            .find(|&i| lines[i].starts_with("\\usepackage"))
            .ok_or("no \\usepackage line")?;
        let preamble_end = starts[pre] + lines[pre].len();
        Ok(Doc {
            text,
            starts,
            paras,
            preamble_end,
        })
    }

    /// The `trial`-th edit site of a region: a paragraph of that tenth, at
    /// the start of its 3rd to 8th word (before the paragraph's inline math,
    /// so the site is plain prose on the paragraph's first lines).
    pub fn site(&self, region: Region, trial: usize) -> Site {
        let n = self.paras.len();
        let (lo, hi) = match region {
            Region::Start => (0, n / 10),
            Region::Middle => (n * 45 / 100, n * 55 / 100),
            Region::End => (n * 9 / 10, n),
        };
        let span = (hi - lo).max(1);
        // A fixed stride visits different paragraphs of the tenth.
        let li = self.paras[lo + (trial * 7 + 3) % span];
        let start = self.starts[li];
        let line = &self.text[start..self.starts.get(li + 1).copied().unwrap_or(self.text.len())];
        let word = 3 + trial % 6;
        let col = line
            .match_indices(' ')
            .nth(word - 1)
            .map(|(i, _)| i + 1)
            .unwrap_or(0);
        Site {
            offset: start + col,
            line: li as u32 + 1,
            col: col as u32,
        }
    }

    /// Apply an edit here too (the host applies the same splice).
    pub fn apply(&mut self, e: &Edit) {
        let at = e.offset as usize;
        self.text
            .replace_range(at..at + e.delete as usize, &e.insert);
        // Line starts after the edit shift (edits never add or remove a
        // newline, so the line numbers stay).
        let delta = e.insert.len() as isize - e.delete as isize;
        for s in self.starts.iter_mut() {
            if *s > at {
                *s = (*s as isize + delta) as usize;
            }
        }
        if self.preamble_end >= at {
            self.preamble_end = (self.preamble_end as isize + delta) as usize;
        }
    }
}

pub fn insert(main: &str, offset: usize, text: &str) -> Edit {
    Edit {
        path: main.into(),
        offset: offset as u64,
        delete: 0,
        insert: text.into(),
    }
}

pub fn delete(main: &str, offset: usize, len: usize) -> Edit {
    Edit {
        path: main.into(),
        offset: offset as u64,
        delete: len as u64,
        insert: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text() -> String {
        let mut t =
            String::from("\\documentclass{article}\n\\usepackage{amsmath}\n\\begin{document}\n\n");
        for k in 0..40 {
            t.push_str(&format!("Para{k} "));
            t.push_str(&"lorem ipsum dolor sit amet ".repeat(10));
            t.push_str("end.\n\n");
        }
        t.push_str("\\end{document}\n");
        t
    }

    #[test]
    fn sites_are_word_starts_in_their_tenth() {
        let d = Doc::new(text()).unwrap();
        for region in Region::ALL {
            for trial in 0..6 {
                let s = d.site(region, trial);
                let b = d.text.as_bytes();
                assert_eq!(b[s.offset - 1], b' ');
                assert!(b[s.offset].is_ascii_lowercase());
                // Paragraph k is on line 5 + 2k.
                let k = (s.line - 5) / 2;
                let ok = match region {
                    Region::Start => k < 4,
                    Region::Middle => (18..22).contains(&k),
                    Region::End => k >= 36,
                };
                assert!(ok, "{region:?} trial {trial}: paragraph {k}");
            }
        }
        assert_eq!(&d.text[d.preamble_end - 9..d.preamble_end], "{amsmath}");
    }

    #[test]
    fn edit_then_revert_restores_the_text_and_sites() {
        let t = text();
        let mut d = Doc::new(t.clone()).unwrap();
        let s = d.site(Region::Middle, 2);
        let later = d.site(Region::End, 0);
        d.apply(&insert("main.tex", s.offset, SENTENCE));
        // A later site moves by the insertion, on the same line and column.
        let moved = d.site(Region::End, 0);
        assert_eq!(moved.offset, later.offset + SENTENCE.len());
        assert_eq!((moved.line, moved.col), (later.line, later.col));
        d.apply(&delete("main.tex", s.offset, SENTENCE.len()));
        assert_eq!(d.text, t);
    }
}
