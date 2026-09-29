//! Math alphabets (`\mathit`, `\mathbf`, `\mathsf`, `\mathtt`) are boxed
//! with the metrics pdfLaTeX loads for them: fontmath.ltx's OT1 `cmr`/
//! `cmss`/`cmtt` shapes (`cmti10`, `cmbx10`, `cmss10`, ... at the
//! `ot1cmr.fd`/`ot1cmss.fd` size) unless `lmodern` redeclares them in
//! `lmr`/`lmss`/`lmtt`.
//!
//! Latin Modern's widths equal Knuth's, but its italic corrections do not.
//! `cmti10` gives `b` 0.063124 em and `ec-lmri10` gives 0.026222 em, and a
//! math alphabet run keeps the correction of its last character (tex.web
//! §752). With Latin Modern's metrics everywhere, `\mathit{b}` was 0.369 pt
//! and 12 pt `\mathit{Ab}` 0.714 pt narrower than pdfTeX, and `\mathbf{x}`
//! 0.058 pt wider, in every document without `lmodern`.
//!
//! The bar over `\sqrt{X}` spans X's math box, so it measures the box. The
//! reference widths are pdfTeX 3.141592653 (TeX Live 2026),
//! `\setbox4\hbox{$X$}\the\wd4` under each preamble.

mod common;

use common::*;
use flashtex_render_pipeline::display::Item;

const BP: f64 = 72.0 / 72.27;
/// pdfTeX prints widths to 1e-5 pt. The worst agreement is `\mathtt`,
/// 0.00012 pt: Latin Modern's typewriter metrics stand in for `cmtt`, which
/// the OT1 `.fd` route does not ship.
const TFM_GATE: f64 = 0.0005;

/// (preamble, [(math source, pdfTeX box width in pt)])
const CASES: &[(&str, &[(&str, f64)])] = &[
    (
        "\\documentclass[10pt]{article}",
        &[
            ("\\mathit{b}", 5.23119),
            ("\\mathit{x}", 5.84303),
            ("\\mathit{Ab}", 12.4089),
            ("\\mathit{diff}", 16.43047),
            ("\\mathit{f}", 5.1861),
            ("\\scriptstyle\\mathit{b}", 4.22606),
            ("\\scriptscriptstyle\\mathit{b}", 3.01862),
            ("\\scriptstyle\\mathit{diff}", 13.47928),
            ("\\mathbf{f}", 4.60413),
            ("\\mathbf{Ab}", 15.08325),
            ("\\mathbf{x}", 6.06941),
            ("\\scriptstyle\\mathbf{f}", 3.63818),
            ("\\scriptscriptstyle\\mathbf{Ax}", 9.5623),
            ("\\mathsf{f}", 3.75002),
            ("\\mathsf{Ab}", 11.83336),
            ("\\scriptstyle\\mathsf{Ab}", 8.79872),
            ("\\mathtt{f}", 5.24995),
            ("\\mathtt{Ab}", 10.49991),
            ("\\scriptstyle\\mathtt{Ab}", 7.43759),
        ],
    ),
    (
        "\\documentclass[10pt]{article}\\usepackage{lmodern}",
        &[
            ("\\mathit{b}", 4.86223),
            ("\\mathit{x}", 5.46397),
            ("\\mathit{Ab}", 11.78459),
            ("\\mathit{diff}", 16.03838),
            ("\\mathit{f}", 4.79442),
            ("\\scriptstyle\\mathit{b}", 3.83966),
            ("\\scriptscriptstyle\\mathit{b}", 2.74261),
            ("\\scriptstyle\\mathit{diff}", 13.08893),
            ("\\mathbf{f}", 4.64789),
            ("\\mathbf{Ab}", 15.08301),
            ("\\mathbf{x}", 6.12787),
            ("\\scriptstyle\\mathbf{f}", 3.63141),
            ("\\scriptscriptstyle\\mathbf{Ax}", 9.5625),
            ("\\mathsf{f}", 3.74771),
            ("\\mathsf{Ab}", 11.83366),
            ("\\scriptstyle\\mathsf{Ab}", 8.799),
            ("\\mathtt{f}", 5.24998),
            ("\\mathtt{Ab}", 10.49997),
            ("\\scriptstyle\\mathtt{Ab}", 7.4375),
        ],
    ),
    (
        "\\documentclass[10pt]{article}\\usepackage[T1]{fontenc}\\usepackage{lmodern}",
        &[
            ("\\mathit{b}", 4.86223),
            ("\\mathit{x}", 5.46397),
            ("\\mathit{Ab}", 11.78459),
            ("\\mathit{diff}", 16.03838),
            ("\\mathit{f}", 4.79442),
            ("\\scriptstyle\\mathit{b}", 3.83966),
            ("\\scriptscriptstyle\\mathit{b}", 2.74261),
            ("\\scriptstyle\\mathit{diff}", 13.08893),
            ("\\mathbf{f}", 4.64789),
            ("\\mathbf{Ab}", 15.08301),
            ("\\mathbf{x}", 6.12787),
            ("\\scriptstyle\\mathbf{f}", 3.63141),
            ("\\scriptscriptstyle\\mathbf{Ax}", 9.5625),
            ("\\mathsf{f}", 3.74771),
            ("\\mathsf{Ab}", 11.83366),
            ("\\scriptstyle\\mathsf{Ab}", 8.799),
            ("\\mathtt{f}", 5.24998),
            ("\\mathtt{Ab}", 10.49997),
            ("\\scriptstyle\\mathtt{Ab}", 7.4375),
        ],
    ),
    (
        "\\documentclass[10pt]{article}\\usepackage[T1]{fontenc}",
        &[
            ("\\mathit{b}", 5.23119),
            ("\\mathit{x}", 5.84303),
            ("\\mathit{Ab}", 12.4089),
            ("\\mathit{diff}", 16.43047),
            ("\\mathit{f}", 5.1861),
            ("\\scriptstyle\\mathit{b}", 4.22606),
            ("\\scriptscriptstyle\\mathit{b}", 3.01862),
            ("\\scriptstyle\\mathit{diff}", 13.47928),
            ("\\mathbf{f}", 4.60413),
            ("\\mathbf{Ab}", 15.08325),
            ("\\mathbf{x}", 6.06941),
            ("\\scriptstyle\\mathbf{f}", 3.63818),
            ("\\scriptscriptstyle\\mathbf{Ax}", 9.5623),
            ("\\mathsf{f}", 3.75002),
            ("\\mathsf{Ab}", 11.83336),
            ("\\scriptstyle\\mathsf{Ab}", 8.79872),
            ("\\mathtt{f}", 5.24995),
            ("\\mathtt{Ab}", 10.49991),
            ("\\scriptstyle\\mathtt{Ab}", 7.43759),
        ],
    ),
    (
        "\\documentclass[12pt]{article}",
        &[
            ("\\mathit{b}", 6.13477),
            ("\\mathit{x}", 6.84174),
            ("\\mathit{Ab}", 14.55707),
            ("\\mathit{diff}", 19.3335),
            ("\\mathit{f}", 6.13338),
            ("\\scriptstyle\\mathit{b}", 4.44029),
            ("\\scriptscriptstyle\\mathit{b}", 3.62234),
            ("\\scriptstyle\\mathit{diff}", 14.007),
            ("\\mathbf{f}", 5.35417),
            ("\\mathbf{Ab}", 17.69444),
            ("\\mathbf{x}", 7.125),
            ("\\scriptstyle\\mathbf{f}", 3.9417),
            ("\\scriptscriptstyle\\mathbf{Ax}", 10.55826),
            ("\\mathsf{f}", 4.4062),
            ("\\mathsf{Ab}", 13.79149),
            ("\\scriptstyle\\mathsf{Ab}", 10.0557),
            ("\\mathtt{f}", 6.175),
            ("\\mathtt{Ab}", 12.35),
            ("\\scriptstyle\\mathtt{Ab}", 8.50012),
        ],
    ),
    (
        "\\documentclass[11pt]{article}",
        &[
            ("\\mathit{b}", 5.72815),
            ("\\mathit{x}", 6.39812),
            ("\\mathit{Ab}", 13.58775),
            ("\\mathit{diff}", 17.99135),
            ("\\mathit{f}", 5.67876),
            ("\\scriptstyle\\mathit{b}", 4.44029),
            ("\\scriptscriptstyle\\mathit{b}", 3.62234),
            ("\\scriptstyle\\mathit{diff}", 14.007),
            ("\\mathbf{f}", 5.04152),
            ("\\mathbf{Ab}", 16.51614),
            ("\\mathbf{x}", 6.646),
            ("\\scriptstyle\\mathbf{f}", 3.9417),
            ("\\scriptscriptstyle\\mathbf{Ax}", 10.55826),
            ("\\mathsf{f}", 4.10625),
            ("\\mathsf{Ab}", 12.95753),
            ("\\scriptstyle\\mathsf{Ab}", 10.0557),
            ("\\mathtt{f}", 5.74869),
            ("\\mathtt{Ab}", 11.49738),
            ("\\scriptstyle\\mathtt{Ab}", 8.50012),
        ],
    ),
    (
        "\\documentclass[10pt]{article}\\usepackage{times}",
        &[
            ("\\mathit{b}", 5.23119),
            ("\\mathit{x}", 5.84303),
            ("\\mathit{Ab}", 12.4089),
            ("\\mathit{diff}", 16.43047),
            ("\\mathit{f}", 5.1861),
            ("\\scriptstyle\\mathit{b}", 4.22606),
            ("\\scriptscriptstyle\\mathit{b}", 3.01862),
            ("\\scriptstyle\\mathit{diff}", 13.47928),
            ("\\mathbf{f}", 4.60413),
            ("\\mathbf{Ab}", 15.08325),
            ("\\mathbf{x}", 6.06941),
            ("\\scriptstyle\\mathbf{f}", 3.63818),
            ("\\scriptscriptstyle\\mathbf{Ax}", 9.5623),
            ("\\mathsf{f}", 3.75002),
            ("\\mathsf{Ab}", 11.83336),
            ("\\scriptstyle\\mathsf{Ab}", 8.79872),
            ("\\mathtt{f}", 5.24995),
            ("\\mathtt{Ab}", 10.49991),
            ("\\scriptstyle\\mathtt{Ab}", 7.43759),
        ],
    ),
];

/// The widths of the rules in the rendering, in document order (page, then
/// top), in TeX pt.
fn rule_widths(doc: &str) -> Vec<f64> {
    let r = render_one(doc);
    let mut out = Vec::new();
    for (p, page) in r.v2.pages.iter().enumerate() {
        for it in page.resident_items() {
            if let Item::Rule(rl) = it {
                out.push((p, rl.top.to_bp() / BP, rl.width.to_bp() / BP));
            }
        }
    }
    out.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).expect("finite"));
    out.into_iter().map(|(_, _, w)| w).collect()
}

#[test]
fn math_alphabets_box_at_pdftex_widths() {
    if !lm_available() {
        return;
    }
    let mut bad = Vec::new();
    for (preamble, cases) in CASES {
        let body: String = cases
            .iter()
            .map(|(src, _)| format!("\\noindent$\\sqrt{{{src}}}$\\par\n"))
            .collect();
        let got = rule_widths(&format!("{preamble}\\begin{{document}}{body}\\end{{document}}"));
        assert_eq!(got.len(), cases.len(), "{preamble}: one bar per radical");
        for ((src, want), w) in cases.iter().zip(&got) {
            if (w - want).abs() >= TFM_GATE {
                bad.push(format!("{preamble} {src}: bar {w} pt, pdfTeX {want} pt"));
            }
        }
    }
    assert!(bad.is_empty(), "{} mismatch:\n{}", bad.len(), bad.join("\n"));
}
