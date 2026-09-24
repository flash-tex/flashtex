//! `\usepackage{calc}` runs the real `calc.sty` (v4.3b, vendored as
//! `crate::packages::CALC_STY`) in the expansion engine: its
//! `\setlength`/`\addtolength`/`\setcounter`/`\addtocounter`/`\stepcounter`
//! replace the engine's, and every value below is what the engine's
//! registers hold afterwards.
//!
//! Expected values are pdflatex's (TeX Live 2026), measured with
//! `\typeout{\the\mylen}` (`\themycnt`, ...) after the same assignment in
//! an `article` document.
use flashtex_compiler::diagnostics::Severity;
use flashtex_compiler::parser::{parse, Block, Inline};

/// `(tag, assignment, what to print)`, and pdflatex's printed value.
const CASES: &[(&str, &str, &str, &str)] = &[
    ("paren", "\\setlength{\\mylen}{(1cm+2cm)*2/3}", "\\the\\mylen", "56.90549pt"),
    ("real", "\\setlength{\\mylen}{1pt*\\real{1.5}}", "\\the\\mylen", "1.5pt"),
    ("maxmin", "\\setlength{\\mylen}{\\maxof{1pt}{2pt}+\\minof{3pt}{4pt}}", "\\the\\mylen", "5.0pt"),
    ("ratio", "\\setlength{\\mylen}{10pt*\\ratio{2pt}{3pt}}", "\\the\\mylen", "6.66672pt"),
    ("bskip", "\\setlength{\\mylen}{2\\baselineskip - 3pt}", "\\the\\mylen", "21.0pt"),
    ("em", "\\setlength{\\mylen}{1em*3/2}", "\\the\\mylen", "15.00002pt"),
    ("div", "\\setlength{\\mylen}{1in / 7}", "\\the\\mylen", "10.32428pt"),
    ("parskip", "\\setlength{\\parskip}{24pt plus 2pt}", "\\the\\parskip", "24.0pt plus 2.0pt"),
    ("parskipadd", "\\addtolength{\\parskip}{1pt*2}", "\\the\\parskip", "26.0pt plus 2.0pt"),
    ("textwidth", "\\setlength{\\textwidth}{0.5\\textwidth - 1cm}", "\\the\\textwidth", "144.04726pt"),
    ("setcnt", "\\setcounter{mycnt}{2*3+1}", "\\themycnt", "7"),
    ("addcnt", "\\addtocounter{mycnt}{3*(2+1)}", "\\themycnt", "16"),
    ("valcnt", "\\setcounter{mycnt}{\\value{mycnt}*2}", "\\themycnt", "32"),
    // calc's `\stepcounter` walks `\cl@<counter>` with `\@stpelt`.
    ("reset", "\\setcounter{kid}{5}\\setcounter{grand}{4}\\stepcounter{par}", "\\thepar/\\thekid/\\thegrand", "1/0/0"),
    ("resettwo", "\\setcounter{kid}{2}\\setcounter{grand}{3}\\stepcounter{kid}", "\\thekid/\\thegrand", "3/0"),
];

fn document(preamble: &str, body: &str) -> String {
    format!(
        "\\documentclass{{article}}\n\\usepackage{{calc}}\n{preamble}\\newlength{{\\mylen}}\\newcounter{{mycnt}}\\newcounter{{par}}\\newcounter{{kid}}[par]\\newcounter{{grand}}[kid]\n\\begin{{document}}\n{body}\n\\end{{document}}\n"
    )
}

fn text(source: &str) -> (String, Vec<String>) {
    let parsed = parse(source);
    let errors = parsed.diagnostics.iter().filter(|d| d.severity == Severity::Error).map(|d| d.message.clone()).collect();
    let mut out = String::new();
    for block in &parsed.blocks {
        if let Block::Paragraph(inlines) = block {
            for inline in inlines {
                if let Inline::Text { text, .. } = inline {
                    out.push_str(text);
                    out.push(' ');
                }
            }
        }
    }
    (out, errors)
}

#[test]
fn calc_expressions_match_pdflatex() {
    let body: String = CASES.iter().map(|(tag, assign, print, _)| format!("{assign}{tag}={print}\n")).collect();
    let (got, errors) = text(&document("", &body));
    assert!(errors.is_empty(), "{errors:?}");
    let words: Vec<&str> = got.split_whitespace().collect();
    for (tag, _, _, want) in CASES {
        let prefix = format!("{tag}=");
        let found = words.iter().position(|w| w.starts_with(&prefix)).unwrap_or_else(|| panic!("{tag} missing in {got:?}"));
        // `24.0pt plus 2.0pt` spans three words.
        let value: String = std::iter::once(&words[found][prefix.len()..])
            .chain(words[found + 1..].iter().copied().take_while(|w| !w.contains('=')))
            .collect::<Vec<_>>()
            .join(" ");
        // Text runs split at `/`; compare without spaces.
        assert_eq!(value.replace(' ', ""), want.replace(' ', ""), "{tag}");
    }
}

/// A package length the host models (array's `\extrarowheight`, natbib's
/// `\bibsep`) is a register once its package is loaded, so calc's
/// `\setlength` assigns it and the parser sees the same `\setlength` it
/// sees without calc: no new diagnostics.
#[test]
fn package_lengths_take_calc_assignments_like_the_kernel_setlength() {
    for (package, length) in [("array", "extrarowheight"), ("natbib", "bibsep"), ("booktabs", "heavyrulewidth"), ("longtable", "LTpre")] {
        let messages = |calc: bool| {
            let source = format!(
                "\\documentclass{{article}}\n{}\\usepackage{{{package}}}\n\\begin{{document}}\n\\setlength{{\\{length}}}{{3pt}}\\addtolength{{\\{length}}}{{1pt*2}}x\n\\end{{document}}\n",
                if calc { "\\usepackage{calc}\n" } else { "" }
            );
            parse(&source).diagnostics.iter().map(|d| d.message.clone()).collect::<Vec<_>>()
        };
        assert_eq!(messages(true), messages(false), "{package}");
        assert!(messages(true).iter().all(|m| !m.contains("not supported")), "{package}: {:?}", messages(true));
    }
}

/// Loading the package reports nothing.
#[test]
fn loading_is_silent() {
    let parsed = parse("\\documentclass{article}\n\\usepackage{calc}\n\\begin{document}\nx\n\\end{document}\n");
    let messages: Vec<_> = parsed.diagnostics.iter().map(|d| &d.message).collect();
    assert!(messages.is_empty(), "{messages:?}");
}
