//! The `multicol` package: `multicols` and `multicols*` (multicol.sty
//! 2025/10/21 v2.0b, Frank Mittelbach; line numbers below are the TeX Live
//! 2026 `tex/latex/tools/multicol.sty`).
//!
//! multicol works through TeX's page builder and output routines, so this
//! module runs them: a vertical-mode contribution list, TeX's page builder
//! (§§980–1028: `\topskip`, `\pagegoal`/`\pagetotal`, page costs,
//! `fire_up` with the material after the best break returned to the
//! contributions), `\vsplit` (§§967–979 `vert_break`, `prune_page_top`)
//! and `\vbox to` (§§668–678 glue setting and badness). On top of that,
//! transcribed from multicol.sty:
//!
//! * `\mult@@cols` (172–205): `\enough@room{\premulticols}` (209–225: an
//!   `\addpenalty\z@`, then `\newpage` when `\pagegoal-\pagetotal` is
//!   short), the preface at full width, `\addvspace\multicolsep`, the
//!   `\prevdepth` kern that keeps the baseline grid, and
//!   `\prepare@multicols` (226–292): the page so far is saved in
//!   `\partial@page` by an `\eject` under a special `\output`,
//!   `\@colroom` loses its height, `\vsize` becomes `\set@mult@vsize`
//!   (297–306), paragraphs are broken at `\hsize` = (`\linewidth` +
//!   `\columnsep`)/n − `\columnsep` with `\tolerance 9999`,
//!   `\pretolerance -1` and `\emergencystretch` 4pt·n;
//! * `\multi@column@out` (422–499): full pages, every column
//!   `\vsplit` to `\@colroom` and set `\vbox to` it (`\raggedcolumns`:
//!   `\vfilmaxdepth`), leftover material back to the contributions;
//! * `\speci@ls` (504–552): `\columnbreak` (penalty −10005) saved in
//!   `\colbreak@box`, the end penalty −10006 running
//!   `\balance@columns@out` (568–605) and `\balance@columns` (606–797:
//!   the start height from `(\ht+\dp)/n` rounded to `\baselineskip`,
//!   1pt steps, `\c@columnbadness`, `\c@finalcolumnbadness`, the
//!   natural-height retry, `\multicolundershoot`, `\maxbalancingoverflow`,
//!   `\c@unbalance`, `\c@minrows`);
//! * `\page@sofar` (382–415) and `\LR@column@boxes` (952–964): columns
//!   side by side, `\columnseprule` centred in the `\hss` gap, depth at
//!   least that of `p`;
//! * `\endmulticols` (310–353) with `\enough@room\postmulticols`, and
//!   `multicols*` (894–917): `\multi@column@out` instead of balancing.
//!
//! The environment is found in the source (`scan`) and blanked before the
//! compiler parses it (`\begin`/`\end` become `\par`, as the environment
//! does), so the compiler never sees it. Only documents that contain a
//! `multicols` environment take this path; every other document is laid
//! out by `pagebuild` exactly as before.
//!
//! Not implemented (a `multicol` diagnostic says so): multicols in a
//! two-column document, nested (boxed) multicols, floats anywhere in a
//! document with multicols, footnotes set full width at the page bottom,
//! `\columnseprulecolor`, right-to-left columns.

use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use flashtex_compiler::{DocumentId, Span};

use crate::adapter::{Block, Doc, Item as AItem, ParaPart, TextStyle};
use crate::display::Diagnostic;
use crate::pagebuild::{badness, BuiltPage, PageParams, Placed, VBlock, AWFUL_BAD, DEPLORABLE, EJECT_PENALTY, INF_BAD, INF_PENALTY};
use crate::style::Stylesheet;

use super::{floatpage, BoxRec, BuiltBlock, Context, Laid};

const COLUMNBREAK: i32 = -10005;
const END_PENALTY: i32 = -10006;
const MAXDIMEN: f64 = 16383.99999;

// ---------------------------------------------------------------------------
// Source scan.

/// A length assignment found in the source (`\setlength{\columnsep}{2em}`).
#[derive(Debug, Clone, PartialEq, Default)]
struct Len {
    /// `(natural, stretch, shrink)`: points plus ems (resolved with the
    /// body font's quad at layout time).
    pt: [f64; 3],
    em: [f64; 3],
}

#[derive(Debug, Clone, PartialEq, Default)]
struct Settings {
    columnsep: Option<Len>,
    columnseprule: Option<Len>,
    multicolsep: Option<Len>,
    premulticols: Option<Len>,
    postmulticols: Option<Len>,
    counters: BTreeMap<String, i64>,
    ragged: bool,
}

/// One top-level `multicols`/`multicols*` environment.
#[derive(Debug, Clone, PartialEq)]
pub struct Region {
    star: bool,
    /// `\begin{multicols}` (through `}`), where the environment starts.
    begin: (usize, usize),
    /// The column count as written.
    columns_arg: i64,
    preface: Option<(usize, usize)>,
    premulticols: Option<Len>,
    body: (usize, usize),
    /// `\end{multicols}`.
    end: (usize, usize),
    /// `\columnbreak[n]` (penalty) and `\newcolumn` (`None`) positions.
    breaks: Vec<(usize, Option<i32>)>,
    settings: Settings,
    /// Counters in force at `\end{multicols}` (`unbalance` and friends).
    end_counters: BTreeMap<String, i64>,
    postmulticols: Option<Len>,
    floats: Vec<usize>,
    nested: Vec<usize>,
}

/// What `scan` found in one document.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scan {
    regions: Vec<Region>,
    /// Byte ranges to blank; `par` ranges start with `\par`.
    masks: Vec<(usize, usize, bool)>,
    /// `\columnbreak` outside any `multicols`.
    stray_breaks: Vec<usize>,
    twocolumn_option: bool,
}

impl Scan {
    /// The source with the environment markup blanked (`None`: nothing to
    /// blank). Byte offsets are unchanged.
    pub fn masked(&self, text: &str) -> Option<String> {
        if self.masks.is_empty() {
            return None;
        }
        let mut bytes = text.as_bytes().to_vec();
        for &(a, b, par) in &self.masks {
            for x in &mut bytes[a..b] {
                *x = b' ';
            }
            if par && b - a >= 4 {
                bytes[a..a + 4].copy_from_slice(b"\\par");
            }
        }
        String::from_utf8(bytes).ok()
    }

    pub fn is_empty(&self) -> bool {
        self.regions.is_empty() && self.stray_breaks.is_empty()
    }
}

fn is_commented(text: &str, pos: usize) -> bool {
    let line_start = text[..pos].rfind('\n').map_or(0, |i| i + 1);
    let b = text[line_start..pos].as_bytes();
    (0..b.len()).any(|i| b[i] == b'%' && (i == 0 || b[i - 1] != b'\\'))
}

fn skip_spaces(b: &[u8], mut i: usize) -> usize {
    // `\@ifnextchar` and argument scanning skip spaces and one end of line.
    let mut newlines = 0;
    while i < b.len() && (b[i] == b' ' || b[i] == b'\t' || b[i] == b'\n' || b[i] == b'\r') {
        if b[i] == b'\n' {
            newlines += 1;
            if newlines > 1 {
                break;
            }
        }
        i += 1;
    }
    i
}

/// The end (exclusive) of a balanced group opening at `open` with `o`/`c`.
fn balanced(b: &[u8], open: usize, o: u8, c: u8) -> Option<usize> {
    let mut depth = 0i32;
    let mut brace = 0i32;
    let mut i = open;
    while i < b.len() {
        match b[i] {
            b'\\' => {
                i += 2;
                continue;
            }
            b'%' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            x if x == o && (o == b'{' || brace == 0) => depth += 1,
            x if x == c && (c == b'}' || brace == 0) => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            b'{' => brace += 1,
            b'}' => brace -= 1,
            _ => {}
        }
        i += 1;
    }
    None
}

/// `12pt plus 4pt minus 3pt`, `2em`, `.4pt`.
fn parse_len(s: &str) -> Option<Len> {
    let mut len = Len::default();
    let s = s.trim();
    let mut slot = 0usize;
    let mut rest = s;
    let mut any = false;
    while !rest.trim().is_empty() {
        let r = rest.trim_start();
        if let Some(t) = r.strip_prefix("plus") {
            slot = 1;
            rest = t;
            continue;
        }
        if let Some(t) = r.strip_prefix("minus") {
            slot = 2;
            rest = t;
            continue;
        }
        let num_end = r.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == ' ')).unwrap_or(r.len());
        let num: String = r[..num_end].chars().filter(|c| *c != ' ').collect();
        let after = &r[num_end..];
        let unit_end = after.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(after.len());
        let unit = &after[..unit_end];
        let value: f64 = if num.is_empty() || num == "-" || num == "+" { if num == "-" { -1.0 } else { 1.0 } } else { num.parse().ok()? };
        match unit {
            "em" => len.em[slot] += value,
            "ex" => len.em[slot] += value * 0.43,
            "fil" | "fill" | "filll" => {}
            _ => {
                let sp = flashtex_class_geometry::tex::Sp::parse(&format!("{}{}", if num.is_empty() { "1" } else { &num }, unit))?;
                len.pt[slot] += sp.to_pt();
            }
        }
        any = true;
        rest = &after[unit_end..];
    }
    any.then_some(len)
}

/// Finds every `multicols` environment and the settings multicol reads.
pub fn scan(text: &str) -> Scan {
    let mut out = Scan::default();
    if !text.contains("multicols") && !text.contains("\\columnbreak") {
        return out;
    }
    let b = text.as_bytes();
    let body_start = text.find("\\begin{document}").unwrap_or(0);
    if let Some(dc) = text.find("\\documentclass") {
        if let Some(open) = text[dc..].find('[').map(|o| dc + o) {
            if let Some(close) = text[open..].find(']').map(|c| open + c) {
                if text[..open].len() == open && !text[dc..open].contains('{') {
                    out.twocolumn_option = text[open + 1..close].split(',').any(|o| o.trim() == "twocolumn");
                }
            }
        }
    }
    let mut settings = Settings::default();
    // (region start index into out.regions, settings before it) while open.
    let mut open: Option<(Region, Settings, usize)> = None;
    let mut i = 0usize;
    while let Some(rel) = text[i..].find('\\') {
        let at = i + rel;
        let name_end = at + 1 + text[at + 1..].find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(text.len() - at - 1);
        let name = &text[at + 1..name_end];
        i = name_end.max(at + 2);
        if name.is_empty() || is_commented(text, at) {
            continue;
        }
        let cur = match &mut open {
            Some((_, s, _)) => s,
            None => &mut settings,
        };
        match name {
            "begin" | "end" => {
                let Some(g) = (b.get(name_end) == Some(&b'{')).then(|| balanced(b, name_end, b'{', b'}')).flatten() else { continue };
                let env = &text[name_end + 1..g - 1];
                i = g;
                if name == "begin" && matches!(env, "figure" | "table") && at > body_start {
                    if let Some((r, _, _)) = &mut open {
                        r.floats.push(at);
                    }
                    continue;
                }
                if env != "multicols" && env != "multicols*" {
                    continue;
                }
                if name == "begin" {
                    if let Some((r, _, depth)) = &mut open {
                        r.nested.push(at);
                        *depth += 1;
                        out.masks.push((at, g, true));
                        continue;
                    }
                    // `{<n>}`
                    let j = skip_spaces(b, g);
                    let (columns_arg, mut k) = match (b.get(j) == Some(&b'{')).then(|| balanced(b, j, b'{', b'}')).flatten() {
                        Some(e) => (text[j + 1..e - 1].trim().parse::<i64>().unwrap_or(2), e),
                        None => (2, g),
                    };
                    let head_end = k;
                    let mut preface = None;
                    let mut premulticols = None;
                    let mut masks = vec![(at, head_end, true)];
                    let j = skip_spaces(b, k);
                    if b.get(j) == Some(&b'[') {
                        if let Some(e) = balanced(b, j, b'[', b']') {
                            preface = Some((j + 1, e - 1));
                            masks.push((j, j + 1, false));
                            masks.push((e - 1, e, false));
                            k = e;
                            let j2 = skip_spaces(b, k);
                            if b.get(j2) == Some(&b'[') {
                                if let Some(e2) = balanced(b, j2, b'[', b']') {
                                    premulticols = parse_len(&text[j2 + 1..e2 - 1]);
                                    masks.push((j2, e2, false));
                                    k = e2;
                                }
                            }
                        }
                    }
                    out.masks.extend(masks);
                    let region = Region {
                        star: env.ends_with('*'),
                        begin: (at, g),
                        columns_arg,
                        preface,
                        premulticols,
                        body: (k, k),
                        end: (k, k),
                        breaks: Vec::new(),
                        settings: settings.clone(),
                        end_counters: BTreeMap::new(),
                        postmulticols: None,
                        floats: Vec::new(),
                        nested: Vec::new(),
                    };
                    let inner = settings.clone();
                    open = Some((region, inner, 0));
                    i = k;
                } else if let Some((mut r, inner, depth)) = open.take() {
                    out.masks.push((at, g, true));
                    if depth > 0 {
                        open = Some((r, inner, depth - 1));
                        continue;
                    }
                    r.body.1 = at;
                    r.end = (at, g);
                    r.end_counters = inner.counters.clone();
                    r.postmulticols = settings.postmulticols.clone();
                    r.settings.ragged = inner.ragged;
                    // `\setcounter` is global; `\c@unbalance` is reset.
                    settings.counters = inner.counters;
                    settings.counters.remove("unbalance");
                    out.regions.push(r);
                }
            }
            "columnbreak" | "newcolumn" => {
                let mut e = name_end;
                let mut pen = if name == "newcolumn" { None } else { Some(COLUMNBREAK) };
                if name == "columnbreak" && b.get(e) == Some(&b'[') {
                    if let Some(close) = balanced(b, e, b'[', b']') {
                        // `\columnbreak[n]`: -\@m, 3333, 6666, 9999 or \@Mv (927).
                        pen = Some(match text[e + 1..close - 1].trim().parse::<i32>().unwrap_or(4) {
                            0 => -1000,
                            1 => -3333,
                            2 => -6666,
                            3 => -9999,
                            _ => COLUMNBREAK,
                        });
                        e = close;
                    }
                }
                out.masks.push((at, e, false));
                i = e;
                match &mut open {
                    Some((r, _, 0)) => r.breaks.push((at, pen)),
                    Some(_) => {}
                    None => out.stray_breaks.push(at),
                }
            }
            "raggedcolumns" | "flushcolumns" => {
                cur.ragged = name == "raggedcolumns";
                out.masks.push((at, name_end, false));
            }
            "setlength" | "addtolength" => {
                // `\setlength{\name}{value}` or `\setlength\name{value}`.
                let j = skip_spaces(b, name_end);
                let (target, after) = if b.get(j) == Some(&b'{') {
                    match balanced(b, j, b'{', b'}') {
                        Some(e) => (text[j + 1..e - 1].trim(), e),
                        None => continue,
                    }
                } else if b.get(j) == Some(&b'\\') {
                    let e = j + 1 + text[j + 1..].find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(0);
                    (&text[j..e], e)
                } else {
                    continue;
                };
                let v = skip_spaces(b, after);
                let Some(e) = (b.get(v) == Some(&b'{')).then(|| balanced(b, v, b'{', b'}')).flatten() else { continue };
                let Some(len) = parse_len(&text[v + 1..e - 1]) else { continue };
                if name == "addtolength" {
                    continue;
                }
                let slot = match target {
                    "\\columnsep" => &mut cur.columnsep,
                    "\\columnseprule" => &mut cur.columnseprule,
                    "\\multicolsep" => &mut cur.multicolsep,
                    "\\premulticols" => &mut cur.premulticols,
                    "\\postmulticols" => &mut cur.postmulticols,
                    _ => continue,
                };
                *slot = Some(len);
                i = e;
            }
            "setcounter" => {
                let j = skip_spaces(b, name_end);
                let Some(e) = (b.get(j) == Some(&b'{')).then(|| balanced(b, j, b'{', b'}')).flatten() else { continue };
                let counter = text[j + 1..e - 1].trim().to_string();
                if !matches!(counter.as_str(), "unbalance" | "columnbadness" | "finalcolumnbadness" | "collectmore" | "minrows") {
                    continue;
                }
                let v = skip_spaces(b, e);
                let Some(e2) = (b.get(v) == Some(&b'{')).then(|| balanced(b, v, b'{', b'}')).flatten() else { continue };
                if let Ok(n) = text[v + 1..e2 - 1].trim().parse::<i64>() {
                    cur.counters.insert(counter, n);
                }
                i = e2;
            }
            _ => {}
        }
    }
    if let Some((r, _, _)) = open {
        // No `\end{multicols}`: the compiler reports the open environment;
        // its markup stays blanked and the body is set as plain text.
        let _ = r;
    }
    out
}

// ---------------------------------------------------------------------------
// Pass state kept on the `Context`.

#[derive(Debug, Default)]
pub struct State {
    scans: Vec<Scan>,
    /// The outer document of this build replaced region bodies by markers.
    active: bool,
    /// Region bodies (adapter blocks) by `(document, region)`.
    bodies: BTreeMap<(usize, usize), Vec<Block>>,
    /// Extra x offset of every placed line, per built page.
    dx: Vec<Vec<f64>>,
}

/// Hands the scans of the project's documents (indexed like the paths) to
/// a context before `build_with_floats`.
pub fn attach(ctx: &mut Context, scans: &[Scan]) {
    ctx.multicol = State { scans: scans.to_vec(), ..State::default() };
}

fn source(ctx: &Context, document: usize, start: usize, end: usize) -> Vec<crate::display::SourceRange> {
    vec![ctx.source(Span::in_document(DocumentId(document), start, end))]
}

fn warn(ctx: &mut Context, key: String, message: String, document: usize, at: (usize, usize)) {
    let d = Diagnostic::warning("multicol", message, source(ctx, document, at.0, at.1));
    ctx.report_once(key, d);
}

// ---------------------------------------------------------------------------
// Hook 1: the outer document.

fn item_start(it: &AItem) -> Option<usize> {
    match it {
        AItem::Word(w) => w.segments.iter().flat_map(|s| s.chars.iter()).map(|c| c.start).min(),
        AItem::Math { span, .. } => Some(span.start),
        _ => None,
    }
}

fn part_start(p: &ParaPart) -> Option<(usize, usize)> {
    match p {
        ParaPart::Lines(items) => crate::incremental::block_origin(items).map(|(d, s)| (d.0, s)),
        ParaPart::Rows { span, .. } | ParaPart::Display { span, .. } => Some((span.document.0, span.start)),
    }
}

fn block_start(b: &Block) -> Option<(usize, usize)> {
    match b {
        Block::Paragraph { parts, .. } => parts.iter().find_map(part_start),
        Block::Heading { span, .. }
        | Block::Chapter { span, .. }
        | Block::Part { span, .. }
        | Block::Title { span, .. }
        | Block::ClearPage { span, .. }
        | Block::Chrome { span, .. }
        | Block::Rule { span, .. } => Some((span.document.0, span.start)),
        Block::TocEntry(e) => Some((e.list_span.document.0, e.list_span.start)),
        Block::Picture { document, picture, .. } => Some((document.0, picture.start)),
    }
}

/// Source start of the first block of a region body.
fn body_first_start(body: &[Block]) -> Option<usize> {
    body.first().and_then(block_start).map(|(_, s)| s)
}

/// Splits a paragraph whose lines straddle `at` (a preface that ends in
/// the middle of a paragraph: `[...]` is blanked, not a `\par`).
fn split_paragraph(b: &Block, document: usize, at: usize) -> Option<(Block, Block)> {
    let Block::Paragraph { parts, indent, style, env_open, env_close, eject_before, vspace_before, addvspace_before, endlist_adjust, list } = b else { return None };
    let mut before: Vec<ParaPart> = Vec::new();
    let mut after: Vec<ParaPart> = Vec::new();
    for p in parts {
        match p {
            ParaPart::Lines(items) => {
                let mut split = None;
                for (k, it) in items.iter().enumerate() {
                    if let Some(s) = item_start(it) {
                        if s >= at {
                            split = Some(k);
                            break;
                        }
                    }
                }
                match split {
                    Some(0) => after.push(p.clone()),
                    Some(k) if after.is_empty() => {
                        before.push(ParaPart::Lines(items[..k].to_vec()));
                        let rest: Vec<AItem> = items[k..].iter().skip_while(|i| matches!(i, AItem::Space { .. })).cloned().collect();
                        after.push(ParaPart::Lines(rest));
                    }
                    _ if after.is_empty() => before.push(p.clone()),
                    _ => after.push(p.clone()),
                }
            }
            _ => match part_start(p) {
                Some((d, s)) if d == document && s >= at => after.push(p.clone()),
                _ if after.is_empty() => before.push(p.clone()),
                _ => after.push(p.clone()),
            },
        }
    }
    if before.is_empty() || after.is_empty() {
        return None;
    }
    let first = Block::Paragraph {
        parts: before,
        indent: *indent,
        style: *style,
        env_open: *env_open,
        env_close: false,
        eject_before: *eject_before,
        vspace_before: *vspace_before,
        addvspace_before: *addvspace_before,
        endlist_adjust: *endlist_adjust,
        list: list.clone(),
    };
    let second = Block::Paragraph {
        parts: after,
        indent: true,
        style: *style,
        env_open: None,
        env_close: *env_close,
        eject_before: false,
        vspace_before: 0.0,
        addvspace_before: 0.0,
        endlist_adjust: 0.0,
        list: None,
    };
    Some((first, second))
}

#[derive(Clone, Copy, PartialEq)]
enum Class {
    Outer,
    Preface(usize, usize),
    Body(usize, usize),
}

/// When the document has `multicols` environments this build can set:
/// the document with every environment's body replaced by two markers
/// (`\begin`: `\enough@room`; `\end`: the columns), its preface kept
/// between them. The bodies wait in the context for [`paginate`].
pub(super) fn outer_doc(ctx: &mut Context, doc: &Doc, floats: &[floatpage::FloatSpec]) -> Option<Doc> {
    if ctx.multicol.active || ctx.multicol.scans.iter().all(Scan::is_empty) {
        return None;
    }
    let scans = ctx.multicol.scans.clone();
    for (d, s) in scans.iter().enumerate() {
        for &at in &s.stray_breaks {
            let e = Diagnostic::error("multicol", "\\columnbreak outside multicols: this command can only be used within a multicols or multicols* environment", source(ctx, d, at, at + 12));
            ctx.report_once(format!("multicol-stray-{d}-{at}"), e);
        }
        for r in &s.regions {
            if r.columns_arg < 2 {
                warn(ctx, format!("multicol-few-{d}-{}", r.begin.0), format!("Using `{}' columns doesn't seem a good idea. I therefore use two columns instead", r.columns_arg), d, r.begin);
            }
            if r.columns_arg > 20 {
                let e = Diagnostic::error("multicol", "Too many columns: the current implementation doesn't support more than 20 columns; 20 columns are used", source(ctx, d, r.begin.0, r.begin.1));
                ctx.report_once(format!("multicol-many-{d}-{}", r.begin.0), e);
            }
            for &at in &r.nested {
                warn(ctx, format!("multicol-nested-{d}-{at}"), "nested multicols (boxed mode) are not implemented: the inner environment's text is set in the outer columns".into(), d, (at, at + 6));
            }
            for &at in &r.floats {
                warn(ctx, format!("multicol-float-{d}-{at}"), "Floats and marginpars not allowed inside `multicols' environment!".into(), d, (at, at + 6));
            }
        }
    }
    let regions: Vec<(usize, usize, Region)> = scans.iter().enumerate().flat_map(|(d, s)| s.regions.iter().enumerate().map(move |(k, r)| (d, k, r.clone()))).collect();
    if regions.is_empty() {
        return None;
    }
    let geo = ctx.style.class_geometry.as_deref();
    if geo.is_some_and(|g| g.frame.columns.len() > 1) || scans.iter().any(|s| s.twocolumn_option) {
        let (d, _, r) = &regions[0];
        warn(ctx, "multicol-twocolumn".into(), "May not work with the twocolumn option".into(), *d, r.begin);
        warn(ctx, "multicol-twocolumn-layout".into(), "multicols in a two-column document is not implemented: the environment's text is set in the page columns".into(), *d, r.begin);
        return None;
    }
    // Footnotes go through `footnotes`' `\insert` page builder, which this
    // module's output routines do not model (`\init@mult@footins`,
    // `\leave@mult@footins`, `\mult@footnotetext`).
    let has_notes = ctx.texts.iter().any(|t| ["\\footnote", "\\thanks", "\\footnotetext"].iter().any(|n| t.match_indices(n).any(|(at, _)| !is_commented(t, at))));
    if has_notes {
        let (d, _, r) = &regions[0];
        warn(ctx, "multicol-footnotes".into(), "multicols in a document with footnotes is not implemented: the environment's text is set at full width".into(), *d, r.begin);
        return None;
    }
    if !floats.is_empty() {
        let (d, _, r) = &regions[0];
        warn(ctx, "multicol-floats".into(), "multicols in a document with floats is not implemented: the environment's text is set at full width".into(), *d, r.begin);
        return None;
    }
    // Classify the blocks, splitting a paragraph that straddles a preface's
    // end.
    let classify = |start: Option<(usize, usize)>| -> Option<Class> {
        let (d, s) = start?;
        for (rd, k, r) in &regions {
            if *rd != d {
                continue;
            }
            if let Some((a, b)) = r.preface {
                if s >= a && s < b {
                    return Some(Class::Preface(*rd, *k));
                }
            }
            if s >= r.body.0 && s < r.body.1 {
                return Some(Class::Body(*rd, *k));
            }
        }
        Some(Class::Outer)
    };
    // `(index in doc.blocks, block)`: `page_starts` follow the blocks.
    let mut blocks: Vec<(usize, Block)> = Vec::with_capacity(doc.blocks.len());
    for (orig, b) in doc.blocks.iter().enumerate() {
        let mut pending = vec![b.clone()];
        for (d, _, r) in &regions {
            if let Some((_, e)) = r.preface {
                let mut next = Vec::new();
                for p in pending {
                    match split_paragraph(&p, *d, e) {
                        Some((x, y)) if block_start(&x).is_some_and(|(bd, s)| bd == *d && s < e) => {
                            next.push(x);
                            next.push(y);
                        }
                        _ => next.push(p),
                    }
                }
                pending = next;
            }
        }
        blocks.extend(pending.into_iter().map(|p| (orig, p)));
    }
    let mut new_index: BTreeMap<usize, usize> = BTreeMap::new();
    let mut out: Vec<Block> = Vec::with_capacity(blocks.len() + 2 * regions.len());
    let mut bodies: BTreeMap<(usize, usize), Vec<Block>> = BTreeMap::new();
    let mut last = Class::Outer;
    let mut next_region = 0usize;
    let marker = |d: usize, at: (usize, usize)| Block::Rule { span: Span::in_document(DocumentId(d), at.0, at.1), eject_before: false, vspace_before: 0.0 };
    let mut open: Option<usize> = None;
    let close = |out: &mut Vec<Block>, open: &mut Option<usize>| {
        if let Some(i) = open.take() {
            let (d, _, r) = &regions[i];
            out.push(marker(*d, r.end));
        }
    };
    for (orig, b) in blocks {
        let start = block_start(&b);
        let class = classify(start).unwrap_or(last);
        // Start markers for every region at or before this block.
        if let Some((d, s)) = start {
            while next_region < regions.len() && regions[next_region].0 == d && regions[next_region].2.begin.0 <= s {
                close(&mut out, &mut open);
                let (rd, _, r) = &regions[next_region];
                out.push(marker(*rd, r.begin));
                open = Some(next_region);
                next_region += 1;
            }
        }
        match class {
            Class::Outer => {
                if let (Some(i), Some((d, s))) = (open, start) {
                    let r = &regions[i].2;
                    if d != regions[i].0 || s >= r.end.0 {
                        close(&mut out, &mut open);
                    }
                }
                new_index.entry(orig).or_insert(out.len());
                out.push(b);
            }
            Class::Preface(..) => {
                new_index.entry(orig).or_insert(out.len());
                out.push(b);
            }
            Class::Body(d, k) => {
                if matches!(b, Block::Chrome { .. }) {
                    new_index.entry(orig).or_insert(out.len());
                    out.push(b);
                } else {
                    bodies.entry((d, k)).or_default().push(b);
                }
            }
        }
        last = class;
    }
    close(&mut out, &mut open);
    while next_region < regions.len() {
        let (rd, _, r) = &regions[next_region];
        out.push(marker(*rd, r.begin));
        out.push(marker(*rd, r.end));
        next_region += 1;
    }
    ctx.multicol.active = true;
    ctx.multicol.bodies = bodies;
    let page_starts = doc.page_starts.iter().filter_map(|i| new_index.get(i).copied()).collect();
    Some(Doc {
        style: doc.style.clone(),
        blocks: out,
        diagnostics: Vec::new(),
        limitations: Vec::new(),
        secnumdepth: doc.secnumdepth,
        page_starts,
        default_color: doc.default_color,
        math_colors: doc.math_colors.clone(),
        page_color: doc.page_color,
    })
}

// ---------------------------------------------------------------------------
// TeX's vertical lists.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Glue {
    w: f64,
    /// Stretch of order normal, fil, fill.
    st: [f64; 3],
    sh: f64,
}

impl Glue {
    fn fixed(w: f64) -> Glue {
        Glue { w, ..Glue::default() }
    }
    fn of(t: (f64, f64, f64)) -> Glue {
        Glue { w: t.0, st: [t.1, 0.0, 0.0], sh: t.2 }
    }
    fn neg(self) -> Glue {
        Glue { w: -self.w, st: [-self.st[0], -self.st[1], -self.st[2]], sh: -self.sh }
    }
    fn fil(n: f64) -> Glue {
        Glue { st: [0.0, n, 0.0], ..Glue::default() }
    }
    fn is_zero(&self) -> bool {
        self.w == 0.0 && self.st == [0.0; 3] && self.sh == 0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
enum N {
    Line { h: f64, d: f64, bi: usize, li: usize },
    Cols { h: f64, d: f64, id: usize },
    /// `\null` set with `\topskip\z@` (`\prepare@multicols`, 236).
    Null,
    Glue(Glue),
    Kern(f64),
    Penalty(i32),
}

impl N {
    fn is_box(&self) -> bool {
        matches!(self, N::Line { .. } | N::Cols { .. } | N::Null)
    }
    fn hd(&self) -> (f64, f64) {
        match self {
            N::Line { h, d, .. } | N::Cols { h, d, .. } => (*h, *d),
            _ => (0.0, 0.0),
        }
    }
}

struct Pack {
    h: f64,
    d: f64,
    st: [f64; 3],
    sh: f64,
}

/// `vpack(p, natural)` with `\boxmaxdepth` = `max_depth` (§668).
fn natural(items: &[N], max_depth: f64) -> Pack {
    let (mut x, mut d) = (0.0, 0.0);
    let (mut st, mut sh) = ([0.0; 3], 0.0);
    for it in items {
        match it {
            N::Line { .. } | N::Cols { .. } | N::Null => {
                let (h, dd) = it.hd();
                x += d + h;
                d = dd;
            }
            N::Glue(g) => {
                x += d + g.w;
                d = 0.0;
                for o in 0..3 {
                    st[o] += g.st[o];
                }
                sh += g.sh;
            }
            N::Kern(w) => {
                x += d + w;
                d = 0.0;
            }
            N::Penalty(_) => {}
        }
    }
    if d > max_depth {
        x += d - max_depth;
        d = max_depth;
    }
    Pack { h: x, d, st, sh }
}

#[derive(Clone, Copy, Debug)]
enum Set {
    Natural,
    Stretch(usize, f64),
    Shrink(f64),
}

fn eq(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.5 / 65536.0
}

/// `vpackage(p, h, exactly, max_depth)` (§§668–678): the glue setting,
/// `\badness` and the box depth.
fn pack_to(items: &[N], h: f64, max_depth: f64) -> (Set, i64, f64) {
    let p = natural(items, max_depth);
    let x = h - p.h;
    if eq(x, 0.0) || items.is_empty() {
        return (Set::Natural, 0, p.d);
    }
    if x > 0.0 {
        let o = (0..3).rev().find(|&o| p.st[o] != 0.0).unwrap_or(0);
        let set = if p.st[o] != 0.0 { Set::Stretch(o, x / p.st[o]) } else { Set::Natural };
        let b = if o == 0 { badness(x, p.st[0]) } else { 0 };
        (set, b, p.d)
    } else if p.sh < -x - 0.5 / 65536.0 {
        (Set::Shrink(1.0), 1_000_000, p.d)
    } else {
        let set = if p.sh != 0.0 { Set::Shrink(-x / p.sh) } else { Set::Natural };
        (set, badness(-x, p.sh), p.d)
    }
}

/// Baselines (from the box top) of the boxes of `items` set with `set`
/// (§§629–637 `vlist_out`): `(item index, baseline)`.
fn place(items: &[N], set: Set) -> Vec<(usize, f64)> {
    let mut v = 0.0;
    let mut out = Vec::new();
    for (i, it) in items.iter().enumerate() {
        match it {
            N::Line { .. } | N::Cols { .. } | N::Null => {
                let (h, d) = it.hd();
                v += h;
                out.push((i, v));
                v += d;
            }
            N::Glue(g) => {
                v += g.w
                    + match set {
                        Set::Stretch(o, r) => r * g.st[o],
                        Set::Shrink(r) => -r * g.sh,
                        Set::Natural => 0.0,
                    };
            }
            N::Kern(w) => v += w,
            N::Penalty(_) => {}
        }
    }
    out
}

/// `vert_break(p, h, d)` (§§970–976): the index of the best break (the
/// list length for the end of the list).
fn vert_break(items: &[N], h: f64, d: f64) -> usize {
    let (mut cur, mut prev_dp) = (0.0, 0.0);
    let (mut st, mut sh) = ([0.0f64; 3], 0.0);
    let mut least = AWFUL_BAD;
    let mut best = items.len();
    for i in 0..=items.len() {
        let node = items.get(i);
        let pi = match node {
            None => Some(EJECT_PENALTY),
            Some(n) if n.is_box() => {
                let (bh, bd) = n.hd();
                cur += prev_dp + bh;
                prev_dp = bd;
                None
            }
            Some(N::Glue(_)) => (i > 0 && items[i - 1].is_box()).then_some(0),
            Some(N::Kern(_)) => matches!(items.get(i + 1), Some(N::Glue(_))).then_some(0),
            Some(N::Penalty(v)) => Some(*v),
            Some(_) => None,
        };
        if let Some(pi) = pi {
            if pi < INF_PENALTY {
                let mut b = if cur < h - 0.5 / 65536.0 {
                    if st[1] != 0.0 || st[2] != 0.0 {
                        0
                    } else {
                        badness(h - cur, st[0])
                    }
                } else if cur - h > sh + 0.5 / 65536.0 {
                    AWFUL_BAD
                } else {
                    badness(cur - h, sh)
                };
                if b < AWFUL_BAD {
                    b = if pi <= EJECT_PENALTY {
                        i64::from(pi)
                    } else if b < INF_BAD {
                        b + i64::from(pi)
                    } else {
                        DEPLORABLE
                    };
                }
                if b <= least {
                    best = i;
                    least = b;
                }
                if b == AWFUL_BAD || pi <= EJECT_PENALTY {
                    return best;
                }
            }
        }
        match node {
            Some(N::Glue(g)) => {
                for o in 0..3 {
                    st[o] += g.st[o];
                }
                sh += g.sh;
                cur += prev_dp + g.w;
                prev_dp = 0.0;
            }
            Some(N::Kern(w)) => {
                cur += prev_dp + w;
                prev_dp = 0.0;
            }
            _ => {}
        }
        if prev_dp > d {
            cur += prev_dp - d;
            prev_dp = d;
        }
    }
    best
}

/// `prune_page_top` (§968): discardable items before the first box go,
/// `\splittopskip` glue goes in front of it.
fn prune_page_top(items: &[N], split_top: Glue) -> Vec<N> {
    let Some(first) = items.iter().position(N::is_box) else { return Vec::new() };
    let (h, _) = items[first].hd();
    let mut out = Vec::with_capacity(items.len() - first + 1);
    out.push(N::Glue(Glue { w: if split_top.w > h { split_top.w - h } else { 0.0 }, ..split_top }));
    out.extend(items[first..].iter().cloned());
    out
}

/// `\vsplit` (§977): the material before the best break for height `h`
/// and what remains after `prune_page_top`.
fn vsplit(items: &[N], h: f64, split_top: Glue, split_max_depth: f64) -> (Vec<N>, Vec<N>) {
    let q = vert_break(items, h, split_max_depth);
    (items[..q].to_vec(), prune_page_top(&items[q..], split_top))
}

/// `\remove@discardable@items` (850–868).
fn remove_discardable(v: &mut Vec<N>) {
    loop {
        if matches!(v.last(), Some(N::Penalty(_))) {
            v.pop();
        }
        let a = match v.last() {
            Some(N::Glue(g)) => {
                let g = *g;
                v.pop();
                g
            }
            _ => Glue::default(),
        };
        if a.is_zero() {
            let b = match v.last() {
                Some(N::Glue(g)) => *g,
                _ => break,
            };
            if b.is_zero() {
                break;
            }
            v.pop();
            if matches!(v.last(), Some(N::Penalty(p)) if *p == INF_PENALTY) {
                v.push(N::Glue(b));
                v.push(N::Glue(a));
                break;
            }
        }
    }
}

fn sp(pt: f64) -> i64 {
    (pt * 65536.0).round() as i64
}

fn pt(sp: i64) -> f64 {
    sp as f64 / 65536.0
}

// ---------------------------------------------------------------------------
// The page builder with LaTeX's and multicol's output routines.

/// One environment's layout parameters.
#[derive(Debug, Clone)]
struct Run {
    n: usize,
    star: bool,
    ragged: bool,
    hsize: f64,
    gap: f64,
    rule: f64,
    multicolsep: Glue,
    premulticols: f64,
    postmulticols: f64,
    unbalance: i64,
    columnbadness: i64,
    finalcolumnbadness: i64,
    minrows: i64,
    collectmore: i64,
}

#[derive(Debug, Clone, Default)]
struct ColsBox {
    /// `(block, line, baseline from the box top, x offset, height, depth)`.
    lines: Vec<(usize, usize, f64, f64, f64, f64)>,
    /// `(x, height + depth)` of each `\columnseprule`.
    rules: Vec<(f64, f64)>,
}

#[derive(Debug, Clone)]
enum Out {
    Line { bi: usize, li: usize, baseline: f64, h: f64, d: f64, dx: f64 },
    Rule { x: f64, bottom: f64, height: f64 },
}

#[derive(Debug, Default)]
struct PageOut {
    lines: Vec<Out>,
    overfull_by: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Normal,
    SavePartial,
    Multicol,
}

struct Machine {
    p: PageParams,
    dp_p: f64,
    contrib: VecDeque<N>,
    page: Vec<N>,
    has_box: bool,
    total: f64,
    depth: f64,
    st: [f64; 3],
    sh: f64,
    goal: f64,
    least: i64,
    best: usize,
    vsize: f64,
    colroom: f64,
    prev_depth: Option<f64>,
    any: bool,
    /// The last material appended was `\endmulticols`.
    after_region: bool,
    mode: Mode,
    partial: Option<Vec<N>>,
    colbreak: Vec<N>,
    run: Option<Run>,
    cols: Vec<ColsBox>,
    pages: Vec<PageOut>,
}

impl Machine {
    fn new(p: PageParams, dp_p: f64) -> Machine {
        Machine {
            p,
            dp_p,
            contrib: VecDeque::new(),
            page: Vec::new(),
            has_box: false,
            total: 0.0,
            depth: 0.0,
            st: [0.0; 3],
            sh: 0.0,
            goal: p.vsize,
            least: AWFUL_BAD,
            best: 0,
            vsize: p.vsize,
            colroom: p.vsize,
            prev_depth: None,
            any: false,
            after_region: false,
            mode: Mode::Normal,
            partial: None,
            colbreak: Vec::new(),
            run: None,
            cols: Vec::new(),
            pages: Vec::new(),
        }
    }

    fn push(&mut self, n: N) {
        self.contrib.push_back(n);
    }

    fn last(&self) -> Option<&N> {
        self.contrib.back().or(self.page.last())
    }

    fn lastskip(&self) -> Option<Glue> {
        match self.last() {
            Some(N::Glue(g)) => Some(*g),
            _ => None,
        }
    }

    /// `\addpenalty` (latex.ltx): before the last skip, which is re-added.
    fn add_penalty(&mut self, pen: i32) {
        match self.lastskip().filter(|g| g.w != 0.0) {
            Some(g) => {
                self.push(N::Glue(g.neg()));
                self.push(N::Penalty(pen));
                self.build_page();
                self.push(N::Glue(g));
            }
            None => {
                self.push(N::Penalty(pen));
                self.build_page();
            }
        }
    }

    /// `\addvspace` (latex.ltx): only the excess over the last skip.
    fn add_vspace(&mut self, s: Glue) {
        match self.lastskip().filter(|g| g.w != 0.0) {
            None => self.push(N::Glue(s)),
            Some(last) => {
                if last.w < s.w && last.w >= 0.0 {
                    self.push(N::Glue(last.neg()));
                    self.push(N::Glue(s));
                }
            }
        }
    }

    fn newpage(&mut self) {
        self.push(N::Glue(Glue::fil(1.0)));
        self.push(N::Penalty(EJECT_PENALTY));
        self.build_page();
    }

    /// Appends a block's lines as `pagebuild::vlist` does, with a column
    /// break penalty after the lines in `after`.
    fn push_block(&mut self, bi: usize, v: &VBlock, after: &BTreeMap<(usize, usize), i32>) {
        if v.lines.is_empty() {
            return;
        }
        // Right after `\endmulticols` the list ends with `\multicolsep`: a
        // heading's `\addpenalty`/`\addvspace` compare against it (between
        // ordinary blocks the builder already folded those skips).
        let after_region = std::mem::take(&mut self.after_region);
        if let Some(pen) = v.penalty_before {
            if self.any {
                if pen <= EJECT_PENALTY {
                    self.push(N::Glue(Glue::fil(1.0)));
                    self.push(N::Penalty(pen));
                    self.build_page();
                } else if after_region {
                    self.add_penalty(pen);
                } else {
                    self.push(N::Penalty(pen));
                    self.build_page();
                }
            }
        }
        if let Some(s) = v.space_before {
            if after_region {
                self.add_vspace(Glue::of(s));
            } else {
                self.push(N::Glue(Glue::of(s)));
            }
        }
        if let Some(s) = v.parskip {
            self.push(N::Glue(Glue::of(s)));
        }
        let bs = v.baselineskip.unwrap_or(self.p.baselineskip);
        let n = v.lines.len();
        for (li, &(h, d)) in v.lines.iter().enumerate() {
            let prev = if li == 0 && v.no_interline_first { None } else { self.prev_depth };
            if let Some(pd) = prev {
                let mut g = bs - pd - h;
                if g < self.p.lineskiplimit {
                    g = self.p.lineskip;
                }
                self.push(N::Glue(Glue::fixed(g)));
            }
            self.push(N::Line { h, d, bi, li });
            self.prev_depth = if li + 1 == n && v.no_interline_after { None } else { Some(d) };
            if let Some(&s) = v.vskip_after.get(li) {
                if s != 0.0 {
                    self.push(N::Glue(Glue::fixed(s)));
                }
            }
            if let Some(&pen) = after.get(&(bi, li)) {
                self.push(N::Penalty(pen));
            }
            if li + 1 < n {
                let mut pen = v.interline_penalty;
                if li == 0 {
                    pen += v.club_penalty;
                }
                if li + 2 == n {
                    pen += v.widow_penalty;
                }
                if pen != 0 {
                    self.push(N::Penalty(pen.min(INF_PENALTY)));
                }
            }
        }
        if let Some(pen) = v.penalty_after {
            self.push(N::Penalty(pen));
        }
        if let Some(s) = v.pre_space_after {
            self.push(N::Glue(Glue::of(s)));
        }
        if let Some(s) = v.space_after {
            self.push(N::Glue(Glue::of(s)));
        }
        self.any = true;
        self.build_page();
    }

    /// TeX's `build_page` (§§994–1008).
    fn build_page(&mut self) {
        let mut guard = 0usize;
        while let Some(front) = self.contrib.front().cloned() {
            guard += 1;
            if guard > 10_000_000 {
                return;
            }
            let mut pi: Option<i32> = None;
            match &front {
                n if n.is_box() => {
                    if !self.has_box {
                        // §987/§1001: freeze the page specs, `\topskip` glue.
                        self.has_box = true;
                        self.goal = self.vsize;
                        self.total = 0.0;
                        self.depth = 0.0;
                        self.st = [0.0; 3];
                        self.sh = 0.0;
                        self.least = AWFUL_BAD;
                        self.best = 0;
                        let (h, _) = n.hd();
                        let ts = if matches!(n, N::Null) { 0.0 } else { self.p.topskip };
                        self.contrib.push_front(N::Glue(Glue::fixed(if ts > h { ts - h } else { 0.0 })));
                        continue;
                    }
                    let (h, d) = n.hd();
                    self.total += self.depth + h;
                    self.depth = d;
                }
                N::Glue(_) => {
                    if !self.has_box {
                        self.contrib.pop_front();
                        continue;
                    }
                    if self.page.last().is_some_and(N::is_box) {
                        pi = Some(0);
                    }
                }
                N::Kern(_) => {
                    if !self.has_box {
                        self.contrib.pop_front();
                        continue;
                    }
                    if self.contrib.len() == 1 {
                        return;
                    }
                    if matches!(self.contrib.get(1), Some(N::Glue(_))) {
                        pi = Some(0);
                    }
                }
                N::Penalty(v) => {
                    if !self.has_box {
                        self.contrib.pop_front();
                        continue;
                    }
                    pi = Some(*v);
                }
                _ => {}
            }
            if let Some(pi) = pi.filter(|&v| v < INF_PENALTY) {
                // §1005.
                let b = if self.total < self.goal - 0.5 / 65536.0 {
                    if self.st[1] != 0.0 || self.st[2] != 0.0 {
                        0
                    } else {
                        badness(self.goal - self.total, self.st[0])
                    }
                } else if self.total - self.goal > self.sh + 0.5 / 65536.0 {
                    AWFUL_BAD
                } else {
                    badness(self.total - self.goal, self.sh)
                };
                let c = if b < AWFUL_BAD {
                    if pi <= EJECT_PENALTY {
                        i64::from(pi)
                    } else if b < INF_BAD {
                        b + i64::from(pi)
                    } else {
                        DEPLORABLE
                    }
                } else {
                    b
                };
                if c <= self.least {
                    self.best = self.page.len();
                    self.least = c;
                }
                if c == AWFUL_BAD || pi <= EJECT_PENALTY {
                    self.fire();
                    continue;
                }
            }
            match &front {
                N::Glue(g) => {
                    for o in 0..3 {
                        self.st[o] += g.st[o];
                    }
                    self.sh += g.sh;
                    self.total += self.depth + g.w;
                    self.depth = 0.0;
                }
                N::Kern(w) => {
                    self.total += self.depth + w;
                    self.depth = 0.0;
                }
                _ => {}
            }
            if self.depth > self.p.maxdepth {
                self.total += self.depth - self.p.maxdepth;
                self.depth = self.p.maxdepth;
            }
            let n = self.contrib.pop_front().expect("front");
            self.page.push(n);
        }
    }

    /// `fire_up` (§§1012–1025) and the output routine in force.
    fn fire(&mut self) {
        let best = self.best.min(self.page.len());
        let rest: Vec<N> = self.page.drain(best..).collect();
        let box255 = std::mem::take(&mut self.page);
        // The break node heads the returned material, or is the node that
        // triggered the fire (still at the head of the contributions).
        for n in rest.into_iter().rev() {
            self.contrib.push_front(n);
        }
        let op = match self.contrib.front_mut() {
            Some(N::Penalty(v)) => {
                let o = *v;
                *v = INF_PENALTY;
                o
            }
            _ => INF_PENALTY,
        };
        let fired_total = self.total;
        let goal = self.goal;
        self.has_box = false;
        self.depth = 0.0;
        let out = match self.mode {
            Mode::Normal => {
                self.emit_page(box255, goal);
                self.colroom = self.p.vsize;
                Vec::new()
            }
            Mode::SavePartial => {
                let mut items = box255;
                if items.last().is_some_and(N::is_box) {
                    items.pop();
                }
                self.partial = Some(items);
                Vec::new()
            }
            Mode::Multicol => self.multicol_output(box255, op, fired_total),
        };
        for n in out.into_iter().rev() {
            self.contrib.push_front(n);
        }
    }

    /// `\@makecol\@outputpage`: the page box `\vbox to\@colht` holding
    /// `items`, `\vskip-\dp` and `\@textbottom`.
    fn emit_page(&mut self, items: Vec<N>, _goal: f64) {
        let colht = self.p.vsize;
        let dp = natural(&items, self.p.maxdepth).d;
        let mut list = items;
        list.push(N::Glue(Glue::fixed(-dp)));
        if !self.p.flushbottom {
            list.push(N::Glue(Glue { st: [0.0, 0.0001, 0.0], ..Glue::default() }));
        }
        let (set, _, _) = pack_to(&list, colht, self.p.maxdepth);
        let mut page = PageOut::default();
        let mut bottom: f64 = 0.0;
        for (i, baseline) in place(&list, set) {
            match &list[i] {
                N::Line { h, d, bi, li } => {
                    page.lines.push(Out::Line { bi: *bi, li: *li, baseline, h: *h, d: *d, dx: 0.0 });
                    bottom = bottom.max(baseline + (d - self.p.maxdepth).max(0.0));
                }
                N::Cols { h, id, d } => {
                    let top = baseline - h;
                    let c = &self.cols[*id];
                    for &(bi, li, b, dx, lh, ld) in &c.lines {
                        page.lines.push(Out::Line { bi, li, baseline: top + b, h: lh, d: ld, dx });
                    }
                    for &(x, height) in &c.rules {
                        page.lines.push(Out::Rule { x, bottom: top + height, height });
                    }
                    bottom = bottom.max(baseline + (d - self.p.maxdepth).max(0.0));
                }
                _ => {}
            }
        }
        if bottom > colht + 1e-6 {
            page.overfull_by = bottom - colht;
        }
        self.pages.push(page);
    }

    fn run(&self) -> &Run {
        self.run.as_ref().expect("inside multicols")
    }

    /// `\set@mult@vsize` (297–306).
    fn mult_vsize(&self) -> f64 {
        let r = self.run();
        let n = r.n as f64;
        let t = self.p.baselineskip - self.p.topskip;
        n * (self.colroom + t) - t + n * self.p.baselineskip + r.collectmore as f64 * self.p.baselineskip
    }

    /// The column box of `\page@sofar`: each column list set `\vbox to h`.
    fn make_cols(&mut self, cols: Vec<Vec<N>>, h: f64) -> (usize, f64) {
        let r = self.run().clone();
        let mut b = ColsBox::default();
        let mut depth = self.dp_p;
        for (k, col) in cols.iter().enumerate() {
            let (set, _, d) = pack_to(col, h, self.p.maxdepth);
            depth = depth.max(d);
            let dx = k as f64 * (r.hsize + r.gap);
            for (i, baseline) in place(col, set) {
                if let N::Line { h: lh, d: ld, bi, li } = &col[i] {
                    b.lines.push((*bi, *li, baseline, dx, *lh, *ld));
                }
            }
        }
        if r.rule > 0.0 {
            for k in 1..r.n {
                let x = k as f64 * (r.hsize + r.gap) - r.gap + (r.gap - r.rule) / 2.0;
                b.rules.push((x, h + depth));
            }
        }
        self.cols.push(b);
        (self.cols.len() - 1, depth)
    }

    fn multicol_output(&mut self, items: Vec<N>, op: i32, fired_total: f64) -> Vec<N> {
        if op < -10000 && op < -10001 {
            // `\speci@ls` (504–552).
            if op == COLUMNBREAK {
                self.vsize -= fired_total;
                let mut it = items;
                remove_discardable(&mut it);
                let d = natural(&it, self.p.maxdepth).d;
                if !self.colbreak.is_empty() {
                    self.colbreak.push(N::Penalty(COLUMNBREAK));
                }
                self.colbreak.extend(it);
                self.colbreak.push(N::Kern(-d));
                return Vec::new();
            }
            if op == END_PENALTY {
                if self.run().star {
                    let mut out = self.column_out(items, INF_PENALTY);
                    out.push(N::Penalty(END_PENALTY));
                    return out;
                }
                return self.balance_out(items);
            }
            return items;
        }
        self.column_out(items, op)
    }

    /// `\multi@column@out` (422–499).
    fn column_out(&mut self, items: Vec<N>, op: i32) -> Vec<N> {
        let mut items = items;
        if !self.colbreak.is_empty() {
            let mut v = std::mem::take(&mut self.colbreak);
            v.push(N::Penalty(COLUMNBREAK));
            v.extend(items);
            items = v;
        }
        let r = self.run().clone();
        let h = self.colroom;
        let split_top = Glue::fixed(self.p.topskip);
        let mut cols = Vec::with_capacity(r.n);
        for _ in 0..r.n {
            let (mut first, rest) = vsplit(&items, h, split_top, self.p.maxdepth);
            if r.ragged {
                first.push(N::Glue(Glue { st: [0.0, 0.0001, 0.0], sh: self.p.maxdepth, w: 0.0 }));
            }
            cols.push(first);
            items = rest;
        }
        let mut out = Vec::new();
        if !items.is_empty() {
            out = items;
            if op != INF_PENALTY {
                out.push(N::Penalty(op));
            }
        }
        let (id, depth) = self.make_cols(cols, h);
        let mut page = self.partial.take().unwrap_or_default();
        page.push(N::Cols { h, d: depth, id });
        page.push(N::Kern(-depth));
        let goal = self.goal;
        self.emit_page(page, goal);
        self.colroom = self.p.vsize;
        self.vsize = self.mult_vsize();
        out
    }

    /// `\balance@columns@out` (568–605).
    fn balance_out(&mut self, items: Vec<N>) -> Vec<N> {
        let mut mb = std::mem::take(&mut self.colbreak);
        if !mb.is_empty() {
            mb.push(N::Penalty(COLUMNBREAK));
        }
        mb.extend(items);
        remove_discardable(&mut mb);
        match self.balance(mb) {
            Err(pruned) => {
                let undershoot = 2.0;
                let mut b = vec![N::Glue(Glue::fixed(self.p.topskip)), N::Glue(Glue { w: self.p.topskip, st: [undershoot, 0.0, 0.0], sh: 0.0 }.neg())];
                b.extend(pruned);
                b.push(N::Penalty(END_PENALTY));
                self.column_out(b, INF_PENALTY)
            }
            Ok((h, cols)) => {
                let partial = self.partial.take().unwrap_or_default();
                self.vsize = self.colroom + natural(&partial, MAXDIMEN).h;
                let (id, depth) = self.make_cols(cols, h);
                let mut out = partial;
                out.push(N::Cols { h, d: depth, id });
                out.push(N::Kern(-depth));
                out.push(N::Penalty(0));
                out
            }
        }
    }

    /// `\balance@columns` (606–797): the balanced height and the column
    /// lists, or the pruned material when balancing fails.
    fn balance(&mut self, mb: Vec<N>) -> Result<(f64, Vec<Vec<N>>), Vec<N>> {
        let r = self.run().clone();
        let n = r.n;
        let (undershoot, overshoot) = (2.0, 0.0);
        let split_top = Glue { w: self.p.topskip, st: [undershoot, 0.0, 0.0], sh: overshoot };
        let md = self.p.maxdepth;
        let mut list = vec![N::Penalty(-10000)];
        list.extend(mb);
        let (_, list) = vsplit(&list, 0.0, split_top, md);
        let nat = natural(&list, MAXDIMEN);
        let bs = sp(self.p.baselineskip);
        let ts = sp(self.p.topskip);
        let tempdima = flashtex_class_geometry::tex::Sp(sp(nat.h + nat.d)).over(n as i64).0;
        let count = tempdima / bs;
        let mut dimen = count * bs + ts;
        if dimen > tempdima {
            dimen -= bs;
        }
        let minrows = ts + r.minrows * bs - bs;
        if dimen < minrows {
            dimen = minrows;
        }
        dimen += r.unbalance * bs;
        if dimen < ts {
            dimen = ts;
        }
        let one = 65536i64;
        let mut last_try = -one;
        let colroom = sp(self.colroom);
        let overflow = sp(12.0);
        let mut cols: Vec<Vec<N>>;
        let mut iterations = 0;
        let mut forced_leftover;
        loop {
            iterations += 1;
            let h = pt(dimen);
            let mut g = list.clone();
            let mut too_bad = false;
            forced_leftover = false;
            cols = Vec::with_capacity(n);
            for _ in 0..n - 1 {
                let (first, rest) = vsplit(&g, h, split_top, md);
                let (_, b, _) = pack_to(&first, h, md);
                if b > r.columnbadness {
                    too_bad = true;
                }
                cols.push(first);
                g = rest;
            }
            let nat_first = sp(natural(&cols[0], md).h);
            if sp(natural(&g, md).h) > dimen {
                too_bad = true;
            } else {
                let (_, leftover) = vsplit(&g, MAXDIMEN, split_top, md);
                if leftover.is_empty() {
                    let (_, b, _) = pack_to(&g, h, md);
                    if b > r.finalcolumnbadness {
                        g.push(N::Glue(Glue::fil(1.0)));
                    }
                } else if dimen < colroom + overflow {
                    too_bad = true;
                } else {
                    forced_leftover = true;
                }
            }
            cols.push(g);
            if nat_first < dimen && nat_first > last_try {
                too_bad = true;
                dimen = nat_first;
                last_try = dimen;
                dimen -= one;
            }
            if too_bad && iterations < 5000 {
                dimen += one;
                continue;
            }
            break;
        }
        if forced_leftover {
            return Err(list);
        }
        if dimen > colroom {
            dimen = colroom;
        }
        let h = pt(dimen);
        let mut too_bad = false;
        let mut out = Vec::with_capacity(n);
        for col in cols {
            let mut v = vec![N::Glue(Glue { w: 0.0, st: [-undershoot, 0.0, 0.0], sh: -overshoot })];
            v.extend(col);
            if r.ragged {
                v.push(N::Glue(Glue { st: [0.0, 0.0001, 0.0], sh: md, w: 0.0 }));
            }
            let (_, b, _) = pack_to(&v, h, md);
            if b > 10_000 {
                let mut t = vec![N::Glue(Glue::fixed(-12.0))];
                t.extend(v.iter().cloned());
                if pack_to(&t, h, md).1 > 10_000 {
                    too_bad = true;
                }
            }
            out.push(v);
        }
        if too_bad {
            return Err(list);
        }
        Ok((h, out))
    }

    /// `\mult@@cols` from `\addvspace\multicolsep` through
    /// `\prepare@multicols`.
    fn begin(&mut self, run: Run) {
        self.add_vspace(run.multicolsep);
        if let Some(pd) = self.prev_depth {
            let (pd, bs, ts) = (sp(pd), sp(self.p.baselineskip), sp(self.p.topskip));
            let q = pd / bs + 1;
            let dimen = pd - q * bs + ts;
            self.push(N::Kern(-pt(dimen)));
        }
        self.run = Some(run);
        // `\nointerlineskip {\topskip\z@\null}`, `\eject` under the
        // partial-page `\output`.
        self.push(N::Null);
        self.prev_depth = Some(0.0);
        self.mode = Mode::SavePartial;
        self.push(N::Penalty(EJECT_PENALTY));
        self.build_page();
        let partial = self.partial.clone().unwrap_or_default();
        self.partial = Some(partial.clone());
        self.colroom -= natural(&partial, MAXDIMEN).h;
        self.vsize = self.mult_vsize();
        self.mode = Mode::Multicol;
        self.any = true;
    }

    /// `\endmulticols` (310–353), `\endmulticols*` (906–917) first.
    fn end(&mut self) {
        let r = self.run().clone();
        if r.star {
            if let Some(g) = self.lastskip().filter(|g| g.w > 0.0) {
                self.push(N::Glue(Glue::fixed(-g.w)));
            }
            if let Some(pd) = self.prev_depth.filter(|&d| d > 0.0) {
                self.push(N::Glue(Glue::fixed(-pd)));
            }
            if !r.ragged {
                self.push(N::Glue(Glue::fil(1.0)));
            }
        }
        if !self.has_box && !self.colbreak.is_empty() {
            let v = std::mem::take(&mut self.colbreak);
            self.contrib.extend(v);
        }
        self.push(N::Penalty(0));
        self.push(N::Penalty(END_PENALTY));
        self.build_page();
        if let Some(p) = self.partial.take() {
            self.contrib.extend(p);
        }
        self.mode = Mode::Normal;
        self.build_page();
        if !self.has_box {
            self.vsize = self.colroom;
        } else {
            self.add_penalty(0);
            let free = self.goal - self.total;
            if free < r.postmulticols {
                self.newpage();
            }
        }
        self.add_vspace(r.multicolsep);
        self.prev_depth = Some(0.0);
        self.run = None;
        self.after_region = true;
    }

    /// `\enough@room` (209–225).
    fn enough_room(&mut self, need: f64) {
        self.add_penalty(0);
        let free = if self.has_box { self.goal - self.total } else { MAXDIMEN };
        if free < need {
            self.newpage();
        }
    }

    fn finish(&mut self) {
        self.newpage();
        if self.has_box || !self.contrib.is_empty() {
            self.push(N::Penalty(EJECT_PENALTY));
            self.build_page();
        }
    }
}

// ---------------------------------------------------------------------------
// Hook 2: pagination.

fn rec_span(ctx: &Context, r: usize) -> Option<Span> {
    match ctx.recs.get(r)? {
        BoxRec::Text { clusters, .. } => clusters.first().map(|c| c.span),
        BoxRec::Math(m) => ctx.maths.get(*m).map(|m| m.span),
        BoxRec::Rule { span, .. } => Some(*span),
        BoxRec::Picture(p) => Some(p.span),
        BoxRec::Table(t) => Some(t.span),
        BoxRec::ColorBox(b) => Some(b.span),
        BoxRec::Graphic { span, .. } => Some(*span),
    }
}

/// Source start of each line of a block.
fn line_starts(ctx: &Context, b: &BuiltBlock) -> Vec<Option<usize>> {
    b.block
        .lines
        .lines
        .iter()
        .map(|l| l.items.clone().filter_map(|i| b.recs.get(i).copied().flatten()).filter_map(|r| rec_span(ctx, r)).map(|s| s.start).min())
        .collect()
}

/// Moves a sub-build's blocks and records into `ctx`.
fn adopt(ctx: &mut Context, laid: Laid) -> Vec<BuiltBlock> {
    let rec_off = ctx.recs.len();
    let math_off = ctx.maths.len();
    let fix = |b: &mut BuiltBlock| {
        for r in b.recs.iter_mut().flatten() {
            *r += rec_off;
        }
        b.cache_key = None;
    };
    let mut recs = laid.recs;
    for r in &mut recs {
        match r {
            BoxRec::Math(m) => *m += math_off,
            BoxRec::Table(t) => {
                let mut tr = (**t).clone();
                for p in &mut tr.pieces {
                    fix(&mut p.block);
                }
                *t = Rc::new(tr);
            }
            _ => {}
        }
    }
    ctx.recs.extend(recs);
    ctx.maths.extend(laid.maths);
    let mut blocks = laid.blocks;
    for b in &mut blocks {
        fix(b);
    }
    blocks
}

fn resolve(len: &Option<Len>, quad: f64, default: Glue) -> Glue {
    match len {
        Some(l) => Glue { w: l.pt[0] + l.em[0] * quad, st: [l.pt[1] + l.em[1] * quad, 0.0, 0.0], sh: l.pt[2] + l.em[2] * quad },
        None => default,
    }
}

/// Pages for the outer document of [`outer_doc`]: `None` when this build
/// has no multicols.
pub(super) fn paginate(ctx: &mut Context, doc: &Doc, blocks: &mut Vec<BuiltBlock>, params: &PageParams) -> Option<Vec<BuiltPage>> {
    if !ctx.multicol.active {
        return None;
    }
    ctx.multicol.active = false;
    let scans = ctx.multicol.scans.clone();
    let mut bodies = std::mem::take(&mut ctx.multicol.bodies);
    let style: Stylesheet = ctx.style.clone();
    let quad = ctx.text_params(TextStyle::default(), style.body_size_pt).quad;
    let dp_p = {
        let role = ctx.text_role(TextStyle::default(), style.body_size_pt).0;
        let r = ctx.fonts.resolve(style.family, role, style.body_size_pt);
        r.face.tfm.as_ref().and_then(|t| t.metrics(b'p')).map_or(0.194443 * style.body_size_pt, |m| crate::tfm::Tfm::pt(m.depth, style.body_size_pt))
    };
    let geo = style.class_geometry.as_deref();
    let text_width_sp = geo.map_or(sp(style.text_width_pt), |g| g.frame.columns[0].width.0);
    let default_sep = geo.map_or(10.0, |g| g.params.columnsep.to_pt());
    // Markers: `(block index, document, region, is end)`.
    let mut markers: BTreeMap<usize, (usize, usize, bool)> = BTreeMap::new();
    for (bi, b) in blocks.iter().enumerate() {
        let [Some(r)] = b.recs.as_slice() else { continue };
        let Some(BoxRec::Rule { span, .. }) = ctx.recs.get(*r) else { continue };
        let d = span.document.0;
        let Some(scan) = scans.get(d) else { continue };
        for (k, reg) in scan.regions.iter().enumerate() {
            if (span.start, span.end) == reg.begin {
                markers.insert(bi, (d, k, false));
            } else if (span.start, span.end) == reg.end {
                markers.insert(bi, (d, k, true));
            }
        }
    }
    let n_outer = blocks.len();
    // Column material of every region, built at `\hsize`.
    let mut runs: BTreeMap<(usize, usize), (Run, std::ops::Range<usize>, BTreeMap<(usize, usize), i32>)> = BTreeMap::new();
    for &(d, k, is_end) in markers.values() {
        if !is_end {
            continue;
        }
        let reg = &scans[d].regions[k];
        let n = reg.columns_arg.clamp(2, 20) as usize;
        let s = &reg.settings;
        let sep = sp(resolve(&s.columnsep, quad, Glue::fixed(default_sep)).w);
        // `\hsize\linewidth \advance\hsize\columnsep
        // \advance\hsize-\col@number\columnsep \divide\hsize\col@number`.
        let hsize_sp = flashtex_class_geometry::tex::Sp(text_width_sp + sep - n as i64 * sep).over(n as i64).0;
        let hsize = pt(hsize_sp);
        let gap = pt(text_width_sp - n as i64 * hsize_sp) / (n as f64 - 1.0);
        let counter = |name: &str, default: i64| reg.end_counters.get(name).copied().unwrap_or(default);
        let run = Run {
            n,
            star: reg.star,
            ragged: s.ragged,
            hsize,
            gap,
            rule: resolve(&s.columnseprule, quad, Glue::fixed(style.columnseprule_pt)).w,
            multicolsep: resolve(&s.multicolsep, quad, Glue { w: 12.0, st: [4.0, 0.0, 0.0], sh: 3.0 }),
            premulticols: resolve(&reg.premulticols.clone().or(s.premulticols.clone()), quad, Glue::fixed(50.0)).w,
            postmulticols: resolve(&reg.postmulticols, quad, Glue::fixed(20.0)).w,
            unbalance: counter("unbalance", 0),
            columnbadness: counter("columnbadness", 10000),
            finalcolumnbadness: counter("finalcolumnbadness", 9999),
            minrows: counter("minrows", 1),
            collectmore: counter("collectmore", 0),
        };
        let mut body = bodies.remove(&(d, k)).unwrap_or_default();
        // The first paragraph starts in the columns' fresh vertical list:
        // `\@afterheading`'s suppressed indentation of a heading before the
        // environment does not reach it (only an explicit `\noindent`).
        if let Some(first) = body_first_start(&body) {
            let text = ctx.texts.get(d).copied().unwrap_or("");
            // `\enough@room`'s `\@nobreakfalse` is global: a heading before
            // `\begin{multicols}` no longer suppresses the indentation, a
            // heading in the preface (after it) still does.
            let preface_heading = reg.preface.and_then(|(a, b)| text.get(a..b)).is_some_and(|p| ["\\section", "\\subsection", "\\subsubsection", "\\chapter", "\\part", "\\paragraph"].iter().any(|h| p.contains(h)));
            let explicit = preface_heading || text.get(reg.body.0..first).is_some_and(|s| s.contains("\\noindent"));
            if let Some(Block::Paragraph { indent, list: None, .. }) = body.first_mut() {
                if !explicit {
                    *indent = true;
                }
            }
        }
        let mut col_style = style.clone();
        col_style.text_width_pt = hsize;
        col_style.tolerance = 9999.0;
        col_style.pretolerance = -1.0;
        col_style.emergency_stretch_pt = 4.0 * n as f64;
        col_style.class_geometry = None;
        let sub_doc = Doc {
            style: col_style.clone(),
            blocks: body,
            diagnostics: Vec::new(),
            limitations: Vec::new(),
            secnumdepth: doc.secnumdepth,
            page_starts: Vec::new(),
            default_color: doc.default_color,
            math_colors: doc.math_colors.clone(),
            page_color: doc.page_color,
        };
        let laid = {
            let mut sub = Context::with_texts(ctx.fonts, &col_style, ctx.paths, ctx.texts);
            // `\includegraphics` inside a column reads its files through the
            // same project root and per-request cache as the rest of the page.
            sub.images = ctx.images;
            let laid = super::build_with_floats(&mut sub, &sub_doc, None, &[]);
            let diags = sub.take_diagnostics();
            for d in diags {
                if !ctx.diagnostics.iter().any(|x| x.code == d.code && x.message == d.message && x.sources == d.sources) {
                    ctx.diagnostics.push(d);
                }
            }
            laid
        };
        let built = adopt(ctx, laid);
        let start = blocks.len();
        blocks.extend(built);
        let range = start..blocks.len();
        // Column breaks: after the line holding one (`\vadjust`), or
        // before the next block in vertical mode.
        let mut after: BTreeMap<(usize, usize), i32> = BTreeMap::new();
        for &(at, pen) in &reg.breaks {
            let mut target: Option<(usize, usize, bool)> = None;
            for bi in range.clone() {
                let starts = line_starts(ctx, &blocks[bi]);
                let first = starts.iter().flatten().min().copied();
                let last_line = starts.len().saturating_sub(1);
                for (li, s) in starts.iter().enumerate() {
                    if let Some(s) = s {
                        if *s < at {
                            target = Some((bi, li, li == last_line && first.is_some()));
                        }
                    }
                }
            }
            let Some((bi, li, _)) = target else { continue };
            match pen {
                Some(p) => {
                    after.insert((bi, li), p);
                }
                None => {
                    // `\newcolumn`: `\nobreak\vfill\kern\z@\penalty-\@Mv`,
                    // approximated by the forced break.
                    after.insert((bi, li), COLUMNBREAK);
                }
            }
        }
        runs.insert((d, k), (run, range, after));
    }
    let mut m = Machine::new(*params, dp_p);
    let empty = BTreeMap::new();
    for bi in 0..n_outer {
        match markers.get(&bi) {
            Some(&(d, k, false)) => {
                if let Some((run, _, _)) = runs.get(&(d, k)) {
                    m.enough_room(run.premulticols);
                }
            }
            Some(&(d, k, true)) => {
                let Some((run, range, after)) = runs.get(&(d, k)).cloned() else { continue };
                m.begin(run);
                for cb in range {
                    let v = blocks[cb].vertical.clone();
                    m.push_block(cb, &v, &after);
                }
                m.end();
            }
            None => {
                let v = blocks[bi].vertical.clone();
                m.push_block(bi, &v, &empty);
            }
        }
    }
    m.finish();
    let span0 = Span::in_document(DocumentId(0), 0, 0);
    let mut built = Vec::with_capacity(m.pages.len());
    let mut dx = Vec::with_capacity(m.pages.len());
    for page in std::mem::take(&mut m.pages) {
        let mut bp = BuiltPage { lines: Vec::with_capacity(page.lines.len()), overfull_by: page.overfull_by };
        let mut pdx = Vec::with_capacity(page.lines.len());
        for l in page.lines {
            match l {
                Out::Line { bi, li, baseline, h, d, dx } => {
                    bp.lines.push(Placed { payload: (bi, li), baseline, height: h, depth: d });
                    pdx.push(dx);
                }
                Out::Rule { x, bottom, height } => {
                    let rule_w = runs.values().map(|r| r.0.rule).fold(0.0, f64::max);
                    let b = ctx.rule_block_sized(span0, rule_w, height, x);
                    blocks.push(b);
                    bp.lines.push(Placed { payload: (blocks.len() - 1, 0), baseline: bottom, height, depth: 0.0 });
                    pdx.push(0.0);
                }
            }
        }
        if !bp.lines.is_empty() {
            built.push(bp);
            dx.push(pdx);
        }
    }
    ctx.multicol.dx = dx;
    Some(built)
}

// ---------------------------------------------------------------------------
// Hook 3: column x offsets.

/// Adds each column's x offset to the placed lines (before the page chrome
/// is inserted, so line indices still match the built pages).
pub(super) fn shift(ctx: &mut Context, pages: &mut flashtex_paragraph_layout::Pages, line_dx: &mut [Vec<f64>], blocks: &[BuiltBlock]) {
    let dx = std::mem::take(&mut ctx.multicol.dx);
    if dx.is_empty() {
        return;
    }
    for (pi, page) in pages.pages.iter_mut().enumerate() {
        let Some(pdx) = dx.get(pi) else { continue };
        let mut run = 0usize;
        for (k, l) in page.lines.iter().enumerate() {
            let n_runs = blocks.get(l.paragraph).and_then(|b| b.block.lines.lines.get(l.line)).map_or(0, |line| line.runs.len());
            let extra = pdx.get(k).copied().unwrap_or(0.0);
            if extra != 0.0 {
                if let Some(d) = line_dx.get_mut(pi).and_then(|v| v.get_mut(k)) {
                    *d += extra;
                }
                for r in page.runs.iter_mut().skip(run).take(n_runs) {
                    r.x += extra;
                }
            }
            run += n_runs;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_finds_the_environment_its_arguments_and_breaks() {
        let t = "\\documentclass{article}\n\\usepackage{multicol}\n\\setlength{\\columnsep}{2em}\n\\begin{document}\nA\n\\begin{multicols}{3}[\\section{P}][80pt]\nx \\columnbreak y\n\\end{multicols}\n\\end{document}\n";
        let s = scan(t);
        assert_eq!(s.regions.len(), 1);
        let r = &s.regions[0];
        assert_eq!(r.columns_arg, 3);
        assert_eq!(&t[r.preface.unwrap().0..r.preface.unwrap().1], "\\section{P}");
        assert_eq!(r.premulticols.as_ref().unwrap().pt[0], 80.0);
        assert_eq!(r.breaks.len(), 1);
        assert_eq!(r.settings.columnsep.as_ref().unwrap().em[0], 2.0);
        let masked = s.masked(t).unwrap();
        assert_eq!(masked.len(), t.len());
        assert!(!masked.contains("multicols"));
        assert!(!masked.contains("columnbreak"));
        assert!(masked.contains("\\section{P}"));
    }

    #[test]
    fn vsplit_breaks_at_the_best_place_and_prunes() {
        let line = |i: usize| N::Line { h: 7.0, d: 2.0, bi: 0, li: i };
        let mut v = Vec::new();
        for i in 0..6 {
            if i > 0 {
                v.push(N::Glue(Glue::fixed(3.0)));
            }
            v.push(line(i));
        }
        // Lines at 7, 19, 31, 43, ...: three fit in 31pt.
        let (first, rest) = vsplit(&v, 31.0, Glue::fixed(10.0), 4.0);
        assert_eq!(first.iter().filter(|n| n.is_box()).count(), 3);
        assert!(matches!(rest[0], N::Glue(g) if g.w == 3.0));
        assert_eq!(rest.iter().filter(|n| n.is_box()).count(), 3);
    }

    #[test]
    fn discardable_items_are_removed_from_the_end() {
        let mut v = vec![N::Line { h: 1.0, d: 0.0, bi: 0, li: 0 }, N::Glue(Glue::fixed(3.0)), N::Penalty(0), N::Glue(Glue { st: [1.0, 0.0, 0.0], ..Glue::default() })];
        remove_discardable(&mut v);
        assert_eq!(v.len(), 1);
    }
}
