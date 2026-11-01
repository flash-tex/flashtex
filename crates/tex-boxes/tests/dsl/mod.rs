//! A tiny interpreter for the TeX/LaTeX subset used by the oracle fixtures.
//! It tokenizes the fixture source and calls one [`BoxEngine`] / `latex`
//! function per primitive or LaTeX command. It is test support only: real
//! documents are driven by the KC-101 expansion engine.

use flashtex_tex_boxes::engine::{BoxContext, BoxDim, BoxEngine, BoxKind};
use flashtex_tex_boxes::latex;
use flashtex_tex_boxes::node::{GlueOrder, GlueSpec, LeaderKind, Rule};
use flashtex_tex_boxes::pack::PackSpec;
use flashtex_tex_boxes::scaled::{Scaled, UNITY, Unit, dimen_from_parts, scale_internal};

/// Expands `<<N*text>>` repetitions (same rule as generate.py).
pub fn expand_repeats(s: &str) -> String {
    // Innermost `<<N*text>>` first so nested repeats expand like generate.py.
    let mut s = s.to_string();
    loop {
        let Some(end) = s.find(">>") else { return s };
        let start = s[..end].rfind("<<").expect("repeat start");
        let inner = &s[start + 2..end];
        let star = inner.find('*').expect("repeat count");
        let n: usize = inner[..star].parse().expect("repeat number");
        let expanded = inner[star + 1..].repeat(n);
        s = format!("{}{}{}", &s[..start], expanded, &s[end + 2..]);
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Cs(String),
    Ch(char),
    Space,
}

fn tokenize(s: &str) -> Vec<Tok> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            i += 1;
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphabetic() || chars[i] == '@') {
                i += 1;
            }
            if i == start {
                out.push(Tok::Cs(chars[i].to_string()));
                i += 1;
            } else {
                out.push(Tok::Cs(chars[start..i].iter().collect()));
                while i < chars.len() && chars[i] == ' ' {
                    i += 1;
                }
            }
        } else if c == ' ' {
            out.push(Tok::Space);
            while i < chars.len() && chars[i] == ' ' {
                i += 1;
            }
        } else {
            out.push(Tok::Ch(c));
            i += 1;
        }
    }
    out
}

#[derive(Debug)]
pub struct DslError(pub String);

impl std::fmt::Display for DslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<flashtex_tex_boxes::engine::BoxError> for DslError {
    fn from(e: flashtex_tex_boxes::engine::BoxError) -> Self {
        DslError(format!("engine: {e}"))
    }
}

type R<T = ()> = Result<T, DslError>;

fn err<T>(msg: impl Into<String>) -> R<T> {
    Err(DslError(msg.into()))
}

struct Interp {
    toks: Vec<Tok>,
    pos: usize,
    with_pos: bool,
}

const BA: u32 = 0x2000;
const BB: u32 = 0x2001;

/// Executes `src` on the engine. `with_pos` makes `\POS` a whatsit.
pub fn execute(e: &mut BoxEngine, src: &str, with_pos: bool) -> R {
    let mut it = Interp { toks: tokenize(src), pos: 0, with_pos };
    it.run_until_end(e)
}

fn skip_param(name: &str) -> bool {
    matches!(name, "baselineskip" | "lineskip" | "parskip" | "leftskip" | "rightskip" | "parfillskip" | "La" | "Lb" | "fill")
}
fn dimen_param(name: &str) -> bool {
    matches!(
        name,
        "hfuzz" | "vfuzz" | "boxmaxdepth" | "lineskiplimit" | "parindent" | "hsize" | "fboxsep" | "fboxrule" | "overfullrule"
    )
}
fn int_param(name: &str) -> bool {
    matches!(name, "hbadness" | "vbadness" | "showboxdepth" | "showboxbreadth")
}

impl Interp {
    fn sub(&self, toks: Vec<Tok>) -> Interp {
        Interp { toks, pos: 0, with_pos: self.with_pos }
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn skip_spaces(&mut self) {
        while self.peek() == Some(&Tok::Space) {
            self.pos += 1;
        }
    }
    fn expect_ch(&mut self, c: char) -> R {
        self.skip_spaces();
        match self.next() {
            Some(Tok::Ch(x)) if x == c => Ok(()),
            other => err(format!("expected '{c}', found {other:?} at {}", self.pos)),
        }
    }

    /// Reads a balanced `{...}` group, returning the inner tokens.
    fn group(&mut self) -> R<Vec<Tok>> {
        self.skip_spaces();
        match self.next() {
            Some(Tok::Ch('{')) => {}
            Some(Tok::Cs(name)) => return Ok(vec![Tok::Cs(name)]),
            other => return err(format!("expected group, found {other:?}")),
        }
        let mut depth = 1;
        let mut out = Vec::new();
        loop {
            match self.next() {
                Some(Tok::Ch('{')) => {
                    depth += 1;
                    out.push(Tok::Ch('{'));
                }
                Some(Tok::Ch('}')) => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(out);
                    }
                    out.push(Tok::Ch('}'));
                }
                Some(t) => out.push(t),
                None => return err("unterminated group"),
            }
        }
    }

    /// Reads an optional `[...]` argument (LaTeX `\@ifnextchar[`).
    fn optional(&mut self) -> R<Option<Vec<Tok>>> {
        self.skip_spaces();
        if self.peek() != Some(&Tok::Ch('[')) {
            return Ok(None);
        }
        self.pos += 1;
        let mut out = Vec::new();
        let mut depth = 0;
        loop {
            match self.next() {
                Some(Tok::Ch(']')) if depth == 0 => return Ok(Some(out)),
                Some(Tok::Ch('{')) => {
                    depth += 1;
                    out.push(Tok::Ch('{'));
                }
                Some(Tok::Ch('}')) => {
                    depth -= 1;
                    out.push(Tok::Ch('}'));
                }
                Some(t) => out.push(t),
                None => return err("unterminated optional argument"),
            }
        }
    }

    fn keyword(&mut self, kw: &str) -> bool {
        self.skip_spaces();
        let save = self.pos;
        for c in kw.chars() {
            match self.next() {
                Some(Tok::Ch(x)) if x == c => {}
                _ => {
                    self.pos = save;
                    return false;
                }
            }
        }
        true
    }

    fn optional_equals(&mut self) {
        self.skip_spaces();
        if self.peek() == Some(&Tok::Ch('=')) {
            self.pos += 1;
        }
    }

    fn scan_int(&mut self) -> R<i32> {
        self.skip_spaces();
        let mut neg = false;
        while let Some(Tok::Ch(c @ ('-' | '+'))) = self.peek().cloned() {
            if c == '-' {
                neg = !neg;
            }
            self.pos += 1;
            self.skip_spaces();
        }
        let mut v: i64 = 0;
        let mut any = false;
        while let Some(Tok::Ch(c)) = self.peek().cloned() {
            if let Some(d) = c.to_digit(10) {
                v = v * 10 + i64::from(d);
                any = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if !any {
            return err(format!("expected integer at {}", self.pos));
        }
        if self.peek() == Some(&Tok::Space) {
            self.pos += 1;
        }
        Ok(if neg { -(v as i32) } else { v as i32 })
    }

    fn scan_register(&mut self) -> R<u32> {
        self.skip_spaces();
        match self.peek().cloned() {
            Some(Tok::Cs(n)) if n == "Ba" => {
                self.pos += 1;
                Ok(BA)
            }
            Some(Tok::Cs(n)) if n == "Bb" => {
                self.pos += 1;
                Ok(BB)
            }
            _ => Ok(self.scan_int()? as u32),
        }
    }

    /// Internal dimension (for `<factor><internal>` and bare internals).
    fn internal_dimen(&mut self, e: &BoxEngine) -> R<Option<Scaled>> {
        self.skip_spaces();
        let Some(Tok::Cs(name)) = self.peek().cloned() else { return Ok(None) };
        let v = match name.as_str() {
            "wd" | "ht" | "dp" => {
                self.pos += 1;
                let n = self.scan_register()?;
                let which = match name.as_str() {
                    "wd" => BoxDim::Width,
                    "ht" => BoxDim::Height,
                    _ => BoxDim::Depth,
                };
                return Ok(Some(e.box_dimen(n, which)));
            }
            "width" => latex::width(e),
            "height" => latex::height(e),
            "depth" => latex::depth(e),
            "totalheight" => latex::totalheight(e),
            "lastkern" => e.last_kern(),
            "lastskip" => e.last_skip().width,
            n if dimen_param(n) => e.dimen(n),
            n if skip_param(n) => e.skip(n).0.width,
            _ => return Ok(None),
        };
        self.pos += 1;
        Ok(Some(v))
    }

    /// `scan_dimen` for the fixture subset.
    fn scan_dimen(&mut self, e: &BoxEngine) -> R<Scaled> {
        self.skip_spaces();
        let mut neg = false;
        while let Some(Tok::Ch(c @ ('-' | '+'))) = self.peek().cloned() {
            if c == '-' {
                neg = !neg;
            }
            self.pos += 1;
            self.skip_spaces();
        }
        if let Some(v) = self.internal_dimen(e)? {
            return Ok(if neg { -v } else { v });
        }
        let (int, frac) = self.scan_decimal()?;
        if let Some(v) = self.internal_dimen(e)? {
            return scale_internal(neg, int, &frac, v).map_err(|_| DslError("arith".into()));
        }
        let (unit, fil) = self.scan_unit(false)?;
        assert!(fil.is_none());
        dimen_from_parts(neg, int, &frac, unit).map_err(|_| DslError("dimen too large".into()))
    }

    fn scan_decimal(&mut self) -> R<(i32, Vec<u8>)> {
        let mut int: i64 = 0;
        let mut frac = Vec::new();
        let mut any = false;
        while let Some(Tok::Ch(c)) = self.peek().cloned() {
            if let Some(d) = c.to_digit(10) {
                int = int * 10 + i64::from(d);
                any = true;
                self.pos += 1;
            } else {
                break;
            }
        }
        if let Some(Tok::Ch('.' | ',')) = self.peek() {
            self.pos += 1;
            while let Some(Tok::Ch(c)) = self.peek().cloned() {
                if let Some(d) = c.to_digit(10) {
                    frac.push(d as u8);
                    any = true;
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }
        if !any {
            return err(format!("expected number at {} ({:?})", self.pos, self.peek()));
        }
        Ok((int as i32, frac))
    }

    /// Unit keyword; with `allow_fil`, returns a fil order instead.
    fn scan_unit(&mut self, allow_fil: bool) -> R<(Unit, Option<GlueOrder>)> {
        self.skip_spaces();
        if allow_fil && self.keyword("fil") {
            let mut order = GlueOrder::Fil;
            while self.keyword("l") {
                order = match order {
                    GlueOrder::Fil => GlueOrder::Fill,
                    _ => GlueOrder::Filll,
                };
            }
            if self.peek() == Some(&Tok::Space) {
                self.pos += 1;
            }
            return Ok((Unit::Pt, Some(order)));
        }
        for (kw, unit) in [
            ("pt", Unit::Pt),
            ("in", Unit::In),
            ("pc", Unit::Pc),
            ("cm", Unit::Cm),
            ("mm", Unit::Mm),
            ("bp", Unit::Bp),
            ("dd", Unit::Dd),
            ("cc", Unit::Cc),
            ("sp", Unit::Sp),
        ] {
            if self.keyword(kw) {
                if self.peek() == Some(&Tok::Space) {
                    self.pos += 1;
                }
                return Ok((unit, None));
            }
        }
        err(format!("unknown unit at {} ({:?})", self.pos, self.peek()))
    }

    fn scan_stretch(&mut self, e: &BoxEngine) -> R<(Scaled, GlueOrder)> {
        self.skip_spaces();
        let mut neg = false;
        while let Some(Tok::Ch(c @ ('-' | '+'))) = self.peek().cloned() {
            if c == '-' {
                neg = !neg;
            }
            self.pos += 1;
        }
        if let Some(v) = self.internal_dimen(e)? {
            return Ok((if neg { -v } else { v }, GlueOrder::Normal));
        }
        let (int, frac) = self.scan_decimal()?;
        let (unit, fil) = self.scan_unit(true)?;
        match fil {
            Some(order) => {
                let v = dimen_from_parts(neg, int, &frac, Unit::Pt).map_err(|_| DslError("fil".into()))?;
                Ok((v, order))
            }
            None => Ok((dimen_from_parts(neg, int, &frac, unit).map_err(|_| DslError("dimen".into()))?, GlueOrder::Normal)),
        }
    }

    /// `scan_glue`; returns the spec and whether it is a `zero_glue` reference.
    fn scan_glue(&mut self, e: &BoxEngine) -> R<(GlueSpec, bool)> {
        self.skip_spaces();
        if let Some(Tok::Cs(name)) = self.peek().cloned() {
            if skip_param(&name) {
                self.pos += 1;
                if name == "fill" {
                    return Ok((GlueSpec { stretch: UNITY, stretch_order: GlueOrder::Fill, ..GlueSpec::ZERO }, false));
                }
                return Ok(e.skip(&name));
            }
            if name == "lastskip" {
                self.pos += 1;
                let s = e.last_skip();
                return Ok((s, s.is_zero()));
            }
            if name == "z@skip" {
                self.pos += 1;
                return Ok((GlueSpec::ZERO, true));
            }
        }
        let width = self.scan_dimen(e)?;
        let mut spec = GlueSpec::fixed(width);
        if self.keyword("plus") {
            let (s, o) = self.scan_stretch(e)?;
            spec.stretch = s;
            spec.stretch_order = o;
        }
        if self.keyword("minus") {
            let (s, o) = self.scan_stretch(e)?;
            spec.shrink = s;
            spec.shrink_order = o;
        }
        Ok((spec, false))
    }

    fn scan_spec(&mut self, e: &BoxEngine) -> R<PackSpec> {
        let spec = if self.keyword("to") {
            PackSpec::Exactly(self.scan_dimen(e)?)
        } else if self.keyword("spread") {
            PackSpec::Additional(self.scan_dimen(e)?)
        } else {
            PackSpec::NATURAL
        };
        self.expect_ch('{')?;
        Ok(spec)
    }

    fn scan_rule(&mut self, e: &BoxEngine, mut rule: Rule) -> R<Rule> {
        loop {
            if self.keyword("width") {
                rule.width = self.scan_dimen(e)?;
            } else if self.keyword("height") {
                rule.height = self.scan_dimen(e)?;
            } else if self.keyword("depth") {
                rule.depth = self.scan_dimen(e)?;
            } else {
                return Ok(rule);
            }
        }
    }

    /// `scan_box` (§1084): a box command in the given context.
    fn scan_box(&mut self, e: &mut BoxEngine, context: BoxContext) -> R {
        self.skip_spaces();
        let Some(Tok::Cs(name)) = self.next() else { return err("expected box") };
        match name.as_str() {
            "hbox" | "vbox" | "vtop" => {
                let kind = match name.as_str() {
                    "hbox" => BoxKind::HBox,
                    "vbox" => BoxKind::VBox,
                    _ => BoxKind::VTop,
                };
                let spec = self.scan_spec(e)?;
                e.begin_box(kind, spec, context)?;
                self.run_until_close(e)?;
                e.end_box()?;
            }
            "box" => {
                let n = self.scan_register()?;
                e.use_box(n, context)?;
            }
            "copy" => {
                let n = self.scan_register()?;
                e.copy_box(n, context)?;
            }
            "lastbox" => e.last_box(context)?,
            "null" => {
                e.begin_box(BoxKind::HBox, PackSpec::NATURAL, context)?;
                e.end_box()?;
            }
            "vrule" | "hrule" => {
                if let BoxContext::Leaders(kind) = context {
                    let base = if name == "vrule" { Rule::vrule() } else { Rule::hrule() };
                    let rule = self.scan_rule(e, base)?;
                    e.leaders_rule(kind, rule);
                } else {
                    return err("rule outside leaders");
                }
            }
            other => return err(format!("unsupported box command \\{other}")),
        }
        Ok(())
    }

    /// Runs until the matching `}` of a box group has been consumed.
    fn run_until_close(&mut self, e: &mut BoxEngine) -> R {
        // Simple `{...}` groups opened inside the box are closed by their own
        // `}` (end_group); only the brace at depth 0 ends the box.
        let mut depth = 0usize;
        loop {
            match self.peek() {
                None => return err("missing }"),
                Some(Tok::Ch('{')) => {
                    depth += 1;
                    self.step(e)?;
                }
                Some(Tok::Ch('}')) if depth > 0 => {
                    depth -= 1;
                    self.step(e)?;
                }
                Some(Tok::Ch('}')) => {
                    self.pos += 1;
                    return Ok(());
                }
                _ => self.step(e)?,
            }
        }
    }

    fn run_until_end(&mut self, e: &mut BoxEngine) -> R {
        while self.peek().is_some() {
            self.step(e)?;
        }
        Ok(())
    }

    fn content<'s>(&'s self, toks: Vec<Tok>) -> impl FnMut(&mut BoxEngine) -> Result<(), flashtex_tex_boxes::engine::BoxError> + 's {
        move |e: &mut BoxEngine| {
            let mut sub = self.sub(toks.clone());
            sub.run_until_end(e).map_err(|d| flashtex_tex_boxes::engine::BoxError::Tex(d.0))
        }
    }

    fn len_arg(&self, toks: Vec<Tok>) -> impl Fn(&BoxEngine) -> Scaled + '_ {
        move |e: &BoxEngine| {
            let mut sub = self.sub(toks.clone());
            sub.scan_dimen(e).expect("length argument")
        }
    }

    fn glue_arg(&self, e: &BoxEngine, toks: Vec<Tok>) -> R<GlueSpec> {
        let mut sub = self.sub(toks);
        Ok(sub.scan_glue(e)?.0)
    }

    fn single_char(toks: &Option<Vec<Tok>>) -> Option<char> {
        toks.as_ref().and_then(|t| match t.as_slice() {
            [Tok::Ch(c)] => Some(*c),
            _ => None,
        })
    }

    fn env_name(&mut self) -> R<String> {
        let g = self.group()?;
        Ok(g.iter()
            .map(|t| match t {
                Tok::Ch(c) => *c,
                _ => '?',
            })
            .collect())
    }

    /// Collects tokens up to `\end{name}`.
    fn env_body(&mut self, name: &str) -> R<Vec<Tok>> {
        let mut out = Vec::new();
        let mut depth = 0;
        loop {
            match self.next() {
                None => return err(format!("missing \\end{{{name}}}")),
                Some(Tok::Cs(cs)) if cs == "begin" => {
                    depth += 1;
                    out.push(Tok::Cs(cs));
                }
                Some(Tok::Cs(cs)) if cs == "end" => {
                    if depth == 0 {
                        let n = self.env_name()?;
                        assert_eq!(n, name);
                        return Ok(out);
                    }
                    depth -= 1;
                    out.push(Tok::Cs(cs));
                }
                Some(t) => out.push(t),
            }
        }
    }

    fn step(&mut self, e: &mut BoxEngine) -> R {
        let t = self.next().expect("token");
        match t {
            Tok::Space => {
                if e.mode().is_vertical() {
                    Ok(())
                } else {
                    err(format!("space token in horizontal mode at {}", self.pos))
                }
            }
            Tok::Ch('{') => {
                e.begin_group();
                Ok(())
            }
            Tok::Ch('}') => Ok(e.end_group()?),
            Tok::Ch(c) => err(format!("unexpected character '{c}' at {}", self.pos)),
            Tok::Cs(name) => self.command(e, &name),
        }
    }

    fn command(&mut self, e: &mut BoxEngine, name: &str) -> R {
        match name {
            "begingroup" => e.begin_group(),
            "endgroup" => e.end_group()?,
            "relax" => {}
            "POS" => {
                if self.with_pos {
                    e.whatsit(1, "pdfsavepos");
                }
            }
            "global" => {
                self.skip_spaces();
                match self.next() {
                    Some(Tok::Cs(n)) if n == "setbox" => {
                        let reg = self.scan_register()?;
                        self.optional_equals();
                        self.scan_box(e, BoxContext::SetBox { register: reg, global: true })?;
                    }
                    other => return err(format!("unsupported \\global {other:?}")),
                }
            }
            "setbox" => {
                let reg = self.scan_register()?;
                self.optional_equals();
                self.scan_box(e, BoxContext::SetBox { register: reg, global: false })?;
            }
            "hbox" | "vbox" | "vtop" | "box" | "copy" | "lastbox" => {
                self.pos -= 1;
                self.scan_box(e, BoxContext::APPEND)?;
            }
            "raise" | "lower" | "moveleft" | "moveright" => {
                let d = self.scan_dimen(e)?;
                let shift = if name == "raise" || name == "moveleft" { -d } else { d };
                if (name == "raise" || name == "lower") == e.mode().is_vertical() {
                    return err(format!("You can't use \\{name} in this mode"));
                }
                self.scan_box(e, BoxContext::Shift(shift))?;
            }
            "leaders" | "cleaders" | "xleaders" => {
                let kind = match name {
                    "leaders" => LeaderKind::Aligned,
                    "cleaders" => LeaderKind::Centered,
                    _ => LeaderKind::Expanded,
                };
                self.scan_box(e, BoxContext::Leaders(kind))?;
            }
            "vrule" => {
                let r = self.scan_rule(e, Rule::vrule())?;
                e.vrule(r)?;
            }
            "hrule" => {
                let r = self.scan_rule(e, Rule::hrule())?;
                e.hrule(r)?;
            }
            "hskip" => {
                let (g, z) = self.scan_glue(e)?;
                e.hskip(g, z)?;
            }
            "vskip" => {
                let (g, z) = self.scan_glue(e)?;
                e.vskip(g, z)?;
            }
            "hfil" => e.hskip(GlueSpec::FIL, false)?,
            "hfill" => e.hskip(GlueSpec::FILL, false)?,
            "hss" => e.hskip(GlueSpec::SS, false)?,
            "hfilneg" => e.hskip(GlueSpec::FIL_NEG, false)?,
            "vfil" => e.vskip(GlueSpec::FIL, false)?,
            "vfill" => e.vskip(GlueSpec::FILL, false)?,
            "vss" => e.vskip(GlueSpec::SS, false)?,
            "vfilneg" => e.vskip(GlueSpec::FIL_NEG, false)?,
            "kern" => {
                let d = self.scan_dimen(e)?;
                e.kern(d);
            }
            "penalty" => {
                let n = self.scan_int()?;
                e.penalty(n);
            }
            "unhbox" | "unhcopy" | "unvbox" | "unvcopy" => {
                let n = self.scan_register()?;
                e.unpackage(n, name.ends_with("copy"), name.starts_with("unv"))?;
            }
            "unskip" => e.unskip(),
            "unkern" => e.unkern(),
            "unpenalty" => e.unpenalty(),
            "wd" | "ht" | "dp" => {
                let n = self.scan_register()?;
                self.optional_equals();
                let d = self.scan_dimen(e)?;
                let which = match name {
                    "wd" => BoxDim::Width,
                    "ht" => BoxDim::Height,
                    _ => BoxDim::Depth,
                };
                e.set_box_dimen(n, which, d);
            }
            "par" => e.par()?,
            "indent" => e.indent(),
            "noindent" => e.noindent(),
            "leavevmode" => e.leavevmode(),
            n if int_param(n) => {
                self.optional_equals();
                let v = self.scan_int()?;
                e.set_int(n, v, false);
            }
            n if dimen_param(n) => {
                self.optional_equals();
                let v = self.scan_dimen(e)?;
                e.set_dimen(n, v, false);
            }
            n if skip_param(n) => {
                self.optional_equals();
                let (g, _) = self.scan_glue(e)?;
                e.set_skip(n, g, false);
            }
            // ----- LaTeX -----
            "mbox" => {
                let body = self.group()?;
                latex::mbox(e, &mut self.content(body))?;
            }
            "makebox" | "framebox" => {
                let w = self.optional()?;
                let p = if w.is_some() { self.optional()? } else { None };
                let body = self.group()?;
                let pos = Self::single_char(&p);
                let wf = w.map(|t| self.len_arg(t));
                let wref = wf.as_ref().map(|f| f as &dyn Fn(&BoxEngine) -> Scaled);
                if name == "makebox" {
                    latex::makebox(e, wref, pos, &mut self.content(body))?;
                } else {
                    latex::framebox(e, wref, pos, &mut self.content(body))?;
                }
            }
            "fbox" => {
                let body = self.group()?;
                latex::fbox(e, &mut self.content(body))?;
            }
            "raisebox" => {
                let lift = self.group()?;
                let h = self.optional()?;
                let d = if h.is_some() { self.optional()? } else { None };
                let body = self.group()?;
                let lf = self.len_arg(lift);
                let empty = |t: &Option<Vec<Tok>>| t.as_ref().is_none_or(|v| v.is_empty());
                let hf = if empty(&h) { None } else { Some(self.len_arg(h.clone().unwrap())) };
                let df = if empty(&d) { None } else { Some(self.len_arg(d.clone().unwrap())) };
                latex::raisebox(
                    e,
                    &lf,
                    hf.as_ref().map(|f| f as &dyn Fn(&BoxEngine) -> Scaled),
                    df.as_ref().map(|f| f as &dyn Fn(&BoxEngine) -> Scaled),
                    &mut self.content(body),
                )?;
            }
            "parbox" => {
                let p = self.optional()?;
                let h = if p.is_some() { self.optional()? } else { None };
                let i = if h.is_some() { self.optional()? } else { None };
                let w = self.group()?;
                let body = self.group()?;
                let wf = self.len_arg(w);
                let hf = h.map(|t| self.len_arg(t));
                latex::parbox(
                    e,
                    Self::single_char(&p),
                    hf.as_ref().map(|f| f as &dyn Fn(&BoxEngine) -> Scaled),
                    Self::single_char(&i),
                    &wf,
                    &mut self.content(body),
                )?;
            }
            "begin" => {
                let env = self.env_name()?;
                match env.as_str() {
                    "minipage" => {
                        let p = self.optional()?;
                        let h = if p.is_some() { self.optional()? } else { None };
                        let i = if h.is_some() { self.optional()? } else { None };
                        let w = self.group()?;
                        let body = self.env_body("minipage")?;
                        let wf = self.len_arg(w);
                        let hf = h.map(|t| self.len_arg(t));
                        e.begin_group(); // \begin
                        latex::minipage(
                            e,
                            Self::single_char(&p),
                            hf.as_ref().map(|f| f as &dyn Fn(&BoxEngine) -> Scaled),
                            Self::single_char(&i),
                            &wf,
                            &mut self.content(body),
                            None,
                        )?;
                        e.end_group()?; // \end
                    }
                    "lrbox" => {
                        let reg = {
                            let g = self.group()?;
                            self.sub(g).scan_register()?
                        };
                        let body = self.env_body("lrbox")?;
                        latex::lrbox(e, reg, &mut self.content(body))?;
                    }
                    other => return err(format!("unsupported environment {other}")),
                }
            }
            "rule" => {
                let r = self.optional()?;
                let w = self.group()?;
                let h = self.group()?;
                let raise = match r {
                    Some(t) => self.sub(t).scan_dimen(e)?,
                    None => 0,
                };
                let wv = self.sub(w).scan_dimen(e)?;
                let hv = self.sub(h).scan_dimen(e)?;
                latex::rule(e, raise, wv, hv)?;
            }
            "hspace" | "vspace" => {
                self.skip_spaces();
                let star = self.peek() == Some(&Tok::Ch('*'));
                if star {
                    self.pos += 1;
                }
                let g = self.group()?;
                let spec = self.glue_arg(e, g)?;
                match (name, star) {
                    ("hspace", false) => latex::hspace(e, spec)?,
                    ("hspace", true) => latex::hspace_star(e, spec)?,
                    ("vspace", false) => latex::vspace(e, spec)?,
                    _ => latex::vspace_star(e, spec)?,
                }
            }
            "strut" => latex::strut(e)?,
            "phantom" | "hphantom" | "vphantom" => {
                let body = self.group()?;
                let (v, h) = match name {
                    "phantom" => (true, true),
                    "hphantom" => (false, true),
                    _ => (true, false),
                };
                latex::phantom(e, v, h, &mut self.content(body))?;
            }
            "smash" => {
                let body = self.group()?;
                latex::smash(e, &mut self.content(body))?;
            }
            "llap" | "rlap" | "clap" => {
                let body = self.group()?;
                let (l, r) = match name {
                    "llap" => (true, false),
                    "rlap" => (false, true),
                    _ => (true, true),
                };
                latex::lap(e, l, r, &mut self.content(body))?;
            }
            "null" => {
                self.pos -= 1;
                self.scan_box(e, BoxContext::APPEND)?;
            }
            "centering" => latex::centering(e),
            "raggedright" => latex::raggedright(e),
            "raggedleft" => latex::raggedleft(e),
            "sbox" => {
                let reg = {
                    let g = self.group()?;
                    self.sub(g).scan_register()?
                };
                let body = self.group()?;
                latex::sbox(e, reg, &mut self.content(body))?;
            }
            "savebox" => {
                let reg = {
                    let g = self.group()?;
                    self.sub(g).scan_register()?
                };
                let w = self.optional()?;
                let p = if w.is_some() { self.optional()? } else { None };
                let body = self.group()?;
                let wf = w.map(|t| self.len_arg(t));
                latex::savebox(
                    e,
                    reg,
                    wf.as_ref().map(|f| f as &dyn Fn(&BoxEngine) -> Scaled),
                    Self::single_char(&p),
                    &mut self.content(body),
                )?;
            }
            "usebox" => {
                let reg = {
                    let g = self.group()?;
                    self.sub(g).scan_register()?
                };
                latex::usebox(e, reg)?;
            }
            "settowidth" | "settoheight" | "settodepth" => {
                let target = self.group()?;
                let Some(Tok::Cs(tname)) = target.first().cloned() else { return err("settodim target") };
                let body = self.group()?;
                let which = match name {
                    "settowidth" => BoxDim::Width,
                    "settoheight" => BoxDim::Height,
                    _ => BoxDim::Depth,
                };
                // \La is a skip register (\newlength); assign via a scratch dimen.
                latex::settodim(e, which, "@settodim-scratch", &mut self.content(body))?;
                let v = e.dimen("@settodim-scratch");
                e.set_skip(&tname, GlueSpec::fixed(v), false);
            }
            other => return err(format!("unsupported command \\{other}")),
        }
        Ok(())
    }
}

#[allow(dead_code)]
fn _unused(_: KindMarker) {}
#[allow(dead_code)]
enum KindMarker {}
