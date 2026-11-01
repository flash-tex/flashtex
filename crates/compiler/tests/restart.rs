//! Restart and repair determinism.
//!
//! The worker holds warm sessions in memory. If it is restarted — crash, upgrade,
//! eviction — the author must not be able to tell from the output. A restarted
//! worker that produces different text, different spans or different diagnostics
//! than the one it replaced is a bug the user experiences as their document
//! changing by itself.
//!
//! "Restart" here means a fresh `Session`, which is exactly the state a new
//! process begins with.

use flashtex_compiler::incremental::{compile_full, Session};
use flashtex_compiler::layout::LayoutConstraints;

fn render(session: &mut Session, text: &str) -> String {
    format!(
        "{:#?}",
        session.compile(text, LayoutConstraints::default()).output
    )
}

fn clean(text: &str) -> String {
    format!("{:#?}", compile_full(text, LayoutConstraints::default()))
}

/// A realistic editing session: valid, broken, repaired, extended.
const SEQUENCE: &[&str] = &[
    "\\section{Intro}\nFirst paragraph.\n",
    "\\section{Intro}\nFirst paragraph edited.\n",
    // broken: unmatched brace mid-typing
    "\\section{Intro}\nFirst paragraph edited. {oops\n",
    // repaired
    "\\section{Intro}\nFirst paragraph edited. {fixed}\n",
    // a second section renumbers nothing yet but adds global state
    "\\section{Intro}\nFirst paragraph edited. {fixed}\n\n\\section{Second}\nMore.\n",
    // a forward reference, which forces the convergence pass
    "\\section{Intro}\nSee \\ref{s2}.\n\n\\section{Second}\\label{s2}\nMore.\n",
    // broken again: unterminated math while typing
    "\\section{Intro}\nSee \\ref{s2} and $x^\n\n\\section{Second}\\label{s2}\nMore.\n",
    // repaired again
    "\\section{Intro}\nSee \\ref{s2} and $x^2$.\n\n\\section{Second}\\label{s2}\nMore.\n",
];

#[test]
fn a_restart_at_any_point_produces_identical_output() {
    // Walk the sequence in one warm session, and at every step compare against a
    // session that started fresh at that step — which is what a restart gives you.
    let mut warm = Session::new();
    for (step, text) in SEQUENCE.iter().enumerate() {
        let warm_out = render(&mut warm, text);
        let restarted = render(&mut Session::new(), text);
        let from_scratch = clean(text);

        assert_eq!(
            warm_out, restarted,
            "step {step}: a restarted worker produced different output for the same text"
        );
        assert_eq!(
            warm_out, from_scratch,
            "step {step}: warm output diverged from a clean build"
        );
    }
}

#[test]
fn repairing_a_document_returns_it_to_its_pre_breakage_output() {
    // Typing a broken construct and deleting it again must land exactly where it
    // started. If reuse retains anything from the broken state, this catches it.
    let good = "\\section{Intro}\nStable text here.\n";
    let broken = "\\section{Intro}\nStable text here. {\n";

    let mut session = Session::new();
    let before = render(&mut session, good);
    let _ = render(&mut session, broken);
    let after = render(&mut session, good);

    assert_eq!(
        before, after,
        "a break-then-repair cycle changed the output"
    );
    assert_eq!(
        after,
        clean(good),
        "repaired output diverged from a clean build"
    );
}

#[test]
fn diagnostics_are_deterministic_across_restarts() {
    let broken = "\\section{Intro}\nText {unclosed and \\nosuch here.\n";

    let first = compile_full(broken, LayoutConstraints::default());
    let mut session = Session::new();
    session.compile("\\section{Intro}\nText.\n", LayoutConstraints::default());
    let warm = session.compile(broken, LayoutConstraints::default());

    let messages = |ds: &[flashtex_compiler::diagnostics::Diagnostic]| {
        ds.iter()
            .map(|d| (d.severity, d.message.clone(), d.span))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        messages(&first.diagnostics),
        messages(&warm.output.diagnostics),
        "the same broken document produced different diagnostics warm vs cold"
    );
}

#[test]
fn maketitle_title_block_survives_incremental_edits_without_drift() {
    // `Block::TitleBlock` needed its own `shift_block`/signature handling
    // (see `incremental.rs`); this exercises that path directly — editing
    // body text after `\maketitle` (the title block's spans must shift,
    // not vanish or duplicate), then the title itself, then the author
    // list — each checked warm against a clean rebuild.
    let sequence = [
        "\\title{Draft Title}\\author{A. Author}\\begin{document}\\maketitle\nBody text.\n\\end{document}",
        "\\title{Draft Title}\\author{A. Author}\\begin{document}\\maketitle\nBody text edited.\n\\end{document}",
        "\\title{Final Title}\\author{A. Author}\\begin{document}\\maketitle\nBody text edited.\n\\end{document}",
        "\\title{Final Title}\\author{A. Author \\and B. Author}\\begin{document}\\maketitle\nBody text edited.\n\\end{document}",
    ];
    let mut session = Session::new();
    for (step, text) in sequence.iter().enumerate() {
        let warm = render(&mut session, text);
        assert_eq!(
            warm,
            clean(text),
            "step {step}: warm title-block output diverged from a clean build"
        );
    }
}

#[test]
fn a_long_editing_session_never_drifts_from_a_clean_build() {
    // Many small edits in one session: drift, if any, accumulates.
    let mut session = Session::new();
    let mut text = String::from("\\section{Doc}\n");
    for i in 0..120 {
        text.push_str(&format!("Paragraph {i} with $x^{i}$ content.\n\n"));
        let incremental = render(&mut session, &text);
        if i % 17 == 0 {
            assert_eq!(
                incremental,
                clean(&text),
                "drift appeared after {i} accumulated edits"
            );
        }
    }
    assert_eq!(
        render(&mut session, &text),
        clean(&text),
        "final state drifted"
    );
}
