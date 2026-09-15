//! HW1 preamble coverage: geometry, enumitem labels and package warnings.
use flashtex_compiler::parser::parse;

const HW1: &str = include_str!("../../../fixtures/real-world/hw1/HW1.tex");

fn messages(source: &str) -> Vec<String> {
    parse(source)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

#[test]
fn hw1_diagnostic_inventory() {
    let parsed = parse(HW1);
    for d in &parsed.diagnostics {
        println!("{:?} | {}", d.severity, d.message);
    }
    println!("HW1 diagnostics: {}", parsed.diagnostics.len());
}

#[test]
fn hw1_enumerate_option_does_not_leak_as_text() {
    let blocks = format!("{:?}", parse(HW1).blocks);
    assert!(
        !blocks.contains("[(a)]"),
        "enumerate option typeset as text"
    );
    assert!(blocks.contains("\"(a)\"") && blocks.contains("\"(b)\""));
}

#[test]
fn enumerate_label_templates() {
    let doc = |options: &str| {
        format!(
            "\\documentclass{{article}}\\begin{{document}}\\begin{{enumerate}}{options}\\item x\\item y\\item z\\item w\\end{{enumerate}}\\end{{document}}"
        )
    };
    let blocks = |options: &str| format!("{:?}", parse(&doc(options)).blocks);
    assert!(blocks("[(a)]").contains("\"(d)\""));
    assert!(blocks("[i.]").contains("\"iv.\""));
    assert!(blocks("[A)]").contains("\"C)\""));
    assert!(blocks("[label=\\Roman*.]").contains("\"III.\""));
    assert!(blocks("").contains("\"4.\""));
}

#[test]
fn packages_matching_the_fixed_layout_do_not_warn() {
    let preamble = |line: &str| {
        messages(&format!(
            "\\documentclass{{article}}{line}\\begin{{document}}x\\end{{document}}"
        ))
    };
    for line in [
        "\\usepackage[margin=1in]{geometry}",
        "\\usepackage[margin=72.27pt]{geometry}",
        "\\usepackage[utf8]{inputenc}",
        "\\usepackage[T1]{fontenc}",
        "\\usepackage[shortlabels]{enumitem}",
        // The AMS math packages are set by this crate, not merely recognised
        // (crate::math's nuclei, `layout_nucleus`, `MathPackages::provides`).
        "\\usepackage{amsmath,amssymb}",
        "\\usepackage{amsfonts}",
        "\\usepackage[centertags,sumlimits,nointlimits,namelimits,reqno]{amsmath}",
    ] {
        assert!(preamble(line).is_empty(), "{line}: {:?}", preamble(line));
    }
    for line in [
        "\\usepackage[margin=2cm]{geometry}",
        "\\usepackage{geometry}",
        "\\usepackage[a4paper,margin=1in]{geometry}",
        "\\usepackage{microtype}",
        // amsmath options that move real output are not read here.
        "\\usepackage[leqno]{amsmath}",
        "\\usepackage[fleqn]{amsmath}",
        "\\usepackage[tbtags]{amsmath}",
        "\\usepackage[nosumlimits]{amsmath}",
        "\\usepackage[intlimits]{amsmath}",
        "\\usepackage[nonamelimits]{amsmath}",
    ] {
        assert!(
            preamble(line)
                .iter()
                .any(|m| m.contains("recognised but not implemented")),
            "{line} must still report a gap"
        );
    }
    // itemsep/topsep/leftmargin are implemented, so setting only those keys
    // is silent...
    assert!(
        preamble("\\setlist[enumerate]{itemsep=1em}").is_empty(),
        "{:?}",
        preamble("\\setlist[enumerate]{itemsep=1em}")
    );
    assert!(
        preamble("\\setlist[enumerate]{leftmargin=*}").is_empty(),
        "{:?}",
        preamble("\\setlist[enumerate]{leftmargin=*}")
    );
    // ...but a key with no layout equivalent still reports a gap, by name.
    assert!(preamble("\\setlist[enumerate]{parsep=1em}")
        .iter()
        .any(|m| m.contains("\\setlist") && m.contains("parsep")));
}

/// The claim `\usepackage{amsmath}` no longer makes has to still be made, by
/// the construct that earns it. Loading the package is silent; an amsmath
/// construct this crate does not set names itself at its own span, so
/// narrowing the package warning trades no false positive for a false
/// negative.
#[test]
fn unimplemented_amsmath_constructs_still_report_themselves() {
    let doc = |body: &str| {
        messages(&format!(
            "\\documentclass{{article}}\\usepackage{{amsmath,amssymb}}\\begin{{document}}{body}\\end{{document}}"
        ))
    };
    // Loading the packages says nothing on its own.
    assert!(doc("x").is_empty(), "{:?}", doc("x"));

    // A construct that is set draws nothing either.
    for body in [
        "$\\dfrac{1}{2}$",
        "$\\binom{n}{k}$",
        "$\\sum_{\\substack{i<j}} x$",
        "$\\operatorname{foo}(x)$",
        "$\\boxed{x}$",
        "$\\mathbb{R} \\nleq \\square$",
        "$\\lim_{x\\to 0} f$",
        "\\begin{align} a &= b \\end{align}",
        "\\begin{gather} a = b \\end{gather}",
        "$\\begin{dcases} a & b \\end{dcases}$",
        "$\\xrightarrow{f}$",
    ] {
        assert!(doc(body).is_empty(), "{body}: {:?}", doc(body));
    }

    // A construct that is not still names itself, at its own span.
    for (body, command) in [
        ("$\\smash{x}$", "\\smash"),
        ("$a\\mspace{3mu}b$", "\\mspace"),
        ("$\\varinjlim x$", "\\varinjlim"),
        ("$\\sideset{_a^b}{_c^d}\\sum$", "\\sideset"),
        ("$\\begin{pmatrix}\\hdotsfor{2}\\end{pmatrix}$", "\\hdotsfor"),
        // `\shoveleft`/`\shoveright` are implemented in `multline` (see
        // `multline_shove`), so they no longer belong in this inventory; they
        // still report themselves in displays that cannot shove, e.g.
        ("\\begin{gather} \\shoveleft{a} \\\\ b \\end{gather}", "\\shoveleft"),
    ] {
        let found = doc(body);
        assert!(
            found
                .iter()
                .any(|m| m.contains(command) && m.contains("not supported")),
            "{body} must still report {command}: {found:?}"
        );
        // ...and never as a blanket claim about the package.
        assert!(
            !found
                .iter()
                .any(|m| m.contains("packages") && m.contains("recognised but not implemented")),
            "{body} must not blame the package: {found:?}"
        );
    }
}

/// The owner's own homework: neither file may be told that the math it is
/// full of was not typeset. `microtype` is a separate, still-true line --
/// this crate really does not protrude or expand -- and the render pipeline
/// supersedes that one for its own consumers.
#[test]
fn homework_is_not_told_its_math_is_unimplemented() {
    const HW2: &str = include_str!("../../../fixtures/real-world/hw2/HW2.tex");
    for (name, source) in [("HW1", HW1), ("HW2", HW2)] {
        let package_warnings: Vec<String> = messages(source)
            .into_iter()
            .filter(|m| m.contains("recognised but not implemented"))
            .collect();
        assert!(
            !package_warnings.iter().any(|m| m.contains("amsmath")
                || m.contains("amssymb")
                || m.contains("amsfonts")),
            "{name} is told its AMS math is unimplemented: {package_warnings:?}"
        );
        assert_eq!(
            package_warnings,
            vec!["packages microtype are recognised but not implemented".to_string()],
            "{name}"
        );
    }
}
