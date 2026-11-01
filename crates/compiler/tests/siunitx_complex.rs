//! siunitx cartesian `\complexnum` / `\complexqty` against pdflatex.
//!
//! Oracle (every value measured, not assumed): TeX Live 2026 pdflatex
//! (`/Library/TeX/texbin/pdflatex -interaction=nonstopmode t.tex`, class and
//! package files found via `kpsewhich`) reports 0 errors for the probe
//! `\complexnum{1+2i} and \complexnum{3-4i}` in a 10pt letter article, and
//! `mutool draw -F stext` gives these glyph origins at baseline y=134.76502:
//! `1` x=148.712, `+` x=155.905, `2` x=165.8676 with `i` directly after
//! (no gap), `and` x=176.94602, `3` x=196.32326, minus (CMSY10) x=203.51625,
//! `4` x=213.47885. The join is a Bin (4mu medmuskip each side, so the `1`
//! and `3` origins sit one digit width plus 4mu left of the sign); the minus
//! of `3-4i` is a real binary minus, and the root is upright `\mathrm{i}`.
//! A standalone `\complexqty{1+2i}{\metre}` is `(1 + 2i) m` at the same
//! baseline: `(` x=148.712, `1` x=152.58747, `+` x=159.78046, `2`
//! x=169.74306, `i` x=174.72435, `)` x=177.49396, `m` x=183.03318.
//!
//! The compiler crate's `layout` uses abstract preview fonts and geometry,
//! so absolute bp origins only exist on the render pipeline's exact route;
//! what this test pins is everything that determines them: the exact glyph
//! stream pdflatex sets, zero diagnostics, and layout identity (texts and
//! origins) with the plain-math equivalent `$1+2\mathrm{i}$`, which the
//! exact route already sets like pdflatex. The relative Bin structure is
//! font-independent (4mu each side of the join, none inside `2i`).
use flashtex_compiler::layout::layout;
use flashtex_compiler::parser::{parse, Block, Inline};

fn doc(body: &str) -> String {
    format!(
        "\\documentclass[10pt,letterpaper]{{article}}\\usepackage{{siunitx}}\\begin{{document}}{body}\\end{{document}}"
    )
}

/// Laid-out `(text, x)` items with the diagnostics; asserts nothing.
fn laid_out(body: &str) -> (Vec<(String, f64)>, Vec<String>) {
    let parsed = parse(&doc(body));
    let diags: Vec<String> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    let items: Vec<(String, f64)> = layout(&parsed.blocks)
        .iter()
        .flat_map(|page| page.items.iter())
        .map(|item| (item.text.clone(), item.x_pt))
        .collect();
    (items, diags)
}

/// The math formulas of the first paragraph, each as spelled atoms.
fn formulas(body: &str) -> (Vec<Vec<String>>, Vec<String>) {
    use flashtex_compiler::math::Nucleus;
    fn spell(list: &flashtex_compiler::math::MathList) -> Vec<String> {
        list.atoms
            .iter()
            .map(|atom| match &atom.nucleus {
                Nucleus::Symbol(s) => format!("S:{s}"),
                Nucleus::Text(s) => format!("T:{s}"),
                Nucleus::Group(g) => format!("G:{:?}", spell(g)),
                Nucleus::Space { em, font_em } => format!("SP:{em}/{font_em}"),
                other => format!("?:{other:?}"),
            })
            .collect()
    }
    let parsed = parse(&doc(body));
    let diags: Vec<String> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    let lists = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph");
    let formulas = lists
        .iter()
        .filter_map(|inline| match inline {
            Inline::Math { list, .. } => Some(spell(list)),
            _ => None,
        })
        .collect();
    (formulas, diags)
}

/// The wave-builder probe: glyph stream, diagnostics and spacing match
/// pdflatex, and the layout is identical to the plain-math equivalent.
#[test]
fn complexnum_probe_matches_pdflatex() {
    let (items, diagnostics) = laid_out("\\complexnum{1+2i} and \\complexnum{3-4i}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let texts: Vec<&str> = items.iter().map(|(text, _)| text.as_str()).collect();
    assert_eq!(texts, ["1", "+", "2", "i", "and", "3", "−", "4", "i"]);
    // The join carries Bin spacing on both sides while `2i` is gapless:
    // the `+` sits further from `1` than `i` sits from `2`, exactly as the
    // measured pdflatex origins do (7.193bp vs 4.981bp).
    let x = |i: usize| items[i].1;
    assert!(x(1) - x(0) > x(3) - x(2), "{items:?}");
    assert!(x(6) - x(5) > x(8) - x(7), "{items:?}");
    // Same atoms as hand-written math: every origin identical.
    let (plain, plain_diagnostics) = laid_out("$1+2\\mathrm{i}$ and $3-4\\mathrm{i}$");
    assert!(plain_diagnostics.is_empty(), "{plain_diagnostics:?}");
    assert_eq!(items.len(), plain.len(), "{items:?} vs {plain:?}");
    let mut worst = 0.0f64;
    for ((text, x), (plain_text, plain_x)) in items.iter().zip(plain.iter()) {
        assert_eq!(text, plain_text, "{items:?} vs {plain:?}");
        worst = worst.max((x - plain_x).abs());
    }
    assert!(
        worst <= 0.1,
        "origins drift by {worst}: {items:?} vs {plain:?}"
    );
}

/// `\complexqty{1+2i}{\metre}` wraps both parts in parentheses before the
/// unit's quantity product; the parenthesised number is the complexnum
/// atoms, and the kern is `\qty`'s own product.
#[test]
fn complexqty_wraps_both_parts_in_parentheses() {
    let (lists, diagnostics) = formulas("\\complexqty{1+2i}{\\metre}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(lists.len(), 1, "{lists:?}");
    let atoms = &lists[0];
    assert_eq!(&atoms[..6], ["S:(", "S:1", "S:+", "S:2", "T:i", "S:)"]);
    assert_eq!(atoms[7], "T:m");
    match atoms[6].as_str() {
        atom if atom.starts_with("SP:") => {
            let em: f64 = atom["SP:".len()..]
                .split('/')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert!((em - 1.0 / 6.0).abs() < 1e-12, "{atoms:?}");
            assert!(atom.ends_with("/true"), "{atoms:?}");
        }
        other => panic!("expected the quantity-product kern, got {other} in {atoms:?}"),
    }
    let (items, _) = laid_out("\\complexqty{1+2i}{\\metre}");
    let texts: Vec<&str> = items.iter().map(|(text, _)| text.as_str()).collect();
    assert_eq!(texts, ["(", "1", "+", "2", "i", ")", "m"]);
}

/// One-sided quantities take the plain product like `\qty`, and an empty
/// number sets nothing at all (pdflatex sets no unit for
/// `\complexqty{}{\metre}` and reports no error).
#[test]
fn single_part_quantities_take_no_parentheses() {
    let (lists, diagnostics) = formulas("\\complexqty{5}{\\metre}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(lists, [["S:5", "SP:0.16666666666666666/true", "T:m"]]);
    let (lists, _) = formulas("\\complexqty{2i}{\\metre}");
    assert_eq!(
        lists,
        [["S:2", "T:i", "SP:0.16666666666666666/true", "T:m"]]
    );
    let (lists, _) = formulas("\\complexqty{1+i}{\\metre}");
    assert_eq!(
        lists,
        [[
            "S:(".to_string(),
            "S:1".to_string(),
            "S:+".to_string(),
            "T:i".to_string(),
            "S:)".to_string(),
            "SP:0.16666666666666666/true".to_string(),
            "T:m".to_string(),
        ]]
    );
    // No unit, no parentheses and no product.
    let (lists, diagnostics) = formulas("\\complexqty{1+2i}{}");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(lists, [["S:1", "S:+", "S:2", "T:i"]]);
    // An empty number is empty, with no diagnostic.
    let (lists, diagnostics) = formulas("E\\complexnum{}F");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert!(lists.is_empty(), "{lists:?}");
    let (lists, diagnostics) = formulas("E\\complexqty{}{\\metre}F");
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert!(lists.is_empty(), "{lists:?}");
}

/// Real-only, imaginary-only, unity (`1+i` is `1 + i`, `01` counts as
/// unity but `1.0` keeps its number), the `j` root (set as `i`) and
/// uncertainties all match pdflatex with no diagnostics.
#[test]
fn parts_unities_and_roots_match_pdflatex() {
    for (input, expected) in [
        ("5", ["S:5"].as_slice()),
        ("2i", &["S:2", "T:i"]),
        ("i", &["T:i"]),
        ("-i", &["S:-", "T:i"]),
        ("+i", &["T:i"]),
        ("1+i", &["S:1", "S:+", "T:i"]),
        ("1+01i", &["S:1", "S:+", "T:i"]),
        ("1+2j", &["S:1", "S:+", "S:2", "T:i"]),
        ("3-4i", &["S:3", "S:-", "S:4", "T:i"]),
        ("-1-2i", &["S:-", "S:1", "S:-", "S:2", "T:i"]),
        (
            "1.5+2.5i",
            &["S:1", "S:.", "S:5", "S:+", "S:2", "S:.", "S:5", "T:i"],
        ),
        (
            "1.2(3)+4i",
            &[
                "S:1", "S:.", "S:2", "S:(", "S:3", "S:)", "S:+", "S:4", "T:i",
            ],
        ),
        ("1+-0.2i", &["S:1", "S:±", "S:0", "S:.", "S:2", "T:i"]),
    ] {
        let (lists, diagnostics) = formulas(&format!("\\complexnum{{{input}}}"));
        assert!(diagnostics.is_empty(), "{input}: {diagnostics:?}");
        assert_eq!(lists.len(), 1, "{input}: {lists:?}");
        assert_eq!(lists[0], expected, "{input}");
    }
}

/// What pdflatex rejects, this compiler diagnoses: a second root, text
/// after the root, an unparseable side and any exponent marker anywhere
/// (`Invalid number '1d3+2i'`, nothing set). Polar form is valid siunitx
/// input this model does not implement, so it is diagnosed and kept.
#[test]
fn invalid_and_polar_inputs_are_diagnosed() {
    for input in ["abc", "1e3+2i", "2+1e3i", "2e3i", "1j+2", "2i3", "1i+2i", "1++2i"] {
        let (lists, diagnostics) = formulas(&format!("\\complexnum{{{input}}}"));
        assert_eq!(
            diagnostics,
            [format!("siunitx: invalid number '{input}'")],
            "{input}"
        );
        assert!(lists.is_empty(), "{input}: {lists:?}");
    }
    let (lists, diagnostics) = formulas("\\complexnum{1:30}");
    assert_eq!(
        diagnostics,
        ["siunitx: \\complexnum polar form '1:30' is not implemented"]
    );
    assert_eq!(lists, [["T:1:30"]]);
    let (lists, diagnostics) = formulas("\\complexqty{1:30}{\\metre}");
    assert_eq!(
        diagnostics,
        ["siunitx: \\complexqty polar form '1:30' is not implemented"]
    );
    assert_eq!(lists, [["T:1:30"]]);
}

/// A lone real part is an ordinary number, exponents included
/// (pdflatex sets `\complexnum{1e3}` as `1 \times 10^{3}` with no error):
/// identical atoms to `\num` and `\qty` with the same input.
#[test]
fn lone_real_part_takes_full_number_formatting() {
    for input in ["1e3", "1.5e-3", "12"] {
        let (complex, diagnostics) = formulas(&format!("\\complexnum{{{input}}}"));
        assert!(diagnostics.is_empty(), "{input}: {diagnostics:?}");
        let (num, _) = formulas(&format!("\\num{{{input}}}"));
        assert_eq!(complex, num, "{input}");
        let (complex_qty, diagnostics) = formulas(&format!("\\complexqty{{{input}}}{{\\metre}}"));
        assert!(diagnostics.is_empty(), "{input}: {diagnostics:?}");
        let (qty, _) = formulas(&format!("\\qty{{{input}}}{{\\metre}}"));
        assert_eq!(complex_qty, qty, "{input}");
    }
}

/// The names are siunitx's: free for `\newcommand` without the package and
/// taken with it, exactly like `\si`/`\unit`
/// (`tests/project_macro_names.rs`).
#[test]
fn names_are_package_gated_like_other_siunitx_names() {
    use flashtex_compiler::incremental::{compile_full, CompileOutput};
    use flashtex_compiler::layout::LayoutConstraints;
    fn compile(source: &str) -> CompileOutput {
        compile_full(source, LayoutConstraints::default())
    }
    fn errors(output: &CompileOutput) -> Vec<String> {
        output
            .diagnostics
            .iter()
            .filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error)
            .map(|d| d.message.clone())
            .collect()
    }
    for name in ["complexnum", "complexqty"] {
        let free = format!(
            "\\documentclass{{article}}\n\\newcommand{{\\{name}}}[1]{{C#1}}\n\\begin{{document}}\n\\{name}{{1+2i}}\n\\end{{document}}"
        );
        assert_eq!(errors(&compile(&free)), Vec::<String>::new(), "{name} free");
        let taken = format!(
            "\\documentclass{{article}}\n\\usepackage{{siunitx}}\n\\newcommand{{\\{name}}}[1]{{C#1}}\n\\begin{{document}}\nx\n\\end{{document}}"
        );
        assert_eq!(
            errors(&compile(&taken)),
            [format!("LaTeX Error: Command \\{name} already defined.")],
            "{name} taken"
        );
    }
}

/// Inside a formula the commands join the surrounding list like every
/// other siunitx command, with no diagnostics.
#[test]
fn complex_commands_work_in_math_mode() {
    let parsed = parse(&doc(
        "$\\complexnum{1+2i}$ and $\\complexqty{3-4i}{\\metre}$",
    ));
    let diagnostics: Vec<String> = parsed
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let maths: Vec<Vec<String>> = parsed
        .blocks
        .iter()
        .find_map(|block| match block {
            Block::Paragraph(inlines) => Some(inlines),
            _ => None,
        })
        .expect("a paragraph")
        .iter()
        .filter_map(|inline| match inline {
            Inline::Math { list, .. } => Some(
                list.atoms
                    .iter()
                    .map(|atom| format!("{:?}", atom.nucleus))
                    .collect(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(maths.len(), 2, "{maths:?}");
    assert!(maths[0].join(" ").contains("\"1\""), "{maths:?}");
    assert!(maths[1].join(" ").contains('('), "{maths:?}");
}
