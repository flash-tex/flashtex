//! Document-level layout with an incremental `relayout`.
//!
//! A document is plain text whose paragraphs are separated by blank lines
//! (LaTeX's rule). `layout_document` breaks every paragraph and pages them;
//! `relayout` takes the previous result and one byte-range edit, reuses the
//! layout of every paragraph whose text did not change (shifting source
//! offsets after the edit), re-breaks only the affected paragraph(s), and
//! re-pages. Because a paragraph's lines depend only on its text and its
//! start offset, the incremental result is identical to a clean layout —
//! `tests/incremental.rs` checks that property over random edits.

use std::ops::Range;

use crate::hyphenate::Hyphenator;
use crate::items::{Glue, ParagraphBuilder};
use crate::linebreak::{LineBreakParams, Lines, layout_paragraph};
use crate::metrics::FontMetricsSource;
use crate::pages::{PageParams, Pages, ParagraphBlock, layout_pages};

/// Everything needed to lay out a document from its text.
pub struct DocumentSpec<'a> {
    pub text: &'a str,
    pub font: &'a dyn FontMetricsSource,
    pub size: f64,
    pub hyphenator: &'a dyn Hyphenator,
    pub line: LineBreakParams,
    pub page: PageParams,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphLayout {
    /// Byte range of the paragraph text in the document.
    pub source: Range<usize>,
    /// The paragraph text (kept so `relayout` can detect unchanged paragraphs).
    pub text: String,
    pub lines: Lines,
}

/// What `relayout` did; `layout_document` reports everything as relaid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelayoutStats {
    /// Paragraphs before the edit reused verbatim.
    pub reused_before: usize,
    /// Paragraphs after the edit reused with shifted offsets.
    pub reused_after: usize,
    /// Paragraphs re-broken.
    pub relaid: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocumentLayout {
    pub paragraphs: Vec<ParagraphLayout>,
    pub pages: Pages,
    pub stats: RelayoutStats,
}

impl DocumentLayout {
    /// Every layout diagnostic with its paragraph index.
    pub fn diagnostics(&self) -> impl Iterator<Item = (usize, &crate::linebreak::Diagnostic)> {
        self.paragraphs
            .iter()
            .enumerate()
            .flat_map(|(i, p)| p.lines.diagnostics.iter().map(move |d| (i, d)))
    }
}

/// Splits `text` into paragraph byte ranges: maximal runs of lines that are
/// not blank, trimmed of surrounding whitespace.
pub fn paragraph_ranges(text: &str) -> Vec<Range<usize>> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // Skip whitespace (including blank lines).
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let start = i;
        // Advance to a blank line (newline followed by optional spaces and newline) or EOF.
        let mut end = i;
        while i < bytes.len() {
            if bytes[i] == b'\n' {
                let mut j = i + 1;
                while j < bytes.len()
                    && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\r')
                {
                    j += 1;
                }
                if j >= bytes.len() || bytes[j] == b'\n' {
                    i = j;
                    break;
                }
            }
            if !bytes[i].is_ascii_whitespace() {
                end = i + 1;
            }
            i += 1;
        }
        out.push(start..end);
    }
    out
}

fn break_one(spec: &DocumentSpec<'_>, range: Range<usize>) -> ParagraphLayout {
    let text = &spec.text[range.clone()];
    let mut b = ParagraphBuilder::new(spec.hyphenator);
    b.text(spec.font, spec.size, text, range.start)
        .expect("document text is plain text; the hyphenator is this crate's own");
    let items = b.finish(Glue::fil());
    ParagraphLayout {
        source: range,
        text: text.to_string(),
        lines: layout_paragraph(&items, &spec.line)
            .expect("document text is a bounded, finite item list"),
    }
}

fn page(
    spec: &DocumentSpec<'_>,
    paragraphs: Vec<ParagraphLayout>,
    stats: RelayoutStats,
) -> DocumentLayout {
    let blocks: Vec<ParagraphBlock> = paragraphs
        .iter()
        .map(|p| ParagraphBlock::body(p.lines.clone()))
        .collect();
    let pages = layout_pages(&blocks, &spec.page);
    DocumentLayout {
        paragraphs,
        pages,
        stats,
    }
}

/// Clean layout of the whole document.
pub fn layout_document(spec: &DocumentSpec<'_>) -> DocumentLayout {
    let paragraphs: Vec<ParagraphLayout> = paragraph_ranges(spec.text)
        .into_iter()
        .map(|r| break_one(spec, r))
        .collect();
    let stats = RelayoutStats {
        relaid: paragraphs.len(),
        ..Default::default()
    };
    page(spec, paragraphs, stats)
}

/// Incremental layout after replacing `edit` (a byte range of the *previous*
/// text) with `replacement_len` bytes; `spec.text` is the new text.
///
/// Paragraphs that end before the edit are reused as they are; paragraphs
/// that start after the edit and whose text is unchanged are reused with
/// their source offsets shifted by the edit's size delta; the rest are
/// re-broken. Pages are always rebuilt (cheap, and page breaks may move).
pub fn relayout(
    previous: &DocumentLayout,
    spec: &DocumentSpec<'_>,
    edit: Range<usize>,
    replacement_len: usize,
) -> DocumentLayout {
    let new_ranges = paragraph_ranges(spec.text);
    let delta = replacement_len as isize - edit.len() as isize;
    let old = &previous.paragraphs;

    // Prefix: identical text at identical offsets, entirely before the edit.
    let mut prefix = 0;
    while prefix < old.len() && prefix < new_ranges.len() {
        let o = &old[prefix];
        let n = &new_ranges[prefix];
        // A paragraph adjacent to the edit may have absorbed or lost a
        // separator, so require it to end strictly before the edit start.
        if o.source == *n && o.source.end < edit.start && spec.text[n.clone()] == o.text {
            prefix += 1;
        } else {
            break;
        }
    }
    // Suffix: identical text, offsets shifted by delta, entirely after the edit.
    let mut suffix = 0;
    while suffix < old.len() - prefix && suffix < new_ranges.len() - prefix {
        let o = &old[old.len() - 1 - suffix];
        let n = &new_ranges[new_ranges.len() - 1 - suffix];
        let shifted =
            (o.source.start as isize + delta) as usize..(o.source.end as isize + delta) as usize;
        if shifted == *n && o.source.start > edit.end && spec.text[n.clone()] == o.text {
            suffix += 1;
        } else {
            break;
        }
    }

    let mut paragraphs = Vec::with_capacity(new_ranges.len());
    paragraphs.extend(old[..prefix].iter().cloned());
    let middle = &new_ranges[prefix..new_ranges.len() - suffix];
    for r in middle {
        paragraphs.push(break_one(spec, r.clone()));
    }
    for o in &old[old.len() - suffix..] {
        let mut p = o.clone();
        p.source =
            (o.source.start as isize + delta) as usize..(o.source.end as isize + delta) as usize;
        shift_lines(&mut p.lines, delta);
        paragraphs.push(p);
    }
    let stats = RelayoutStats {
        reused_before: prefix,
        reused_after: suffix,
        relaid: middle.len(),
    };
    page(spec, paragraphs, stats)
}

fn shift_range(r: &mut Range<usize>, delta: isize) {
    *r = (r.start as isize + delta) as usize..(r.end as isize + delta) as usize;
}

/// Shifts every source byte range in `lines` by `delta`.
pub fn shift_lines(lines: &mut Lines, delta: isize) {
    if delta == 0 {
        return;
    }
    for line in &mut lines.lines {
        for run in &mut line.runs {
            shift_range(&mut run.source, delta);
            for g in &mut run.glyphs {
                shift_range(&mut g.cluster, delta);
            }
        }
    }
    for d in &mut lines.diagnostics {
        if let Some(s) = &mut d.source {
            shift_range(s, delta);
        }
        for b in &mut d.boxes {
            shift_range(b, delta);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_split_on_blank_lines() {
        let t = "  first para\nstill first\n\n\nsecond  \n   \nthird";
        let r = paragraph_ranges(t);
        assert_eq!(r.len(), 3);
        assert_eq!(&t[r[0].clone()], "first para\nstill first");
        assert_eq!(&t[r[1].clone()], "second");
        assert_eq!(&t[r[2].clone()], "third");
        assert!(paragraph_ranges("\n\n  \n").is_empty());
    }
}
