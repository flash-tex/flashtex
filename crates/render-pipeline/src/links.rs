//! `display-list-v2-links` producer: the `navigation` object of
//! `protocol/proposals/display-list-v2-links.md`.
//!
//! The Mac consumer of this capability is already on `main`
//! (`apps/mac/Sources/FlashTeXMac/DisplayListLinks.swift`, decoded by
//! `FlashTeXProtocol/RenderingV2.swift`'s `Navigation`); this is the
//! producer half.
//!
//! ## Where the URI comes from
//!
//! The pinned `vendor/compiler` keeps only the *text* of a link:
//! `\url{u}` typesets `u` and `\href{u}{t}` drops `u` on the floor
//! (`parser.rs`, `"href" => { let (_url, url_span) = self.url_argument(..)`).
//! There is no `Parsed::hyperref` record to read — the compiler PR that
//! would have added one (#131) never merged. What does survive is the
//! *span*: every cluster of a link carries the source range of the
//! construct that produced it, so the URI is recovered by reading the
//! document's own bytes back at that span, the same way `\\[<dimen>]`'s
//! skip is recovered today (see the `linebreak-skip` feature note in
//! `Cargo.toml`), and with the same known limitation: a `\url`/`\href`
//! that comes out of a macro body has the *invocation*'s span, so it is
//! not matched and produces no link.
//!
//! Nothing here resolves, opens or executes anything: a URI is a string
//! copied out of the document, and a rectangle is arithmetic on ticks the
//! layout already produced. The consumer applies its own scheme policy
//! (`DisplayListLinks.allowedURISchemes`).

use std::collections::BTreeMap;
use std::ops::Range;

use crate::display::{Item, Page, PageContent, Provenance, Tick};

/// pdfTeX's `\pdflinkmargin`, which hyperref sets to 1pt (hpdftex.def 320):
/// every side of a link rectangle is grown by it.
pub const LINK_MARGIN_TEX_PT: f64 = 1.0;

/// A URI longer than this is not carried on the wire. pdfTeX imposes no
/// limit, but a link is only useful if a viewer can open it, and an
/// unbounded string from a document must not be able to inflate the
/// `display_list` line on its own.
const MAX_URI_BYTES: usize = 8192;

/// One `\url`/`\href` occurrence located in a source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkSpan {
    /// The destination, verbatim from the document.
    pub uri: String,
    /// hyperref's colour class. `\url` and an external `\href` are both
    /// `url`; `link` is hyperref's class for internal references, which
    /// this producer does not emit yet.
    pub class: &'static str,
    /// The document the span is in.
    pub path: String,
    /// The bytes whose clusters belong to this link: the whole `\url{..}`
    /// for `\url` (the compiler gives every piece of the typeset URL that
    /// span), the *text* group for `\href`.
    pub text: Range<usize>,
    /// The whole construct, for the wire's `source` object.
    pub source: Range<usize>,
}

/// A `navigation` object.
///
/// `destinations` is always serialised, empty included: the Mac model
/// decodes it as a non-optional dictionary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Navigation {
    pub links: Vec<Link>,
    pub destinations: BTreeMap<String, Destination>,
}

impl Navigation {
    pub fn is_empty(&self) -> bool {
        self.links.is_empty() && self.destinations.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    /// 1-based page number, as the envelope numbers pages.
    pub page: u32,
    /// One rectangle per line piece, in reading order.
    pub rects: Vec<LinkRect>,
    pub class: &'static str,
    pub uri: String,
    pub source: Option<Source>,
}

/// `[x0, y0, x1, y1]` in page ticks, y down from the page's top-left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkRect {
    pub x0: Tick,
    pub y0: Tick,
    pub x1: Tick,
    pub y1: Tick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub document: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Destination {
    pub page: u32,
    pub x: Tick,
    pub y: Tick,
}

/// Every `\url`/`\href` in `text`, in document order.
///
/// Mirrors `vendor/compiler`'s `url_argument`: braced arguments only,
/// nested braces counted, `\{`/`\}` unescaped. `%` starts a comment (and
/// `\%` does not), so a commented-out link is not reported. `\nolinkurl`
/// is url.sty's deliberately unlinked form and is skipped.
pub fn scan(path: &str, text: &str) -> Vec<LinkSpan> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < b.len() {
        match b[i] {
            b'%' => {
                // A comment runs to the end of the line.
                i += 1;
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'\\' => {
                let name_start = i + 1;
                let mut j = name_start;
                while j < b.len() && b[j].is_ascii_alphabetic() {
                    j += 1;
                }
                if j == name_start {
                    // A one-character control symbol (`\%`, `\\`, `\{`): it
                    // escapes whatever follows, so step over both bytes.
                    i = (name_start + 1).min(b.len());
                    continue;
                }
                let name = &text[name_start..j];
                match name {
                    "url" => {
                        if let Some(arg) = braced_argument(text, skip_spaces(text, j)) {
                            let uri = arg.content;
                            let span = i..arg.end;
                            push(&mut out, uri, "url", path, span.clone(), span);
                            i = arg.end;
                            continue;
                        }
                    }
                    "href" => {
                        if let Some(target) = braced_argument(text, skip_spaces(text, j)) {
                            if let Some(body) = braced_argument(text, skip_spaces(text, target.end)) {
                                push(
                                    &mut out,
                                    target.content,
                                    "url",
                                    path,
                                    body.inner.clone(),
                                    i..body.end,
                                );
                                i = body.end;
                                continue;
                            }
                        }
                    }
                    _ => {}
                }
                i = j;
            }
            _ => i += 1,
        }
    }
    out
}

fn push(out: &mut Vec<LinkSpan>, uri: String, class: &'static str, path: &str, text: Range<usize>, source: Range<usize>) {
    if uri.is_empty() || uri.len() > MAX_URI_BYTES {
        return;
    }
    out.push(LinkSpan { uri, class, path: path.to_string(), text, source });
}

fn skip_spaces(text: &str, mut i: usize) -> usize {
    let b = text.as_bytes();
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t') {
        i += 1;
    }
    i
}

struct Braced {
    /// The group's contents with `\{`/`\}` unescaped, as the compiler reads them.
    content: String,
    /// Byte range of the contents, braces excluded.
    inner: Range<usize>,
    /// One past the closing brace.
    end: usize,
}

/// Reads `{...}` starting at `open` (which must be the `{`), exactly as
/// `vendor/compiler`'s `url_argument` does.
fn braced_argument(text: &str, open: usize) -> Option<Braced> {
    if text.as_bytes().get(open) != Some(&b'{') {
        return None;
    }
    let mut depth = 1usize;
    let mut content = String::new();
    let mut pos = open + 1;
    let inner_start = pos;
    loop {
        let ch = text[pos..].chars().next()?;
        let ch_len = ch.len_utf8();
        match ch {
            '\\' if matches!(text[pos + ch_len..].chars().next(), Some('{' | '}')) => {
                let escaped = text[pos + ch_len..].chars().next().unwrap();
                content.push(escaped);
                pos += ch_len + escaped.len_utf8();
            }
            '{' => {
                depth += 1;
                content.push(ch);
                pos += ch_len;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(Braced { content, inner: inner_start..pos, end: pos + ch_len });
                }
                content.push(ch);
                pos += ch_len;
            }
            _ => {
                content.push(ch);
                pos += ch_len;
            }
        }
    }
}

/// The `navigation` object for `pages`, given every link span of every
/// source document. `None` when the document has no link at all, so a
/// document without links serialises exactly as it did before.
pub fn navigation(pages: &[Page], spans: &[LinkSpan]) -> Option<Navigation> {
    if spans.is_empty() {
        return None;
    }
    let margin = Tick::from_tex_pt(LINK_MARGIN_TEX_PT);
    // One accumulator per span, so a link's line pieces stay together and
    // in reading order however the pages interleave them.
    let mut pieces: Vec<Vec<(u32, LinkRect)>> = vec![Vec::new(); spans.len()];
    let mut bands: BTreeMap<u32, Vec<(i64, i64)>> = BTreeMap::new();
    // Attribution runs once per cluster -- hundreds of thousands of times on a
    // real document -- so it may not walk the span list. `scan` yields one
    // document's spans sorted by start and never overlapping (a `\href` skips
    // past its own body), so a binary search per document answers it.
    let index = SpanIndex::of(spans);
    for page in pages {
        let PageContent::Resident(items) = &page.content else { continue };
        // pdfTeX takes a link's height and depth from the *line box*, not
        // from the linked glyphs: on one line, a roman `\href` and a
        // typewriter `\url` get the same top and bottom (measured against
        // pdflatex, `tests/links_navigation.rs`). The display list has no
        // line objects, so the line box is recovered as the union of every
        // cluster box the line carries -- which is what TeX's `\hbox` height
        // and depth are.
        let band = bands.entry(page.number).or_default();
        for item in items {
            let Item::GlyphRun(run) = item else { continue };
            for cluster in &run.clusters {
                band.push((cluster.hit_rect.top.0, cluster.hit_rect.top.0 + cluster.hit_rect.height.0));
            }
        }
        band.sort_unstable();
        band.dedup();
        for item in items {
            let Item::GlyphRun(run) = item else { continue };
            for cluster in &run.clusters {
                let Provenance::Source(sr) = &cluster.provenance else { continue };
                let Some(which) = index.containing(spans, sr.path.as_ref(), sr.start_byte, sr.end_byte) else {
                    continue;
                };
                let r = cluster.hit_rect;
                let (x0, y0) = (r.x, r.top);
                let (x1, y1) = (Tick(r.x.0 + r.width.0), Tick(r.top.0 + r.height.0));
                let acc = &mut pieces[which];
                match acc.last_mut() {
                    // Same page, vertically overlapping, and still moving
                    // left to right: the same line piece.
                    Some((p, last))
                        if *p == page.number && y0.0 < last.y1.0 && y1.0 > last.y0.0 && x0.0 >= last.x0.0 =>
                    {
                        last.x0 = Tick(last.x0.0.min(x0.0));
                        last.y0 = Tick(last.y0.0.min(y0.0));
                        last.x1 = Tick(last.x1.0.max(x1.0));
                        last.y1 = Tick(last.y1.0.max(y1.0));
                    }
                    _ => acc.push((page.number, LinkRect { x0, y0, x1, y1 })),
                }
            }
        }
    }
    let mut links = Vec::new();
    for (span, acc) in spans.iter().zip(pieces) {
        // A link's pieces are one wire entry per page, since `page` is a
        // property of the entry and not of the rectangle.
        let mut by_page: Vec<(u32, Vec<LinkRect>)> = Vec::new();
        for (page, rect) in acc {
            let empty: Vec<(i64, i64)> = Vec::new();
            let (top, bottom) = line_extent(bands.get(&page).unwrap_or(&empty), rect.y0.0, rect.y1.0);
            let grown = LinkRect {
                x0: Tick(rect.x0.0 - margin.0),
                y0: Tick(top - margin.0),
                x1: Tick(rect.x1.0 + margin.0),
                y1: Tick(bottom + margin.0),
            };
            match by_page.last_mut() {
                Some((p, rects)) if *p == page => rects.push(grown),
                _ => by_page.push((page, vec![grown])),
            }
        }
        for (page, rects) in by_page {
            links.push(Link {
                page,
                rects,
                class: span.class,
                uri: span.uri.clone(),
                source: Some(Source {
                    document: span.path.clone(),
                    start: span.source.start,
                    end: span.source.end,
                }),
            });
        }
    }
    if links.is_empty() {
        return None;
    }
    links.sort_by_key(|l| l.page);
    Some(Navigation { links, destinations: BTreeMap::new() })
}

/// The vertical extent of the line box `y0..y1` sits in: `y0..y1` grown by
/// every cluster box on the page it overlaps.
///
/// Deliberately one hop, not a transitive merge: chaining would let one tall
/// display formula fuse a whole column into a single band, and pdfTeX's own
/// rectangle is the height and depth of *that* line's `\hbox`.
fn line_extent(bands: &[(i64, i64)], y0: i64, y1: i64) -> (i64, i64) {
    let (mut top, mut bottom) = (y0, y1);
    // Sorted by `top`, so the scan can stop once a band starts below `y1`.
    for &(t, b) in bands {
        if t >= y1 {
            break;
        }
        if b > y0 {
            top = top.min(t);
            bottom = bottom.max(b);
        }
    }
    (top, bottom)
}

/// The spans of each document, sorted by start byte, as indices into the
/// flat span list.
struct SpanIndex {
    by_path: BTreeMap<String, Vec<usize>>,
}

impl SpanIndex {
    fn of(spans: &[LinkSpan]) -> SpanIndex {
        let mut by_path: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, s) in spans.iter().enumerate() {
            by_path.entry(s.path.clone()).or_default().push(i);
        }
        for v in by_path.values_mut() {
            v.sort_by_key(|i| spans[*i].text.start);
        }
        by_path
            .values()
            .for_each(|v| debug_assert!(v.windows(2).all(|w| spans[w[0]].text.end <= spans[w[1]].text.start)));
        SpanIndex { by_path }
    }

    /// The span of `path` containing `start..end`, if any.
    fn containing(&self, spans: &[LinkSpan], path: &str, start: usize, end: usize) -> Option<usize> {
        let of_path = self.by_path.get(path)?;
        // The last span that begins at or before `start`; spans do not
        // overlap, so no earlier one can reach this far.
        let at = of_path.partition_point(|i| spans[*i].text.start <= start).checked_sub(1)?;
        let which = of_path[at];
        (end <= spans[which].text.end).then_some(which)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uris(text: &str) -> Vec<(&'static str, String, Range<usize>)> {
        scan("main.tex", text).into_iter().map(|s| (s.class, s.uri, s.text)).collect()
    }

    #[test]
    fn url_span_covers_the_whole_command_and_href_span_covers_its_text() {
        let text = r"See \url{https://a.example} and \href{https://b.example}{click}.";
        let found = scan("main.tex", text);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].uri, "https://a.example");
        assert_eq!(&text[found[0].text.clone()], r"\url{https://a.example}");
        assert_eq!(found[1].uri, "https://b.example");
        assert_eq!(&text[found[1].text.clone()], "click");
        assert_eq!(&text[found[1].source.clone()], r"\href{https://b.example}{click}");
    }

    #[test]
    fn nolinkurl_is_not_a_link_and_a_commented_url_is_not_either() {
        assert!(uris(r"\nolinkurl{https://a.example}").is_empty());
        assert!(uris("% \\url{https://a.example}\n").is_empty());
        // `\%` is an escaped percent, not the start of a comment.
        assert_eq!(uris(r"100\% \url{https://a.example}").len(), 1);
    }

    #[test]
    fn nested_and_escaped_braces_follow_the_compilers_url_argument() {
        let text = r"\href{https://a.example/\{x\}}{a {bold} b}";
        let found = scan("main.tex", text);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].uri, "https://a.example/{x}");
        assert_eq!(&text[found[0].text.clone()], "a {bold} b");
        assert_eq!(&text[found[0].source.clone()], text);
    }

    #[test]
    fn an_unterminated_or_empty_argument_yields_nothing() {
        assert!(uris(r"\url{https://a.example").is_empty());
        assert!(uris(r"\url{}").is_empty());
        assert!(uris(r"\href{https://a.example}").is_empty());
    }

    #[test]
    fn an_oversized_uri_is_dropped_rather_than_carried() {
        let long = "h".repeat(MAX_URI_BYTES + 1);
        assert!(uris(&format!("\\url{{{long}}}")).is_empty());
    }
}
