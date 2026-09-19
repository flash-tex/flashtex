//! The OpenType-only rules behind `MathFontMetrics::opentype_extras`: with
//! the extras set to exactly the values Appendix G derives from σ₅/ξ₈ the
//! boxes are TeX's, so the OpenType path only moves geometry where a font's
//! constants differ; the cut-in kerns, the LuaTeX degree placement and the
//! glue-distributed glyph assembly are checked against numbers from the
//! LuaTeX oracle (`lualatex` + `unicode-math`, `\setmathfont{Latin Modern
//! Math}`/`{STIX Two Math}`, a Lua walk of `\setbox0\hbox{$...$}` printing
//! every glyph's origin -- oracle tooling only, nothing here runs TeX).

use flashtex_math_layout::metrics::Extensible;
use flashtex_math_layout::{
    Assembly, AssemblyPart, Atom, CmMathMetrics, FontId, Glyph, KernCorner, MathChar,
    MathFontMetrics, MathList, MathParams, OpenTypeExtras, OrdPair, SizeClass, Style, layout,
    positioned_runs,
};

/// Computer Modern metrics that also answer the OpenType questions: extras
/// built from TeX's own rules (so the layout must not move), constant kerns
/// per corner, and an assembly for `(`.
struct Otf {
    cm: CmMathMetrics,
    kerns: [f64; 4],
    assembly: Option<Assembly>,
}

impl Otf {
    fn new() -> Otf {
        Otf {
            cm: CmMathMetrics::latex_10pt(),
            kerns: [0.0; 4],
            assembly: None,
        }
    }
}

impl MathFontMetrics for Otf {
    fn params(&self, size: SizeClass) -> MathParams {
        self.cm.params(size)
    }
    fn font_name(&self, font: FontId) -> String {
        self.cm.font_name(font)
    }
    fn glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.cm.glyph(ch, size)
    }
    fn large_operator(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.cm.large_operator(ch, size)
    }
    fn delimiter_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.cm.delimiter_sizes(ch, size)
    }
    fn radical_sizes(&self, size: SizeClass) -> Vec<Glyph> {
        self.cm.radical_sizes(size)
    }
    fn accent_sizes(&self, ch: char, size: SizeClass) -> Vec<Glyph> {
        self.cm.accent_sizes(ch, size)
    }
    fn delimiter_extensible(&self, ch: char, size: SizeClass) -> Option<Extensible> {
        if self.assembly.is_some() {
            return None;
        }
        self.cm.delimiter_extensible(ch, size)
    }
    fn radical_extensible(&self, size: SizeClass) -> Option<Extensible> {
        self.cm.radical_extensible(size)
    }
    fn text_glyph(&self, ch: char, size: SizeClass) -> Option<Glyph> {
        self.cm.text_glyph(ch, size)
    }
    fn extension_glyph(&self, code: u8, ch: char, size: SizeClass) -> Option<Glyph> {
        self.cm.extension_glyph(code, ch, size)
    }
    fn ord_pair(&self, left: MathChar, right: MathChar, size: SizeClass) -> Option<OrdPair> {
        self.cm.ord_pair(left, right, size)
    }
    fn opentype_extras(&self, size: SizeClass) -> Option<OpenTypeExtras> {
        // Appendix G's own values, spelled as the constants.
        let p = self.cm.params(size);
        let (xh, theta) = (p.x_height, p.default_rule_thickness);
        Some(OpenTypeExtras {
            fraction_numerator_gap_min: theta,
            fraction_num_display_style_gap_min: 3.0 * theta,
            fraction_denominator_gap_min: theta,
            fraction_denom_display_style_gap_min: 3.0 * theta,
            stack_gap_min: 3.0 * theta,
            stack_display_style_gap_min: 7.0 * theta,
            stack_top_display_style_shift_up: p.num1,
            stack_bottom_shift_down: p.denom2,
            stack_bottom_display_style_shift_down: p.denom1,
            sub_superscript_gap_min: 4.0 * theta,
            superscript_bottom_max_with_subscript: xh * 4.0 / 5.0,
            subscript_top_max: xh * 4.0 / 5.0,
            superscript_bottom_min: xh / 4.0,
            space_after_script: p.script_space,
            // cmsy/cmex radical signs are ξ₈ tall above their origin.
            radical_rule_thickness: theta,
            radical_vertical_gap: theta + theta / 4.0,
            radical_display_style_vertical_gap: theta + xh / 4.0,
            radical_extra_ascender: theta,
            radical_kern_before_degree: 5.0 * p.mu(),
            radical_kern_after_degree: -10.0 * p.mu(),
            radical_degree_bottom_raise_percent: 60.0,
            accent_base_height: xh,
            flattened_accent_base_height: 100.0,
            overbar_vertical_gap: 3.0 * theta,
            overbar_rule_thickness: theta,
            overbar_extra_ascender: theta,
            underbar_vertical_gap: 3.0 * theta,
            underbar_rule_thickness: theta,
            underbar_extra_descender: theta,
            display_operator_min_height: 0.0,
        })
    }
    fn math_kern(&self, _glyph: &Glyph, corner: KernCorner, _height: f64) -> f64 {
        match corner {
            KernCorner::TopRight => self.kerns[0],
            KernCorner::TopLeft => self.kerns[1],
            KernCorner::BottomRight => self.kerns[2],
            KernCorner::BottomLeft => self.kerns[3],
        }
    }
    fn delimiter_assembly(&self, ch: char, _size: SizeClass) -> Option<Assembly> {
        (ch == '(').then(|| self.assembly.clone()).flatten()
    }
}

fn origins(list: &MathList, style: Style, m: &dyn MathFontMetrics) -> Vec<(char, f64, f64)> {
    let root = layout(list, style, m);
    let runs = positioned_runs(&root, (0.0, 0.0));
    runs.glyphs
        .iter()
        .map(|g| (g.ch, g.x, g.baseline_y))
        .collect()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn extras_spelling_appendix_g_leave_every_box_where_tex_puts_it() {
    let cm = CmMathMetrics::latex_10pt();
    let otf = Otf::new();
    let x2i = Atom {
        superscript: Some(MathList::symbols("2")),
        subscript: Some(MathList::symbols("i")),
        ..Atom::symbol('x')
    };
    let cases = vec![
        MathList::new(vec![x2i.clone()]),
        MathList::new(vec![Atom {
            superscript: Some(MathList::symbols("ab")),
            subscript: Some(MathList::symbols("cd")),
            ..Atom::symbol('x')
        }]),
        MathList::new(vec![Atom::frac(
            MathList::symbols("a"),
            MathList::symbols("b"),
        )]),
        MathList::new(vec![Atom::frac(
            MathList::symbols("1"),
            MathList::new(vec![x2i]),
        )]),
        MathList::new(vec![Atom::sqrt(MathList::symbols("x"))]),
        MathList::new(vec![Atom::sqrt(MathList::new(vec![Atom::frac(
            MathList::symbols("a"),
            MathList::symbols("b"),
        )]))]),
        MathList::new(vec![Atom::overline(MathList::symbols("AB"))]),
        MathList::new(vec![Atom::underline(MathList::symbols("x"))]),
        MathList::new(vec![Atom::accent('\u{0302}', MathList::symbols("x"))]),
        MathList::new(vec![Atom::left_right(
            Some('('),
            Some(')'),
            MathList::new(vec![Atom::frac(
                MathList::symbols("a"),
                MathList::symbols("b"),
            )]),
        )]),
    ];
    for style in [Style::TEXT, Style::DISPLAY, Style::SCRIPT] {
        for (i, list) in cases.iter().enumerate() {
            // Rule 11's rule is the sign's own height in TeX; the extras
            // spell ξ₈, which cmsy7's 0.68 pt sign does not match at
            // script size, so the two radical cases are text/display only.
            if style == Style::SCRIPT && (i == 4 || i == 5) {
                continue;
            }
            let a = origins(list, style, &cm);
            let b = origins(list, style, &otf);
            assert_eq!(a.len(), b.len(), "case {i} {style:?}");
            for (g, h) in a.iter().zip(&b) {
                assert!(
                    g.0 == h.0 && close(g.1, h.1) && close(g.2, h.2),
                    "case {i} {style:?}: {g:?} vs {h:?}"
                );
            }
            let (ra, rb) = (layout(list, style, &cm), layout(list, style, &otf));
            assert!(
                close(ra.width, rb.width)
                    && close(ra.height, rb.height)
                    && close(ra.depth, rb.depth),
                "case {i} {style:?} box"
            );
        }
    }
}

#[test]
fn cut_in_kerns_move_a_one_character_script_by_the_corner_sum() {
    // LuaTeX `find_math_kern`: base top-right + script bottom-left for a
    // superscript, base bottom-right + script top-left for a subscript,
    // the smaller of the two heights' sums -- constant tables here, so
    // the sums themselves.
    let mut otf = Otf::new();
    otf.kerns = [-0.7, 0.3, -1.6, -0.1];
    let plain = Otf::new();
    let sup = MathList::new(vec![Atom {
        superscript: Some(MathList::symbols("2")),
        ..Atom::symbol('A')
    }]);
    let a = origins(&sup, Style::TEXT, &plain);
    let b = origins(&sup, Style::TEXT, &otf);
    assert!(close(b[1].1, a[1].1 - 0.8), "{a:?} {b:?}");
    assert!(close(b[1].2, a[1].2));
    let sub = MathList::new(vec![Atom {
        subscript: Some(MathList::symbols("a")),
        ..Atom::symbol('V')
    }]);
    let a = origins(&sub, Style::TEXT, &plain);
    let b = origins(&sub, Style::TEXT, &otf);
    assert!(close(b[1].1, a[1].1 - 1.3), "{a:?} {b:?}");
    // Both scripts: each takes its own corners.
    let both = MathList::new(vec![Atom {
        superscript: Some(MathList::symbols("2")),
        subscript: Some(MathList::symbols("a")),
        ..Atom::symbol('F')
    }]);
    let a = origins(&both, Style::TEXT, &plain);
    let b = origins(&both, Style::TEXT, &otf);
    assert!(
        close(b[1].1, a[1].1 - 0.8) && close(b[2].1, a[2].1 - 1.3),
        "{a:?} {b:?}"
    );
    // A two-character script is a box, not a character: no kern.
    let boxed = MathList::new(vec![Atom {
        superscript: Some(MathList::symbols("ab")),
        ..Atom::symbol('A')
    }]);
    assert_eq!(
        origins(&boxed, Style::TEXT, &plain),
        origins(&boxed, Style::TEXT, &otf)
    );
    // Widths follow: the scripts box shrinks by the kern.
    let w = |m: &dyn MathFontMetrics| layout(&sup, Style::TEXT, m).width;
    assert!(close(w(&otf), w(&plain) - 0.8));
}

#[test]
fn the_degree_of_a_root_follows_the_radical_constants() {
    // Latin Modern Math's constants at 10 pt (`ttx -t MATH`), on Computer
    // Modern's glyphs: before 2.78 pt, after −5.56 pt, the degree's
    // baseline 60% of the sign's total height above the sign's bottom.
    // (LuaTeX, `\sqrt[3]{x}` in Latin Modern Math: the `3` at x 2.78 and
    // y 3.605 with the sign spanning −2.395..7.605, the sign at x 0.625.)
    let mut otf = Otf::new();
    let _ = &mut otf;
    struct Degree(Otf);
    impl MathFontMetrics for Degree {
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
        fn opentype_extras(&self, size: SizeClass) -> Option<OpenTypeExtras> {
            let mut e = self.0.opentype_extras(size)?;
            e.radical_kern_before_degree = 2.78;
            e.radical_kern_after_degree = -5.56;
            e.radical_degree_bottom_raise_percent = 60.0;
            Some(e)
        }
    }
    let m = Degree(otf);
    let root = MathList::new(vec![Atom::root(
        MathList::symbols("3"),
        MathList::symbols("x"),
    )]);
    let g = origins(&root, Style::TEXT, &m);
    let three = g.iter().find(|g| g.0 == '3').expect("degree");
    let sign = g.iter().find(|g| g.0 == '\u{221A}').expect("sign");
    let three_w = m.glyph('3', SizeClass::ScriptScript).unwrap().width;
    assert!(close(three.1, 2.78), "{g:?}");
    assert!(close(sign.1, 2.78 + three_w - 5.56), "{g:?}");
    // The sign: cmsy10's radical, 0.4 pt above its origin and 9.6 below
    // (its total 10 pt), raised as Rule 11 raises it; the degree's
    // baseline is 60% of 10 pt above the sign's bottom.
    let s = m.radical_sizes(SizeClass::Text)[0];
    assert!(
        (s.height - 0.4).abs() < 1e-3 && (s.depth - 9.6).abs() < 1e-3,
        "{s:?}"
    );
    let sign_bottom = sign.2 + s.depth;
    assert!(
        close(three.2, sign_bottom - 0.6 * s.total_height()),
        "{g:?}"
    );
}

#[test]
fn an_assembly_shares_the_shortfall_by_each_joints_stretch() {
    // STIX Two Math's `(` (ttx: hooks fullAdvance 1273 with a 250 connector
    // towards the extender, extender 1252 with 1000 on both ends,
    // minConnectorOverlap 100) around a body LuaTeX wanted 48.52 pt of:
    // three extenders, the hook joints overlapping 1.75 pt and the
    // extender joints 5.5 (LuaTeX's glue: each joint −max stretching to
    // −min, the stack packed to 48.52; ratio (48.52 − 38.02)/21 = 0.5).
    let part = |ch: char, fa: f64, start: f64, end: f64, ext: bool| AssemblyPart {
        glyph: Glyph {
            font_id: FontId(0),
            gid: ch as u16,
            ch,
            size: 10.0,
            width: 4.84,
            height: fa,
            depth: 0.0,
            italic: 0.0,
            skew: 0.0,
        },
        start_connector: start,
        end_connector: end,
        full_advance: fa,
        extender: ext,
    };
    let assembly = Assembly {
        parts: vec![
            part('b', 12.73, 0.0, 2.5, false),
            part('e', 12.52, 10.0, 10.0, true),
            part('t', 12.73, 2.5, 0.0, false),
        ],
        min_overlap: 1.0,
    };
    let mut otf = Otf::new();
    otf.assembly = Some(assembly);
    // The body: an empty box of the kernel `\Bigg` spelling, 29.26 pt tall
    // and 0 deep, so Rule 19 asks for δ = max(29.26 − 2.5, 2.5) = 26.76
    // and max(53.52 × 0.901, 53.52 − 5) = 48.52 pt of delimiter.
    let big = Atom::new(
        flashtex_math_layout::AtomClass::Ord,
        flashtex_math_layout::Nucleus::BigDelimiter {
            delim: None,
            sizing: flashtex_math_layout::BigSizing::Kernel { pt: 29.26 },
        },
    );
    let list = MathList::new(vec![Atom::left_right(
        Some('('),
        None,
        MathList::new(vec![big]),
    )]);
    let g = origins(&list, Style::DISPLAY, &otf);
    let parts: Vec<&(char, f64, f64)> = g.iter().filter(|g| "bet".contains(g.0)).collect();
    assert_eq!(
        parts.iter().map(|p| p.0).collect::<String>(),
        "beeet",
        "{g:?}"
    );
    // Rises between consecutive parts (y grows downward): full advance
    // less the joint's overlap.
    let step = |i: usize| parts[i].2 - parts[i + 1].2;
    assert!(close(step(0), 12.73 - 1.75), "{}", step(0));
    assert!(close(step(1), 12.52 - 5.5), "{}", step(1));
    assert!(close(step(2), 12.52 - 5.5), "{}", step(2));
    assert!(close(step(3), 12.52 - 1.75), "{}", step(3));
    // The stack is exactly the wanted 48.52 pt and centred on the axis
    // (the formula's baseline sits `height` below the origin).
    let top = parts[4].2 - 12.73;
    let bottom = parts[0].2;
    assert!(close(bottom - top, 48.52), "{}", bottom - top);
    let root = layout(&list, Style::DISPLAY, &otf);
    let axis = otf.params(SizeClass::Text).axis_height;
    assert!(
        close((top + bottom) / 2.0, root.height - axis),
        "{} {} {}",
        top,
        bottom,
        root.height
    );
}
