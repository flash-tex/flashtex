//! PROPOSAL `display-list-v2-delta` — producer side of
//! `docs/proposals/display-list-v2-delta.md` (r5). Opt-in next to
//! `display-list-v2`: when the consumer acknowledges the installed base
//! (`payload.display_list_base`) that is also this worker's last emitted
//! sibling, the sibling line is one `display_list_delta` carrying the complete
//! header, a `dl2-canon-1` digest and the exact serialised length of EVERY
//! page, the changed pages in full, and per-document source relocations for
//! the unchanged ones. Nothing here changes the bytes of a full `display_list`
//! line; a consumer that does not request `-delta` is untouched.
//!
//! Invariant (the producer gate, `tests/display_list_delta.rs`): the list a
//! consumer reconstructs from base + delta — unchanged pages relocated exactly
//! as §5.2, changed pages as sent — serialises byte-for-byte to the full
//! `display_list` line a fresh compile would emit.

use std::cell::RefCell;

use flashtex_compiler::json::{self, Value};
use flashtex_font_engine::sha256;

use crate::display::{self, DisplayList, Item, LineCap, LineJoin, Page, PathCmd, PathPaintOp, Provenance, Severity, SourceRange, Wire};

pub const CAP: &str = "display-list-v2-delta";
pub const MESSAGE_TYPE: &str = "display_list_delta";
pub const DIGEST_SCHEME: &str = "dl2-canon-1";
/// Retention caps (§6.2), mirrored by the Mac consumer.
pub const MAX_SNAPSHOT_PAGES: usize = 1024;
pub const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_RETAINED_TEXT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RETAINED_TOTAL_TEXT_BYTES: usize = 32 * 1024 * 1024;
/// Policy (§3): a delta is emitted only when it is at most 3/4 of the full line.
pub const SIZE_POLICY: (usize, usize) = (3, 4);

// ---------------------------------------------------------------- dl2-canon-1

/// The canonical binary encoding of §5.1: `i64` little-endian integers and
/// counts, length-prefixed UTF-8 strings, IEEE-754 binary64 bit patterns with
/// −0.0 normalised to +0.0, domain-separated prefixes.
struct Canon(Vec<u8>);

impl Canon {
    fn new(prefix: &str) -> Canon {
        let mut v = Vec::with_capacity(4096);
        v.extend_from_slice(prefix.as_bytes());
        v.push(0);
        Canon(v)
    }
    fn i(&mut self, n: i64) {
        self.0.extend_from_slice(&n.to_le_bytes());
    }
    fn u(&mut self, n: usize) {
        self.i(n as i64);
    }
    fn t(&mut self, t: display::Tick) {
        self.i(t.0);
    }
    fn s(&mut self, s: &str) {
        self.u(s.len());
        self.0.extend_from_slice(s.as_bytes());
    }
    fn f(&mut self, x: f64) {
        let v = if x == 0.0 { 0.0 } else { x };
        self.0.extend_from_slice(&v.to_bits().to_le_bytes());
    }
    fn ranges(&mut self, rs: &[SourceRange]) {
        self.u(rs.len());
        for r in rs {
            self.s(&r.path);
            self.u(r.start_byte);
            self.u(r.end_byte);
        }
    }
    fn provenance(&mut self, p: &Provenance) {
        match p {
            Provenance::Source(_) | Provenance::Sources(_) => {
                self.0.push(0x10);
                self.ranges(p.sources());
            }
            Provenance::Synthetic(reason) => {
                self.0.push(0x11);
                self.s(reason);
            }
        }
    }
    fn paint(&mut self, p: &display::Paint) {
        self.f(p.r);
        self.f(p.g);
        self.f(p.b);
        self.f(p.a);
    }
    fn commands(&mut self, cmds: &[PathCmd]) {
        self.u(cmds.len());
        for c in cmds {
            match c {
                PathCmd::Move(x, y) => {
                    self.0.push(0x6D);
                    self.t(*x);
                    self.t(*y);
                }
                PathCmd::Line(x, y) => {
                    self.0.push(0x6C);
                    self.t(*x);
                    self.t(*y);
                }
                PathCmd::Cubic(a, b, c, d, e, f) => {
                    self.0.push(0x63);
                    for t in [a, b, c, d, e, f] {
                        self.t(*t);
                    }
                }
                PathCmd::Close => self.0.push(0x7A),
            }
        }
    }
    fn sha(&self) -> [u8; 32] {
        sha256::digest(&self.0)
    }
}

/// `dl2-canon-1` digest of one page as serialised under `wire` (image items
/// are on the wire only when negotiated, so they are digested only then).
pub fn page_digest(p: &Page, wire: Wire) -> [u8; 32] {
    let mut c = Canon::new("flashtex:dl2:page:1");
    c.u(p.number as usize);
    c.t(p.width);
    c.t(p.height);
    // The filter reads the unplaced variant (placement never changes which
    // variant an item is), so only the digested items are materialised.
    let on_wire = |it: &&display::PageItem| wire.images || !matches!(it.unplaced(), Item::Image(_));
    c.u(p.placed.iter().filter(on_wire).count());
    for it in p.placed.iter().filter(on_wire).map(|it| p.place(it)) {
        match &*it {
            Item::GlyphRun(r) => {
                c.0.push(0x01);
                c.s(&r.font_id);
                c.t(r.font_size);
                c.s(&r.text);
                c.u(r.glyphs.len());
                for g in &r.glyphs {
                    c.u(g.gid as usize);
                    c.t(g.origin_x);
                    c.t(g.baseline_y);
                    c.t(g.advance_x);
                    c.t(g.advance_y);
                    c.u(g.cluster as usize);
                }
                c.u(r.clusters.len());
                for (ci, cl) in r.clusters.iter().enumerate() {
                    c.u(cl.text_start_byte);
                    c.u(cl.text_end_byte);
                    let rects = cl.hit_rects();
                    c.u(rects.len());
                    for h in rects {
                        c.t(h.x);
                        c.t(h.top);
                        c.t(h.width);
                        c.t(h.height);
                    }
                    let carets = r.carets_of(ci);
                    c.u(carets.len());
                    for k in carets.iter() {
                        c.u(k.text_byte);
                        c.t(k.x);
                        c.t(k.top);
                        c.t(k.height);
                    }
                    c.provenance(&cl.provenance);
                }
                c.paint(&r.paint);
            }
            Item::Rule(r) => {
                c.0.push(0x02);
                c.t(r.x);
                c.t(r.top);
                c.t(r.width);
                c.t(r.height);
                c.paint(&r.paint);
                c.provenance(&r.provenance);
            }
            Item::Image(i) => {
                c.0.push(0x03);
                c.t(i.x);
                c.t(i.top);
                c.t(i.width);
                c.t(i.height);
                c.u(i.transform.len());
                for v in i.transform {
                    // The wire value (rounded to thousandths), printed as the
                    // consumer's `String(format: "%.3f")` prints it.
                    let wire_value = (v * 1000.0).round() / 1000.0 + 0.0;
                    c.s(&format!("{wire_value:.3}"));
                }
                c.s(&i.resource.sha256);
                c.s(&i.resource.path);
                c.u(if i.resource.pdf_box.is_some() { i.resource.pdf_page as usize } else { 0 });
                c.provenance(&i.provenance);
            }
            Item::Path(p) => {
                c.0.push(0x04);
                match &p.op {
                    PathPaintOp::Fill { even_odd } => {
                        c.0.push(0x01);
                        c.s(if *even_odd { "evenodd" } else { "nonzero" });
                    }
                    PathPaintOp::Stroke(s) => {
                        c.0.push(0x02);
                        c.t(s.width);
                        c.s(match s.cap {
                            LineCap::Butt => "butt",
                            LineCap::Round => "round",
                            LineCap::Square => "square",
                        });
                        c.s(match s.join {
                            LineJoin::Miter => "miter",
                            LineJoin::Round => "round",
                            LineJoin::Bevel => "bevel",
                        });
                        c.f(s.miter_limit);
                        if s.dash.is_empty() {
                            c.u(0);
                        } else {
                            c.u(s.dash.len());
                            for t in &s.dash {
                                c.t(*t);
                            }
                            c.t(s.dash_phase);
                        }
                    }
                }
                c.commands(&p.commands);
                c.u(p.clips.len());
                for clip in &p.clips {
                    c.s(if clip.even_odd { "evenodd" } else { "nonzero" });
                    c.commands(&clip.commands);
                }
                c.paint(&p.paint);
                c.provenance(&p.provenance);
            }
        }
    }
    c.sha()
}

/// `dl2-canon-1` header digest (identity, features, documents, fonts, diagnostics).
pub fn header_digest(l: &DisplayList, wire: Wire) -> [u8; 32] {
    let mut c = Canon::new("flashtex:dl2:header:1");
    for s in ["display-list-v2", "bp_2pow20", "srgb", "cluster-actualtext", l.project_id.as_str()] {
        c.s(s);
    }
    c.u(l.revision as usize);
    let features = l.required_features_wire(wire);
    c.u(features.len());
    for f in features {
        c.s(f);
    }
    c.u(l.documents.len());
    for d in &l.documents {
        c.s(&d.path);
        c.u(d.revision as usize);
        c.s(&d.sha256);
        c.i(d.byte_length as i64);
    }
    c.u(l.fonts.len());
    for f in &l.fonts {
        c.s(&f.font_id);
        c.s(&f.sha256);
        c.i(f.byte_length as i64);
        c.s(&f.format);
        c.u(f.face_index as usize);
        c.u(f.units_per_em as usize);
        c.u(f.glyph_count as usize);
        c.s(&f.postscript_name);
    }
    c.u(l.diagnostics.len());
    for d in &l.diagnostics {
        c.s(&d.code);
        c.s(&d.message);
        c.s(match d.severity {
            Severity::Warning => "warning",
            Severity::Error => "error",
        });
        c.ranges(&d.sources);
    }
    c.sha()
}

pub fn list_digest(l: &DisplayList, wire: Wire, page_digests: &[[u8; 32]]) -> [u8; 32] {
    let mut c = Canon::new("flashtex:dl2:list:1");
    c.0.extend_from_slice(&header_digest(l, wire));
    c.u(page_digests.len());
    for d in page_digests {
        c.0.extend_from_slice(d);
    }
    c.sha()
}

// ---------------------------------------------------------------- relocation

/// One document's edit in the BASE text's byte coordinates (§5.2): `[edit_start,
/// edit_end)` was replaced by `[edit_start, edit_end + delta)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relocation {
    pub path: String,
    pub edit_start: usize,
    pub edit_end: usize,
    pub delta: isize,
}

impl Relocation {
    /// The common-prefix/common-suffix diff of `old` → `new`.
    pub fn diff(path: &str, old: &str, new: &str) -> Option<Relocation> {
        if old == new {
            return None;
        }
        let (a, b) = (old.as_bytes(), new.as_bytes());
        let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
        let max_suffix = a.len().min(b.len()) - prefix;
        let suffix = a.iter().rev().zip(b.iter().rev()).take(max_suffix).take_while(|(x, y)| x == y).count();
        Some(Relocation {
            path: path.to_string(),
            edit_start: prefix,
            edit_end: a.len() - suffix,
            delta: b.len() as isize - a.len() as isize,
        })
    }

    /// The moved range, or `None` when it intersects the edited region.
    pub fn apply(&self, r: &SourceRange) -> Option<SourceRange> {
        if r.end_byte <= self.edit_start {
            Some(r.clone())
        } else if r.start_byte >= self.edit_end {
            let s = r.start_byte as isize + self.delta;
            let e = r.end_byte as isize + self.delta;
            (s >= 0 && e >= s).then(|| SourceRange { path: r.path.clone(), start_byte: s as usize, end_byte: e as usize })
        } else {
            None
        }
    }
}

fn digits(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        n.ilog10() as usize + 1
    }
}

/// Lockstep comparison of a base page's provenance against a new page's:
/// equal after relocation. Accumulates the decimal-width change of the moved
/// offsets (the exact `page_bytes` arithmetic of §6.2) in `width_delta`.
fn ranges_match(base: &[SourceRange], new: &[SourceRange], relocs: &[Relocation], width_delta: &mut isize) -> bool {
    if base.len() != new.len() {
        return false;
    }
    for (b, n) in base.iter().zip(new) {
        if b.path != n.path {
            return false;
        }
        match relocs.iter().find(|r| *r.path == *b.path) {
            None => {
                if b != n {
                    return false;
                }
            }
            Some(rl) => match rl.apply(b) {
                Some(moved) if moved == *n => {
                    *width_delta += digits(n.start_byte) as isize - digits(b.start_byte) as isize;
                    *width_delta += digits(n.end_byte) as isize - digits(b.end_byte) as isize;
                }
                _ => return false,
            },
        }
    }
    true
}

fn provenance_matches(base: &Provenance, new: &Provenance, relocs: &[Relocation], width_delta: &mut isize) -> bool {
    match (base, new) {
        (Provenance::Synthetic(a), Provenance::Synthetic(b)) => a == b,
        (Provenance::Synthetic(_), _) | (_, Provenance::Synthetic(_)) => false,
        _ => ranges_match(base.sources(), new.sources(), relocs, width_delta),
    }
}

/// Whether `new` is exactly `base` with every source span moved by `relocs`
/// (§5.2, as serialised under `wire`); returns the decimal-width change of
/// the moved offsets, i.e. `page_bytes(new) − page_bytes(base)`.
pub fn unchanged_after_relocation(base: &Page, new: &Page, relocs: &[Relocation], wire: Wire) -> Option<isize> {
    if base.number != new.number || base.width != new.width || base.height != new.height {
        return None;
    }
    let on_wire = |it: &&display::PageItem| wire.images || !matches!(it.unplaced(), Item::Image(_));
    fn materialise<'a>(p: &'a Page, on_wire: impl Fn(&&display::PageItem) -> bool) -> Vec<std::borrow::Cow<'a, Item>> {
        p.placed.iter().filter(on_wire).map(|it| p.place(it)).collect()
    }
    let (bi, ni) = (materialise(base, on_wire), materialise(new, on_wire));
    if bi.len() != ni.len() {
        return None;
    }
    let mut width_delta = 0isize;
    for (b, n) in bi.iter().zip(ni.iter()) {
        let ok = match (&**b, &**n) {
            (Item::GlyphRun(x), Item::GlyphRun(y)) => {
                x.font_id == y.font_id
                    && x.font_size == y.font_size
                    && x.text == y.text
                    && x.glyphs == y.glyphs
                    && x.paint == y.paint
                    // The carets derive from the cluster bytes, the hit rect
                    // and the run's end caret, all compared here, so this is
                    // the same comparison the per-cluster `carets` made.
                    && x.end_caret == y.end_caret
                    && x.clusters.len() == y.clusters.len()
                    && x.clusters.iter().zip(&y.clusters).all(|(c, d)| {
                        c.text_start_byte == d.text_start_byte
                            && c.text_end_byte == d.text_end_byte
                            && c.hit_rect == d.hit_rect
                            && provenance_matches(&c.provenance, &d.provenance, relocs, &mut width_delta)
                    })
            }
            (Item::Rule(x), Item::Rule(y)) => {
                x.x == y.x && x.top == y.top && x.width == y.width && x.height == y.height && x.paint == y.paint && provenance_matches(&x.provenance, &y.provenance, relocs, &mut width_delta)
            }
            (Item::Image(x), Item::Image(y)) => {
                x.x == y.x && x.top == y.top && x.width == y.width && x.height == y.height && x.transform == y.transform && x.resource == y.resource && provenance_matches(&x.provenance, &y.provenance, relocs, &mut width_delta)
            }
            (Item::Path(x), Item::Path(y)) => x.op == y.op && x.commands == y.commands && x.clips == y.clips && x.paint == y.paint && provenance_matches(&x.provenance, &y.provenance, relocs, &mut width_delta),
            _ => false,
        };
        if !ok {
            return None;
        }
    }
    Some(width_delta)
}

/// A NEW page equal to `base` with every source span moved (the consumer's
/// reconstruction, used by the producer gate); `None` when a span intersects
/// the edited region.
pub fn relocate_page(base: &Page, relocs: &[Relocation]) -> Option<Page> {
    fn prov(p: &Provenance, relocs: &[Relocation]) -> Option<Provenance> {
        match p {
            Provenance::Synthetic(_) => Some(p.clone()),
            _ => {
                let mut out = Vec::with_capacity(p.sources().len());
                for r in p.sources() {
                    out.push(match relocs.iter().find(|rl| *rl.path == *r.path) {
                        Some(rl) => rl.apply(r)?,
                        None => r.clone(),
                    });
                }
                Some(match p {
                    Provenance::Source(_) if out.len() == 1 => Provenance::Source(out.pop().unwrap()),
                    _ => Provenance::Sources(out),
                })
            }
        }
    }
    let mut items = Vec::with_capacity(base.item_count());
    for it in base.items() {
        items.push(match &*it {
            Item::GlyphRun(r) => {
                let mut r = r.clone();
                for c in &mut r.clusters {
                    c.provenance = prov(&c.provenance, relocs)?;
                }
                Item::GlyphRun(r)
            }
            Item::Rule(r) => Item::Rule(display::Rule { provenance: prov(&r.provenance, relocs)?, ..r.clone() }),
            Item::Image(i) => Item::Image(display::Image { provenance: prov(&i.provenance, relocs)?, ..i.clone() }),
            Item::Path(p) => Item::Path(display::PathItem { provenance: prov(&p.provenance, relocs)?, ..p.clone() }),
        });
    }
    // `base.items()` already applied the default colour, so the relocated
    // page carries the paints outright and needs no default of its own.
    Some(Page::from_items(base.number, base.width, base.height, items))
}

// ---------------------------------------------------------------- snapshots

/// The consumer's installed-base acknowledgement (`payload.display_list_base`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base {
    pub request_id: String,
    pub project_id: String,
    pub revision: u64,
    pub page_count: usize,
    pub list_digest: String,
}

impl Base {
    /// Reads a well-formed acknowledgement; anything else is `None` (→ full).
    pub fn from_json(v: &Value) -> Option<Base> {
        let digest = v.get("list_digest")?.as_str()?;
        if digest.len() != 64 || !digest.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return None;
        }
        Some(Base {
            request_id: v.get("request_id")?.as_str()?.to_string(),
            project_id: v.get("project_id")?.as_str()?.to_string(),
            revision: u64::try_from(v.get("revision")?.as_i64()?).ok()?,
            page_count: usize::try_from(v.get("page_count")?.as_i64()?).ok()?,
            list_digest: digest.to_string(),
        })
    }
}

/// The producer's last emitted sibling: the model it would relocate, the
/// exact per-page wire lengths, its digests, and the request texts the
/// relocations are diffed against.
pub struct Snapshot {
    pub request_id: String,
    pub project_id: String,
    pub revision: u64,
    pub wire: Wire,
    pub pages: Vec<Page>,
    pub page_bytes: Vec<usize>,
    pub page_digests: Vec<[u8; 32]>,
    pub list_digest: [u8; 32],
    /// `(path, text)` of every request document.
    pub documents: Vec<(String, String)>,
}

impl Snapshot {
    pub fn acknowledgement(&self) -> Base {
        Base {
            request_id: self.request_id.clone(),
            project_id: self.project_id.clone(),
            revision: self.revision,
            page_count: self.pages.len(),
            list_digest: sha256::hex(&self.list_digest),
        }
    }
}

/// Per-worker delta state: at most one snapshot (§6.2: old + new, the new one
/// replacing the old only after its line is emitted).
#[derive(Default)]
pub struct DeltaState {
    snapshot: RefCell<Option<Snapshot>>,
}

impl DeltaState {
    pub fn new() -> DeltaState {
        DeltaState::default()
    }
    pub fn clear(&self) {
        *self.snapshot.borrow_mut() = None;
    }
    pub fn has_snapshot(&self) -> bool {
        self.snapshot.borrow().is_some()
    }
    pub fn acknowledgement(&self) -> Option<Base> {
        self.snapshot.borrow().as_ref().map(Snapshot::acknowledgement)
    }
    fn install(&self, s: Snapshot) {
        *self.snapshot.borrow_mut() = Some(s);
    }
}

fn texts_retainable(documents: &[(String, String)]) -> bool {
    documents.iter().all(|(_, t)| t.len() <= MAX_RETAINED_TEXT_BYTES) && documents.iter().map(|(_, t)| t.len()).sum::<usize>() <= MAX_RETAINED_TOTAL_TEXT_BYTES
}

/// The sibling line the producer emits for one request, with what to snapshot.
pub struct Sibling {
    pub line: String,
    pub is_delta: bool,
}

/// Records `list` (already emitted as a full `display_list` line whose page
/// objects measured `page_bytes`) as the base for the next request. Not
/// retained when over the caps: the next request then answers full.
pub fn note_full(state: &DeltaState, id: &str, list: &DisplayList, wire: Wire, page_bytes: Vec<usize>, line_len: usize, documents: &[(String, String)]) {
    if list.pages.len() > MAX_SNAPSHOT_PAGES || line_len > MAX_SNAPSHOT_BYTES || !texts_retainable(documents) {
        state.clear();
        return;
    }
    let page_digests: Vec<[u8; 32]> = list.pages.iter().map(|p| page_digest(p, wire)).collect();
    let list_digest = list_digest(list, wire, &page_digests);
    state.install(Snapshot {
        request_id: id.to_string(),
        project_id: list.project_id.clone(),
        revision: list.revision,
        wire,
        pages: list.pages.clone(),
        page_bytes,
        page_digests,
        list_digest,
        documents: documents.to_vec(),
    });
}

/// Tries to emit a delta for `list` against the snapshot `base` names (§3
/// decision order). `Some(line)` when accepted — the snapshot is then
/// replaced by `list`'s; `None` means "answer full" and leaves the snapshot
/// untouched (the caller's full line, once emitted, replaces it via
/// [`note_full`]). `full_len_hint` bounds the policy comparison when the
/// caller has not serialised the full line.
pub fn try_delta(state: &DeltaState, id: &str, list: &DisplayList, wire: Wire, base: &Base, documents: &[(String, String)], limit: usize) -> Option<String> {
    let snapshot = state.snapshot.borrow();
    let snap = snapshot.as_ref()?;
    if snap.acknowledgement() != *base || snap.wire != wire || snap.project_id != list.project_id {
        return None;
    }
    if list.pages.len() > MAX_SNAPSHOT_PAGES || !texts_retainable(documents) {
        return None;
    }
    // Same document set (paths), in any order.
    let mut old_paths: Vec<&str> = snap.documents.iter().map(|(p, _)| p.as_str()).collect();
    let mut new_paths: Vec<&str> = documents.iter().map(|(p, _)| p.as_str()).collect();
    old_paths.sort_unstable();
    new_paths.sort_unstable();
    if old_paths != new_paths {
        return None;
    }
    let relocations: Vec<Relocation> = documents
        .iter()
        .filter_map(|(p, new)| {
            let old = &snap.documents.iter().find(|(q, _)| q == p)?.1;
            Relocation::diff(p, old, new)
        })
        .collect();

    // Classify pages and measure them (§4 rule 2b) before writing anything.
    let n = list.pages.len();
    let mut page_bytes = Vec::with_capacity(n);
    let mut changed: Vec<(usize, String)> = Vec::new();
    for (i, page) in list.pages.iter().enumerate() {
        let reuse = snap.pages.get(i).and_then(|b| unchanged_after_relocation(b, page, &relocations, wire));
        match reuse {
            Some(width_delta) => page_bytes.push((snap.page_bytes[i] as isize + width_delta) as usize),
            None => {
                let mut o = String::with_capacity(4096);
                display::write_page(&mut o, page, wire);
                page_bytes.push(o.len());
                changed.push((i, o));
            }
        }
    }
    let page_digests: Vec<[u8; 32]> = list.pages.iter().map(|p| page_digest(p, wire)).collect();
    let new_list_digest = list_digest(list, wire, &page_digests);

    // The delta line. The header parts are written by the full writer's own
    // functions, so their bytes (and lengths) are those of the full line.
    let mut o = String::with_capacity(4096 + changed.iter().map(|(_, s)| s.len()).sum::<usize>());
    o.push_str("{\"id\":");
    let id_start = o.len();
    json::write_string_into(id, &mut o);
    let id_len = o.len() - id_start;
    o.push_str(",\"payload\":{\"base\":{\"list_digest\":");
    json::write_string_into(&base.list_digest, &mut o);
    o.push_str(",\"page_count\":");
    json::write_number_into(base.page_count as f64, &mut o);
    o.push_str(",\"project_id\":");
    json::write_string_into(&base.project_id, &mut o);
    o.push_str(",\"request_id\":");
    json::write_string_into(&base.request_id, &mut o);
    o.push_str(",\"revision\":");
    json::write_number_into(base.revision as f64, &mut o);
    o.push_str("},\"changed_pages\":[");
    for (k, (_, page)) in changed.iter().enumerate() {
        if k > 0 {
            o.push(',');
        }
        o.push_str(page);
    }
    o.push_str("],\"color_space\":\"srgb\",\"coordinate_unit\":\"bp_2pow20\",\"diagnostics\":");
    let mut header_len = 0;
    let start = o.len();
    display::write_diagnostics(&mut o, &list.diagnostics);
    header_len += o.len() - start;
    o.push_str(",\"digest_scheme\":\"dl2-canon-1\",\"documents\":");
    let start = o.len();
    display::write_documents(&mut o, &list.documents);
    header_len += o.len() - start;
    o.push_str(",\"fonts\":");
    let start = o.len();
    display::write_fonts(&mut o, &list.fonts);
    header_len += o.len() - start;
    o.push_str(",\"list_digest\":");
    json::write_string_into(&sha256::hex(&new_list_digest), &mut o);
    o.push_str(",\"page_bytes\":[");
    for (k, b) in page_bytes.iter().enumerate() {
        if k > 0 {
            o.push(',');
        }
        json::write_number_into(*b as f64, &mut o);
    }
    o.push_str("],\"page_count\":");
    json::write_number_into(n as f64, &mut o);
    o.push_str(",\"page_digests\":[");
    for (k, d) in page_digests.iter().enumerate() {
        if k > 0 {
            o.push(',');
        }
        json::write_string_into(&sha256::hex(d), &mut o);
    }
    o.push_str("],\"project_id\":");
    let start = o.len();
    json::write_string_into(&list.project_id, &mut o);
    header_len += o.len() - start;
    o.push_str(",\"relocations\":[");
    for (k, r) in relocations.iter().enumerate() {
        if k > 0 {
            o.push(',');
        }
        o.push_str("{\"delta\":");
        json::write_number_into(r.delta as f64, &mut o);
        o.push_str(",\"edit_end\":");
        json::write_number_into(r.edit_end as f64, &mut o);
        o.push_str(",\"edit_start\":");
        json::write_number_into(r.edit_start as f64, &mut o);
        o.push_str(",\"path\":");
        json::write_string_into(&r.path, &mut o);
        o.push('}');
    }
    o.push_str("],\"removed_pages\":[");
    for (k, num) in (n + 1..=snap.pages.len()).enumerate() {
        if k > 0 {
            o.push(',');
        }
        json::write_number_into(num as f64, &mut o);
    }
    o.push_str("],\"render_format\":\"display-list-v2\",\"required_features\":");
    let start = o.len();
    display::write_features(&mut o, list, wire);
    header_len += o.len() - start;
    o.push_str(",\"revision\":");
    let start = o.len();
    json::write_number_into(list.revision as f64, &mut o);
    header_len += o.len() - start;
    o.push_str(",\"text_extraction\":\"cluster-actualtext\"},\"protocol_version\":2,\"type\":\"display_list_delta\"}");

    // The exact size of the full line this delta stands for (the consumer's
    // `fullLineBytes`): fixed framing + measured header parts + Σ page_bytes
    // + page separators.
    let full_len = display::FULL_LINE_FRAME_BYTES + id_len + header_len + page_bytes.iter().sum::<usize>() + n.saturating_sub(1);
    if o.len() > limit || full_len > MAX_SNAPSHOT_BYTES || o.len() * SIZE_POLICY.1 > full_len * SIZE_POLICY.0 {
        return None;
    }
    drop(snapshot);
    state.install(Snapshot {
        request_id: id.to_string(),
        project_id: list.project_id.clone(),
        revision: list.revision,
        wire,
        pages: list.pages.clone(),
        page_bytes,
        page_digests,
        list_digest: new_list_digest,
        documents: documents.to_vec(),
    });
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_is_prefix_suffix() {
        let r = Relocation::diff("m", "hello world", "hello brave world").unwrap();
        assert_eq!((r.edit_start, r.edit_end, r.delta), (6, 6, 6));
        let r = Relocation::diff("m", "aaaa", "aa").unwrap();
        assert_eq!((r.edit_start, r.edit_end, r.delta), (2, 4, -2));
        assert!(Relocation::diff("m", "x", "x").is_none());
        let r = Relocation::diff("m", "abc", "abd").unwrap();
        assert_eq!((r.edit_start, r.edit_end, r.delta), (2, 3, 0));
    }

    #[test]
    fn apply_moves_or_refuses() {
        let rl = Relocation { path: "m".into(), edit_start: 10, edit_end: 12, delta: 3 };
        let sr = |s, e| SourceRange { path: std::rc::Rc::from("m"), start_byte: s, end_byte: e };
        assert_eq!(rl.apply(&sr(0, 10)), Some(sr(0, 10)));
        assert_eq!(rl.apply(&sr(12, 20)), Some(sr(15, 23)));
        assert_eq!(rl.apply(&sr(9, 11)), None);
        assert_eq!(rl.apply(&sr(11, 13)), None);
    }

    #[test]
    fn digits_counts_decimal_width() {
        assert_eq!(digits(0), 1);
        assert_eq!(digits(9), 1);
        assert_eq!(digits(10), 2);
        assert_eq!(digits(999), 3);
        assert_eq!(digits(1000), 4);
    }

    #[test]
    fn canon_normalises_negative_zero() {
        let mut a = Canon::new("t");
        a.f(-0.0);
        let mut b = Canon::new("t");
        b.f(0.0);
        assert_eq!(a.0, b.0);
    }
}
