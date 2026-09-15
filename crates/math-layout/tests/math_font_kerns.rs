//! TeX's math font kerns (`make_ord`, tex.web §752) against pdfTeX.
//!
//! Every expected width is pdfTeX's own `\the\wd0` for `\setbox0\hbox{...}`
//! with the source shown, in `\documentclass[10pt]{article}` with no
//! packages (pdfTeX 1.40.29, TeX Live 2026), so the fonts are cmr/cmmi/cmsy
//! 10/7/5. `\showbox0` for the same boxes shows where the kerns are: `$r,a,b$`
//! has `\kern0.27779` (the italic correction of `r`) and `\kern-0.55556`
//! (cmmi10's `r` `,` kern, -0.055555 em) after its first `r`, and no kern
//! after `a` (cmmi10 has no `a` `,` pair). Nothing here runs TeX; the oracle
//! output is evidence recorded in the table.
//!
//! What the rows pin:
//! * an Ord character followed by a character of the same family gets the
//!   font's kern (`$r,a,b$`, `$f,P,Y.$`), at script size too (`$x^{f,P}$`);
//! * a character of a different family gets none (`$f(x)$`: `(` is cmr;
//!   `$\Gamma,\Delta$`: `\Gamma` is cmr, `,` cmmi);
//! * a character with scripts gets none (`$x_1,x_2$`, `$V_{a}$`, `$r^2,a$`);
//! * glue or a non-character nucleus in between blocks it (`$r\,,a$`,
//!   `$r{,}a$`), while a group holding one Ord character is that character
//!   (`${r},a$`, TeX §1186);
//! * in a text font (cmr, fontdimen 2 nonzero) the italic correction is
//!   dropped before the next character (`$\mathrm{f}1$`, §755), but kept at
//!   the end (`$\mathrm{f}$`).

use flashtex_math_layout::{Atom, AtomClass, CmMathMetrics, MathList, Nucleus, Style, layout};

fn sym(ch: char) -> Atom {
    Atom::symbol(ch)
}

fn list(atoms: Vec<Atom>) -> MathList {
    MathList { atoms }
}

fn syms(s: &str) -> Vec<Atom> {
    s.chars().map(sym).collect()
}

fn sub(ch: char, script: &str) -> Atom {
    Atom {
        subscript: Some(list(syms(script))),
        ..sym(ch)
    }
}

fn sup(ch: char, script: Vec<Atom>) -> Atom {
    Atom {
        superscript: Some(list(script)),
        ..sym(ch)
    }
}

fn text_char(ch: char) -> Atom {
    Atom::new(AtomClass::Ord, Nucleus::TextChar(ch))
}

fn width(atoms: Vec<Atom>, style: Style) -> f64 {
    layout(&list(atoms), style, &CmMathMetrics::latex_10pt()).width
}

fn cases() -> Vec<(&'static str, Vec<Atom>, Style, f64)> {
    let t = Style::TEXT;
    vec![
        ("$r,a,b$", syms("r,a,b"), t, 22.70018),
        ("$\\displaystyle r,a,b$", syms("r,a,b"), Style::DISPLAY, 22.70018),
        ("$f(x)$", syms("f(x)"), t, 19.46533),
        ("$\\Gamma,\\Delta$", syms("\u{0393},\u{0394}"), t, 19.02779),
        (
            "$x_1,x_2$",
            vec![sub('x', "1"), sym(','), sub('x', "2")],
            t,
            24.84721,
        ),
        ("$V_{a}$", vec![sub('V', "a")], t, 10.67097),
        ("$f,P,Y.$", syms("f,P,Y."), t, 30.14233),
        ("$\\displaystyle f,P,Y.$", syms("f,P,Y."), Style::DISPLAY, 30.14233),
        ("$a+r, ar, a+b, ab.$", syms("a+r,ar,a+b,ab."), t, 78.74979),
        ("$ab$", syms("ab"), t, 9.57755),
        ("$r^2,a$", vec![sup('r', syms("2")), sym(','), sym('a')], t, 19.0058),
        ("$x^{f,P}$", vec![sup('x', syms("f,P"))], t, 19.0115),
        (
            "${r},a$",
            vec![Atom::group(list(syms("r"))), sym(','), sym('a')],
            t,
            13.96411,
        ),
        (
            "$r{,}a$",
            vec![sym('r'), Atom::group(list(syms(","))), sym('a')],
            t,
            12.85304,
        ),
        (
            "$r\\,,a$",
            vec![sym('r'), Atom::glue(3.0, 0.0), sym(','), sym('a')],
            t,
            16.1863,
        ),
        ("$\\mathrm{f}1$", vec![text_char('f'), sym('1')], t, 8.05559),
        ("$\\mathrm{f}$", vec![text_char('f')], t, 3.83336),
    ]
}

#[test]
fn math_font_kerns_match_pdftex() {
    let mut bad = Vec::new();
    for (source, atoms, style, pdftex) in cases() {
        let ours = width(atoms, style);
        println!("{source:28} flashtex {ours:9.5} pdftex {pdftex:9.5} diff {:+.5}", ours - pdftex);
        // pdfTeX prints five decimals of a scaled-point value.
        if (ours - pdftex).abs() > 1e-4 {
            bad.push(format!("{source}: {ours:.5}pt, pdfTeX {pdftex:.5}pt"));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

#[test]
fn cmmi10_r_comma_kern_is_the_tfm_program() {
    use flashtex_math_layout::cm_tfm::{CMMI10, CMR10, CMSY10};
    use flashtex_math_layout::tfm::{LigKern, scale};
    // tftopl cmmi10.tfm: (LABEL C r) (KRN O 73 R -0.055555).
    let Some(LigKern::Kern(k)) = CMMI10.lig_kern(b'r', 0o73) else {
        panic!("cmmi10 r, has a kern");
    };
    assert_eq!(format!("{:.5}", scale(k, 10.0)), "-0.55556");
    assert_eq!(CMMI10.lig_kern(b'a', 0o73), None);
    // cmr10 `f` `i` is the fi ligature (slot 0o14), not a kern.
    assert_eq!(CMR10.lig_kern(b'f', b'i'), Some(LigKern::Ligature { op: 0, rem: 0o14 }));
    // cmsy10's only kerns are the capitals against the skew character.
    assert!(matches!(CMSY10.lig_kern(b'A', 0o60), Some(LigKern::Kern(_))));
}
