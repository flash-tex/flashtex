//! The state an incremental client holds, from the host's frames (spec §5,
//! §6.4), with every id resolved to what it named when its page arrived:
//! a reference for tests that hold a host's frames to a fresh run's (lane
//! AUDIT-DL; `examples/dl3-audit.rs`, the hosts' audit tests).
//!
//! `STARTED` with `"keep": false` drops everything; a `PAGE` replaces its
//! index, its fonts and images bound as they are then; a `FORM` replaces its
//! id (a page draws the form its id names now: spec §5, with `keep`, one id
//! is one PDF object); `PAGES` (complete) and a `DONE` that is not
//! `cancelled` drop the pages at or past their count. What the frames break
//! of the spec's ordering (a font, image, span or form used before it was
//! sent) is collected in `violations`.

use crate::client::Event;
use crate::json::Json;
use crate::page::{Item, Page};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;

/// A page or form as the client holds it: ids bound when it arrived.
#[derive(Clone)]
pub struct HeldPage {
    pub page: Page,
    /// Font id -> key, as bound when the page arrived.
    pub fonts: HashMap<u16, [u8; 32]>,
    /// Image id -> key (hex), as bound when the page arrived.
    pub images: HashMap<u32, String>,
}

/// What an incremental client holds (spec §5, §6.4), built from the
/// host's frames as they arrive.
#[derive(Default)]
pub struct Model {
    pub pages: BTreeMap<u32, HeldPage>,
    pub forms: HashMap<u32, HeldPage>,
    pub fonts: HashMap<u16, [u8; 32]>,
    pub images: HashMap<u32, String>,
    pub spans: HashMap<u32, (u32, u32)>,
    pub files: HashMap<u32, String>,
    /// Form ids pages drew whose FORM has not come (due before the next
    /// PAGE or DONE, spec §5).
    owed: Vec<(u32, u32)>,
    pub count: usize,
    pub violations: Vec<String>,
    /// This compile's: keep, pages sent.
    pub keep: Option<bool>,
    pub sent: Vec<u32>,
}

fn hex(k: &[u8]) -> String {
    k.iter().map(|b| format!("{b:02x}")).collect()
}

impl Model {
    fn bind(&mut self, p: &Page, what: &str) -> HeldPage {
        let mut fonts = HashMap::new();
        let mut images = HashMap::new();
        for it in &p.items {
            match it {
                Item::Glyph { font, .. } => match self.fonts.get(font) {
                    Some(k) => {
                        fonts.insert(*font, *k);
                    }
                    None => self
                        .violations
                        .push(format!("{what} {}: font {font} not sent", p.index)),
                },
                Item::Image { id, .. } => match self.images.get(id) {
                    Some(k) => {
                        images.insert(*id, k.clone());
                    }
                    None => self
                        .violations
                        .push(format!("{what} {}: image {id} not sent", p.index)),
                },
                Item::Span(s) if *s != 0 && !self.spans.contains_key(s) => self
                    .violations
                    .push(format!("{what} {}: span {s} not sent", p.index)),
                _ => {}
            }
        }
        for l in &p.links {
            if l.span != 0 && !self.spans.contains_key(&l.span) {
                self.violations
                    .push(format!("{what} {}: link span {} not sent", p.index, l.span));
            }
        }
        HeldPage {
            page: p.clone(),
            fonts,
            images,
        }
    }

    fn pay(&mut self, at: &str) {
        let owed = std::mem::take(&mut self.owed);
        for (page, id) in owed {
            if !self.forms.contains_key(&id) {
                self.violations
                    .push(format!("page {page} draws form {id}, not sent by {at}"));
            }
        }
    }

    /// One event; returns the DONE.
    pub fn apply(&mut self, ev: Event) -> Option<Json> {
        match ev {
            Event::Started(j) => {
                let keep = j.get("keep").and_then(Json::as_bool) == Some(true);
                if !keep {
                    let v = std::mem::take(&mut self.violations);
                    *self = Model::default();
                    self.violations = v;
                }
                self.keep = Some(keep);
                self.sent.clear();
            }
            Event::Font(f) => {
                self.fonts.insert(f.id, f.key);
            }
            Event::Image(j) => {
                let id = j.int_field("id").unwrap_or(-1) as u32;
                let key = j.str_field("key").unwrap_or("").to_string();
                self.images.insert(id, key);
            }
            Event::Sources(s) => {
                for (i, p) in s.files {
                    self.files.insert(i, p);
                }
                for (i, f, l) in s.spans {
                    if !self.files.contains_key(&f) {
                        self.violations
                            .push(format!("span {i} names file {f}, not sent"));
                    }
                    self.spans.insert(i, (f, l));
                }
            }
            Event::Page(p) => {
                self.pay(&format!("page {}", p.index));
                let h = self.bind(&p, "page");
                for it in &p.items {
                    if let Item::Form { id, .. } = it {
                        self.owed.push((p.index, *id));
                    }
                }
                self.sent.push(p.index);
                self.pages.insert(p.index, h);
            }
            Event::Form(f) => {
                let h = self.bind(&f, "form");
                self.forms.insert(f.index, h);
            }
            Event::Pages(j) => {
                if j.get("complete").and_then(Json::as_bool) == Some(true) {
                    if let Some(n) = j.int_field("count") {
                        self.pages.retain(|&i, _| (i as i64) < n);
                    }
                }
            }
            Event::Done(d) => {
                self.pay("DONE");
                if d.str_field("status") != Some("cancelled") {
                    let n = d.int_field("pages").unwrap_or(0) as usize;
                    self.count = n;
                    self.pages.retain(|&i, _| (i as usize) < n);
                }
                return Some(d);
            }
            Event::Error(e) => self.violations.push(format!("host ERROR {e}")),
            _ => {}
        }
        None
    }

    /// Page `i` with every id resolved, as text (one line per part).
    pub fn resolve(&self, i: u32, out: &str) -> String {
        match self.pages.get(&i) {
            None => format!("page {i} missing\n"),
            Some(h) => {
                let mut s = String::new();
                self.render(h, out, &mut s, 0, &mut HashSet::new());
                s
            }
        }
    }

    fn loc(&self, span: u32, out: &str) -> String {
        if span == 0 {
            return "-".into();
        }
        match self.spans.get(&span) {
            None => format!("span{span}?"),
            Some((f, l)) => {
                let p = self
                    .files
                    .get(f)
                    .cloned()
                    .unwrap_or_else(|| format!("file{f}?"));
                format!("{}:{l}", p.replace(out, "<out>"))
            }
        }
    }

    fn render(
        &self,
        h: &HeldPage,
        out: &str,
        s: &mut String,
        depth: usize,
        seen: &mut HashSet<u32>,
    ) {
        let p = &h.page;
        let pad = "  ".repeat(depth);
        let _ = writeln!(
            s,
            "{pad}{:?} {} flags={} {}x{} counts={:?} box={:?} hash={}",
            p.kind,
            p.index,
            p.flags,
            p.width,
            p.height,
            p.counts,
            p.pdf_box,
            hex(&p.hash)
        );
        let _ = writeln!(s, "{pad}matrices {:?}", p.matrices);
        let _ = writeln!(s, "{pad}paths {:?}", p.paths);
        let _ = writeln!(s, "{pad}unsupported {:?} meta {:?}", p.unsupported, p.meta);
        let _ = writeln!(s, "{pad}origins {:?}", p.origins);
        let _ = writeln!(s, "{pad}rules {:?}", p.rule_geometry);
        for it in &p.items {
            match it {
                Item::Glyph {
                    font,
                    code,
                    x,
                    y,
                    col,
                } => {
                    let k = h
                        .fonts
                        .get(font)
                        .map(|k| hex(&k[..8]))
                        .unwrap_or("?".into());
                    let _ = writeln!(s, "{pad}g {k} {code} {x} {y} {col}");
                }
                Item::Image { id, matrix } => {
                    let k = h.images.get(id).cloned().unwrap_or("?".into());
                    let _ = writeln!(s, "{pad}img {k} m{matrix}");
                }
                Item::Span(n) => {
                    let _ = writeln!(s, "{pad}span {}", self.loc(*n, out));
                }
                Item::Form { id, matrix } => {
                    let _ = writeln!(s, "{pad}form m{matrix}:");
                    match self.forms.get(id) {
                        Some(f) if depth < 8 && seen.insert(*id) => {
                            self.render(f, out, s, depth + 1, seen);
                            seen.remove(id);
                        }
                        Some(_) => {
                            let _ = writeln!(s, "{pad}  (nested)");
                        }
                        None => {
                            let _ = writeln!(s, "{pad}  form {id} missing");
                        }
                    }
                }
                other => {
                    let _ = writeln!(s, "{pad}{other:?}");
                }
            }
        }
        for l in &p.links {
            let _ = writeln!(
                s,
                "{pad}link {:?} {} {:?} file={:?} data={:?}",
                l.rect,
                self.loc(l.span, out),
                l.kind,
                String::from_utf8_lossy(&l.file),
                String::from_utf8_lossy(&l.data)
            );
        }
        for d in &p.dests {
            let _ = writeln!(s, "{pad}dest {d:?}");
        }
    }
}
