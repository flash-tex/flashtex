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
}

/// An annotation of a page, beyond the display list's links: the whole
/// dictionary dvipdfmx writes, and its rectangle in stream space (bp).
#[derive(Clone, Debug)]
pub struct Annot {
    pub dict: Dict,
    pub rect: [f64; 4],
    /// The display-list link this annotation is, if it is one.
    pub link: Option<usize>,
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
}

/// The document.
#[derive(Default)]
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
    pub forms: Vec<Built>,
    /// `pdf:majorversion`, `pdf:minorversion` (dvipdfmx's default 1.7 is
    /// TeX Live's `dvipdfmx.cfg`'s `V 7`).
    pub version: (u32, u32),
    /// dvipdfmx's colour stack: (fill, stroke). It lasts across pages.
    pub colors: Vec<(Col, Col)>,
    pub tracking: Option<Tracking>,
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
    pub pages: Vec<Built>,
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
            ..Doc::default()
        }
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
