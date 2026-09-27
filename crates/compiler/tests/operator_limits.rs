//! A named operator's limit placement is on its atom (`MathAtom::limits`,
//! PLAN1 site 45): the kernel's declaration (`\lim` keeps `\mathop`'s
//! default `\displaylimits`, `\sin` is `\nolimits`), with a following
//! `\limits`/`\nolimits`/`\displaylimits` applied, whether the operator
//! was written directly or produced by a macro. `\mathrm{lim}` is an
//! ordinary run and carries none.
use flashtex_compiler::math::{AtomClass, Limits, MathAtom, Nucleus};
use flashtex_compiler::parser::{parse, Block, Inline};

fn atoms(preamble: &str, formula: &str) -> Vec<MathAtom> {
    let source = format!("\\documentclass{{article}}\n{preamble}\\begin{{document}}\n${formula}$\n\\end{{document}}\n");
    let parsed = parse(&source);
    parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => inlines.iter().find_map(|inline| match inline {
                Inline::Math { list, .. } => Some(list.atoms.clone()),
                _ => None,
            }),
            _ => None,
        })
        .expect("a formula")
}

fn limits_of(preamble: &str, formula: &str) -> Vec<(String, Option<Limits>)> {
    atoms(preamble, formula)
        .into_iter()
        .filter_map(|atom| match atom.nucleus {
            Nucleus::Text(text) => Some((text, atom.limits)),
            _ => None,
        })
        .collect()
}

#[test]
fn declared_placement() {
    assert_eq!(limits_of("", "\\lim_{n} a"), [("lim".to_string(), Some(Limits::DisplayLimits))]);
    assert_eq!(limits_of("", "\\sin x"), [("sin".to_string(), Some(Limits::NoLimits))]);
    assert_eq!(limits_of("", "\\mathrm{lim} x"), [("lim".to_string(), None)]);
}

#[test]
fn a_switch_after_the_operator_sets_it() {
    assert_eq!(limits_of("", "\\lim\\nolimits_{n} a"), [("lim".to_string(), Some(Limits::NoLimits))]);
    assert_eq!(limits_of("", "\\sin\\limits_{n} x"), [("sin".to_string(), Some(Limits::Limits))]);
    assert_eq!(limits_of("", "\\max \\nolimits\\displaylimits_{n} x"), [("max".to_string(), Some(Limits::DisplayLimits))]);
}

#[test]
fn an_operator_from_a_macro_carries_it_too() {
    assert_eq!(limits_of("\\newcommand\\lm{\\lim}\n", "\\lm_{n} a"), [("lim".to_string(), Some(Limits::DisplayLimits))]);
    assert_eq!(limits_of("\\newcommand\\lmn{\\lim\\limits}\n", "\\lmn_{n} a"), [("lim".to_string(), Some(Limits::Limits))]);
}

/// Every atom's (nucleus glyph or text, limits), for the `largesymbols`
/// operators and `\mathop{...}` below.
fn all_limits(formula: &str) -> Vec<(String, Option<Limits>)> {
    atoms("", formula)
        .into_iter()
        .map(|atom| {
            let name = match &atom.nucleus {
                Nucleus::Symbol(s) | Nucleus::Text(s) => s.clone(),
                Nucleus::Group(_) => "group".to_string(),
                other => format!("{other:?}"),
            };
            (name, atom.limits)
        })
        .collect()
}

/// TeX §1159: a limit switch sets the tail noad when it is an Op noad, so
/// it reaches `\bigcup`, `\bigcap` and the other `largesymbols` operators,
/// the integrals and a `\mathop{...}` exactly as it reaches `\lim`. Without
/// one they carry `None` (the renderer's default: `\displaylimits`, the
/// integrals `\nolimits`).
#[test]
fn a_switch_after_a_large_operator_sets_it() {
    let one = |s: &str, l: Option<Limits>| (s.to_string(), l);
    assert_eq!(all_limits("\\bigcup_{i=1}^n"), [one("⋃", None)]);
    assert_eq!(all_limits("\\bigcup\\limits_{i=1}^n"), [one("⋃", Some(Limits::Limits))]);
    assert_eq!(all_limits("\\bigcap\\nolimits_{i}"), [one("⋂", Some(Limits::NoLimits))]);
    assert_eq!(all_limits("\\sum\\displaylimits_{i}"), [one("∑", Some(Limits::DisplayLimits))]);
    assert_eq!(all_limits("\\int\\limits_0^1"), [one("∫", Some(Limits::Limits))]);
    // After the scripts too: the switch still sets the tail Op noad.
    assert_eq!(all_limits("\\bigcup_{i}\\limits^{n}"), [one("⋃", Some(Limits::Limits))]);
    // The last switch wins.
    assert_eq!(all_limits("\\bigoplus\\limits\\nolimits_i"), [one("⨁", Some(Limits::NoLimits))]);
    for (command, glyph) in [
        ("bigsqcup", "⨆"),
        ("bigvee", "⋁"),
        ("bigwedge", "⋀"),
        ("bigoplus", "⨁"),
        ("bigotimes", "⨂"),
        ("bigodot", "⨀"),
        ("biguplus", "⨄"),
        ("coprod", "∐"),
        ("prod", "∏"),
        ("oint", "∮"),
    ] {
        assert_eq!(all_limits(&format!("\\{command}\\limits_i")), [one(glyph, Some(Limits::Limits))], "\\{command}");
    }
    assert_eq!(all_limits("\\mathop{X}\\limits_k"), [one("group", Some(Limits::Limits))]);
}

/// An explicit `\displaylimits` is recorded on operators whose default is
/// `\nolimits` -- `\int` (plain.tex `\intop\nolimits`) and `\log` (latex.ltx
/// `\mathop{\operator@font log}\nolimits`) -- so the renderer can stack
/// their limits in display style.
#[test]
fn an_explicit_displaylimits_is_recorded_over_a_nolimits_default() {
    let one = |s: &str, l: Option<Limits>| (s.to_string(), l);
    assert_eq!(all_limits("\\int\\displaylimits_0^1"), [one("∫", Some(Limits::DisplayLimits))]);
    assert_eq!(all_limits("\\oint\\displaylimits_C"), [one("∮", Some(Limits::DisplayLimits))]);
    assert_eq!(all_limits("\\log\\displaylimits_2"), [one("log", Some(Limits::DisplayLimits))]);
    assert_eq!(all_limits("\\log_2"), [one("log", Some(Limits::NoLimits))]);
}

/// A switch after anything that is not an Op noad is TeX's "Limit controls
/// must follow a math operator" and changes nothing.
#[test]
fn a_switch_after_a_non_operator_is_ignored() {
    assert_eq!(all_limits("x\\limits_i"), [("x".to_string(), None)]);
    assert_eq!(all_limits("\\cup\\limits_i"), [("∪".to_string(), None)]);
}

/// The "Limit controls must follow a math operator" errors a formula
/// reports, with amsmath, xcolor and bm loaded.
fn limit_errors(formula: &str) -> usize {
    let source = format!(
        "\\documentclass{{article}}\n\\usepackage{{amsmath,xcolor,bm}}\n\\begin{{document}}\n${formula}$\n\\end{{document}}\n"
    );
    parse(&source).diagnostics.iter().filter(|d| d.message == "Limit controls must follow a math operator").count()
}

/// Provenance, not spelling, decides (TeX §1159). `\mathrm{lim}`,
/// `\mathrm{sin}` and `\text{lim}` only spell an operator, and a math
/// alphabet, `\textcolor`, a brace group or `\overset` hides an operator in
/// an Ord noad, so the switch is ignored with pdflatex's error. `\ensuremath`,
/// `\boldsymbol`, `\color`, `\rm`, `\mathop`, `\operatorname`, `\overbrace`
/// and `\sideset` leave a real Op noad, which takes it. Every row was
/// checked against pdflatex (TeX Live 2026): it reports the error on
/// exactly the first group and on none of the second. A `\color` between
/// the operator and the switch puts its whatsit in between; an
/// `\ensuremath` argument keeps its own tail's provenance.
#[test]
fn only_a_real_operator_noad_takes_the_switch() {
    let rejected = [
        "\\mathrm{lim}\\limits_{n} x",
        "\\mathrm{sin}\\nolimits_a",
        "\\text{lim}\\limits_n",
        "\\mathrm{Pr}\\limits_x",
        "\\mathrm{\\sum}\\limits_i",
        "\\mathbf{\\sum}\\limits_i",
        "\\textcolor{red}{\\sum}\\limits_i",
        "{\\sum}\\limits_i",
        "\\overset{a}{\\sum}\\limits_i",
        // `\color`'s whatsit becomes the tail between the operator and the switch.
        "\\sum\\color{red}\\limits_i",
        "\\sum\\textcolor{red}{}\\limits_i",
        // `\ensuremath` is transparent, so its argument's Ord group shows through.
        "\\ensuremath{\\mathrm{\\sum}}\\limits_i",
        "\\ensuremath{\\ensuremath{\\mathrm{\\sum}}}\\limits_i",
        "x\\limits_n",
        "\\limits_n",
    ];
    for formula in rejected {
        assert_eq!(limit_errors(formula), 1, "{formula}");
        assert!(atoms("\\usepackage{amsmath,xcolor}\n", formula).iter().all(|a| a.limits.is_none()), "{formula}");
    }
    let accepted = [
        "\\ensuremath{\\sum}\\limits_i",
        "\\boldsymbol{\\sum}\\limits_i",
        "\\color{red}\\sum\\limits_i",
        "\\rm\\sum\\limits_i",
        "\\mathop{\\mathrm{lim}}\\limits_n",
        "\\operatorname{foo}\\limits_i",
        "\\overbrace{ab}\\nolimits^n",
        "\\sideset{}{'}\\sum\\limits_i",
        "\\lim\\limits_n",
        "\\sin\\limits_n",
    ];
    for formula in accepted {
        assert_eq!(limit_errors(formula), 0, "{formula}");
        assert!(atoms("\\usepackage{amsmath,xcolor,bm}\n", formula).iter().any(|a| a.limits.is_some()), "{formula}");
    }
    // An unsupported command is reported once; the switch after its stand-in
    // adds no cascading error.
    assert_eq!(limit_errors("\\varlimsup\\limits_n"), 0);
}

/// A math alphabet's brace group is an Ord atom (TeX §1186) whatever it
/// holds: `\mathrm{\sum}` is one Ord group around an Op, so a script
/// after it sits beside the group, never in limits position. Checked
/// against pdflatex (TeX Live 2026, display style): the subscript of
/// `\mathrm{\sum}_i` sits beside the group, where `{\sum}_j` and bare
/// `\sum_k` behave as before.
#[test]
fn a_math_alphabet_group_around_an_operator_is_ord() {
    // One atom, an unscripted group around the operator, no limits flag:
    // the group as a whole is Ord, so scripts attach beside it.
    let list = atoms("", "\\mathrm{\\sum}");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(list[0].limits.is_none(), "{list:?}");
    let Nucleus::Group(inner) = &list[0].nucleus else {
        panic!("\\mathrm{{\\sum}} is one group, got {:?}", list[0].nucleus);
    };
    assert_eq!(inner.atoms.len(), 1, "{inner:?}");
    assert!(matches!(&inner.atoms[0].nucleus, Nucleus::Symbol(s) if s == "∑"), "{inner:?}");
    // A script after the group sits on the group atom itself...
    let list = atoms("", "\\mathrm{\\sum}_i");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(matches!(&list[0].nucleus, Nucleus::Group(_)), "{list:?}");
    assert!(list[0].subscript.is_some(), "{list:?}");
    // ...exactly like a bare brace group, and unlike a bare operator.
    let list = atoms("", "{\\sum}_j");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(matches!(&list[0].nucleus, Nucleus::Group(_)), "{list:?}");
    assert!(list[0].subscript.is_some(), "{list:?}");
    let list = atoms("", "\\sum_k");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(matches!(&list[0].nucleus, Nucleus::Symbol(s) if s == "∑"), "{list:?}");
    // `\mathbf` is a math alphabet too, so it groups the same way, while
    // `\boldsymbol` keeps its argument's class (still a bare operator).
    let list = atoms("", "\\mathbf{\\sum}");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(matches!(&list[0].nucleus, Nucleus::Group(_)), "{list:?}");
    let list = atoms("\\usepackage{bm}\n", "\\boldsymbol{\\sum}");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(matches!(&list[0].nucleus, Nucleus::Symbol(s) if s == "∑"), "{list:?}");
    // An all-ordinary body still flattens like a bare `{...}` group.
    let list = atoms("", "\\mathrm{\\alpha}");
    assert_eq!(list.len(), 1, "{list:?}");
    assert!(matches!(&list[0].nucleus, Nucleus::Symbol(s) if s == "α"), "{list:?}");
}

/// amsmath sets `\overset`/`\underset` as
/// `\binrel@{#2}{\mathop{\kern\z@#2}\limits...}`: the `\kern\z@` makes
/// the `\mathop` nucleus a box, never a single character, so TeX never
/// axis-centres the base the way it centres `\stackrel`'s plain
/// `\mathop{#2}` character nucleus. The compiler marks a lone ordinary
/// character base `Op` so the single-character axis-centring downstream
/// does not apply to it. Checked against pdflatex (TeX Live 2026):
/// `\overset{a}{b}`'s `a` at baseline 128.801 and `\underset{c}{d}`'s
/// `d` at 137.712 — both 0.969bp above the centred position, exactly
/// the `(h-d)/2` minus axis-height shift of a one-character nucleus.
#[test]
fn overset_and_underset_mark_a_lone_letter_base_as_a_non_character_op() {
    fn base_of(formula: &str) -> Vec<MathAtom> {
        let list = atoms("\\usepackage{amsmath}\n", formula);
        assert_eq!(list.len(), 1, "{formula}");
        match &list[0].nucleus {
            Nucleus::Stacked { base, .. } => base.atoms.clone(),
            other => panic!("{formula} is stacked, got {other:?}"),
        }
    }
    for formula in ["\\overset{a}{b}", "\\underset{c}{d}"] {
        let base = base_of(formula);
        assert_eq!(base.len(), 1, "{formula}: {base:?}");
        assert_eq!(base[0].class_override, Some(AtomClass::Op), "{formula}");
    }
    // `\stackrel` is a plain `\mathop{#2}` character nucleus, which TeX
    // does centre: its base stays untouched...
    let base = base_of("\\stackrel{e}{f}");
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].class_override, None);
    // ...as do bases that never centred: relations, operators, scripted
    // bases and multi-atom bases.
    for formula in ["\\overset{?}{=}", "\\overset{a}{\\sum}", "\\underset{x}{\\min}"] {
        let base = base_of(formula);
        assert_eq!(base.len(), 1, "{formula}: {base:?}");
        assert_eq!(base[0].class_override, None, "{formula}");
    }
    let base = base_of("\\overset{a}{b_2}");
    assert_eq!(base.len(), 1);
    assert_eq!(base[0].class_override, None);
    assert!(base[0].subscript.is_some(), "{base:?}");
    let base = base_of("\\overset{a}{b+c}");
    assert_eq!(base.len(), 3, "{base:?}");
    assert!(base.iter().all(|a| a.class_override.is_none()), "{base:?}");
}

/// TeX §1176: after `\color`'s whatsit the tail is not a noad, so a script
/// goes on a new empty Ord noad, not on the operator before the `\color`
/// (pdflatex sets `$\sum\color{red}\limits_{i}$`'s `i` beside an empty box).
#[test]
fn a_script_after_color_opens_an_empty_ord() {
    let list = atoms("\\usepackage{xcolor}\n", "\\sum\\color{red}_i x");
    assert_eq!(list.len(), 3, "{list:?}");
    assert!(matches!(&list[0].nucleus, Nucleus::Symbol(s) if s == "∑") && list[0].subscript.is_none());
    assert!(matches!(&list[1].nucleus, Nucleus::Symbol(s) if s.is_empty()) && list[1].subscript.is_some());
    // Without the `\color` the script is the operator's.
    let list = atoms("", "\\sum_i x");
    assert!(list[0].subscript.is_some(), "{list:?}");
}
