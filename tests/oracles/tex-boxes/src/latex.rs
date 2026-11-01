//! LaTeX box commands, executed step by step as their `latex.ltx` definitions
//! (TeX Live 2026, `tex/latex/base/latex.ltx`; line numbers cited) expand
//! into primitives on a [`BoxEngine`].
//!
//! Content arguments are closures that run the argument's material on the
//! engine; length arguments are closures evaluated at the moment LaTeX's
//! `\setlength` would read them, so `\width`, `\height`, `\depth` and
//! `\totalheight` (see [`width`] etc.) work inside `\makebox`, `\framebox`,
//! `\raisebox` and `\parbox` exactly as in LaTeX.

use crate::engine::{BoxContext, BoxDim, BoxEngine, BoxKind, BoxResult, Mode};
use crate::node::{BoxNode, GlueOrder, GlueSpec, ListKind, Rule};
use crate::pack::PackSpec;
use crate::scaled::{MAX_DIMEN, Scaled, UNITY, scale_internal};

/// `\@tempboxa` (allocated by `\newbox` in latex.ltx; any private number works).
pub const TEMPBOXA: u32 = 0x1_0000;
/// `\strutbox` (latex.ltx line 620).
pub const STRUTBOX: u32 = 0x1_0001;

/// A length argument evaluated lazily.
pub type LenArg<'a> = &'a dyn Fn(&BoxEngine) -> Scaled;
/// A content argument.
pub type Content<'a> = &'a mut dyn FnMut(&mut BoxEngine) -> BoxResult;

/// `\width` inside `\@begin@tempboxa` (latex.ltx line 16088).
pub fn width(e: &BoxEngine) -> Scaled {
    e.box_dimen(TEMPBOXA, BoxDim::Width)
}
/// `\height` (line 16089).
pub fn height(e: &BoxEngine) -> Scaled {
    e.box_dimen(TEMPBOXA, BoxDim::Height)
}
/// `\depth` (line 16090).
pub fn depth(e: &BoxEngine) -> Scaled {
    e.box_dimen(TEMPBOXA, BoxDim::Depth)
}
/// `\totalheight` (lines 16091–16093).
pub fn totalheight(e: &BoxEngine) -> Scaled {
    e.dimen("@ovri")
}

/// `\@flushglue` = `0pt plus 1fil` (line 8732).
pub const FLUSHGLUE: GlueSpec = GlueSpec { stretch: UNITY, stretch_order: GlueOrder::Fil, ..GlueSpec::ZERO };

/// Parameters of `article` at `10pt` after `\begin{document}` (latex.ltx
/// lines 493–532, 546–547; size10.clo; article.cls lines 449–450).
pub fn setup_article_10pt(e: &mut BoxEngine) {
    e.set_int("hbadness", 1000, true);
    e.set_int("vbadness", 1000, true);
    e.set_int("showboxbreadth", -1, true);
    e.set_int("showboxdepth", -1, true);
    e.set_dimen("hfuzz", 6554, true);
    e.set_dimen("vfuzz", 6554, true);
    e.set_dimen("overfullrule", 0, true);
    e.set_dimen("boxmaxdepth", MAX_DIMEN, true);
    e.set_dimen("fboxsep", 3 * UNITY, true);
    e.set_dimen("fboxrule", 26214, true);
    e.set_dimen("parindent", 15 * UNITY, true);
    e.set_dimen("hsize", 345 * UNITY, true);
    e.set_dimen("textwidth", 345 * UNITY, true);
    e.set_dimen("linewidth", 345 * UNITY, true);
    e.set_dimen("columnwidth", 345 * UNITY, true);
    e.set_dimen("lineskiplimit", 0, true);
    e.set_dimen("normallineskiplimit", 0, true);
    e.set_dimen("mathsurround", 0, true);
    e.set_skip("baselineskip", GlueSpec::fixed(12 * UNITY), true);
    e.set_skip("normalbaselineskip", GlueSpec::fixed(12 * UNITY), true);
    e.set_skip("lineskip", GlueSpec::fixed(UNITY), true);
    e.set_skip("normallineskip", GlueSpec::fixed(UNITY), true);
    e.set_skip("parskip", GlueSpec { stretch: UNITY, ..GlueSpec::ZERO }, true);
    e.set_skip("parfillskip", FLUSHGLUE, true);
    e.axis_height = 163840; // cmsy10 \fontdimen22 at 10pt
    e.latex_para_hooks = true;
    size_update_strut(e, true);
}

/// `\size@update`'s `\strutbox` (latex.ltx lines 12596–12599).
pub fn size_update_strut(e: &mut BoxEngine, global: bool) {
    let bs = e.skip("baselineskip").0.width;
    let h = scale_internal(false, 0, &[7], bs).unwrap_or(0);
    let d = scale_internal(false, 0, &[3], bs).unwrap_or(0);
    let _ = e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register: STRUTBOX, global });
    let _ = e.vrule(Rule { width: 0, height: h, depth: d });
    let _ = e.end_box();
}

/// `\color@begingroup`/`\color@setgroup` without the color package (line 16114).
fn color_begingroup(e: &mut BoxEngine) {
    e.begin_group();
}
/// `\color@endgroup` = `\endgraf\@endpefalse\endgroup` (line 16121).
fn color_endgroup(e: &mut BoxEngine) -> BoxResult {
    e.par()?;
    e.end_group()
}

/// `\@begin@tempboxa#1#2` (lines 16085–16093).
fn begin_tempboxa(e: &mut BoxEngine, kind: BoxKind, content: Content) -> BoxResult {
    e.begin_group();
    e.begin_box(kind, PackSpec::NATURAL, BoxContext::SetBox { register: TEMPBOXA, global: false })?;
    color_begingroup(e);
    content(e)?;
    color_endgroup(e)?;
    e.end_box()?;
    let th = height(e) + depth(e);
    e.set_dimen("@ovri", th, false);
    Ok(())
}

/// `\@end@tempboxa` (line 16094).
fn end_tempboxa(e: &mut BoxEngine) -> BoxResult {
    e.end_group()
}

fn warn(e: &mut BoxEngine, msg: &str) {
    // \GenericWarning: \immediate\write\@unused{^^J...\on@line.^^J}. write_out
    // starts with print_nl(""), a newline when the terminal or log is mid-line.
    let nl = if e.term_offset > 0 || e.log_mid_line() { "\n" } else { "" };
    let text = format!("{nl}\nLaTeX Warning: {msg} on input line {}.\n\n", e.line);
    e.log_text(&text);
    e.term_offset = 0;
    e.warnings.push(msg.to_string());
}

/// `\bm@c`, `\bm@l`, `\bm@r`, `\bm@s` (and `t`=`l`, `b`=`r`), lines 16095–16098.
/// With `vertical`, `\hss`→`\vss` and `\unhbox`→`\unvbox` as in `\@iiiparbox`.
/// Returns false for an unknown position (caller warns after `\bm@c`).
fn bm(e: &mut BoxEngine, pos: char, vertical: bool) -> BoxResult<bool> {
    let ss = |e: &mut BoxEngine| if vertical { e.vskip(GlueSpec::SS, false) } else { e.hskip(GlueSpec::SS, false) };
    let (lead, trail, known) = match pos {
        'c' => (true, true, true),
        'l' | 't' => (false, true, true),
        'r' | 'b' => (true, false, true),
        's' => (false, false, true),
        _ => (true, true, false),
    };
    if lead {
        ss(e)?;
    }
    e.unpackage(TEMPBOXA, false, vertical)?;
    if trail {
        ss(e)?;
    }
    Ok(known)
}

/// `\mbox{#1}` (line 16082).
pub fn mbox(e: &mut BoxEngine, content: Content) -> BoxResult {
    e.leavevmode();
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::APPEND)?;
    content(e)?;
    e.end_box()
}

/// `\makebox[#1][#2]{#3}` (lines 16077–16109); `width=None` is `\mbox`.
pub fn makebox(e: &mut BoxEngine, width: Option<LenArg>, pos: Option<char>, content: Content) -> BoxResult {
    e.leavevmode();
    match width {
        None => mbox(e, content),
        Some(w) => imakebox(e, w, pos.unwrap_or('c'), content),
    }
}

/// `\@imakebox[#1][#2]#3` (lines 16099–16109).
pub fn imakebox(e: &mut BoxEngine, w: LenArg, pos: char, content: Content) -> BoxResult {
    begin_tempboxa(e, BoxKind::HBox, content)?;
    let wd = w(e);
    e.set_dimen("@tempdima", wd, false);
    e.begin_box(BoxKind::HBox, PackSpec::Exactly(wd), BoxContext::APPEND)?;
    if !bm(e, pos, false)? {
        warn(e, &format!("Unexpected alignment {pos}"));
    }
    e.end_box()?;
    end_tempboxa(e)
}

/// `\fbox{#1}` (lines 16182–16188).
pub fn fbox(e: &mut BoxEngine, content: Content) -> BoxResult {
    e.leavevmode();
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register: TEMPBOXA, global: false })?;
    color_begingroup(e);
    let sep = e.dimen("fboxsep");
    e.kern(sep);
    e.begin_group();
    content(e)?;
    e.end_group()?;
    e.kern(sep);
    color_endgroup(e)?;
    e.end_box()?;
    frameb_x(e, None)
}

/// `\framebox[#1][#2]{#3}` (lines 16189–16210); `width=None` is `\fbox`.
pub fn framebox(e: &mut BoxEngine, width: Option<LenArg>, pos: Option<char>, content: Content) -> BoxResult {
    let Some(w) = width else { return fbox(e, content) };
    let pos = pos.unwrap_or('c');
    e.leavevmode();
    begin_tempboxa(e, BoxKind::HBox, content)?;
    let wd = w(e);
    e.set_dimen("@tempdima", wd, false);
    e.begin_box(BoxKind::HBox, PackSpec::Exactly(wd), BoxContext::SetBox { register: TEMPBOXA, global: false })?;
    let sep = e.dimen("fboxsep");
    e.kern(sep);
    if !bm(e, pos, false)? {
        warn(e, &format!("Unexpected alignment {pos}"));
    }
    e.kern(sep);
    e.end_box()?;
    let rule = e.dimen("fboxrule");
    frameb_x(e, Some(-rule))?;
    end_tempboxa(e)
}

/// `\@frameb@x#1` (lines 16211–16230); `kern` is `#1` (`\relax` = None).
fn frameb_x(e: &mut BoxEngine, kern: Option<Scaled>) -> BoxResult {
    let rule = e.dimen("fboxrule");
    let sep = e.dimen("fboxsep");
    let lower = rule + sep + e.box_dimen(TEMPBOXA, BoxDim::Depth);
    e.set_dimen("@tempdima", lower, false);
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::APPEND)?;
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::Shift(lower))?;
    e.begin_box(BoxKind::VBox, PackSpec::NATURAL, BoxContext::APPEND)?;
    e.hrule(Rule { height: rule, ..Rule::hrule() })?;
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::APPEND)?;
    e.vrule(Rule { width: rule, ..Rule::vrule() })?;
    if let Some(k) = kern {
        e.kern(k);
    }
    e.begin_box(BoxKind::VBox, PackSpec::NATURAL, BoxContext::APPEND)?;
    e.vskip(GlueSpec::fixed(sep), false)?;
    e.use_box(TEMPBOXA, BoxContext::APPEND)?;
    e.vskip(GlueSpec::fixed(sep), false)?;
    e.end_box()?;
    if let Some(k) = kern {
        e.kern(k);
    }
    e.vrule(Rule { width: rule, ..Rule::vrule() })?;
    e.end_box()?;
    e.hrule(Rule { height: rule, ..Rule::hrule() })?;
    e.end_box()?;
    e.end_box()?;
    e.end_box()
}

/// `\raisebox{#1}[#2][#3]{#4}` (lines 16373–16396).
pub fn raisebox(e: &mut BoxEngine, lift: LenArg, ht: Option<LenArg>, dp: Option<LenArg>, content: Content) -> BoxResult {
    e.leavevmode();
    begin_tempboxa(e, BoxKind::HBox, content)?;
    let l = lift(e);
    e.set_dimen("@tempdima", l, false);
    let h = ht.map(|f| f(e));
    if let Some(h) = h {
        e.set_dimen("@tempdimb", h, false);
    }
    let d = if h.is_some() { dp.map(|f| f(e)) } else { None };
    if let Some(d) = d {
        e.set_dimen("dimen@", d, false);
    }
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register: TEMPBOXA, global: false })?;
    e.use_box(TEMPBOXA, BoxContext::Shift(-l))?;
    e.end_box()?;
    if let Some(h) = h {
        e.set_box_dimen(TEMPBOXA, BoxDim::Height, h);
    }
    if let Some(d) = d {
        e.set_box_dimen(TEMPBOXA, BoxDim::Depth, d);
    }
    e.use_box(TEMPBOXA, BoxContext::APPEND)?;
    end_tempboxa(e)
}

/// `\@arrayparboxrestore` + `\let\\\@normalcr` (lines 16272–16288).
pub fn parboxrestore(e: &mut BoxEngine) {
    e.set_dimen("parindent", 0, false);
    e.set_skip("parskip", GlueSpec::ZERO, false);
    e.set_int("everypar-minipagefalse", 0, false); // \everypar{}
    let hs = e.dimen("hsize");
    e.set_dimen("linewidth", hs, false);
    e.set_dimen("@totalleftmargin", 0, false);
    e.set_skip("leftskip", GlueSpec::ZERO, false);
    e.set_skip("rightskip", GlueSpec::ZERO, false);
    e.set_skip("@rightskip", GlueSpec::ZERO, false);
    e.set_skip("parfillskip", FLUSHGLUE, false);
    let (nls, _) = e.skip("normallineskip");
    e.set_skip("lineskip", nls, false);
    let nlsl = e.dimen("normallineskiplimit");
    e.set_dimen("lineskiplimit", nlsl, false);
    let (nbs, _) = e.skip("normalbaselineskip");
    e.set_skip("baselineskip", nbs, false);
    // \sloppy (line 18332): \tolerance 9999 \emergencystretch 3em
    e.set_int("tolerance", 9999, false);
}

/// `\parbox[#1][#2][#3]{#4}{#5}` (lines 16236–16271). `pos`: `c`/`t`/`b`;
/// `height`: the optional height; `inner`: the inner position (defaults
/// per `\@iiparbox`: `s` without height, `pos` with height).
pub fn parbox(
    e: &mut BoxEngine,
    pos: Option<char>,
    height_arg: Option<LenArg>,
    inner: Option<char>,
    width_arg: LenArg,
    content: Content,
) -> BoxResult {
    let pos = pos.unwrap_or('c');
    let inner = match (height_arg.is_some(), inner) {
        (_, Some(i)) => i,
        (true, None) => pos,
        (false, None) => 's',
    };
    iiiparbox(e, pos, height_arg, inner, width_arg, content)
}

/// `\@iiiparbox#1#2[#3]#4#5` (lines 16249–16271).
fn iiiparbox(e: &mut BoxEngine, pos: char, height_arg: Option<LenArg>, inner: char, width_arg: LenArg, content: Content) -> BoxResult {
    e.leavevmode();
    let mut pboxsw = false;
    let wd = width_arg(e);
    e.set_dimen("@tempdima", wd, false);
    begin_tempboxa(e, BoxKind::VBox, &mut |e: &mut BoxEngine| {
        e.set_dimen("hsize", wd, false);
        parboxrestore(e);
        content(e)?;
        e.par()
    })?;
    let spec = match height_arg {
        Some(h) => {
            let h = h(e);
            e.set_dimen("@tempdimb", h, false);
            PackSpec::Exactly(h)
        }
        None => PackSpec::NATURAL,
    };
    let centered = match pos {
        'b' => {
            e.begin_box(BoxKind::VBox, spec, BoxContext::APPEND)?;
            false
        }
        't' => {
            e.begin_box(BoxKind::VTop, spec, BoxContext::APPEND)?;
            false
        }
        _ => {
            if e.mode() != Mode::Math {
                pboxsw = true;
                e.begin_math()?;
            }
            e.begin_vcenter(spec)?;
            true
        }
    };
    if !bm(e, inner, true)? {
        warn(e, &format!("Unexpected alignment {inner}"));
    }
    if centered { e.end_vcenter()? } else { e.end_box()? }
    if pboxsw {
        e.set_dimen("mathsurround", 0, false); // \m@th
        e.end_math()?;
    }
    end_tempboxa(e)
}

/// Minipage footnote material (`\@mpfootins`, lines 16329–16341): the skip
/// `\skip\@mpfootins`, the `\footnoterule` material and the footnote vlist.
pub struct MinipageFootnotes<'a> {
    pub skip: GlueSpec,
    pub rule_and_notes: Content<'a>,
}

/// `\begin{minipage}[#1][#2][#3]{#4} ... \end{minipage}` (lines 16293–16341).
pub fn minipage(
    e: &mut BoxEngine,
    pos: Option<char>,
    height_arg: Option<LenArg>,
    inner: Option<char>,
    width_arg: LenArg,
    content: Content,
    footnotes: Option<MinipageFootnotes>,
) -> BoxResult {
    let pos = pos.unwrap_or('c');
    let inner = match (height_arg.is_some(), inner) {
        (_, Some(i)) => i,
        (true, None) => pos,
        (false, None) => 's',
    };
    e.leavevmode();
    let wd = width_arg(e);
    e.set_dimen("@tempdima", wd, false);
    e.begin_box(BoxKind::VBox, PackSpec::NATURAL, BoxContext::SetBox { register: TEMPBOXA, global: false })?;
    color_begingroup(e);
    e.set_dimen("hsize", wd, false);
    e.set_dimen("textwidth", wd, false);
    e.set_dimen("columnwidth", wd, false);
    parboxrestore(e);
    // \@setminipage (lines 17497–17500): \@minipagetrue\everypar{\@minipagefalse\everypar{}}
    e.set_int("@minipage", 1, true);
    e.set_int("everypar-minipagefalse", 1, false);
    content(e)?;
    // \endminipage
    e.par()?;
    e.unskip();
    if let Some(f) = footnotes {
        e.vskip(f.skip, false)?;
        (f.rule_and_notes)(e)?;
    }
    e.set_int("@minipage", 0, true);
    color_endgroup(e)?;
    e.end_box()?;
    iiiparbox(e, pos, height_arg, inner, &|_| wd, &mut |e: &mut BoxEngine| e.unpackage(TEMPBOXA, false, true))
}

/// `\rule[#1]{#2}{#3}` (lines 16359–16367).
pub fn rule(e: &mut BoxEngine, raise: Scaled, w: Scaled, h: Scaled) -> BoxResult {
    e.leavevmode();
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::APPEND)?;
    e.set_dimen("@tempdima", raise, false);
    e.set_dimen("@tempdimb", w, false);
    e.set_dimen("@tempdimc", h + raise, false);
    e.vrule(Rule { width: w, height: h + raise, depth: -raise })?;
    e.end_box()
}

/// `\hspace{#1}` (lines 9423–9425).
pub fn hspace(e: &mut BoxEngine, skip: GlueSpec) -> BoxResult {
    e.set_skip("sp@ce@skip", skip, false);
    e.skip_from_register("sp@ce@skip", false)
}

/// `\hspace*{#1}` (lines 9427–9428).
pub fn hspace_star(e: &mut BoxEngine, skip: GlueSpec) -> BoxResult {
    e.vrule(Rule { width: 0, ..Rule::vrule() })?;
    e.penalty(10000);
    hspace(e, skip)?;
    e.hskip(GlueSpec::ZERO, true)
}

/// `\vspace{#1}` (lines 9362–9373).
pub fn vspace(e: &mut BoxEngine, skip: GlueSpec) -> BoxResult {
    if e.mode().is_vertical() {
        e.set_skip("sp@ce@skip", skip, false);
        e.skip_from_register("sp@ce@skip", true)?;
        e.vskip(GlueSpec::ZERO, true)
    } else {
        let (savsk, savsf) = bsphack(e);
        e.begin_vadjust()?;
        e.set_skip("sp@ce@skip", skip, false);
        e.skip_from_register("sp@ce@skip", true)?;
        e.vskip(GlueSpec::ZERO, true)?;
        e.end_vadjust()?;
        esphack(e, savsk, savsf)
    }
}

/// `\vspace*{#1}` (lines 9374–9393).
pub fn vspace_star(e: &mut BoxEngine, skip: GlueSpec) -> BoxResult {
    if e.mode().is_vertical() {
        let pd = e.prev_depth();
        e.set_dimen("dimen@", pd, false);
        e.hrule(Rule { height: 0, ..Rule::hrule() })?;
        e.penalty(10000);
        e.set_skip("sp@ce@skip", skip, false);
        e.skip_from_register("sp@ce@skip", true)?;
        e.vskip(GlueSpec::ZERO, true)?;
        e.set_prev_depth(pd);
        Ok(())
    } else {
        let (savsk, savsf) = bsphack(e);
        e.begin_vadjust()?;
        e.hrule(Rule { height: 0, ..Rule::hrule() })?;
        e.penalty(10000);
        e.set_skip("sp@ce@skip", skip, false);
        e.skip_from_register("sp@ce@skip", true)?;
        e.vskip(GlueSpec::ZERO, true)?;
        e.end_vadjust()?;
        esphack(e, savsk, savsf)
    }
}

/// `\@bsphack` (lines 9276–9281).
fn bsphack(e: &mut BoxEngine) -> (Scaled, i32) {
    if e.mode().is_horizontal() { (e.last_skip().width, 1000) } else { (0, 0) }
}

/// `\@esphack` (lines 9282–9295), horizontal branch.
fn esphack(e: &mut BoxEngine, savsk: Scaled, _savsf: i32) -> BoxResult {
    if e.mode().is_horizontal() && savsk > 0 && e.last_skip().width == 0 {
        e.penalty(10000);
        e.hskip(GlueSpec::ZERO, true)?;
    }
    Ok(())
}

/// `\hfill`, `\hfil`, `\hss`, `\hfilneg` primitives.
pub fn hfill(e: &mut BoxEngine) -> BoxResult {
    e.hskip(GlueSpec::FILL, false)
}
pub fn hfil(e: &mut BoxEngine) -> BoxResult {
    e.hskip(GlueSpec::FIL, false)
}
pub fn hss(e: &mut BoxEngine) -> BoxResult {
    e.hskip(GlueSpec::SS, false)
}
pub fn vfill(e: &mut BoxEngine) -> BoxResult {
    e.vskip(GlueSpec::FILL, false)
}
pub fn vfil(e: &mut BoxEngine) -> BoxResult {
    e.vskip(GlueSpec::FIL, false)
}
pub fn vss(e: &mut BoxEngine) -> BoxResult {
    e.vskip(GlueSpec::SS, false)
}

/// `\strut` (line 621).
pub fn strut(e: &mut BoxEngine) -> BoxResult {
    if e.mode() == Mode::Math { e.copy_box(STRUTBOX, BoxContext::APPEND) } else { e.unpackage(STRUTBOX, true, false) }
}

/// `\leavevmode@ifvmode` (line 9433).
fn leavevmode_ifvmode(e: &mut BoxEngine) {
    if e.mode().is_vertical() {
        e.indent();
    }
}

/// `\phantom`, `\hphantom`, `\vphantom` in text mode (lines 15599–15613).
pub fn phantom(e: &mut BoxEngine, v: bool, h: bool, content: Content) -> BoxResult {
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register: 0, global: false })?;
    color_begingroup(e);
    content(e)?;
    color_endgroup(e)?;
    e.end_box()?;
    // \finph@nt
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register: 2, global: false })?;
    e.end_box()?;
    if v {
        let (ht, dp) = (e.box_dimen(0, BoxDim::Height), e.box_dimen(0, BoxDim::Depth));
        e.set_box_dimen(2, BoxDim::Height, ht);
        e.set_box_dimen(2, BoxDim::Depth, dp);
    }
    if h {
        let wd = e.box_dimen(0, BoxDim::Width);
        e.set_box_dimen(2, BoxDim::Width, wd);
    }
    leavevmode_ifvmode(e);
    e.use_box(2, BoxContext::APPEND)
}

/// `\smash` in text mode (lines 15618–15629).
pub fn smash(e: &mut BoxEngine, content: Content) -> BoxResult {
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register: 0, global: false })?;
    color_begingroup(e);
    content(e)?;
    color_endgroup(e)?;
    e.end_box()?;
    e.set_box_dimen(0, BoxDim::Height, 0);
    e.set_box_dimen(0, BoxDim::Depth, 0);
    leavevmode_ifvmode(e);
    e.use_box(0, BoxContext::APPEND)
}

/// `\rlap`, `\llap`, `\clap` (lines 16407–16409).
pub fn lap(e: &mut BoxEngine, left_fill: bool, right_fill: bool, content: Content) -> BoxResult {
    e.begin_box(BoxKind::HBox, PackSpec::Exactly(0), BoxContext::APPEND)?;
    if left_fill {
        hss(e)?;
    }
    content(e)?;
    if right_fill {
        hss(e)?;
    }
    e.end_box()
}

/// `\centering` (lines 15409–15413).
pub fn centering(e: &mut BoxEngine) {
    e.set_skip("rightskip", FLUSHGLUE, false);
    e.set_skip("leftskip", FLUSHGLUE, false);
    e.set_int("finalhyphendemerits", 0, false);
    e.set_dimen("parindent", 0, false);
    e.set_skip("parfillskip", GlueSpec::ZERO, false);
}

/// `\raggedright` (lines 15414–15418).
pub fn raggedright(e: &mut BoxEngine) {
    e.set_skip("@rightskip", FLUSHGLUE, false);
    e.set_skip("rightskip", FLUSHGLUE, false);
    e.set_int("finalhyphendemerits", 0, false);
    e.set_skip("leftskip", GlueSpec::ZERO, false);
    e.set_dimen("parindent", 0, false);
}

/// `\raggedleft` (lines 15419–15423).
pub fn raggedleft(e: &mut BoxEngine) {
    e.set_skip("rightskip", GlueSpec::ZERO, false);
    e.set_skip("leftskip", FLUSHGLUE, false);
    e.set_int("finalhyphendemerits", 0, false);
    e.set_dimen("parindent", 0, false);
    e.set_skip("parfillskip", GlueSpec::ZERO, false);
}

/// `\sbox{#1}{#2}` (lines 16142–16143).
pub fn sbox(e: &mut BoxEngine, register: u32, content: Content) -> BoxResult {
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register, global: false })?;
    color_begingroup(e);
    content(e)?;
    color_endgroup(e)?;
    e.end_box()
}

/// `\savebox{#1}[#2][#3]{#4}` (lines 16139–16147); `width=None` is `\sbox`.
pub fn savebox(e: &mut BoxEngine, register: u32, width: Option<LenArg>, pos: Option<char>, content: Content) -> BoxResult {
    match width {
        None => sbox(e, register, content),
        Some(w) => sbox(e, register, &mut |e: &mut BoxEngine| imakebox(e, w, pos.unwrap_or('c'), content)),
    }
}

/// `\usebox{#1}` (line 16165).
pub fn usebox(e: &mut BoxEngine, register: u32) -> BoxResult {
    e.leavevmode();
    e.copy_box(register, BoxContext::APPEND)
}

/// `\settowidth`, `\settoheight`, `\settodepth` (lines 10255–10260).
pub fn settodim(e: &mut BoxEngine, which: BoxDim, dimen_name: &str, content: Content) -> BoxResult {
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register: TEMPBOXA, global: false })?;
    e.begin_group();
    content(e)?;
    e.end_group()?;
    e.end_box()?;
    let v = e.box_dimen(TEMPBOXA, which);
    e.set_dimen(dimen_name, v, false);
    e.set_box_register(TEMPBOXA, None, false);
    Ok(())
}

/// `\begin{lrbox}{#1} ... \end{lrbox}` (lines 16153–16164).
pub fn lrbox(e: &mut BoxEngine, register: u32, content: Content) -> BoxResult {
    e.begin_box(BoxKind::HBox, PackSpec::NATURAL, BoxContext::SetBox { register, global: false })?;
    e.begin_group();
    color_begingroup(e);
    content(e)?;
    e.unskip();
    color_endgroup(e)?;
    e.end_group()?;
    e.end_box()
}

/// `\null` = `\hbox{}` (line 570) as a standalone box value.
pub fn null_box() -> BoxNode {
    BoxNode::null(ListKind::H)
}
