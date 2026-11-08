//! What the output keeps across pages: fonts and images, the named objects
//! of the specials, the outline, the document information, the forms,
//! dvipdfmx's colour stack and the link annotation being tracked, and every
//! page built so far with what the display list cannot carry.

use super::fonts::Fonts;
use super::images::Images;
use super::pdfobj::{Dict, Obj};
use flashtex_display_list::page::Page;
use flashtex_engine::displaylist::fixed::Fx;
use std::collections::{BTreeMap, HashMap};

/// A colour: DeviceGray, DeviceRGB or DeviceCMYK components.
#[derive(Clone, Debug, PartialEq)]
pub struct Col(pub Vec<f64>);

impl Col {
    pub fn black() -> Col {
        Col(vec![0.0])
    }
}

/// A named object (`pdf:obj @name ...`, `pdf:stream`, `pdf:bxobj`).
#[derive(Clone, Debug)]
pub enum Named {
    Obj(Obj),
    /// A stream: its dictionary and data.
    Stream(Dict, Vec<u8>),
    /// A form made by `pdf:bxobj`: display-list form `id`.
    Form(u32),
    /// An image made by `pdf:image @name`: display-list image `id`.
    Image(u32),
}

/// One outline entry (`pdf:outline [-] level <<dict>>`).
#[derive(Clone, Debug)]
pub struct Outline {
    pub level: i32,
    /// `[-]`: closed; `[]` or nothing: open (dvipdfmx's default, `-O 0`,
    /// is closed unless the special says `[]`).
    pub open: bool,
    pub dict: Dict,
    /// Its destination's new name ([`Doc::add_goto`]).
    pub goto: Option<Goto>,
}

/// dvipdfmx's destination optimisation (pdfdoc.c's `pdf_doc_add_goto`, on
/// unless `dvipdfmx:config C 0x0010`): a link's or outline entry's named
/// destination, `/Dest (name)` or `/A <</S/GoTo/D (name)>>`, is written
/// under a new name, the number of names renamed before it in hexadecimal
/// (`(0)`, `(1)`, …, `(a)`); only destinations so named are written, under
/// their new names. hyperref sets 0x0010, so its names stay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goto {
    /// In `/A`'s `/D` (else the dictionary's own `/Dest`).
    pub in_action: bool,
    pub name: Vec<u8>,
}

impl Goto {
    /// Write the new name into `d` (a copy of the dictionary as written).
    pub fn apply(&self, d: &mut Dict) {
        if self.in_action {
            if let Some(Obj::Dict(a)) = d.get_mut(b"A") {
                a.set(b"D", Obj::Str(self.name.clone()));
            }
        } else {
            d.set(b"Dest", Obj::Str(self.name.clone()));
        }
    }
}

/// An annotation of a page, beyond the display list's links: the whole
/// dictionary dvipdfmx writes, and its rectangle in stream space (bp).
#[derive(Clone, Debug)]
pub struct Annot {
    pub dict: Dict,
    pub rect: [f64; 4],
    /// The display-list link this annotation is, if it is one.
    pub link: Option<usize>,
    /// Its destination's new name in the PDF ([`Doc::add_goto`]); the
    /// display list keeps the name the special gave.
    pub goto: Option<Goto>,
    /// `pdf:ann @name` (and the first part of `pdf:bann @name`): the
    /// annotation is the named object `name` of [`Doc::named`], whose
    /// dictionary (with its `/Rect`, and whatever `pdf:put @name` adds
    /// later) is written once, for both the page's `/Annots` and every
    /// `@name` (an AcroForm's `/Fields`), as dvipdfmx's
    /// `spc_push_object` makes it one object.
    pub name: Option<Vec<u8>>,
}

/// Content of a page or form that the display list cannot express (an
/// `UNSUPPORTED` entry): the PDF operators, to be written as they are
/// under `ctm` (stream space). `paints` is false for operators that only
/// change the graphics state (a `gs`, a colour space), which are written
/// without a `q`/`Q` around them.
#[derive(Clone, Debug)]
pub struct Raw {
    pub ops: Vec<u8>,
    pub ctm: [f64; 6],
    pub paints: bool,
}

/// What a page or form needs in its PDF beyond its display list.
#[derive(Clone, Debug, Default)]
pub struct Supplement {
    pub annots: Vec<Annot>,
    /// By `UNSUPPORTED` entry: the operators, or `None` for an entry that
    /// has nothing to write (a feature that was dropped).
    pub raw: Vec<Option<Raw>>,
    /// `pdf:put @resources <<...>>`.
    pub resources: Dict,
    /// `pdf:put @thispage <<...>>` (pages only).
    pub page_dict: Dict,
    /// `pdf:exobj <<...>>` (forms only).
    pub form_dict: Dict,
}

/// A page or form as built.
#[derive(Clone, Debug)]
pub struct Built {
    pub dl: Page,
    pub sup: Supplement,
    /// Resources the display list uses, in order of first use.
    pub fonts: Vec<u16>,
    pub images: Vec<u32>,
    pub forms: Vec<u32>,
    /// Its `PAGE`/`FORM` body (`out::encode`), once made: a resident run
    /// restarted after this page sends it again (`Output::resume`).
    pub frame: std::sync::OnceLock<Vec<u8>>,
    /// A page: how many forms had been made when it shipped.
    pub forms_at: usize,
    /// A resident run's page: what it was built from, so that a later run
    /// that converges with this one builds it again into its own document
    /// (`crate::conv`, `out::replay_page`).
    pub replay: Option<std::sync::Arc<Replay>>,
}

/// The keys of a document's fonts and images by id (`out::encode` hashes a
/// page's resources by these): a run's, for a later run that sends its pages
/// again (`out::replay_page`). Both tables only grow, so an id keeps its key.
#[derive(Clone, Debug, Default)]
pub struct DocKeys {
    pub fonts: Vec<[u8; 32]>,
    pub images: Vec<[u8; 32]>,
}

impl DocKeys {
    pub fn of(doc: &Doc) -> DocKeys {
        DocKeys {
            fonts: doc.fonts.res.iter().map(|r| r.key).collect(),
            images: doc.images.res.iter().map(|i| i.key).collect(),
        }
    }

    /// Whether every id both documents have gives the same key in `doc`
    /// as here: a page that names the same ids (the same display list) then
    /// encodes alike in both.
    pub fn agree(&self, doc: &Doc) -> bool {
        let (f, i) = (&doc.fonts.res, &doc.images.res);
        self.fonts.iter().zip(f).all(|(k, r)| *k == r.key)
            && self.images.iter().zip(i).all(|(k, r)| *k == r.key)
    }
}

/// What a page was built from ([`Built::replay`]): its XDV bytes, the
/// source map's markers in them, `\mag`, and the pictures the engine's
/// lookup found for it.
#[derive(Debug)]
pub struct Replay {
    pub xdv: Vec<u8>,
    pub marks: Vec<(u32, u32, u16)>,
    pub mag: i32,
    pub pictures: Vec<String>,
}

/// A link annotation being tracked between `pdf:bann` and `pdf:eann`.
#[derive(Clone, Debug)]
pub struct Tracking {
    pub dict: Dict,
    /// The DVI stack depth at `pdf:bann`.
    pub depth: usize,
    /// The rectangle so far on this page (stream space, bp): llx, lly,
    /// urx, ury, or `None` when nothing has been drawn yet.
    pub rect: Option<[f64; 4]>,
    /// `pdf:bann @name`, until the link's first part is written: that part
    /// is the named object, and later parts are copies of its dictionary
    /// (dvipdfmx's `pdf_doc_break_annot`).
    pub name: Option<Vec<u8>>,
    /// A part has been written (dvipdfmx's `breaking_state.broken`): only
    /// the first goes through `pdf_doc_add_goto`.
    pub broken: bool,
    /// The first part's new destination name when it is in `/A`, which the
    /// later parts' copies share (`pdf_merge_dict` links `/A`); a renamed
    /// `/Dest` is the first part's own.
    pub goto: Option<Goto>,
}

/// `dvipdfmx:catch_phantom` (dvipdfmx's dvi.c, `catch_phantom`): whether a
/// move to the right inside a link widens the link, and how high and deep
/// the box it crosses is. hyperref's `hdvipdfmx` and beamer's navigation
/// symbols set it so that a link over empty space has a rectangle. Like
/// dvipdfmx's, it lasts past the link and the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Phantom {
    /// `catch_phantom false`, and dvipdfmx's start.
    #[default]
    Off,
    /// `catch_phantom true` alone: the current font's size high, 0 deep.
    FontSize,
    /// `catch_phantom true height … depth …` (DVI units, truncated as
    /// dvipdfmx's `spt_t` assignment truncates).
    Box { height: i32, depth: i32 },
}

/// The document. A resident engine's page checkpoint keeps a copy
/// (`crate::ckpt`): the pages and forms are shared, so a copy costs a
/// pointer a page.
#[derive(Clone, Default)]
pub struct Doc {
    pub fonts: Fonts,
    pub images: Images,
    pub named: HashMap<Vec<u8>, Named>,
    /// Named objects closed by `pdf:close`.
    pub closed: Vec<Vec<u8>>,
    pub outlines: Vec<Outline>,
    pub docinfo: Dict,
    /// `pdf:docview` and `pdf:put @catalog`.
    pub catalog: Dict,
    /// `pdf:put @names`.
    pub names: Dict,
    /// Named destinations, in order: the name and the destination array
    /// (with `@page<n>` for its page).
    pub dests: Vec<(Vec<u8>, Obj)>,
    /// Forms by id - 1.
    pub forms: Vec<std::sync::Arc<Built>>,
    /// `pdf:majorversion`, `pdf:minorversion` (dvipdfmx's default 1.7 is
    /// TeX Live's `dvipdfmx.cfg`'s `V 7`).
    pub version: (u32, u32),
    /// dvipdfmx's colour stack: (fill, stroke). It lasts across pages.
    pub colors: Vec<(Col, Col)>,
    pub tracking: Option<Tracking>,
    /// `dvipdfmx:catch_phantom`'s mode.
    pub phantom: Phantom,
    /// `pdf:nolink` (until `pdf:link`): what is drawn does not widen the
    /// link being tracked (dvipdfmx's `link_annot` off).
    pub nolink: bool,
    /// dvipdfmx's destination optimisation is on ([`Goto`]): no
    /// `dvipdfmx:config C` with 0x0010 on the first page (dvipdfmx.c reads
    /// the first page's config specials before it opens the document).
    pub check_gotos: bool,
    /// The destinations renamed so far: old name to new.
    pub gotos: HashMap<Vec<u8>, Vec<u8>>,
    /// Specials not understood, by their first word, with their count.
    pub unknown: BTreeMap<String, u32>,
    /// Specials understood but not carried out, by name, with their count.
    pub ignored: BTreeMap<String, u32>,
    pub diagnostics: Vec<String>,
    /// The paper size of a page without `pdf:pagesize` (bp).
    pub default_paper: Option<(Fx, Fx)>,
    /// The size the last `pdf:pagesize`/`papersize` gave, kept for later
    /// pages (dvipdfmx keeps it).
    pub paper: Option<(Fx, Fx)>,
    pub pages: Vec<std::sync::Arc<Built>>,
    /// The XDV preamble's comment (` XeTeX output ...`): xdvipdfmx's
    /// `/Creator` when the document gives none.
    pub dvi_comment: Option<Vec<u8>>,
    /// The picture files the engine's own lookup found (`find_pic_file`):
    /// `pdf:image` takes these paths as they are.
    pub pictures: std::collections::BTreeSet<String>,
    /// What went wrong reading the output: the run fails (a page that
    /// cannot be read is never dropped silently).
    pub errors: Vec<String>,
}

impl Doc {
    pub fn new() -> Doc {
        Doc {
            version: (1, 7),
            check_gotos: true,
            ..Doc::default()
        }
    }

    /// pdfdoc.c's `pdf_doc_add_goto` for a link annotation's or an outline
    /// entry's dictionary `d`: its named destination's new name, or none
    /// (the optimisation off, not a link, a destination array, or another
    /// action).
    pub fn add_goto(&mut self, d: &Dict) -> Option<Goto> {
        if !self.check_gotos {
            return None;
        }
        let unknown = |doc: &mut Doc| {
            doc.diag("Unknown PDF annotation format. Output file may be broken.".into());
            None
        };
        match d.get(b"Subtype") {
            None => {}
            Some(Obj::Name(n)) if n == b"Link" => {}
            Some(Obj::Name(_)) => return None,
            Some(_) => return unknown(self),
        }
        let dest = d.get(b"Dest");
        let (in_action, dv) = match d.get(b"A") {
            None => (false, dest),
            Some(Obj::Dict(a)) if dest.is_none() => match a.get(b"S") {
                Some(Obj::Name(s)) if s == b"GoTo" => (true, a.get(b"D")),
                Some(Obj::Name(_)) => return None,
                _ => return unknown(self),
            },
            Some(_) => return unknown(self),
        };
        let old = match dv {
            Some(Obj::Str(s)) => s.clone(),
            Some(Obj::Array(_)) => return None,
            _ => return unknown(self),
        };
        let n = self.gotos.len();
        let name = self
            .gotos
            .entry(old)
            .or_insert_with(|| format!("{n:x}").into_bytes())
            .clone();
        Some(Goto { in_action, name })
    }

    pub fn unknown_special(&mut self, what: &str) {
        *self.unknown.entry(what.to_string()).or_insert(0) += 1;
    }

    pub fn ignored_special(&mut self, what: &str) {
        *self.ignored.entry(what.to_string()).or_insert(0) += 1;
    }

    pub fn diag(&mut self, msg: String) {
        if self.diagnostics.len() < 1000 && !self.diagnostics.contains(&msg) {
            self.diagnostics.push(msg);
        }
    }

    /// The dictionary of named object `n`, if it is a dictionary.
    pub fn named_dict(&self, n: &[u8]) -> Option<&Dict> {
        match self.named.get(n)? {
            Named::Obj(Obj::Dict(d)) => Some(d),
            Named::Stream(d, _) => Some(d),
            _ => None,
        }
    }

    /// `o` with a `@name` reference replaced by the object it names (one
    /// level), for reading values such as an ExtGState's `ca`.
    pub fn deref<'a>(&'a self, o: &'a Obj) -> &'a Obj {
        match o {
            Obj::Named(n) => match self.named.get(n.as_slice()) {
                Some(Named::Obj(x)) => x,
                _ => o,
            },
            _ => o,
        }
    }
}
