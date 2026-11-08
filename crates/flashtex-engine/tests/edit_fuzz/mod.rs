//! Shared by the hosts' integration tests: what a display-list client holds
//! of a compile (`View`, `receive`, `resolve`), and random edit sequences
//! against scratch compiles (`run`; lane TEST-GAPS), for the pdfTeX host
//! (tests/host_incremental.rs) and the Unicode host
//! (crates/flashtex-xetex/tests/edit_fuzz.rs, which takes this file by
//! `#[path]`: that crate is a workspace of its own).
#![allow(dead_code)]

use flashtex_display_list::client::{Client, CompileRequest, Edit, Event};
use flashtex_display_list::json::Json;
use flashtex_display_list::page::{Item, Page};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let t = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &t);
        } else {
            std::fs::copy(e.path(), t).unwrap();
        }
    }
}

/// What a client holds: its pages, and what their ids mean.
#[derive(Default)]
pub struct View {
    pub pages: BTreeMap<u32, Page>,
    /// Font id -> key as bound when each page arrived.
    pub page_fonts: BTreeMap<u32, HashMap<u16, [u8; 32]>>,
    pub fonts: HashMap<u16, [u8; 32]>,
    pub spans: HashMap<u32, (u32, u32)>,
    pub files: HashMap<u32, String>,
    /// `IMAGE` messages, in order.
    pub images: Vec<Json>,
    /// `FORM` messages, by form id: the latest one of each.
    pub forms: HashMap<u32, Page>,
    pub count: usize,
}

pub struct Outcome {
    pub started: Json,
    pub done: Json,
    /// Indices of the PAGE messages, in order.
    pub order: Vec<u32>,
    pub pages_msgs: Vec<Json>,
    pub first_page: Option<Duration>,
    /// `PROGRESS` heartbeats received (`progress-v1`, spec §6.8).
    pub progress: usize,
    /// The `file`s the `PROGRESS` heartbeats named, in order.
    pub progress_files: Vec<String>,
    /// `SPAN_SHIFT` messages (`span-shift-v1`, spec §5.3.1), and the spans
    /// `SOURCES` declared (new or moved).
    pub span_shifts: usize,
    pub declared: usize,
}

pub fn compile(c: &mut Client, view: &mut View, req: &CompileRequest) -> Outcome {
    c.compile(req).unwrap();
    receive(c, view, None)
}

/// The messages of one compile, to its DONE (`first`: its first message,
/// already read: a follow-up compile's STARTED).
pub fn receive(c: &mut Client, view: &mut View, mut first: Option<Event>) -> Outcome {
    let t0 = Instant::now();
    let mut started = Json::Null;
    let mut order = vec![];
    let mut pages_msgs = vec![];
    let mut first_page = None;
    let mut progress = 0;
    let mut progress_files = vec![];
    let (mut span_shifts, mut declared) = (0, 0);
    let done = loop {
        let ev = match first.take() {
            Some(e) => e,
            None => c.next_event().unwrap().expect("host closed the connection"),
        };
        match ev {
            Event::Started(j) => {
                if j.get("keep").and_then(Json::as_bool) != Some(true) {
                    *view = View::default();
                }
                started = j;
            }
            Event::Font(f) => {
                assert!(
                    !f.program.is_empty() || f.info.str_field("format") != Some("type1"),
                    "font {} without its program",
                    f.id
                );
                view.fonts.insert(f.id, f.key);
            }
            Event::Sources(s) => {
                declared += s.spans.len();
                for (i, p) in s.files {
                    view.files.insert(i, p);
                }
                for (i, f, l) in s.spans {
                    assert!(
                        view.files.contains_key(&f),
                        "span {i} names unknown file {f}"
                    );
                    view.spans.insert(i, (f, l));
                }
            }
            Event::Page(p) => {
                first_page.get_or_insert(t0.elapsed());
                for it in &p.items {
                    match it {
                        Item::Glyph { font, .. } => {
                            assert!(view.fonts.contains_key(font), "font {font} not sent")
                        }
                        Item::Span(s) if *s != 0 => {
                            assert!(view.spans.contains_key(s), "span {s} not sent")
                        }
                        _ => {}
                    }
                }
                order.push(p.index);
                view.page_fonts.insert(p.index, view.fonts.clone());
                view.pages.insert(p.index, p);
            }
            // (a form by its id, as the client keys it: `resolve` compares
            // the forms a page draws, whose content its hash leaves out;
            // its fonts sent before it, as a page's, spec §5)
            Event::Form(f) => {
                for it in &f.items {
                    if let Item::Glyph { font, .. } = it {
                        assert!(
                            view.fonts.contains_key(font),
                            "form {}: font {font} not sent",
                            f.index
                        )
                    }
                }
                view.forms.insert(f.index, f);
            }
            Event::Pages(j) => pages_msgs.push(j),
            Event::Image(j) => view.images.push(j),
            Event::Done(d) => break d,
            Event::Error(e) => panic!("host error: {e}"),
            Event::Other(k, body) if k == flashtex_display_list::kind::SPAN_SHIFT => {
                span_shifts += 1;
                let j = Json::parse(std::str::from_utf8(&body).unwrap()).unwrap();
                let sh = flashtex_display_list::resource::SpanShifts::from_json(&j).unwrap();
                for at in view.spans.values_mut() {
                    *at = sh.apply(*at);
                }
            }
            Event::Other(k, body) if k == flashtex_display_list::kind::PROGRESS => {
                progress += 1;
                let j = Json::parse(std::str::from_utf8(&body).unwrap()).unwrap();
                if let Some(f) = j.str_field("file") {
                    progress_files.push(f.to_string());
                }
            }
            _ => {}
        }
    };
    let count = done.int_field("pages").unwrap_or(0) as usize;
    view.count = count;
    view.pages.retain(|&i, _| (i as usize) < count);
    Outcome {
        started,
        done,
        order,
        pages_msgs,
        first_page,
        progress,
        progress_files,
        span_shifts,
        declared,
    }
}

/// A page with its spans resolved to (file relative to `root`, line) and
/// its glyphs' fonts to keys, for comparing across hosts.
#[derive(Debug, PartialEq)]
pub struct Resolved {
    pub hash: [u8; 32],
    pub page: Page,
    pub spans: Vec<(String, u32)>,
    pub fonts: Vec<(u16, [u8; 32])>,
    /// The forms the page draws, directly or through other forms, in order
    /// of first use: id and content hash (lane TEST-GAPS: a page's hash
    /// names its forms by id only, so a stale form went unnoticed).
    pub forms: Vec<(u32, [u8; 32])>,
}

/// The forms `items` draw, and the forms those draw, in order of first
/// use, as `view` holds them; a form drawn but never sent fails.
pub fn forms_drawn(view: &View, items: &[Item], what: &str, into: &mut Vec<(u32, [u8; 32])>) {
    for it in items {
        if let Item::Form { id, .. } = it {
            if into.iter().any(|(k, _)| k == id) {
                continue;
            }
            let f = view
                .forms
                .get(id)
                .unwrap_or_else(|| panic!("{what} draws form {id}, which was never sent"));
            into.push((*id, f.hash));
            forms_drawn(view, &f.items, what, into);
        }
    }
}

pub fn resolve(view: &View, i: u32, root: &Path, out: &Path) -> Resolved {
    let mut page = view
        .pages
        .get(&i)
        .unwrap_or_else(|| panic!("page {i} missing"))
        .clone();
    // Files under the project or the output directory, by their place
    // there (the scratch compile's copies are elsewhere).
    let prefixes: Vec<(String, &str)> = [(root, ""), (out, "<out>/")]
        .iter()
        .flat_map(|(d, tag)| {
            let canon = std::fs::canonicalize(d).unwrap_or_else(|_| d.to_path_buf());
            [
                (format!("{}/", d.display()), *tag),
                (format!("{}/", canon.display()), *tag),
            ]
        })
        .collect();
    let loc = |s: u32| -> (String, u32) {
        if s == 0 {
            return (String::new(), 0);
        }
        let (f, l) = view.spans[&s];
        let p = view.files[&f].clone();
        for (pre, tag) in &prefixes {
            if let Some(rest) = p.strip_prefix(pre.as_str()) {
                return (format!("{tag}{rest}"), l);
            }
        }
        (p, l)
    };
    let mut spans = vec![];
    for it in page.items.iter_mut() {
        if let Item::Span(s) = it {
            spans.push(loc(*s));
            *s = 0;
        }
    }
    for l in page.links.iter_mut() {
        spans.push(loc(l.span));
        l.span = 0;
    }
    let bound = &view.page_fonts[&i];
    let mut fonts: Vec<(u16, [u8; 32])> = page
        .items
        .iter()
        .filter_map(|it| match it {
            Item::Glyph { font, .. } => Some((*font, bound[font])),
            _ => None,
        })
        .collect();
    let mut forms = vec![];
    forms_drawn(view, &page.items, &format!("page {i}"), &mut forms);
    // (and the fonts of the forms it draws: a form's font may be used
    // nowhere else, review of #2019)
    for (id, _) in &forms {
        for it in &view.forms[id].items {
            if let Item::Glyph { font, .. } = it {
                let key = bound.get(font).or_else(|| view.fonts.get(font));
                let key = *key.unwrap_or_else(|| panic!("form {id}: font {font} not sent"));
                fonts.push((*font, key));
            }
        }
    }
    fonts.sort();
    fonts.dedup();
    Resolved {
        hash: page.hash,
        page,
        spans,
        fonts,
        forms,
    }
}

/// What differs between two resolved pages (the first difference).
pub fn describe_difference(a: &Resolved, b: &Resolved) -> String {
    if a.hash != b.hash {
        return format!(
            "content hash {:02x?}.. vs {:02x?}..",
            &a.hash[..4],
            &b.hash[..4]
        );
    }
    if a.fonts != b.fonts {
        return format!("fonts {:?} vs {:?}", a.fonts, b.fonts);
    }
    if a.forms != b.forms {
        let short = |v: &[(u32, [u8; 32])]| -> Vec<String> {
            v.iter()
                .map(|(k, h)| format!("{k}:{:02x?}", &h[..4]))
                .collect()
        };
        return format!("forms {:?} vs {:?}", short(&a.forms), short(&b.forms));
    }
    if let Some(k) =
        (0..a.spans.len().max(b.spans.len())).find(|&k| a.spans.get(k) != b.spans.get(k))
    {
        return format!(
            "span #{k}: {:?} vs {:?} (of {} and {})",
            a.spans.get(k),
            b.spans.get(k),
            a.spans.len(),
            b.spans.len()
        );
    }
    let (p, q) = (&a.page, &b.page);
    if let Some(k) =
        (0..p.items.len().max(q.items.len())).find(|&k| p.items.get(k) != q.items.get(k))
    {
        return format!("item #{k}: {:?} vs {:?}", p.items.get(k), q.items.get(k));
    }
    if p.links != q.links {
        return format!("links {:?} vs {:?}", p.links, q.links);
    }
    if p.dests != q.dests {
        return format!("dests {:?} vs {:?}", p.dests, q.dests);
    }
    format!(
        "header/other: {:?} vs {:?}",
        (p.index, p.flags, p.counts),
        (q.index, q.flags, q.counts)
    )
}

// ---------------------------------------------------------------------------
// Lane TEST-GAPS: random edit sequences against scratch compiles.
//
// From a seed, each document gets a sequence of edits as an author makes
// them: letters typed into and deleted from words, a control sequence's
// name broken and mended, preamble lines, labels, references, citations
// and sections (the `.aux` and the passes), edits in the files the main
// file `\input`s, `.bib` entries and index terms (the external tools),
// reverts to an earlier text (undo), and jumps far from the last edit; in
// a fontspec or unicode-math document also non-ASCII text, font options
// and font changes mid-document. Half arrive as COMPILE edits (byte
// splices), half as files written on disk. After every step the host's
// pages -- with every form they draw -- and its log equal a scratch
// compile's on a copy of the directory. A mismatch is shrunk to a shortest
// failing sequence found (steps removed while it still fails), and the
// panic names the seed, the document and that sequence.
//
// Short by default (each host's `random_edit_sequences_equal_scratch_compiles`:
// a few documents, 8 steps each). The long mode is the same test with
// FLASHTEX_FUZZ_SEED (first seed, default 1), FLASHTEX_FUZZ_SEEDS (how
// many), FLASHTEX_FUZZ_STEPS (per document) and FLASHTEX_FUZZ_DOCS (fixture
// names, comma-separated, or `all`): .github/workflows/edit-fuzz.yml.
// FLASHTEX_FUZZ_TRACE prints each step with its compile's mode.
// ---------------------------------------------------------------------------

/// A host under the fuzz.
pub trait FuzzHost {
    /// A host running (it stops when the box is dropped) and its socket;
    /// `scratch`: the one for the scratch compiles.
    fn start(&self, tag: &str, scratch: bool) -> (Box<dyn std::any::Any>, PathBuf);
    /// The fuzz's compile of `main` in `proj`, written to `out`.
    fn request(&self, id: i64, proj: &Path, out: &Path, main: &str) -> CompileRequest;
    /// The scratch compile of the copy (`tools`: it runs the external
    /// tools; its first compile must say `cold`).
    fn scratch(&self, proj: &Path, out: &Path, main: &str, tools: bool) -> CompileRequest;
}

/// A directory of this process's own, `$TMPDIR/<prefix>-<pid>-<nanos>`.
fn fresh_dir(prefix: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

/// xorshift64*: the sequences are the seed's, on every machine.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

/// One step of a sequence: a splice of one file (offsets are clamped when a
/// shrunk sequence replays it on another text), or a revert of every file
/// to the text before step `to` of the sequence.
#[derive(Clone, Debug)]
enum FuzzStep {
    Splice {
        file: String,
        at: usize,
        delete: usize,
        insert: String,
        /// as a COMPILE edit (else written on disk)
        wire: bool,
    },
    Revert {
        to: usize,
    },
}

/// The document's editable sources: the main file and the `.tex` files
/// under its directory (what it `\input`s or `\include`s).
fn fuzz_sources(proj: &Path, main: &str) -> Vec<String> {
    let mut v = vec![main.to_string()];
    let mut stack = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        for e in std::fs::read_dir(proj.join(&rel)).unwrap().flatten() {
            let r = rel.join(e.file_name());
            if e.file_type().unwrap().is_dir() {
                stack.push(r);
            } else if r.extension().is_some_and(|x| x == "tex" || x == "bib") {
                let s = r.to_string_lossy().into_owned();
                if s != main {
                    v.push(s);
                }
            }
        }
    }
    v[1..].sort();
    v
}

/// Byte offsets inside words (between two ASCII letters) of `text`, from
/// `lo` on; not in a control word's name or an environment's (`\begin{..}`,
/// `\end{..}`): a name broken is an edit kind of its own, and the error
/// it gives stays until a revert mends it.
fn inside_words(text: &str, lo: usize) -> Vec<usize> {
    let b = text.as_bytes();
    let mut v = vec![];
    let mut run = 0;
    for i in 1..b.len() {
        if !b[i - 1].is_ascii_alphabetic() {
            run = i;
        }
        if i < lo || !b[i - 1].is_ascii_alphabetic() || !b[i].is_ascii_alphabetic() {
            continue;
        }
        // (`run`, the word's first letter, is ASCII: a char boundary)
        let name = run > 0
            && (b[run - 1] == b'\\'
                || text[..run].ends_with("\\begin{")
                || text[..run].ends_with("\\end{"));
        if !name && !names_a_file(text, i) {
            v.push(i);
        }
    }
    v
}

/// Whether offset `i` is in the name of a file that `text` loads there: a
/// class, a package, a theme, an `\input` or `\include`. A letter typed or
/// deleted there names a file that does not exist, and TeX stops before
/// the first page -- on both sides alike, so that the steps until a revert
/// would compare no page.
fn names_a_file(text: &str, i: usize) -> bool {
    let ls = text[..i].rfind('\n').map_or(0, |k| k + 1);
    let Some(bs) = text[ls..i].rfind('\\').map(|k| ls + k) else {
        return false;
    };
    let name: String = text[bs + 1..i]
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    let loads = matches!(
        name.as_str(),
        "documentclass"
            | "usepackage"
            | "RequirePackage"
            | "input"
            | "include"
            | "includegraphics"
            | "bibliography"
            | "bibliographystyle"
    ) || (name.starts_with("use") && name.ends_with("theme"));
    // (in its arguments, up to the closing brace)
    loads && !text[bs + 1 + name.len()..i].contains('}')
}

/// FNV-1a, 64 bits: a document's name as a number for its sequences' seed.
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The keys of a `.bib` text's entries (`@type{key,`).
fn bib_keys(t: &str) -> Vec<String> {
    t.match_indices('@')
        .filter_map(|(i, _)| {
            let r = &t[i..];
            let open = r.find('{')?;
            let key = r[open + 1..].split(',').next()?.trim();
            (!key.is_empty() && !key.contains(['\n', '}', ' '])).then(|| key.to_string())
        })
        .collect()
}

/// A `.bib` edit: letters typed into an author, a year changed, an entry
/// added (keyed as the citations of `fuzz_step` that the `.bib` lacks) or
/// one taken out.
fn fuzz_bib_step(
    rng: &mut Rng,
    file: &str,
    text: &str,
    keys: &[String],
    labels: &mut usize,
    wire: bool,
) -> FuzzStep {
    let step = |at: usize, delete: usize, insert: String| FuzzStep::Splice {
        file: file.to_string(),
        at,
        delete,
        insert,
        wire,
    };
    let field = |name: &str| -> Vec<(usize, usize)> {
        text.match_indices(name)
            .filter_map(|(i, _)| {
                let open = i + text[i..].find('{')? + 1;
                Some((open, open + text[open..].find('}')?))
            })
            .collect()
    };
    let entries: Vec<usize> = text
        .match_indices('@')
        .map(|(i, _)| i)
        .filter(|&i| i == 0 || text.as_bytes()[i - 1] == b'\n')
        .collect();
    match rng.below(100) {
        k if k < 40 => {
            let authors = field("author");
            if !authors.is_empty() {
                let (a, b) = authors[rng.below(authors.len())];
                let inside: Vec<usize> = inside_words(&text[..b], a);
                if !inside.is_empty() {
                    let at = inside[rng.below(inside.len())];
                    let ins: String = (0..1 + rng.below(2))
                        .map(|_| (b'a' + rng.below(26) as u8) as char)
                        .collect();
                    return step(at, 0, ins);
                }
            }
        }
        k if k < 60 => {
            let years = field("year");
            if !years.is_empty() {
                let (a, b) = years[rng.below(years.len())];
                return step(a, b - a, format!("{}", 1950 + rng.below(76)));
            }
        }
        k if k < 80 && entries.len() > 1 => {
            let i = rng.below(entries.len());
            let a = entries[i];
            let b = entries.get(i + 1).copied().unwrap_or(text.len());
            return step(a, b - a, String::new());
        }
        _ => {}
    }
    // an entry added: one cited and missing, or a new one
    let n = 1 + rng.below((*labels).max(1));
    let key = format!("fz{n}");
    let key = if keys.contains(&key) {
        *labels += 1;
        format!("fz{}", *labels)
    } else {
        key
    };
    step(
        text.len(),
        0,
        format!(
            "\n@article{{{key},\n  author = {{Fuzz Author}},\n  title = {{A Fuzzed Title}},\n  \
             journal = {{Journal of Edits}},\n  year = {{{}}}\n}}\n",
            1990 + rng.below(30)
        ),
    )
}

/// The next step for `texts` (the sources' current text), near `last` (the
/// last edit's file and offset) unless the step jumps.
fn fuzz_step(
    rng: &mut Rng,
    texts: &[(String, String)],
    main: &str,
    last: &mut Option<(usize, usize)>,
    done: usize,
    labels: &mut usize,
) -> FuzzStep {
    if done > 1 && rng.chance(12) {
        return FuzzStep::Revert {
            to: rng.below(done),
        };
    }
    let wire = rng.chance(50);
    // which file: the last one mostly, another sometimes (a `.bib` file
    // less often, so that its edits mix with the text's)
    let leave = |f: usize| if texts[f].0.ends_with(".bib") { 50 } else { 25 };
    let fi = match *last {
        Some((f, _)) if texts.len() < 2 || !rng.chance(leave(f)) => f,
        // (another file than the last)
        Some((f, _)) => (f + 1 + rng.below(texts.len() - 1)) % texts.len(),
        None => rng.below(texts.len()),
    };
    let (file, text) = &texts[fi];
    // the `.bib` keys, for citations
    let keys: Vec<String> = texts
        .iter()
        .filter(|(f, _)| f.ends_with(".bib"))
        .flat_map(|(_, t)| bib_keys(t))
        .collect();
    if file.ends_with(".bib") {
        let s = fuzz_bib_step(rng, file, text, &keys, labels, wire);
        if let FuzzStep::Splice { at, .. } = &s {
            *last = Some((fi, *at));
        }
        return s;
    }
    let body = if file == main {
        text.find("\\begin{document}").map_or(0, |i| i + 16)
    } else {
        0
    };
    // `\includeonly`: a chapter put in or taken out of its list
    if let Some(io) = text.find("\\includeonly{").filter(|_| rng.chance(10)) {
        let open = io + "\\includeonly{".len();
        let close = open + text[open..].find('}').unwrap_or(0);
        let names: Vec<&str> = texts
            .iter()
            .flat_map(|(_, t)| t.match_indices("\\include{").map(move |(i, _)| &t[i + 9..]))
            .filter_map(|r| r.split('}').next())
            .collect();
        if !names.is_empty() {
            let n = names[rng.below(names.len())];
            let mut list: Vec<&str> = text[open..close]
                .split(',')
                .filter(|s| !s.is_empty())
                .collect();
            if let Some(k) = list.iter().position(|x| *x == n) {
                list.remove(k);
            } else {
                list.push(n);
            }
            *last = Some((fi, open));
            return FuzzStep::Splice {
                file: file.clone(),
                at: open,
                delete: close - open,
                insert: list.join(","),
                wire,
            };
        }
    }
    let end = text.find("\\end{document}").unwrap_or(text.len());
    // where: near the last edit, or anywhere (a jump)
    let near = |rng: &mut Rng, cands: &[usize]| -> Option<usize> {
        if cands.is_empty() {
            return None;
        }
        if let Some((f, o)) = *last {
            if f == fi && !rng.chance(30) {
                let close: Vec<usize> = cands
                    .iter()
                    .copied()
                    .filter(|&c| c.abs_diff(o) < 400)
                    .collect();
                if !close.is_empty() {
                    return Some(close[rng.below(close.len())]);
                }
            }
        }
        Some(cands[rng.below(cands.len())])
    };
    let words: Vec<usize> = inside_words(text, body)
        .into_iter()
        .filter(|&i| i < end)
        .collect();
    let kind = rng.below(100);
    let step = |at: usize, delete: usize, insert: String| FuzzStep::Splice {
        file: file.clone(),
        at,
        delete,
        insert,
        wire,
    };
    // a fontspec or unicode-math document (the Unicode host): its own
    // edits, a third of the time (the draw only there, so that the other
    // documents' sequences stay as they were)
    let unicode = texts.iter().any(|(_, t)| {
        t.contains("\\usepackage{fontspec}") || t.contains("\\usepackage{unicode-math}")
    });
    let s = if unicode && rng.chance(35) {
        fuzz_unicode_step(rng, text, body, end, labels, &near, &step, &words)
    } else if kind < 35 {
        // letters typed into a word
        near(rng, &words).map(|at| {
            let n = 1 + rng.below(3);
            let ins: String = (0..n)
                .map(|_| (b'a' + rng.below(26) as u8) as char)
                .collect();
            step(at, 0, ins)
        })
    } else if kind < 55 {
        // letters deleted from a word (both sides stay letters)
        near(rng, &words).map(|at| {
            let b = text.as_bytes();
            let mut n = 1 + rng.below(2);
            while n > 0 && !(at + n < b.len() && b[at + n].is_ascii_alphabetic()) {
                n -= 1;
            }
            step(at, n.max(1).min(b.len() - at), String::new())
        })
    } else if kind < 65 {
        // a control sequence's name broken (a letter in it), or mended by
        // a later revert
        let cs: Vec<usize> = text
            .match_indices('\\')
            .map(|(i, _)| i + 2)
            .filter(|&i| {
                i < end
                    && i >= body
                    && text
                        .as_bytes()
                        .get(i - 1)
                        .is_some_and(|c| c.is_ascii_alphabetic())
                    && text
                        .as_bytes()
                        .get(i)
                        .is_some_and(|c| c.is_ascii_alphabetic())
            })
            .collect();
        near(rng, &cs).map(|at| step(at, 0, "x".into()))
    } else if kind < 73 && file == main && body > 0 {
        // the preamble: a definition, a comment, or a letter of a word
        let at = text[..body - 16].rfind('\n').map_or(0, |i| i + 1);
        let pre: Vec<usize> = inside_words(text, 0)
            .into_iter()
            .filter(|&i| i < body - 16)
            .collect();
        Some(match rng.below(3) {
            // (a new name each time: `\newcommand` of a defined one is an
            // error that would stay)
            0 => {
                *labels += 1;
                step(
                    at,
                    0,
                    format!("\\newcommand\\fuzz{}{{x}}\n", roman_fuzz(*labels)),
                )
            }
            1 => step(at, 0, "% a preamble comment\n".into()),
            _ => match near(rng, &pre) {
                Some(p) => step(p, 0, "q".into()),
                None => step(at, 0, "% a preamble comment\n".into()),
            },
        })
    } else if kind < 90 {
        // the .aux: a label, a reference to one, a citation, a section
        let lines: Vec<usize> = std::iter::once(body)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .filter(|&i| i >= body && i < end)
            .collect();
        near(rng, &lines).map(|at| {
            *labels += 1;
            let n = *labels;
            let ins = match rng.below(4) {
                0 => format!("\\label{{fz{n}}}"),
                1 => format!(
                    "See~\\ref{{fz{}}} on page~\\pageref{{fz{}}}.\n",
                    n / 2,
                    n / 2
                ),
                // (a key of the `.bib` half the time, else one it lacks)
                2 if !keys.is_empty() && rng.chance(50) => {
                    format!("\\cite{{{}}}\n", keys[rng.below(keys.len())])
                }
                2 => format!("\\cite{{fz{}}}\n", 1 + rng.below(n)),
                _ => format!("\\section{{Fuzz {n}}}\\label{{fz{n}}}\n"),
            };
            step(at, 0, ins)
        })
    } else if kind < 95 && text.contains("\\index{") {
        // an index term: one added after a word, or one taken out
        let terms: Vec<(usize, usize)> = text
            .match_indices("\\index{")
            .filter_map(|(i, _)| Some((i, i + text[i..].find('}')? + 1)))
            .filter(|&(i, _)| i >= body && i < end)
            .collect();
        if rng.chance(60) || terms.is_empty() {
            let ends: Vec<usize> = (body.max(1)..end)
                .filter(|&i| {
                    let b = text.as_bytes();
                    b[i - 1].is_ascii_alphabetic() && !b[i].is_ascii_alphabetic()
                })
                .collect();
            const TERMS: [&str; 6] = [
                "glue",
                "badness",
                "fuzz term",
                "page",
                "penalty!widow",
                "zeta function",
            ];
            near(rng, &ends).map(|at| {
                step(
                    at,
                    0,
                    format!("\\index{{{}}}", TERMS[rng.below(TERMS.len())]),
                )
            })
        } else {
            let (a, b) = terms[rng.below(terms.len())];
            Some(step(a, b - a, String::new()))
        }
    } else {
        // a paragraph break, or two lines joined
        let nl: Vec<usize> = text
            .match_indices('\n')
            .map(|(i, _)| i)
            .filter(|&i| i >= body && i < end)
            .collect();
        near(rng, &nl).map(|at| {
            if rng.chance(50) {
                step(at, 0, "\n".into())
            } else {
                step(at, 1, " ".into())
            }
        })
    };
    let s = s.unwrap_or_else(|| step(body.min(end), 0, "\n% nothing to edit here\n".into()));
    if let FuzzStep::Splice { at, .. } = &s {
        *last = Some((fi, *at));
    }
    s
}

/// A fontspec or unicode-math document's own edits: non-ASCII text typed
/// into a word or after one, a non-ASCII character taken out, a font's
/// options changed (`\setmainfont{..}[..]` and its kin, in the preamble or
/// the body), a font changed mid-document, or math, in another math
/// version where the document declares them. `None`: nowhere to edit.
#[allow(clippy::too_many_arguments)]
fn fuzz_unicode_step(
    rng: &mut Rng,
    text: &str,
    body: usize,
    end: usize,
    labels: &mut usize,
    near: &dyn Fn(&mut Rng, &[usize]) -> Option<usize>,
    step: &dyn Fn(usize, usize, String) -> FuzzStep,
    words: &[usize],
) -> Option<FuzzStep> {
    // (Latin with accents and ligatures, Greek, Cyrillic, Devanagari,
    // Chinese: a font without them says so in the log, as XeTeX does)
    const TEXT: [&str; 12] = [
        "é",
        "ß",
        "ø",
        "ﬁ",
        "—",
        "€",
        " naïve café ",
        " Ἑλληνικά ",
        " Привет мир ",
        " नमस्ते दुनिया ",
        " 中文排版测试 ",
        " “quoted” ",
    ];
    const OPTS: [&str; 10] = [
        "Numbers=OldStyle",
        "Numbers=Lining",
        "LetterSpace=6",
        "Scale=1.1",
        "WordSpace=1.4",
        "Ligatures=Common",
        "Ligatures=NoCommon",
        "Color=336699",
        "FakeBold=2",
        "Letters=SmallCaps",
    ];
    let lines: Vec<usize> = std::iter::once(body)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .filter(|&i| i >= body && i < end)
        .collect();
    let k = rng.below(100);
    if k < 30 {
        return near(rng, words).map(|at| step(at, 0, TEXT[rng.below(TEXT.len())].into()));
    }
    if k < 45 {
        let cands: Vec<usize> = text[body..end]
            .char_indices()
            .filter(|(_, c)| !c.is_ascii())
            .map(|(i, _)| body + i)
            .collect();
        if let Some(at) = near(rng, &cands) {
            let n = text[at..].chars().next().map_or(1, char::len_utf8);
            return Some(step(at, n, String::new()));
        }
    }
    if k < 70 {
        // a declaration's options: `{name}` then `[options]`, or none
        let decls: Vec<(usize, Option<usize>)> = [
            "\\setmainfont",
            "\\setsansfont",
            "\\setmonofont",
            "\\newfontfamily",
            "\\setmathfont",
            "\\fontspec",
        ]
        .iter()
        .flat_map(|d| text.match_indices(*d).map(|(i, _)| i))
        .filter_map(|i| {
            let name = i + text[i..].find('{')?;
            let close = name + text[name..].find('}')? + 1;
            let opts = text[close..]
                .starts_with('[')
                .then(|| text[close..].find(']').map(|e| close + e))
                .flatten();
            Some((close, opts))
        })
        .collect();
        if !decls.is_empty() {
            let (close, opts) = decls[rng.below(decls.len())];
            let o = OPTS[rng.below(OPTS.len())];
            return Some(match opts {
                None => step(close, 0, format!("[{o}]")),
                Some(e) => {
                    let mut parts: Vec<String> = text[close + 1..e]
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let key = |p: &str| p.split('=').next().unwrap_or("").trim().to_string();
                    if let Some(i) = parts.iter().position(|p| p == o) {
                        parts.remove(i);
                    } else if let Some(i) = parts.iter().position(|p| key(p) == key(o)) {
                        parts[i] = o.into();
                    } else {
                        parts.push(o.into());
                    }
                    step(close + 1, e - close - 1, parts.join(", "))
                }
            });
        }
    }
    let at = near(rng, &lines)?;
    if k < 90 || !text.contains("unicode-math") {
        // a font changed mid-document: for a group, or to the end
        *labels += 1;
        let fam = format!("fuzzfam{}", roman_fuzz(*labels));
        let ins = match rng.below(7) {
            0 => "{\\sffamily Sans words, ﬁne and ﬂat.}\n".to_string(),
            1 => "\\fontspec{texgyretermes-regular.otf}\n".to_string(),
            2 => "{\\fontspec{texgyreschola-regular.otf}[Numbers=OldStyle]Schola 1234567890.}\n"
                .to_string(),
            3 => "\\setmainfont{texgyreadventor-regular.otf}\\rmfamily\n".to_string(),
            4 => "{\\addfontfeatures{LetterSpace=8}spaced words}\n".to_string(),
            5 => format!("\\newfontfamily\\{fam}{{texgyreheros-regular.otf}}{{\\{fam} Heros}}\n"),
            _ => "\\normalfont\n".to_string(),
        };
        return Some(step(at, 0, ins));
    }
    // math, in a declared version (`version=`) or the document's
    let versions: Vec<&str> = text
        .match_indices("version=")
        .filter_map(|(i, _)| text[i + 8..].split([']', ',']).next())
        .collect();
    let ins = match rng.below(3) {
        0 if !versions.is_empty() => format!(
            "{{\\mathversion{{{}}}$\\int_0^1 x^2\\,dx = \\frac13$}}\n",
            versions[rng.below(versions.len())]
        ),
        1 => "\\[ \\sum_{k=1}^{n} k^2 = \\frac{n(n+1)(2n+1)}{6} \\]\n".to_string(),
        _ => "$\\symbf{\\alpha} + \\mathbb{R} \\subseteq \\mathcal{F}$\n".to_string(),
    };
    Some(step(at, 0, ins))
}

fn roman_fuzz(mut k: usize) -> String {
    // (a control sequence name of letters only)
    let mut s = String::new();
    loop {
        s.push((b'a' + (k % 26) as u8) as char);
        k /= 26;
        if k == 0 {
            return s;
        }
    }
}

/// The log with TeX's line wrapping undone (a line of `max_print_line`, 79
/// bytes, goes on in the next), `from` replaced by `to` (the copy's
/// directory, the same length, by the host's), and without DESIGN.md
/// §1.1's capacity accounting.
fn fuzz_log(log: &[u8], from: &Path, to: &Path) -> String {
    let raw = String::from_utf8_lossy(log);
    let mut t = String::new();
    for ln in raw.split('\n') {
        t.push_str(ln);
        if ln.len() != 79 {
            t.push('\n');
        }
    }
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    for (a, b) in [
        (from.to_path_buf(), to.to_path_buf()),
        (canon(from), canon(to)),
    ] {
        t = t.replace(&*a.to_string_lossy(), &b.to_string_lossy());
    }
    let mut out = vec![];
    let mut in_block = false;
    for ln in t.split('\n') {
        if ln == "Here is how much of TeX's memory you used:" || ln == "PDF statistics:" {
            in_block = true;
            continue;
        }
        if in_block && ln.starts_with(' ') {
            continue;
        }
        in_block = false;
        match ln.rfind(" bytes).") {
            Some(_) if ln.starts_with("Output written on ") => {
                out.push(ln[..ln.rfind(", ").unwrap_or(ln.len())].to_string())
            }
            _ => out.push(ln.to_string()),
        }
    }
    out.join("\n")
}

/// The document runs external tools (a `.bib`, `\makeindex`): its compiles
/// ask for them (`external_tools: auto`), as the app does.
fn fuzz_tools(proj: &Path, main: &str) -> bool {
    fuzz_sources(proj, main).iter().any(|f| {
        f.ends_with(".bib")
            || std::fs::read_to_string(proj.join(f)).is_ok_and(|t| t.contains("\\makeindex"))
    })
}

/// A compile and, with `tools`, the follow-up compiles the host runs after
/// bibtex and makeindex, until it says the cycle is `settled`: the first
/// compile's mode (with `+tools` when a tool ran, whatever its status),
/// the last compile's outcome, and how many tool runs succeeded (status
/// `ok` or `warnings`: a fuzzed `.bib` may make bibtex fail, on both
/// sides alike, but a failed run proves nothing about the reruns).
fn fuzz_cycle(
    c: &mut Client,
    view: &mut View,
    r: &CompileRequest,
    tools: bool,
) -> (String, Outcome, usize) {
    let first = compile(c, view, r);
    let mut mode = first.done.str_field("mode").unwrap_or("?").to_string();
    if !tools {
        return (mode, first, 0);
    }
    let mut last = first;
    let mut ok = 0;
    loop {
        match c.next_event().unwrap().expect("host closed the connection") {
            Event::Tool(t)
                if t.str_field("event") == Some("settled") && t.int_field("id") == Some(r.id) =>
            {
                return (mode, last, ok);
            }
            Event::Tool(t) if t.str_field("event") == Some("done") => {
                if !mode.ends_with("+tools") {
                    mode.push_str("+tools");
                }
                ok += matches!(t.str_field("status"), Some("ok" | "warnings")) as usize;
            }
            ev @ Event::Started(_) => last = receive(c, view, Some(ev)),
            Event::Error(e) => panic!("host error: {e}"),
            _ => {}
        }
    }
}

/// `view` (the host's pages, compiled in `base/a/proj` and `base/a/out`)
/// against a scratch compile of a copy in `base/b`: pages, the forms they
/// draw, the log (whose paths, the same length, are put back). With
/// `tools`, the scratch compile starts without the tools' outputs and runs
/// the tools and the compiles after them, latexmk's way. `Err`: the first
/// difference.
fn fuzz_compare(
    host: &dyn FuzzHost,
    sock: &Path,
    view: &View,
    base: &Path,
    main: &str,
    tools: bool,
) -> Result<(), String> {
    let (proj, out) = (base.join("a/proj"), base.join("a/out"));
    let (proj, out) = (proj.as_path(), out.as_path());
    // (`b` and `c` by turns: the scratch host holds one document, so a
    // directory other than the last one's compiles cold)
    static TURN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    let copy = if TURN.fetch_xor(true, std::sync::atomic::Ordering::Relaxed) {
        "c"
    } else {
        "b"
    };
    let (p2, o2) = (base.join(copy).join("proj"), base.join(copy).join("out"));
    let _ = std::fs::remove_dir_all(base.join(copy));
    copy_dir(proj, &p2);
    copy_dir(out, &o2);
    let stem = Path::new(main)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    // (the scratch compile writes its own log: a copy of the host's would
    // compare equal however the scratch compile went)
    let _ = std::fs::remove_file(o2.join(format!("{stem}.log")));
    if tools {
        // (the tools' outputs made again: the scratch compile runs bibtex
        // and makeindex itself; the `.aux` files stay, which LaTeX reads
        // as the last run left them -- an `\includeonly` keeps the parts it
        // leaves out by theirs)
        for e in std::fs::read_dir(&o2).unwrap().flatten() {
            let p = e.path();
            if p.extension()
                .is_some_and(|x| ["bbl", "blg", "ind", "ilg"].iter().any(|t| x == *t))
            {
                let _ = std::fs::remove_file(p);
            }
        }
    }
    let r = (|| {
        let mut s = Client::connect(sock).map_err(|e| e.to_string())?;
        let mut sv = View::default();
        let rs = host.scratch(&p2, &o2, main, tools);
        let (mode, so, _) = fuzz_cycle(&mut s, &mut sv, &rs, tools);
        let _ = s.bye();
        // (a harness fault, not the host's: say so)
        // (with `tools`, "cold+tools": the scratch compile ran them)
        let want = if tools { "cold+tools" } else { "cold" };
        assert_eq!(mode, want, "the scratch compile was not cold: {}", so.done);
        if sv.count != view.count {
            return Err(format!(
                "{} pages, a scratch compile {}",
                view.count, sv.count
            ));
        }
        for i in 0..view.count as u32 {
            if !view.pages.contains_key(&i) {
                return Err(format!("page {i} missing"));
            }
            let a = resolve(view, i, proj, out);
            let b = resolve(&sv, i, &p2, &o2);
            if a != b {
                return Err(format!("page {i}: {}", describe_difference(&a, &b)));
            }
        }
        // (a log missing on both sides is not a match)
        let log = |d: &Path, whose: &str| {
            std::fs::read(d.join(format!("{stem}.log")))
                .map_err(|e| format!("{whose} {stem}.log: {e}"))
        };
        let (mine, theirs) = (
            fuzz_log(&log(out, "the host's")?, &base.join("a"), &base.join("a")),
            fuzz_log(
                &log(&o2, "a scratch compile's")?,
                &base.join(copy),
                &base.join("a"),
            ),
        );
        if mine != theirs {
            let k = mine
                .lines()
                .zip(theirs.lines())
                .position(|(a, b)| a != b)
                .unwrap_or(mine.lines().count().min(theirs.lines().count()));
            return Err(format!(
                "the log differs at line {}: {:?} vs a scratch compile's {:?} (status {:?})",
                k + 1,
                mine.lines().nth(k),
                theirs.lines().nth(k),
                so.done.str_field("status")
            ));
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(base.join(copy));
    r
}

/// What a replay compiled: the compiles by mode; `paged`, the steps whose
/// compile was incremental (neither cold nor unchanged) and had pages, in
/// any status; `live`, those of them that were `ok` -- the steps that test
/// the incremental host (a name broken stays in error until a revert mends
/// it, so a sequence may have no `live` step; a seed may not); and the
/// tool runs that succeeded before the first edit (`baseline_tools`) and
/// after it (`tools`; a seed's tool fixtures need one).
#[derive(Default, Debug)]
struct FuzzStats {
    modes: BTreeMap<String, usize>,
    paged: usize,
    live: usize,
    tools_wanted: bool,
    baseline_tools: usize,
    tools: usize,
}

/// Replay `steps` on a fresh copy of `src` in a fresh host: `Err` names the
/// first step whose compile differs from a scratch compile (and how), or
/// step 0 when the document itself does not compile `ok` with pages (and,
/// with tools, a tool run that succeeded).
fn fuzz_replay(
    fh: &dyn FuzzHost,
    src: &Path,
    main: &str,
    steps: &[FuzzStep],
    tag: &str,
) -> Result<FuzzStats, (usize, String)> {
    let base = fresh_dir(&format!("flashtex-fuzz-{tag}"));
    let _ = std::fs::remove_dir_all(&base);
    // (the host's directories under `a`, the scratch compile's copies under
    // `b`: the same length, so that the logs wrap alike, `fuzz_log`)
    let (proj, out) = (base.join("a/proj"), base.join("a/out"));
    copy_dir(src, &proj);
    std::fs::create_dir_all(&out).unwrap();
    let host = fh.start(&format!("fz{tag}"), false);
    let scratch = fh.start(&format!("fzs{tag}"), true);
    let selftest: Option<usize> = std::env::var("FLASHTEX_FUZZ_SELFTEST")
        .ok()
        .and_then(|v| v.parse().ok());
    let mut stats = FuzzStats::default();
    let tools = fuzz_tools(&proj, main);
    let request = |id: i64| {
        let mut r = fh.request(id, &proj, &out, main);
        if tools {
            r.external_tools = Some("auto".into());
        }
        r
    };
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut c = Client::connect(&host.1).unwrap();
        let mut view = View::default();
        let mut id = 0;
        let mut status = None;
        for _ in 0..4 {
            id += 1;
            let (mode, o, ran) = fuzz_cycle(&mut c, &mut view, &request(id), tools);
            status = o.done.str_field("status").map(String::from);
            stats.baseline_tools += ran;
            if mode == "unchanged" {
                break;
            }
        }
        // (a document that does not compile would compare only its error
        // log, the same on both sides, at every step; one with tools must
        // have run them)
        if status.as_deref() != Some("ok")
            || view.count == 0
            || (tools && stats.baseline_tools == 0)
        {
            return Err((
                0,
                format!(
                    "the document itself does not compile: status {status:?}, {} pages, {} tool runs ok",
                    view.count, stats.baseline_tools
                ),
            ));
        }
        fuzz_compare(fh, &scratch.1, &view, &base, main, tools).map_err(|e| (0, e))?;
        let files = fuzz_sources(&proj, main);
        let read_all = || -> Vec<(String, String)> {
            files
                .iter()
                .map(|f| (f.clone(), std::fs::read_to_string(proj.join(f)).unwrap()))
                .collect()
        };
        let mut history = vec![read_all()];
        for (k, s) in steps.iter().enumerate() {
            id += 1;
            let mut r = request(id);
            match s {
                FuzzStep::Splice {
                    file,
                    at,
                    delete,
                    insert,
                    wire,
                } => {
                    let text = std::fs::read_to_string(proj.join(file)).unwrap_or_default();
                    // (a shrunk sequence's offsets on another text)
                    let mut at = (*at).min(text.len());
                    while !text.is_char_boundary(at) {
                        at -= 1;
                    }
                    let mut end = (at + delete).min(text.len());
                    while !text.is_char_boundary(end) {
                        end += 1;
                    }
                    if *wire {
                        r.edits = vec![Edit {
                            path: file.clone(),
                            offset: at as u64,
                            delete: (end - at) as u64,
                            insert: insert.clone(),
                        }];
                    } else {
                        let mut t = text.clone();
                        t.replace_range(at..end, insert);
                        std::fs::write(proj.join(file), t).unwrap();
                    }
                }
                FuzzStep::Revert { to } => {
                    for (f, t) in &history[(*to).min(history.len() - 1)] {
                        std::fs::write(proj.join(f), t).unwrap();
                    }
                }
            }
            let (mode, o, ran) = fuzz_cycle(&mut c, &mut view, &r, tools);
            // FLASHTEX_FUZZ_TRACE: each step, its compile's mode and status
            if std::env::var_os("FLASHTEX_FUZZ_TRACE").is_some() {
                let st = o.done.str_field("status").unwrap_or("?");
                eprintln!(
                    "edit fuzz:   {tag} {:>3}: {mode} {st} (tools ok {ran}) {s:?}",
                    k + 1
                );
            }
            let paged =
                !mode.starts_with("cold") && !mode.starts_with("unchanged") && view.count > 0;
            let live = paged && o.done.str_field("status") == Some("ok");
            *stats.modes.entry(mode).or_insert(0) += 1;
            history.push(read_all());
            // FLASHTEX_FUZZ_SELFTEST=N: the harness's own check, a page
            // dropped from the client's view from step N on, which must
            // fail and shrink to N steps
            if selftest.is_some_and(|n| k + 1 >= n) {
                let last = view.count.saturating_sub(1) as u32;
                view.pages.remove(&last);
            }
            fuzz_compare(fh, &scratch.1, &view, &base, main, tools).map_err(|e| (k + 1, e))?;
            stats.paged += paged as usize;
            stats.live += live as usize;
            stats.tools += ran;
        }
        let _ = c.bye();
        stats.tools_wanted = tools;
        eprintln!(
            "edit fuzz:   {tag}: compiles {:?}, {} incremental with pages, {} of them ok, {} tool runs ok",
            stats.modes, stats.paged, stats.live, stats.tools
        );
        Ok(())
    }));
    drop(host);
    drop(scratch);
    let _ = std::fs::remove_dir_all(&base);
    match r {
        Ok(r) => r.map(|()| stats),
        Err(p) => {
            let msg = p
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "a panic".into());
            Err((usize::MAX, format!("panicked: {msg}")))
        }
    }
}

/// A shortest failing sequence found from `steps` (which fail): the prefix
/// to the first failing step, then steps removed one at a time while the
/// replay still fails (at most `budget` replays).
fn fuzz_shrink(
    fh: &dyn FuzzHost,
    src: &Path,
    main: &str,
    steps: Vec<FuzzStep>,
    failed_at: usize,
    mut budget: usize,
) -> (Vec<FuzzStep>, String) {
    let mut cur: Vec<FuzzStep> = steps[..failed_at.min(steps.len())].to_vec();
    let mut why = String::new();
    let mut i = 0;
    while i < cur.len() && budget > 0 {
        let mut trial = cur.clone();
        trial.remove(i);
        // (a revert names a step of the sequence: those after `i` move)
        for s in trial.iter_mut().skip(i) {
            if let FuzzStep::Revert { to } = s {
                if *to > i {
                    *to -= 1;
                }
            }
        }
        budget -= 1;
        match fuzz_replay(fh, src, main, &trial, "shrink") {
            Err((at, e)) => {
                trial.truncate(at.min(trial.len()));
                cur = trial;
                why = e;
            }
            Ok(_) => i += 1,
        }
    }
    if why.is_empty() {
        why = match fuzz_replay(fh, src, main, &cur, "shrink") {
            Err((_, e)) => e,
            Ok(_) => "(the shrunk sequence passed on its last replay: timing-dependent)".into(),
        };
    }
    (cur, why)
}

/// The fuzz on `fh`: the documents of FLASHTEX_FUZZ_DOCS (or `defaults`),
/// found in `roots` (directories of fixture directories with a
/// `main.tex`), for each seed; panics with every seed's and document's
/// shortest failing sequence.
pub fn run(fh: &dyn FuzzHost, roots: &[PathBuf], defaults: &[&str]) {
    let env_n = |k: &str, d: u64| -> u64 {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
    let first = env_n("FLASHTEX_FUZZ_SEED", 1);
    let seeds = env_n("FLASHTEX_FUZZ_SEEDS", 1);
    let steps_n = env_n("FLASHTEX_FUZZ_STEPS", 8) as usize;
    let find = |doc: &str| -> PathBuf {
        roots
            .iter()
            .map(|r| r.join(doc))
            .find(|p| p.join("main.tex").is_file())
            .unwrap_or_else(|| panic!("no fixture {doc}"))
    };
    let docs: Vec<String> = match std::env::var("FLASHTEX_FUZZ_DOCS").ok().as_deref() {
        Some("all") => {
            let mut v: Vec<String> = roots
                .iter()
                .flat_map(|r| std::fs::read_dir(r).unwrap().flatten())
                .filter(|e| e.path().join("main.tex").is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect();
            v.sort();
            v
        }
        Some(list) => list.split(',').map(|s| s.trim().to_string()).collect(),
        None => defaults.iter().map(|s| s.to_string()).collect(),
    };
    // (a dispatch that runs nothing must not pass)
    assert!(
        seeds >= 1 && steps_n >= 1 && !docs.is_empty() && docs.iter().all(|d| !d.is_empty()),
        "edit fuzz: nothing to run (FLASHTEX_FUZZ_SEEDS={seeds}, FLASHTEX_FUZZ_STEPS={steps_n}, FLASHTEX_FUZZ_DOCS={docs:?})"
    );
    for doc in &docs {
        find(doc);
    }
    let mut failures = vec![];
    for seed in first..first + seeds {
        let (failed, mut live) = (failures.len(), 0);
        // (the seed's documents with tools, and their tool runs that
        // succeeded after an edit)
        let (mut tool_docs, mut tools_ok) = (0, 0);
        for (d, doc) in docs.iter().enumerate() {
            let src = find(doc);
            let main = "main.tex";
            // the sequence: generated against the texts as they evolve, from
            // the seed and the document's name (not its place in the list:
            // the rerun line names the document alone)
            let mut rng = Rng::new(seed.wrapping_mul(1_000_003).wrapping_add(fnv1a(doc)));
            let tmp = fresh_dir(&format!("flashtex-fuzz-gen-{d}"));
            let _ = std::fs::remove_dir_all(&tmp);
            copy_dir(&src, &tmp);
            let files = fuzz_sources(&tmp, main);
            let read = |p: &Path| -> Vec<(String, String)> {
                files
                    .iter()
                    .map(|f| (f.clone(), std::fs::read_to_string(p.join(f)).unwrap()))
                    .collect()
            };
            let mut history = vec![read(&tmp)];
            let (mut last, mut labels) = (None, 0);
            let mut steps = vec![];
            for k in 0..steps_n {
                let texts = history.last().unwrap().clone();
                let s = fuzz_step(&mut rng, &texts, main, &mut last, k + 1, &mut labels);
                let next = match &s {
                    FuzzStep::Splice {
                        file,
                        at,
                        delete,
                        insert,
                        ..
                    } => texts
                        .iter()
                        .map(|(f, t)| {
                            let mut t = t.clone();
                            if f == file {
                                t.replace_range(*at..(*at + delete).min(t.len()), insert);
                            }
                            (f.clone(), t)
                        })
                        .collect(),
                    FuzzStep::Revert { to } => history[*to].clone(),
                };
                history.push(next);
                steps.push(s);
            }
            let _ = std::fs::remove_dir_all(&tmp);
            eprintln!("edit fuzz: seed {seed}, {doc}: {} steps", steps.len());
            match fuzz_replay(fh, &src, main, &steps, &format!("{seed}-{d}")) {
                Ok(st) if st.paged == 0 => failures.push(format!(
                    "seed {seed}, {doc} ({}): no step compiled incrementally with pages (compiles {:?}): the sequence tested nothing",
                    src.display(),
                    st.modes
                )),
                Ok(st) => {
                    live += st.live;
                    tool_docs += st.tools_wanted as usize;
                    tools_ok += st.tools;
                }
                Err((0, why)) => failures.push(format!(
                    "seed {seed}, {doc} ({}): before any edit: {why}",
                    src.display()
                )),
                Err((at, why)) => {
                    eprintln!(
                        "edit fuzz: seed {seed}, {doc}: step {at} differs: {why}; shrinking"
                    );
                    let (min, why) = fuzz_shrink(fh, &src, main, steps, at, 40);
                    failures.push(format!(
                        "seed {seed}, {doc} ({}): {why}\n  shortest failing sequence ({} steps):\n{}",
                        src.display(),
                        min.len(),
                        min.iter()
                            .enumerate()
                            .map(|(k, s)| format!("    {}: {s:?}", k + 1))
                            .collect::<Vec<_>>()
                            .join("\n")
                    ));
                }
            }
        }
        if failures.len() == failed && live == 0 {
            failures.push(format!(
                "seed {seed}: no step of {docs:?} compiled incrementally and ok with pages: the seed tested only error states"
            ));
        }
        // (a fuzzed .bib may make bibtex fail, on both sides alike; but a
        // seed whose tool runs all failed did not test the reruns)
        if failures.len() == failed && tool_docs > 0 && tools_ok == 0 {
            failures.push(format!(
                "seed {seed}: no tool run after an edit succeeded in {docs:?}: the reruns after bibtex and makeindex were not tested"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "random edit sequences: a step differs from a scratch compile, or a sequence tested nothing (rerun one: FLASHTEX_FUZZ_SEED=<seed> FLASHTEX_FUZZ_SEEDS=1 FLASHTEX_FUZZ_DOCS=<doc> FLASHTEX_FUZZ_STEPS={steps_n}):\n{}",
        failures.join("\n")
    );
}
