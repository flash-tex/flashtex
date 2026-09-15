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

use flashtex_math_layout::{
    Atom, AtomClass, CmMathMetrics, FontId, Glyph, Limits, MathChar, MathFontMetrics, MathList,
    MathParams, Nucleus, OrdLigature, OrdPair, SizeClass, Style, layout,
};

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

fn text_sup(ch: char, script: Vec<Atom>) -> Atom {
    Atom {
        superscript: Some(list(script)),
        ..text_char(ch)
    }
}

/// `\mathrm{..}` of several characters, as the compiler emits it.
fn text(s: &str) -> Atom {
    Atom::new(AtomClass::Ord, Nucleus::Text(s.to_string()))
}

/// amsopn `\operatorname{..}`: `\mathop{\operator@font ..}\nolimits`.
fn operator(s: &str) -> Atom {
    Atom::text_op(s).with_limits(Limits::NoLimits)
}

fn width(atoms: Vec<Atom>, style: Style) -> f64 {
    layout(&list(atoms), style, &CmMathMetrics::latex_10pt()).width
}

fn cases() -> Vec<(&'static str, Vec<Atom>, Style, f64)> {
    let t = Style::TEXT;
    vec![
        ("$r,a,b$", syms("r,a,b"), t, 22.70018),
        (
            "$\\displaystyle r,a,b$",
            syms("r,a,b"),
            Style::DISPLAY,
            22.70018,
        ),
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
        (
            "$\\displaystyle f,P,Y.$",
            syms("f,P,Y."),
            Style::DISPLAY,
            30.14233,
        ),
        ("$a+r, ar, a+b, ab.$", syms("a+r,ar,a+b,ab."), t, 78.74979),
        ("$ab$", syms("ab"), t, 9.57755),
        (
            "$r^2,a$",
            vec![sup('r', syms("2")), sym(','), sym('a')],
            t,
            19.0058,
        ),
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
        // Ligatures (§752-§753). Adjacent one-character `\mathrm` groups
        // are math characters of family 0 (§1186), so cmr10's program joins
        // them; `=:` gives the ligature the right atom's scripts.
        (
            "$\\mathrm{f}\\mathrm{i}$",
            vec![text_char('f'), text_char('i')],
            t,
            5.55557,
        ),
        (
            "$\\mathrm{f}\\mathrm{f}\\mathrm{i}$",
            vec![text_char('f'), text_char('f'), text_char('i')],
            t,
            8.33336,
        ),
        (
            "$\\mathrm{f}\\mathrm{i}^2$",
            vec![text_char('f'), text_sup('i', syms("2"))],
            t,
            10.0417,
        ),
        (
            "$\\mathrm{A}\\mathrm{V}$",
            vec![text_char('A'), text_char('V')],
            t,
            14.02777,
        ),
        (
            "$\\mathrm{T}\\mathrm{r}^2$",
            vec![text_char('T'), text_sup('r', syms("2"))],
            t,
            14.79169,
        ),
        // `-` is `\mathcode"2200` (cmsy, not variable-family): two minus
        // signs even in `\mathrm`, never cmr's `--` en dash.
        (
            "$\\mathrm{--}$",
            vec![Atom::group(list(syms("--")))],
            t,
            15.5556,
        ),
        // A multi-character run (`Nucleus::Text`): the same program between
        // its characters, the last one keeping its italic correction.
        ("$\\mathrm{ff}$", vec![text("ff")], t, 6.61115),
        ("$\\mathrm{fi}$", vec![text("fi")], t, 5.55557),
        ("$\\mathrm{ffi}$", vec![text("ffi")], t, 8.33336),
        ("$\\mathrm{fl}$", vec![text("fl")], t, 5.55557),
        ("$\\mathrm{AV}$", vec![text("AV")], t, 14.02777),
        ("$\\mathrm{AVA}$", vec![text("AVA")], t, 20.27779),
        ("$\\mathrm{Tr}$", vec![text("Tr")], t, 10.30556),
        ("$\\mathrm{Wa}$", vec![text("Wa")], t, 14.44447),
        ("$\\mathrm{off}x$", vec![text("off"), sym('x')], t, 17.32643),
        (
            "$\\mathrm{AV}_1$",
            vec![Atom {
                subscript: Some(list(syms("1"))),
                ..text("AV")
            }],
            t,
            18.5139,
        ),
        (
            "$x^{\\mathrm{AV}}$",
            vec![sup('x', vec![text("AV")])],
            t,
            17.26741,
        ),
        ("$\\operatorname{Tr}$", vec![operator("Tr")], t, 10.30556),
        (
            "$\\operatorname{Tr}x$",
            vec![operator("Tr"), sym('x')],
            t,
            17.68745,
        ),
    ]
}

#[test]
fn math_font_kerns_match_pdftex() {
    let mut bad = Vec::new();
    for (source, atoms, style, pdftex) in cases() {
        let ours = width(atoms, style);
        println!(
            "{source:28} flashtex {ours:9.5} pdftex {pdftex:9.5} diff {:+.5}",
            ours - pdftex
        );
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
    assert_eq!(
        CMR10.lig_kern(b'f', b'i'),
        Some(LigKern::Ligature { op: 0, rem: 0o14 })
    );
    // cmsy10's only kerns are the capitals against the skew character.
    assert!(matches!(
        CMSY10.lig_kern(b'A', 0o60),
        Some(LigKern::Kern(_))
    ));
}

/// CM metrics whose roman program also has one instruction of every
/// ligature form (tex.web §545) — cmr has only `=:` — so the list rewriting
/// of `make_ord` (§753) is checked on both paths: adjacent one-character
/// groups and a multi-character run.
struct LigatureForms(CmMathMetrics);

impl MathFontMetrics for LigatureForms {
    fn params(&self, size: SizeClass) -> MathParams {
        self.0.params(size)
    }
    fn font_name(&self, font: FontId) -> String {
        self.0.font_name(font)
    }
    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.0.glyph(ch, size)
    }
    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.0.large_operator(ch, size)
    }
    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.0.delimiter_sizes(ch, size)
    }
    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.0.radical_sizes(size)
    }
    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.0.accent_sizes(ch, size)
    }
    fn text_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.0.text_glyph(ch, size)
    }
    fn ord_pair(&self, left: MathChar, right: MathChar, size: SizeClass) -> Option<OrdPair> {
        let op = match (left, right) {
            (MathChar::Text('a'), MathChar::Text('b')) => 0, // a b =: x
            (MathChar::Text('c'), MathChar::Text('b')) => 1, // c b =:| x
            (MathChar::Text('d'), MathChar::Text('b')) => 2, // d b |=: x
            (MathChar::Text('e'), MathChar::Text('b')) => 3, // e b |=:| x
            (MathChar::Text('g'), MathChar::Text('b')) => 5, // g b =:|> x
            (MathChar::Text('h'), MathChar::Text('b')) => 11, // h b |=:|>> x
            // Kerns the rewritten pairs meet, to show which pairs are retried.
            (MathChar::Text('x'), MathChar::Text(_)) | (MathChar::Text(_), MathChar::Text('x')) => {
                return Some(OrdPair {
                    kern: 1.0,
                    text_font: true,
                    ligature: None,
                });
            }
            _ => return self.0.ord_pair(left, right, size),
        };
        Some(OrdPair {
            kern: 0.0,
            text_font: true,
            ligature: Some(OrdLigature {
                op,
                ch: MathChar::Text('x'),
            }),
        })
    }
}

#[test]
fn every_ligature_form_rewrites_the_list_as_make_ord_does() {
    let m = LigatureForms(CmMathMetrics::latex_10pt());
    let w = |c: char| m.0.text_glyph(c, SizeClass::Text).expect("cmr10").width;
    let italic = |c: char| m.0.text_glyph(c, SizeClass::Text).expect("cmr10").italic;
    // What each form leaves, as characters and the kerns (1pt, against an
    // `x`) of the pairs `make_ord` retries; the last character keeps its
    // italic correction.
    let cases: [(&str, f64); 6] = [
        // =: → x; nothing follows.
        ("ab", w('x') + italic('x')),
        // =:| → x b, retried: kern x b.
        ("cb", w('x') + 1.0 + w('b') + italic('b')),
        // |=: → d x, retried: kern d x.
        ("db", w('d') + 1.0 + w('x') + italic('x')),
        // |=:| → e x b, retried: kern e x; then x b is the next pair: kern.
        ("eb", w('e') + 1.0 + w('x') + 1.0 + w('b') + italic('b')),
        // =:|> → x b, not retried; x b is not looked at again from x.
        ("gb", w('x') + w('b') + italic('b')),
        // |=:|>> → h x b, not retried, and x does not combine with b.
        ("hb", w('h') + w('x') + w('b') + italic('b')),
    ];
    let mut bad = Vec::new();
    for (pair, want) in cases {
        let chars: Vec<char> = pair.chars().collect();
        let groups = width_with(vec![text_char(chars[0]), text_char(chars[1])], &m);
        let run = width_with(vec![text(pair)], &m);
        for (how, got) in [("groups", groups), ("run", run)] {
            if (got - want).abs() > 1e-6 {
                bad.push(format!("{pair} as {how}: {got:.5}pt, want {want:.5}pt"));
            }
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

fn width_with(atoms: Vec<Atom>, m: &dyn MathFontMetrics) -> f64 {
    layout(&list(atoms), Style::TEXT, m).width
}
