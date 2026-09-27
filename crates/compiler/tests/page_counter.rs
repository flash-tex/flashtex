//! LaTeX's `page` counter as a real, settable parser counter.
//!
//! `page` is always defined and starts at 1 (latex.ltx
//! `\countdef\c@page=0 \c@page=1`), so `\fnsymbol{page}` resolves against
//! it instead of reporting "No counter 'page' defined", and
//! `\setcounter`/`\addtocounter` keep the mirror current through the
//! engine's observed-register markers.
//!
//! What this does NOT cover (out of scope for the parser side, pinned
//! here): `\Roman{page}`/`\roman{page}`/`\alph{page}`/`\Alph{page}` and
//! `\arabic{page}` expand in the engine against its own register, which
//! starts at 0 rather than 1 there, and `\thepage` resolves at layout
//! time on the physical page rather than from this counter. Those need
//! the engine/register and layout halves; this file pins only the
//! parser-owned behaviour.

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

fn texts(output: &CompileOutput) -> Vec<String> {
    output
        .pages
        .iter()
        .flat_map(|page| &page.items)
        .map(|item| item.text.clone())
        .collect()
}

fn errors(output: &CompileOutput) -> Vec<String> {
    output
        .diagnostics
        .iter()
        .filter(|d| d.severity == flashtex_compiler::diagnostics::Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

#[test]
fn fnsymbol_page_reads_the_initial_page_counter() {
    // `\fnsymbol`'s first symbol is U+2217 ASTERISK OPERATOR (pdflatex
    // oracle; the engine's own table agrees).
    let source = r"\documentclass{article}\begin{document}x \fnsymbol{page} y\end{document}";
    let output = compile(source);
    assert_eq!(
        errors(&output),
        Vec::<String>::new(),
        "{:?}",
        output.diagnostics
    );
    let texts = texts(&output);
    assert!(texts.iter().any(|t| t == "\u{2217}"), "{texts:?}");
}

#[test]
fn setcounter_page_updates_the_parser_mirror() {
    let source = r"\documentclass{article}\begin{document}\setcounter{page}{3}x \fnsymbol{page} y\end{document}";
    let output = compile(source);
    assert_eq!(
        errors(&output),
        Vec::<String>::new(),
        "{:?}",
        output.diagnostics
    );
    let texts = texts(&output);
    assert!(texts.iter().any(|t| t == "\u{2021}"), "{texts:?}");
}

#[test]
fn addtocounter_page_updates_the_parser_mirror() {
    // From an explicitly set base (5) the engine and the mirror agree, so
    // `+2` makes 7, whose `\fnsymbol` is the doubled asterisk. Note:
    // `\addtocounter` on the never-set register inherits the engine
    // register's own start value (see the module docs), so only the
    // explicitly based form is pinned here.
    let source = r"\documentclass{article}\begin{document}\setcounter{page}{5}\addtocounter{page}{2}x \fnsymbol{page} y\end{document}";
    let output = compile(source);
    assert_eq!(
        errors(&output),
        Vec::<String>::new(),
        "{:?}",
        output.diagnostics
    );
    let texts = texts(&output);
    assert!(texts.iter().any(|t| t == "\u{2217}\u{2217}"), "{texts:?}");
}

#[test]
fn report_class_has_the_page_counter_too() {
    let source = r"\documentclass{report}\begin{document}x \fnsymbol{page} y\end{document}";
    let output = compile(source);
    assert_eq!(
        errors(&output),
        Vec::<String>::new(),
        "{:?}",
        output.diagnostics
    );
    let texts = texts(&output);
    assert!(texts.iter().any(|t| t == "\u{2217}"), "{texts:?}");
}
