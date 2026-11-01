//! The `geometry` package, v6.0 (2026/03/07), re-implemented macro by
//! macro. Line numbers cite TeX Live 2026
//! `texmf-dist/tex/latex/geometry/geometry.sty`.
//!
//! State mirrors the package's macros: `\Gm@width`… are `Option<Sp>`
//! (undefined = `None`), `\Gm@dimlist` is a list of deferred register
//! assignments replayed by every `\Gm@process`, and the class booleans
//! (`\if@twoside`, `\if@mparswitch`, `\if@twocolumn`, `\if@reversemargin`)
//! are carried through.

use crate::class::{ClassOptions, FontMetrics, Glue, PageParams};
use crate::tex::{strip_pt, Sp};

/// `\usepackage[<package_options>]{geometry}` followed by zero or more
/// preamble `\geometry{<call>}` invocations, in source order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GeometryInput {
    pub package_options: String,
    pub calls: Vec<String>,
}

/// Class booleans geometry reads and may change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutFlags {
    pub twoside: bool,
    pub mparswitch: bool,
    pub twocolumn: bool,
    pub reversemargin: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeometryOutcome {
    pub params: PageParams,
    pub flags: LayoutFlags,
    /// `* h-part:(L,W,R)` and `* v-part:(T,H,B)` of the verbose log.
    pub h_part: (Sp, Sp, Sp),
    pub v_part: (Sp, Sp, Sp),
    /// Package warnings geometry would print (and unsupported keys).
    pub warnings: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reg {
    PaperWidth,
    PaperHeight,
    LayoutWidth,
    LayoutHeight,
    HeadHeight,
    HeadSep,
    FootSkip,
    FootIns,
    MarginParWidth,
    MarginParSep,
    ColumnSep,
    HOffset,
    VOffset,
    BindingOffset,
    LayoutHOffset,
    LayoutVOffset,
}

/// Paper keys, lines 384–420: (width, height, unit).
fn paper_table(name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    Some(match name {
        "a0paper" => ("841", "1189", "mm"),
        "a1paper" => ("594", "841", "mm"),
        "a2paper" => ("420", "594", "mm"),
        "a3paper" => ("297", "420", "mm"),
        "a4paper" => ("210", "297", "mm"),
        "a5paper" => ("148", "210", "mm"),
        "a6paper" => ("105", "148", "mm"),
        "b0paper" => ("1000", "1414", "mm"),
        "b1paper" => ("707", "1000", "mm"),
        "b2paper" => ("500", "707", "mm"),
        "b3paper" => ("353", "500", "mm"),
        "b4paper" => ("250", "353", "mm"),
        "b5paper" => ("176", "250", "mm"),
        "b6paper" => ("125", "176", "mm"),
        "c0paper" => ("917", "1297", "mm"),
        "c1paper" => ("648", "917", "mm"),
        "c2paper" => ("458", "648", "mm"),
        "c3paper" => ("324", "458", "mm"),
        "c4paper" => ("229", "324", "mm"),
        "c5paper" => ("162", "229", "mm"),
        "c6paper" => ("114", "162", "mm"),
        "b0j" => ("1030", "1456", "mm"),
        "b1j" => ("728", "1030", "mm"),
        "b2j" => ("515", "728", "mm"),
        "b3j" => ("364", "515", "mm"),
        "b4j" => ("257", "364", "mm"),
        "b5j" => ("182", "257", "mm"),
        "b6j" => ("128", "182", "mm"),
        "ansiapaper" => ("8.5", "11", "in"),
        "ansibpaper" => ("11", "17", "in"),
        "ansicpaper" => ("17", "22", "in"),
        "ansidpaper" => ("22", "34", "in"),
        "ansiepaper" => ("34", "44", "in"),
        "letterpaper" => ("8.5", "11", "in"),
        "legalpaper" => ("8.5", "14", "in"),
        "executivepaper" => ("7.25", "10.5", "in"),
        "screen" => ("225", "180", "mm"),
        _ => return None,
    })
}

/// Keys defined with `\define@key{Gm}` (lines 421–618). Used to decide
/// which *class* options geometry picks up (`\ProcessOptionsKV[c]`,
/// lines 995–997).
fn is_key(name: &str) -> bool {
    paper_table(name).is_some()
        || matches!(
            name,
            "paper"
                | "papername"
                | "paperwidth"
                | "paperheight"
                | "papersize"
                | "layout"
                | "layoutname"
                | "layoutwidth"
                | "layoutheight"
                | "layoutsize"
                | "landscape"
                | "portrait"
                | "hscale"
                | "vscale"
                | "scale"
                | "width"
                | "height"
                | "total"
                | "totalwidth"
                | "totalheight"
                | "textwidth"
                | "textheight"
                | "text"
                | "body"
                | "lines"
                | "includehead"
                | "includefoot"
                | "includeheadfoot"
                | "includemp"
                | "includeall"
                | "ignorehead"
                | "ignorefoot"
                | "ignoreheadfoot"
                | "ignoremp"
                | "ignoreall"
                | "heightrounded"
                | "hdivide"
                | "vdivide"
                | "divide"
                | "lmargin"
                | "rmargin"
                | "left"
                | "inner"
                | "innermargin"
                | "right"
                | "outer"
                | "outermargin"
                | "tmargin"
                | "bmargin"
                | "top"
                | "bottom"
                | "hmargin"
                | "vmargin"
                | "margin"
                | "hmarginratio"
                | "vmarginratio"
                | "marginratio"
                | "hratio"
                | "vratio"
                | "ratio"
                | "hcentering"
                | "vcentering"
                | "centering"
                | "twoside"
                | "asymmetric"
                | "bindingoffset"
                | "headheight"
                | "head"
                | "headsep"
                | "footskip"
                | "foot"
                | "nohead"
                | "nofoot"
                | "noheadfoot"
                | "footnotesep"
                | "marginparwidth"
                | "marginpar"
                | "marginparsep"
                | "nomarginpar"
                | "columnsep"
                | "hoffset"
                | "voffset"
                | "offset"
                | "layouthoffset"
                | "layoutvoffset"
                | "layoutoffset"
                | "twocolumn"
                | "onecolumn"
                | "reversemp"
                | "reversemarginpar"
                | "driver"
                | "dvips"
                | "dvipdfm"
                | "dvipdfmx"
                | "xdvipdfmx"
                | "pdftex"
                | "luatex"
                | "xetex"
                | "vtex"
                | "verbose"
                | "reset"
                | "resetpaper"
                | "mag"
                | "truedimen"
                | "pass"
                | "showframe"
                | "showcrop"
        )
}

/// Split a keyval list at top-level commas; `(key, Some(value))`.
pub fn split_keyvals(list: &str) -> Vec<(String, Option<String>)> {
    split_top(list, ',')
        .into_iter()
        .filter_map(|item| {
            let item = item.trim();
            if item.is_empty() {
                return None;
            }
            match find_top(item, '=') {
                Some(p) => Some((
                    item[..p].trim().to_string(),
                    Some(strip_braces(item[p + 1..].trim())),
                )),
                None => Some((item.to_string(), None)),
            }
        })
        .collect()
}

fn split_top(s: &str, sep: char) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '{' => {
                depth += 1;
                cur.push(c)
            }
            '}' => {
                depth -= 1;
                cur.push(c)
            }
            c if c == sep && depth == 0 => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

fn find_top(s: &str, ch: char) -> Option<usize> {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            c if c == ch && depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

fn strip_braces(s: &str) -> String {
    let t = s.trim();
    if t.starts_with('{') && t.ends_with('}') && find_top(&t[1..t.len() - 1], '}').is_none() {
        let inner = &t[1..t.len() - 1];
        // only strip when the braces enclose the whole value
        let mut depth = 0i32;
        let balanced = inner.chars().all(|c| {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            depth >= 0
        });
        if balanced && depth == 0 {
            return inner.trim().to_string();
        }
    }
    t.to_string()
}

struct Gm {
    fm: FontMetrics,
    p: PageParams,
    base: PageParams,
    flags: LayoutFlags,
    base_flags: LayoutFlags,
    layoutwidth: Sp,
    layoutheight: Sp,
    layouthoffset: Sp,
    layoutvoffset: Sp,
    bindingoffset: Sp,
    dimlist: Vec<(Reg, Sp)>,
    footins_glue: Option<Glue>,
    paper: bool,
    landscape: bool,
    swap: bool,
    layout: bool,
    hbody: bool,
    vbody: bool,
    heightrounded: bool,
    includehead: bool,
    includefoot: bool,
    includemp: bool,
    pass: bool,
    newgm: bool,
    width: Option<Sp>,
    height: Option<Sp>,
    textwidth: Option<Sp>,
    textheight: Option<Sp>,
    lines: Option<i64>,
    hscale: Option<String>,
    vscale: Option<String>,
    hratio: Option<String>,
    vratio: Option<String>,
    lmargin: Option<Sp>,
    rmargin: Option<Sp>,
    tmargin: Option<Sp>,
    bmargin: Option<Sp>,
    wd_mp: Sp,
    odd_mp: Sp,
    even_mp: Sp,
    cnth: i64,
    cntv: i64,
    warnings: Vec<String>,
}

impl Gm {
    fn warn(&mut self, s: impl Into<String>) {
        self.warnings.push(s.into());
    }

    /// A user length: physical units, or `em`/`ex` of the preamble font.
    fn length(&mut self, key: &str, v: &str) -> Option<Sp> {
        let v = v.trim();
        if let Some(sp) = Sp::parse(v) {
            return Some(sp);
        }
        for (unit, base) in [("em", self.fm.em), ("ex", self.fm.ex)] {
            if let Some(num) = v.strip_suffix(unit) {
                let num = num.trim();
                let num = if num.is_empty() { "1" } else { num };
                if let Some(sp) = base.scaled(num) {
                    return Some(sp);
                }
            }
        }
        self.warn(format!(
            "unsupported length `{v}' for `{key}' (only explicit units are modelled)"
        ));
        None
    }

    /// `\Gm@doif` (lines 165–181): Some(true/false), None = warning.
    fn boolean(&mut self, key: &str, v: &Option<String>) -> Option<bool> {
        match v.as_deref().map(|s| s.trim().to_ascii_lowercase()) {
            None => Some(true),
            Some(s) if s.is_empty() || s == "true" => Some(true),
            Some(s) if s == "false" => Some(false),
            Some(_) => {
                self.warn(format!("`{key}' should be set to `true' or `false'"));
                None
            }
        }
    }

    fn setlength(&mut self, reg: Reg, v: Sp) {
        self.dimlist.push((reg, v));
    }

    /// `\Gm@setsize` (lines 377–381).
    fn setsize(&mut self, paper: bool, w: Sp, h: Sp) {
        if paper {
            self.setlength(Reg::PaperWidth, w);
            self.setlength(Reg::PaperHeight, h);
        } else {
            self.setlength(Reg::LayoutWidth, w);
            self.setlength(Reg::LayoutHeight, h);
        }
        self.swap = self.landscape;
    }

    fn named_size(&mut self, name: &str, paper: bool) -> bool {
        match paper_table(name) {
            Some((w, h, u)) => {
                let w = Sp::parse(&format!("{w}{u}")).unwrap();
                let h = Sp::parse(&format!("{h}{u}")).unwrap();
                self.setsize(paper, w, h);
                true
            }
            None => false,
        }
    }

    /// `\Gm@branch` (lines 346–359): one value sets both keys.
    fn branch(&mut self, v: &str, k1: &str, k2: &str) {
        let parts: Vec<String> = split_top(v, ',')
            .into_iter()
            .map(|s| strip_braces(&s))
            .collect();
        let first = parts.first().cloned().unwrap_or_default();
        self.setkey(k1, &Some(first.clone()));
        if parts.len() >= 2 {
            self.setkey(k2, &Some(parts[1].clone()));
        } else {
            self.setkey(k2, &Some(first));
        }
    }

    /// `\Gm@parse@divide` (lines 329–345).
    fn divide(&mut self, v: &str, keys: [&str; 3]) {
        for (i, part) in split_top(v, ',').into_iter().enumerate() {
            let val = strip_braces(&part);
            let key = keys[i.min(2)];
            if val.is_empty() || val == "*" {
                continue;
            }
            self.setkey(key, &Some(val));
        }
    }

    fn setkeys(&mut self, list: &str) {
        for (k, v) in split_keyvals(list) {
            self.setkey(&k, &v);
        }
    }

    fn setkey(&mut self, key: &str, v: &Option<String>) {
        let val = v.clone().unwrap_or_default();
        if paper_table(key).is_some() {
            // `[true]` default; \Gm@setpaper@ifpre ignores the value.
            if self.newgm {
                self.warn(format!(
                    "`{key}': not available in `\\newgeometry'; skipped"
                ));
            } else {
                self.paper = true;
                self.named_size(key, true);
            }
            return;
        }
        match key {
            "paper" | "papername" => self.setkeys(&val),
            "paperwidth" => {
                if let Some(l) = self.length(key, &val) {
                    self.paper = true;
                    self.setlength(Reg::PaperWidth, l);
                }
            }
            "paperheight" => {
                if let Some(l) = self.length(key, &val) {
                    self.paper = true;
                    self.setlength(Reg::PaperHeight, l);
                }
            }
            "papersize" => {
                self.paper = true;
                self.branch(&val, "paperwidth", "paperheight");
            }
            "layout" | "layoutname" => {
                self.layout = true;
                if !self.named_size(val.trim(), false) {
                    self.warn(format!("unknown layout `{val}'"));
                }
            }
            "layoutwidth" | "layoutheight" => {
                self.layout = true;
                if let Some(l) = self.length(key, &val) {
                    let r = if key == "layoutwidth" {
                        Reg::LayoutWidth
                    } else {
                        Reg::LayoutHeight
                    };
                    self.setlength(r, l);
                }
            }
            "layoutsize" => self.branch(&val, "layoutwidth", "layoutheight"),
            "landscape" | "portrait" => {
                if let Some(b) = self.boolean(key, v) {
                    let want = if key == "landscape" { b } else { !b };
                    if want != self.landscape {
                        self.landscape = want;
                        self.swap = !self.swap;
                    }
                }
            }
            "hscale" => {
                self.hbody = true;
                self.hscale = Some(val.trim().to_string());
            }
            "vscale" => {
                self.vbody = true;
                self.vscale = Some(val.trim().to_string());
            }
            "scale" => self.branch(&val, "hscale", "vscale"),
            "width" | "totalwidth" => {
                self.hbody = true;
                self.width = self.length(key, &val);
            }
            "height" | "totalheight" => {
                self.vbody = true;
                self.height = self.length(key, &val);
            }
            "total" => self.branch(&val, "width", "height"),
            "textwidth" => {
                self.hbody = true;
                self.textwidth = self.length(key, &val);
            }
            "textheight" => {
                self.vbody = true;
                self.textheight = self.length(key, &val);
            }
            "text" | "body" => self.branch(&val, "textwidth", "textheight"),
            "lines" => {
                self.vbody = true;
                match val.trim().parse::<i64>() {
                    Ok(n) => self.lines = Some(n),
                    Err(_) => self.warn(format!("unsupported lines value `{val}'")),
                }
            }
            "includehead" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includehead = b
                }
            }
            "includefoot" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includefoot = b
                }
            }
            "includemp" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includemp = b
                }
            }
            "includeheadfoot" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includehead = b;
                    self.includefoot = b;
                }
            }
            "includeall" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includehead = b;
                    self.includefoot = b;
                    self.includemp = b;
                }
            }
            "ignorehead" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includehead = !b
                }
            }
            "ignorefoot" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includefoot = !b
                }
            }
            "ignoremp" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includemp = !b
                }
            }
            "ignoreheadfoot" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includehead = !b;
                    self.includefoot = !b;
                }
            }
            "ignoreall" => {
                if let Some(b) = self.boolean(key, v) {
                    self.includehead = !b;
                    self.includefoot = !b;
                    self.includemp = !b;
                }
            }
            "heightrounded" => {
                if let Some(b) = self.boolean(key, v) {
                    self.heightrounded = b
                }
            }
            "hdivide" => self.divide(&val, ["lmargin", "width", "rmargin"]),
            "vdivide" => self.divide(&val, ["tmargin", "height", "bmargin"]),
            "divide" => {
                self.divide(&val, ["lmargin", "width", "rmargin"]);
                self.divide(&val, ["tmargin", "height", "bmargin"]);
            }
            "lmargin" | "left" | "inner" | "innermargin" => self.lmargin = self.length(key, &val),
            "rmargin" | "right" | "outer" | "outermargin" => self.rmargin = self.length(key, &val),
            "tmargin" | "top" => self.tmargin = self.length(key, &val),
            "bmargin" | "bottom" => self.bmargin = self.length(key, &val),
            "hmargin" => self.branch(&val, "lmargin", "rmargin"),
            "vmargin" => self.branch(&val, "tmargin", "bmargin"),
            "margin" => {
                self.branch(&val, "lmargin", "tmargin");
                self.branch(&val, "rmargin", "bmargin");
            }
            "hmarginratio" | "hratio" => self.hratio = Some(val.trim().to_string()),
            "vmarginratio" | "vratio" => self.vratio = Some(val.trim().to_string()),
            "marginratio" | "ratio" => self.branch(&val, "hmarginratio", "vmarginratio"),
            "hcentering" | "vcentering" | "centering" => {
                if self.boolean(key, v) == Some(true) {
                    if key != "vcentering" {
                        self.hratio = Some("1:1".into());
                    }
                    if key != "hcentering" {
                        self.vratio = Some("1:1".into());
                    }
                }
            }
            "twoside" => {
                if let Some(b) = self.boolean(key, v) {
                    self.flags.twoside = b;
                    self.flags.mparswitch = b;
                }
            }
            "asymmetric" => {
                if self.boolean(key, v) == Some(true) {
                    self.flags.twoside = true;
                    self.flags.mparswitch = false;
                }
            }
            "bindingoffset" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::BindingOffset, l)
                }
            }
            "headheight" | "head" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::HeadHeight, l)
                }
            }
            "headsep" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::HeadSep, l)
                }
            }
            "footskip" | "foot" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::FootSkip, l)
                }
            }
            "nohead" | "nofoot" | "noheadfoot" => {
                if self.boolean(key, v) == Some(true) {
                    if key != "nofoot" {
                        self.setlength(Reg::HeadHeight, Sp::ZERO);
                        self.setlength(Reg::HeadSep, Sp::ZERO);
                    }
                    if key != "nohead" {
                        self.setlength(Reg::FootSkip, Sp::ZERO);
                    }
                }
            }
            "footnotesep" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::FootIns, l)
                }
            }
            "marginparwidth" | "marginpar" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::MarginParWidth, l)
                }
            }
            "marginparsep" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::MarginParSep, l)
                }
            }
            "nomarginpar" => {
                if self.boolean(key, v) == Some(true) {
                    self.setlength(Reg::MarginParWidth, Sp::ZERO);
                    self.setlength(Reg::MarginParSep, Sp::ZERO);
                }
            }
            "columnsep" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::ColumnSep, l)
                }
            }
            "hoffset" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::HOffset, l)
                }
            }
            "voffset" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::VOffset, l)
                }
            }
            "offset" => self.branch(&val, "hoffset", "voffset"),
            "layouthoffset" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::LayoutHOffset, l)
                }
            }
            "layoutvoffset" => {
                if let Some(l) = self.length(key, &val) {
                    self.setlength(Reg::LayoutVOffset, l)
                }
            }
            "layoutoffset" => self.branch(&val, "layouthoffset", "layoutvoffset"),
            "twocolumn" => {
                if let Some(b) = self.boolean(key, v) {
                    self.flags.twocolumn = b
                }
            }
            "onecolumn" => {
                if let Some(b) = self.boolean(key, v) {
                    self.flags.twocolumn = !b
                }
            }
            "reversemp" | "reversemarginpar" => {
                if let Some(b) = self.boolean(key, v) {
                    self.flags.reversemargin = b
                }
            }
            "pass" => {
                if let Some(b) = self.boolean(key, v) {
                    self.pass = b
                }
            }
            "mag" | "truedimen" | "reset" => {
                self.warn(format!("`{key}' is not modelled"));
            }
            "driver" | "dvips" | "dvipdfm" | "dvipdfmx" | "xdvipdfmx" | "pdftex" | "luatex"
            | "xetex" | "vtex" | "verbose" | "resetpaper" | "showframe" | "showcrop" => {}
            _ => self.warn(format!("Package keyval Error: {key} undefined")),
        }
    }

    /// `\Gm@setdefaultpaper` (lines 619–624).
    fn setdefaultpaper(&mut self) {
        if !self.paper {
            let (w, h) = (self.p.paperwidth, self.p.paperheight);
            // \strip@pt round-trips through print_scaled exactly.
            debug_assert_eq!(Sp::parse(&format!("{}pt", strip_pt(w))), Some(w));
            self.setsize(true, w, h);
            self.setsize(false, w, h);
            self.swap = false;
        }
    }

    /// `\Gm@clean` (lines 313–328), run by `\geometry` before its keys.
    fn clean(&mut self) {
        if self.cnth < 4 {
            self.lmargin = None;
        }
        if self.cnth % 2 == 0 {
            self.rmargin = None;
        }
        if self.cntv < 4 {
            self.tmargin = None;
        }
        if self.cntv % 2 == 0 {
            self.bmargin = None;
        }
        if !self.hbody {
            self.hscale = None;
            self.width = None;
            self.textwidth = None;
        }
        if !self.vbody {
            self.vscale = None;
            self.height = None;
            self.textheight = None;
        }
    }

    fn scale_of(&mut self, factor: Option<String>, of: Sp) -> Sp {
        let f = factor.unwrap_or_else(|| "0.7".into()); // \Gm@Dhscale/\Gm@Dvscale, lines 70–71
        match of.scaled(&f) {
            Some(v) => v,
            None => {
                self.warn(format!("unsupported scale `{f}'"));
                of.scaled("0.7").unwrap()
            }
        }
    }

    /// `\Gm@adjustmp` (lines 676–699).
    fn adjustmp(&mut self) {
        if !self.includemp {
            return;
        }
        let w = self.p.marginparwidth + self.p.marginparsep;
        self.wd_mp = w;
        self.odd_mp = Sp::ZERO;
        self.even_mp = Sp::ZERO;
        if self.flags.twocolumn {
            self.wd_mp = w.times(2);
            self.odd_mp = w;
            self.even_mp = w;
        } else if self.flags.reversemargin {
            self.odd_mp = w;
            if !self.flags.mparswitch {
                self.even_mp = w;
            }
        } else if self.flags.mparswitch {
            self.even_mp = w;
        }
    }

    /// `\Gm@adjustbody` (lines 700–749).
    fn adjustbody(&mut self) {
        if self.hbody {
            if self.width.is_none() {
                let s = self.hscale.clone();
                self.width = Some(self.scale_of(s, self.layoutwidth));
            }
            if let Some(tw) = self.textwidth {
                self.width = Some(if self.includemp { tw + self.wd_mp } else { tw });
            }
        }
        if self.vbody {
            if self.height.is_none() {
                let s = self.vscale.clone();
                self.height = Some(self.scale_of(s, self.layoutheight));
            }
            if let Some(n) = self.lines {
                // \ht\strutbox = .7\baselineskip (latex.ltx \set@fontsize).
                let strut = self.p.baselineskip.scaled(".7").unwrap();
                if self.p.topskip < strut {
                    self.warn(format!(
                        "\\topskip was changed from {} to {}",
                        self.p.topskip, strut
                    ));
                    self.p.topskip = strut;
                }
                self.textheight =
                    Some(self.p.baselineskip.times(n) + self.p.topskip - self.p.baselineskip);
            }
            if let Some(th) = self.textheight {
                let mut h = th;
                if self.includehead {
                    h += self.p.headheight + self.p.headsep;
                }
                if self.includefoot {
                    h += self.p.footskip;
                }
                self.height = Some(h);
            }
        }
    }

    fn ratio(&mut self, horizontal: bool, explicit: &Option<String>) -> (i64, i64) {
        let default = if horizontal {
            if self.flags.twoside {
                "2:3" // \Gm@Dhratiotwo, line 68
            } else {
                "1:1" // \Gm@Dhratio, line 67
            }
        } else {
            "2:3" // \Gm@Dvratio, line 69
        };
        let parse = |s: &str| -> Option<(i64, i64)> {
            let (a, b) = s.split_once(':')?;
            Some((a.trim().parse().ok()?, b.trim().parse().ok()?))
        };
        match explicit {
            None => parse(default).unwrap(),
            Some(s) => match parse(s) {
                Some((a, b)) if b > 0 => (a, b),
                _ => {
                    self.warn("margin ratio a:b should be non-zero; default used");
                    parse(default).unwrap()
                }
            },
        }
    }

    /// `\Gm@detall` (lines 248–312) for one direction.
    fn detall(&mut self, horizontal: bool) {
        let layout = if horizontal {
            self.layoutwidth
        } else {
            self.layoutheight
        };
        let (mut body, mut m1, mut m2, bodyset, mratio) = if horizontal {
            (
                self.width,
                self.lmargin,
                self.rmargin,
                self.hbody,
                self.hratio.clone(),
            )
        } else {
            (
                self.height,
                self.tmargin,
                self.bmargin,
                self.vbody,
                self.vratio.clone(),
            )
        };
        let default_body = |gm: &mut Gm| {
            let s = if horizontal {
                gm.hscale_default()
            } else {
                gm.vscale_default()
            };
            gm.scale_of(Some(s), layout)
        };
        let cnt = if m1.is_some() { 4 } else { 0 }
            + if bodyset { 2 } else { 0 }
            + if m2.is_some() { 1 } else { 0 };
        if horizontal {
            self.cnth = cnt;
        } else {
            self.cntv = cnt;
        }
        // \Gm@detiiandiii: split layout - body by the ratio.
        let split = |gm: &mut Gm, body: Sp| -> (Sp, Sp) {
            let rest = layout - body;
            if rest < Sp::ZERO {
                gm.warn(format!("margins result in NEGATIVE ({rest})"));
            }
            let (a, b) = gm.ratio(horizontal, &mratio);
            let first = rest.over(a + b).times(a);
            (first, rest - first)
        };
        let detiv = |gm: &mut Gm, x: Sp, y: Sp, what: &str| -> Sp {
            let v = layout - x - y;
            if v < Sp::ZERO {
                gm.warn(format!("`{what}' results in NEGATIVE ({v})"));
            }
            v
        };
        let by_ratio = |gm: &mut Gm, known: Sp, backward: bool| -> Sp {
            let (mut a, mut b) = {
                let s = mratio.clone().unwrap();
                match s.split_once(':').and_then(|(x, y)| {
                    Some((x.trim().parse::<i64>().ok()?, y.trim().parse::<i64>().ok()?))
                }) {
                    Some(p) => p,
                    None => {
                        gm.warn(format!("bad ratio `{s}'"));
                        (1, 1)
                    }
                }
            };
            if backward {
                std::mem::swap(&mut a, &mut b);
            }
            if b > 0 {
                known.times(a).over(b)
            } else {
                known
            }
        };
        match cnt {
            0 => {
                let b = default_body(self);
                body = Some(b);
                let (x, y) = split(self, b);
                m1 = Some(x);
                m2 = Some(y);
            }
            1 => {
                if mratio.is_none() {
                    let keep = m2.unwrap();
                    let b = default_body(self);
                    let (x, _) = split(self, b);
                    m1 = Some(x);
                    m2 = Some(keep);
                } else {
                    m1 = Some(by_ratio(self, m2.unwrap(), false));
                }
                body = Some(detiv(self, m1.unwrap(), m2.unwrap(), "width"));
            }
            2 => {
                let (x, y) = split(self, body.unwrap());
                m1 = Some(x);
                m2 = Some(y);
            }
            3 => m1 = Some(detiv(self, body.unwrap(), m2.unwrap(), "lmargin")),
            4 => {
                if mratio.is_none() {
                    // line 300: \Gm@detiiandiii{#2}{#4}{#3} -- the *first*
                    // ratio share goes to the right/bottom margin.
                    let keep = m1.unwrap();
                    let b = default_body(self);
                    let (x, _) = split(self, b);
                    m1 = Some(keep);
                    m2 = Some(x);
                } else {
                    m2 = Some(by_ratio(self, m1.unwrap(), true));
                }
                body = Some(detiv(self, m1.unwrap(), m2.unwrap(), "width"));
            }
            5 => body = Some(detiv(self, m1.unwrap(), m2.unwrap(), "width")),
            6 => m2 = Some(detiv(self, body.unwrap(), m1.unwrap(), "rmargin")),
            _ => {
                self.warn(format!(
                    "Over-specification in `{}'-direction. `{}' ({}) is ignored",
                    if horizontal { 'h' } else { 'v' },
                    if horizontal { "width" } else { "height" },
                    body.unwrap()
                ));
                body = Some(detiv(self, m1.unwrap(), m2.unwrap(), "width"));
            }
        }
        if horizontal {
            self.width = body;
            self.lmargin = m1;
            self.rmargin = m2;
        } else {
            self.height = body;
            self.tmargin = m1;
            self.bmargin = m2;
        }
    }

    fn hscale_default(&self) -> String {
        "0.7".into()
    }
    fn vscale_default(&self) -> String {
        "0.7".into()
    }

    /// `\Gm@@process` (lines 756–814).
    fn process(&mut self) {
        if self.pass {
            self.p = self.base;
            self.flags = self.base_flags;
            return;
        }
        // \Gm@expandlengths
        for (reg, v) in self.dimlist.clone() {
            match reg {
                Reg::PaperWidth => self.p.paperwidth = v,
                Reg::PaperHeight => self.p.paperheight = v,
                Reg::LayoutWidth => self.layoutwidth = v,
                Reg::LayoutHeight => self.layoutheight = v,
                Reg::HeadHeight => self.p.headheight = v,
                Reg::HeadSep => self.p.headsep = v,
                Reg::FootSkip => self.p.footskip = v,
                Reg::FootIns => self.footins_glue = Some(Glue::fixed(v)),
                Reg::MarginParWidth => self.p.marginparwidth = v,
                Reg::MarginParSep => self.p.marginparsep = v,
                Reg::ColumnSep => self.p.columnsep = v,
                Reg::HOffset => self.p.hoffset = v,
                Reg::VOffset => self.p.voffset = v,
                Reg::BindingOffset => self.bindingoffset = v,
                Reg::LayoutHOffset => self.layouthoffset = v,
                Reg::LayoutVOffset => self.layoutvoffset = v,
            }
        }
        if let Some(g) = self.footins_glue {
            self.p.skip_footins = g;
        }
        // \Gm@adjustpaper
        if self.p.paperwidth <= Sp::pt(1) || self.p.paperheight <= Sp::pt(1) {
            self.warn("paper too short (set a paper type)");
        }
        if self.swap {
            std::mem::swap(&mut self.p.paperwidth, &mut self.p.paperheight);
        }
        if !self.layout {
            self.layoutwidth = self.p.paperwidth;
            self.layoutheight = self.p.paperheight;
        }
        self.layoutwidth -= self.bindingoffset;
        self.adjustmp();
        self.adjustbody();
        self.detall(true);
        self.detall(false);
        let inch = Sp::parse("1in").unwrap();
        let p = &mut self.p;
        p.textwidth = self.width.unwrap();
        p.textheight = self.height.unwrap();
        p.topmargin = self.tmargin.unwrap();
        p.oddsidemargin = self.lmargin.unwrap() - inch;
        if self.includemp {
            p.textwidth -= self.wd_mp;
            p.oddsidemargin += self.odd_mp;
        }
        if self.flags.mparswitch {
            p.evensidemargin = self.rmargin.unwrap() - inch;
            if self.includemp {
                p.evensidemargin += self.even_mp;
            }
        } else {
            p.evensidemargin = p.oddsidemargin;
        }
        p.oddsidemargin += self.bindingoffset;
        p.topmargin -= inch;
        if self.includehead {
            p.textheight -= p.headheight;
            p.textheight -= p.headsep;
        } else {
            p.topmargin -= p.headheight;
            p.topmargin -= p.headsep;
        }
        if self.includefoot {
            p.textheight -= p.footskip;
        }
        if self.heightrounded {
            let rest = p.textheight - p.topskip;
            let n = rest.0 / p.baselineskip.0;
            let mut lines = p.baselineskip.times(n);
            let remainder = (rest - lines).times(2);
            if remainder > p.baselineskip {
                lines += p.baselineskip;
            }
            p.textheight = lines + p.topskip;
        }
        p.oddsidemargin += self.layouthoffset;
        p.evensidemargin += self.layouthoffset;
        p.topmargin += self.layoutvoffset;
        self.layoutwidth += self.bindingoffset;
    }
}

/// Run geometry over a class's parameters: `\ProcessOptionsKV[c]` (class
/// options that are geometry keys), `\Gm@setdefaultpaper`,
/// `\ProcessOptionsKV[p]`, `\Gm@process`, then each `\geometry{}` call
/// (`\Gm@clean`, keys, `\Gm@process`) — lines 1011–1018 and 1125–1128.
pub fn apply_geometry(
    class: &ClassOptions,
    base: &PageParams,
    fm: FontMetrics,
    input: &GeometryInput,
) -> GeometryOutcome {
    let flags = LayoutFlags {
        twoside: class.twoside,
        mparswitch: class.twoside,
        twocolumn: class.twocolumn,
        reversemargin: false,
    };
    let mut gm = Gm {
        fm,
        p: *base,
        base: *base,
        flags,
        base_flags: flags,
        layoutwidth: Sp::ZERO,
        layoutheight: Sp::ZERO,
        layouthoffset: Sp::ZERO,
        layoutvoffset: Sp::ZERO,
        bindingoffset: Sp::ZERO,
        dimlist: Vec::new(),
        footins_glue: None,
        paper: false,
        landscape: false,
        swap: false,
        layout: false,
        hbody: false,
        vbody: false,
        heightrounded: false,
        includehead: false,
        includefoot: false,
        includemp: false,
        pass: false,
        newgm: false,
        width: None,
        height: None,
        textwidth: None,
        textheight: None,
        lines: None,
        hscale: None,
        vscale: None,
        hratio: None,
        vratio: None,
        lmargin: None,
        rmargin: None,
        tmargin: None,
        bmargin: None,
        wd_mp: Sp::ZERO,
        odd_mp: Sp::ZERO,
        even_mp: Sp::ZERO,
        cnth: 0,
        cntv: 0,
        warnings: Vec::new(),
    };
    for opt in &class.given {
        let (k, _) = split_keyvals(opt).into_iter().next().unwrap_or_default();
        if is_key(&k) {
            gm.setkeys(opt);
        }
    }
    gm.setdefaultpaper();
    gm.setkeys(&input.package_options);
    gm.process();
    for call in &input.calls {
        gm.clean();
        gm.setkeys(call);
        gm.process();
    }
    GeometryOutcome {
        params: gm.p,
        flags: gm.flags,
        h_part: (
            gm.lmargin.unwrap_or_default(),
            gm.width.unwrap_or_default(),
            gm.rmargin.unwrap_or_default(),
        ),
        v_part: (
            gm.tmargin.unwrap_or_default(),
            gm.height.unwrap_or_default(),
            gm.bmargin.unwrap_or_default(),
        ),
        warnings: gm.warnings,
    }
}
