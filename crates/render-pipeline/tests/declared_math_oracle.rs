//! The exhaustive declared-math oracle: every symbol LaTeX declares, through
//! the pipeline, against pdfTeX's glyph origins.
//!
//! `fixtures/declared-math/generate.py` derives the documents from the same
//! declarations the compiler's `math_symbols` table is generated from
//! (`fontmath.ltx`, `latexsym.sty`, `amsfonts.sty`/`amssymb.sty`,
//! `stmaryrd.sty`, `amsmath.sty`): every `\DeclareMathSymbol` in four styles,
//! every delimiter at six sizes, every accent over two nuclei, the 8x8
//! atom-class spacing matrix in three styles, the math alphabets, and the
//! kernel composites. It runs MacTeX's pdflatex once and commits
//! `expected/<doc>.txt`; nothing here runs TeX.
//!
//! For every formula (one per line, labelled `NNNN` in text before it) the
//! engine's glyphs are compared with pdfTeX's relative to the label's origin:
//! each pdfTeX glyph needs an engine glyph of the same size within
//! `TOL_BP` in x and y (x only for `largesymbols`, whose Type 1 origin sits
//! at the top of the TFM box), painting the character the declaration table
//! gives that font slot; and the engine may paint nothing pdfTeX did not.
//!
//! `KNOWN` lists the formulas that fail today, by document and command, so a
//! regression anywhere else fails the test and a fix has to remove its entry.

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const TOL_BP: f64 = 0.5;
const SIZE_TOL_BP: f64 = 0.05;

/// (document, command or `*`): formulas known to fail, with the reason.
/// `*diagnostics` stands for the document's error diagnostics (every
/// unsupported command is one). Slice 1 of the generated-table work
/// records these; the switch-over to `math_symbols` removes entries.
#[rustfmt::skip]
const KNOWN: &[(&str, &str, &str)] = &[
    // alphabets
    ("alphabets", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("alphabets", "mathbb", "advance: the width the engine gives it differs from the TFM"),
    ("alphabets", "mathit", "advance: the width the engine gives it differs from the TFM"),
    ("alphabets", "mathscr", "advance: the width the engine gives it differs from the TFM"),
    // amsfonts-composites
    ("amsfonts-composites", "dashleftarrow", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("amsfonts-composites", "dashrightarrow", "composite: pdfTeX builds it from pieces the engine draws differently"),
    // amsfonts-delimiters
    ("amsfonts-delimiters", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("amsfonts-delimiters", "llcorner", "advance: the width the engine gives it differs from the TFM"),
    ("amsfonts-delimiters", "lrcorner", "advance: the width the engine gives it differs from the TFM"),
    ("amsfonts-delimiters", "ulcorner", "advance: the width the engine gives it differs from the TFM"),
    ("amsfonts-delimiters", "urcorner", "advance: the width the engine gives it differs from the TFM"),
    // amsfonts-symbols
    ("amsfonts-symbols", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("amsfonts-symbols", "checkmark", "size: a text-size \\mathhexbox symbol is scaled to the script size"),
    ("amsfonts-symbols", "circledR", "size: a text-size \\mathhexbox symbol is scaled to the script size"),
    ("amsfonts-symbols", "dasharrow", "geometry differs from pdfTeX"),
    ("amsfonts-symbols", "leadsto", "unsupported: the engine drops the command"),
    ("amsfonts-symbols", "lhd", "unsupported: the engine drops the command"),
    ("amsfonts-symbols", "maltese", "size: a text-size \\mathhexbox symbol is scaled to the script size"),
    ("amsfonts-symbols", "rhd", "unsupported: the engine drops the command"),
    ("amsfonts-symbols", "unlhd", "unsupported: the engine drops the command"),
    ("amsfonts-symbols", "unrhd", "unsupported: the engine drops the command"),
    ("amsfonts-symbols", "yen", "size: a text-size \\mathhexbox symbol is scaled to the script size"),
    // amsmath-delimiters
    ("amsmath-delimiters", "lVert", "geometry differs from pdfTeX"),
    ("amsmath-delimiters", "lvert", "geometry differs from pdfTeX"),
    ("amsmath-delimiters", "rVert", "geometry differs from pdfTeX"),
    ("amsmath-delimiters", "rvert", "geometry differs from pdfTeX"),
    // amsmath-symbols
    ("amsmath-symbols", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("amsmath-symbols", "varDelta", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varGamma", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varLambda", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varOmega", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varPhi", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varPi", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varPsi", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varSigma", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varTheta", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varUpsilon", "unsupported: the engine drops the command"),
    ("amsmath-symbols", "varXi", "unsupported: the engine drops the command"),
    // amssymb-symbols
    ("amssymb-symbols", "Finv", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "Game", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "backepsilon", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "bigstar", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "blacklozenge", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "blacktriangle", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "blacktriangledown", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "centerdot", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "circledS", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "diagdown", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "diagup", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "digamma", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "doublebarwedge", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "ngeqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "ngeqslant", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "nleqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "nleqslant", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "nsubseteqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "nsupseteqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "pitchfork", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "precapprox", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "precnapprox", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "precneqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "subseteqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "subsetneqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "succapprox", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "succnapprox", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "succneqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "supseteqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "supsetneqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "triangledown", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "varnothing", "advance: the width the engine gives it differs from the TFM"),
    ("amssymb-symbols", "varsubsetneqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "varsupsetneqq", "geometry differs from pdfTeX"),
    ("amssymb-symbols", "vartriangle", "geometry differs from pdfTeX"),
    // kernel-accents
    ("kernel-accents", "vec", "advance: the width the engine gives it differs from the TFM"),
    // kernel-composites
    ("kernel-composites", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("kernel-composites", "angle", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "cong", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "doteq", "unsupported: the engine drops the command"),
    ("kernel-composites", "hbar", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "hookleftarrow", "unsupported: the engine drops the command"),
    ("kernel-composites", "hookrightarrow", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "longmapsto", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "mapsto", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "mathellipsis", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "mathsterling", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "models", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "ne", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "neq", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "notin", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "rightleftharpoons", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("kernel-composites", "surd", "composite: pdfTeX builds it from pieces the engine draws differently"),
    // kernel-delimiters
    ("kernel-delimiters", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("kernel-delimiters", "<", "geometry differs from pdfTeX"),
    ("kernel-delimiters", ">", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "Arrowvert", "unsupported: the engine drops the command"),
    ("kernel-delimiters", "Downarrow", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "Uparrow", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "Updownarrow", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "Vert", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "arrowvert", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "backslash", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "bracevert", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "downarrow", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "lgroup", "advance: the width the engine gives it differs from the TFM"),
    ("kernel-delimiters", "lmoustache", "unsupported: the engine drops the command"),
    ("kernel-delimiters", "rgroup", "advance: the width the engine gives it differs from the TFM"),
    ("kernel-delimiters", "rmoustache", "unsupported: the engine drops the command"),
    ("kernel-delimiters", "uparrow", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "updownarrow", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "vert", "geometry differs from pdfTeX"),
    ("kernel-delimiters", "|", "geometry differs from pdfTeX"),
    // kernel-symbols
    ("kernel-symbols", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("kernel-symbols", "Im", "advance: the width the engine gives it differs from the TFM"),
    ("kernel-symbols", "Re", "advance: the width the engine gives it differs from the TFM"),
    ("kernel-symbols", "braceld", "unsupported: the engine drops the command"),
    ("kernel-symbols", "bracelu", "unsupported: the engine drops the command"),
    ("kernel-symbols", "bracerd", "unsupported: the engine drops the command"),
    ("kernel-symbols", "braceru", "unsupported: the engine drops the command"),
    ("kernel-symbols", "lhook", "unsupported: the engine drops the command"),
    ("kernel-symbols", "mapstochar", "unsupported: the engine drops the command"),
    ("kernel-symbols", "not", "advance: the width the engine gives it differs from the TFM"),
    ("kernel-symbols", "rhook", "unsupported: the engine drops the command"),
    ("kernel-symbols", "smallint", "advance: the width the engine gives it differs from the TFM"),
    // latexsym-symbols
    ("latexsym-symbols", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("latexsym-symbols", "Box", "advance: the width the engine gives it differs from the TFM"),
    ("latexsym-symbols", "Diamond", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "Join", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "leadsto", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "lhd", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "mho", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "rhd", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "sqsubset", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "sqsupset", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "unlhd", "unsupported: the engine drops the command"),
    ("latexsym-symbols", "unrhd", "unsupported: the engine drops the command"),
    // stmaryrd-composites
    ("stmaryrd-composites", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("stmaryrd-composites", "Longarrownot", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("stmaryrd-composites", "Longmapsfrom", "unsupported: the engine drops the command"),
    ("stmaryrd-composites", "Longmapsto", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("stmaryrd-composites", "Mapsfrom", "unsupported: the engine drops the command"),
    ("stmaryrd-composites", "Mapsto", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("stmaryrd-composites", "longarrownot", "composite: pdfTeX builds it from pieces the engine draws differently"),
    ("stmaryrd-composites", "longmapsfrom", "unsupported: the engine drops the command"),
    ("stmaryrd-composites", "mapsfrom", "unsupported: the engine drops the command"),
    // stmaryrd-delimiters
    ("stmaryrd-delimiters", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("stmaryrd-delimiters", "llbracket", "unsupported: the engine drops the command"),
    ("stmaryrd-delimiters", "rrbracket", "unsupported: the engine drops the command"),
    // stmaryrd-symbols
    ("stmaryrd-symbols", "*diagnostics", "unsupported commands in the document are diagnosed as errors"),
    ("stmaryrd-symbols", "Arrownot", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "Lbag", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "Mapsfromchar", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "Mapstochar", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "Rbag", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "Ydown", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "Yleft", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "Yright", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "Yup", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "arrownot", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "baro", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bbslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bigbox", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bigcurlyvee", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bigcurlywedge", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "biginterleave", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bignplus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bigparallel", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bigsqcap", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "bigtriangledown", "advance: the width the engine gives it differs from the TFM"),
    ("stmaryrd-symbols", "bigtriangleup", "advance: the width the engine gives it differs from the TFM"),
    ("stmaryrd-symbols", "binampersand", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "bindnasrepma", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "boxast", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "boxbar", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "boxbox", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "boxbslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "boxcircle", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "boxdot", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "boxempty", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "boxslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "curlyveedownarrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "curlyveeuparrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "curlywedgedownarrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "curlywedgeuparrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "fatbslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "fatsemi", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "fatslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "inplus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "interleave", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "lbag", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "leftarrowtriangle", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "leftrightarroweq", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "leftrightarrowtriangle", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "leftslice", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "lightning", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "llceil", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "llfloor", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "llparenthesis", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "mapsfromchar", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "merge", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "minuso", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "moo", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "niplus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "nnearrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "nnwarrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "nplus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "ntrianglelefteqslant", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "ntrianglerighteqslant", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "obar", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "oblong", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "obslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "ogreaterthan", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "olessthan", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "ovee", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "owedge", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "rbag", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "rightarrowtriangle", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "rightslice", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "rrceil", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "rrfloor", "geometry differs from pdfTeX"),
    ("stmaryrd-symbols", "rrparenthesis", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "shortdownarrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "shortleftarrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "shortrightarrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "shortuparrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "ssearrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "sslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "sswarrow", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "subsetplus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "subsetpluseq", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "supsetplus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "supsetpluseq", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "talloblong", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "trianglelefteqslant", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "trianglerighteqslant", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varbigcirc", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varcurlyvee", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varcurlywedge", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varoast", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varobar", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varobslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varocircle", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varodot", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varogreaterthan", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varolessthan", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varominus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varoplus", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varoslash", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varotimes", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varovee", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "varowedge", "unsupported: the engine drops the command"),
    ("stmaryrd-symbols", "vartimes", "unsupported: the engine drops the command"),
];

#[derive(Debug, Clone)]
struct ExpectedGlyph {
    font: String,
    code: u32,
    size: f64,
    dx: f64,
    dy: f64,
    /// The texts declarations give the slot; empty when none has one.
    text: Vec<String>,
}

#[derive(Debug, Clone)]
struct Formula {
    label: String,
    category: String,
    name: String,
    style: String,
    tex: String,
    glyphs: Vec<ExpectedGlyph>,
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/declared-math")
}

fn parse_expected(text: &str) -> Vec<Formula> {
    let mut out: Vec<Formula> = Vec::new();
    let mut anchor = (0.0, 0.0);
    for line in text.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("formula ") {
            let mut it = rest.splitn(9, ' ');
            let label = it.next().unwrap().to_string();
            let _page = it.next().unwrap();
            let category = it.next().unwrap().to_string();
            let name = it.next().unwrap().to_string();
            let style = it.next().unwrap().to_string();
            let _slot = it.next().unwrap();
            let ax: f64 = it.next().unwrap().parse().unwrap();
            let ay: f64 = it.next().unwrap().parse().unwrap();
            let tex = it.next().unwrap_or("").to_string();
            anchor = (ax, ay);
            out.push(Formula { label, category, name, style, tex, glyphs: Vec::new() });
        } else if let Some(rest) = line.strip_prefix("g ") {
            let f: Vec<&str> = rest.split(' ').collect();
            // Every text a declaration gives the slot, `|`-separated.
            let text: Vec<String> = if f[5] == "-" {
                Vec::new()
            } else {
                f[5].split('|')
                    .map(|t| {
                        t.split("U+")
                            .filter(|s| !s.is_empty())
                            .map(|h| char::from_u32(u32::from_str_radix(h, 16).unwrap()).unwrap())
                            .collect()
                    })
                    .collect()
            };
            out.last_mut().unwrap().glyphs.push(ExpectedGlyph {
                font: f[0].to_string(),
                code: f[1].parse().unwrap(),
                size: f[2].parse().unwrap(),
                dx: f[3].parse::<f64>().unwrap() - anchor.0,
                dy: f[4].parse::<f64>().unwrap() - anchor.1,
                text,
            });
        }
    }
    out
}

/// Whether the character the engine paints is one the slot's declarations
/// spell (or the slot has no spelling at all).
fn identity_ok(engine: &str, declared: &[String]) -> bool {
    // Overprint marks (`\not`'s slash U+0338, `\mapstochar` U+F8FE) are
    // painted under the character they negate or extend, whose text the
    // engine's cluster carries; any text stands for them.
    let mark = declared.iter().any(|t| t == "\u{0338}" || t == "\u{F8FE}");
    // The compiler spells cmmi "1E `\phi` as U+03C6 and "27 `\varphi` as
    // U+03D5 (the declaration table keeps that convention) and the pipeline
    // corrects the extracted text to Unicode's naming, U+03D5 `\phi` /
    // U+03C6 `\varphi` (`mathfont::extraction_text`); either spelling names
    // the slot.
    let phi_swap = |t: &str| match t {
        "\u{03C6}" => engine == "\u{03D5}",
        "\u{03D5}" => engine == "\u{03C6}",
        _ => false,
    };
    declared.is_empty() || mark || declared.iter().any(|t| t == engine || phi_swap(t))
}

#[derive(Debug, Clone)]
struct EngineGlyph {
    text: String,
    size: f64,
    x: f64,
    y: f64,
    line: usize,
    is_label: bool,
}

fn line_of(offsets: &[(usize, usize)], byte: usize) -> Option<usize> {
    offsets.iter().position(|&(s, e)| byte >= s && byte < e)
}

fn run_doc(doc: &str) -> (usize, Vec<String>, BTreeMap<String, Vec<String>>) {
    let tex = std::fs::read_to_string(fixture_dir().join(format!("{doc}.tex"))).unwrap();
    let expected = parse_expected(&std::fs::read_to_string(fixture_dir().join(format!("expected/{doc}.txt"))).unwrap());
    // Byte range of every formula line (`NNNN $...$\par`).
    let mut offsets = Vec::new();
    let mut pos = 0;
    let mut label_lines = BTreeMap::new();
    for line in tex.split_inclusive('\n') {
        let range = (pos, pos + line.len());
        if line.len() > 5 && line.as_bytes()[4] == b' ' && line[..4].bytes().all(|b| b.is_ascii_digit()) {
            label_lines.insert(line[..4].to_string(), offsets.len());
        }
        offsets.push(range);
        pos += line.len();
    }
    let r = render_docs(&[("main.tex", &tex)], "main.tex");
    let errors: Vec<String> = r
        .v2
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_render_pipeline::display::Severity::Error)
        .map(|d| d.message.clone())
        .collect();
    let mut glyphs: Vec<EngineGlyph> = Vec::new();
    for page in &r.v2.pages {
        for item in page.resident_items() {
            let Item::GlyphRun(run) = item else { continue };
            let is_label = run.role == RunRole::Text
                && run.text.len() == 4
                && run.text.bytes().all(|b| b.is_ascii_digit());
            for g in &run.glyphs {
                let c = &run.clusters[g.cluster as usize];
                let text = run.text[c.text_start_byte..c.text_end_byte].to_string();
                let line = c
                    .provenance
                    .sources()
                    .first()
                    .and_then(|s| line_of(&offsets, s.start_byte));
                let Some(line) = line else { continue };
                glyphs.push(EngineGlyph {
                    text,
                    size: run.font_size.to_bp(),
                    x: g.origin_x.to_bp(),
                    y: g.baseline_y.to_bp(),
                    line,
                    is_label,
                });
            }
        }
    }
    let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut passed = Vec::new();
    for f in &expected {
        let Some(&line) = label_lines.get(&f.label) else {
            failures.entry(f.name.clone()).or_default().push(format!("{}: no source line", f.label));
            continue;
        };
        let mine: Vec<&EngineGlyph> = glyphs.iter().filter(|g| g.line == line).collect();
        let Some(anchor) = mine.iter().find(|g| g.is_label) else {
            failures.entry(f.name.clone()).or_default().push(format!("{} {} {}: label not painted", f.label, f.name, f.style));
            continue;
        };
        let (ax, ay) = (anchor.x, anchor.y);
        let mut used = vec![false; mine.len()];
        let mut problems = Vec::new();
        for e in &f.glyphs {
            let x_only = e.font.starts_with("CMEX");
            let hit = mine.iter().enumerate().find(|(i, g)| {
                !used[*i]
                    && !g.is_label
                    && (g.size - e.size).abs() <= SIZE_TOL_BP
                    && (g.x - ax - e.dx).abs() <= TOL_BP
                    && (x_only || (g.y - ay - e.dy).abs() <= TOL_BP)
            });
            match hit {
                Some((i, g)) => {
                    used[i] = true;
                    if !identity_ok(&g.text, &e.text) {
                        problems.push(format!(
                            "{} {} at ({:+.3},{:+.3}): engine paints {:?}, the slot is declared as {:?}",
                            e.font, e.code, e.dx, e.dy, g.text, e.text
                        ));
                    }
                }
                None => {
                    let near: Vec<String> = mine
                        .iter()
                        .filter(|g| !g.is_label && (g.x - ax - e.dx).abs() <= 3.0)
                        .map(|g| format!("{:?}@({:+.3},{:+.3},{:.2}pt)", g.text, g.x - ax, g.y - ay, g.size))
                        .collect();
                    problems.push(format!(
                        "{} {} {:?} at ({:+.3},{:+.3},{:.2}pt) unmatched; engine near: {}",
                        e.font, e.code, e.text, e.dx, e.dy, e.size, near.join(" ")
                    ));
                }
            }
        }
        for (i, g) in mine.iter().enumerate() {
            if !used[i] && !g.is_label {
                problems.push(format!("engine extra {:?} at ({:+.3},{:+.3},{:.2}pt)", g.text, g.x - ax, g.y - ay, g.size));
            }
        }
        if problems.is_empty() {
            passed.push(format!("{} {} {}", f.label, f.name, f.style));
        } else {
            failures
                .entry(f.name.clone())
                .or_default()
                .push(format!("{} {} {} `{}`:\n      {}", f.label, f.category, f.style, f.tex, problems.join("\n      ")));
        }
    }
    if !errors.is_empty() {
        failures.entry("*diagnostics".into()).or_default().extend(errors);
    }
    (expected.len(), passed, failures)
}

fn check(doc: &str) {
    if !lm_available() {
        return;
    }
    let (total, passed, failures) = run_doc(doc);
    let known: BTreeSet<&str> = KNOWN.iter().filter(|(d, _, _)| *d == doc).map(|(_, n, _)| *n).collect();
    let all_known = known.contains("*");
    let unexpected: Vec<_> = failures
        .iter()
        .filter(|(n, _)| !all_known && !known.contains(n.as_str()))
        .collect();
    let stale: Vec<_> = known.iter().filter(|n| **n != "*" && !failures.contains_key(**n)).collect();
    let n_fail = total - passed.len();
    eprintln!(
        "declared-math {doc}: {} formulas passed, {n_fail} failed in {} commands ({} known), {} unexpected",
        passed.len(),
        failures.len(),
        failures.len() - unexpected.len(),
        unexpected.len()
    );
    if std::env::var_os("DECLARED_MATH_VERBOSE").is_some() {
        for (n, v) in &failures {
            eprintln!("  \\{n}:\n    {}", v.join("\n    "));
        }
    }
    assert!(
        unexpected.is_empty(),
        "{doc}: {} command(s) fail outside KNOWN:\n{}",
        unexpected.len(),
        unexpected
            .iter()
            .map(|(n, v)| format!("  \\{n}:\n    {}", v.iter().take(4).cloned().collect::<Vec<_>>().join("\n    ")))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(stale.is_empty(), "{doc}: KNOWN entries that pass now; remove them: {stale:?}");
    if all_known {
        assert!(!failures.is_empty(), "{doc}: marked wholly known-failing but passes; remove the `*` entry");
    }
}

#[test]
fn kernel_symbols() {
    check("kernel-symbols");
}

#[test]
fn kernel_delimiters() {
    check("kernel-delimiters");
}

#[test]
fn kernel_accents() {
    check("kernel-accents");
}

#[test]
fn kernel_composites() {
    check("kernel-composites");
}

#[test]
fn spacing_matrix() {
    check("spacing");
}

#[test]
fn alphabets() {
    check("alphabets");
}

#[test]
fn latexsym_symbols() {
    check("latexsym-symbols");
}

#[test]
fn amsfonts_symbols() {
    check("amsfonts-symbols");
}

#[test]
fn amsfonts_delimiters() {
    check("amsfonts-delimiters");
}

#[test]
fn amsfonts_composites() {
    check("amsfonts-composites");
}

#[test]
fn amssymb_symbols() {
    check("amssymb-symbols");
}

#[test]
fn amssymb_accents() {
    check("amssymb-accents");
}

#[test]
fn amsmath_symbols() {
    check("amsmath-symbols");
}

#[test]
fn amsmath_delimiters() {
    check("amsmath-delimiters");
}

#[test]
fn amsmath_accents() {
    check("amsmath-accents");
}

#[test]
fn stmaryrd_symbols() {
    check("stmaryrd-symbols");
}

#[test]
fn stmaryrd_delimiters() {
    check("stmaryrd-delimiters");
}

#[test]
fn stmaryrd_composites() {
    check("stmaryrd-composites");
}
