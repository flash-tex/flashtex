//! The two-column *starting state* must be the resolved one, not the bare
//! `\documentclass` option.
//!
//! `\usepackage[twocolumn]{geometry}` sets `\if@twocolumn` exactly as the
//! class option would (`flashtex_class_geometry::resolve`'s `apply_geometry`
//! folds it into `ResolvedDocument::flags`), so a document that asks for two
//! columns entirely through `geometry` — `\documentclass{article}
//! \usepackage[twocolumn]{geometry}`, with no `twocolumn` class option at
//! all — is two-column from the first page. `adapt_cached` seeded the
//! `columns` scan from `resolved.options.twocolumn` (`\documentclass`'s own
//! option, unset here) instead of `resolved.flags.twocolumn` (the state
//! `resolve` already resolved), so it started in one column regardless of
//! what `geometry` said.

use flashtex_compiler::parser::{parse, SourceDocument};
use flashtex_render_pipeline::adapter::{adapt, Labels};
use flashtex_render_pipeline::RenderOptions;

fn columns(src: &str) -> usize {
    let docs = [SourceDocument { path: "main.tex", text: src }];
    let parsed = parse(src);
    let doc = adapt(&[src], 0, &parsed, &RenderOptions::default(), &Labels::default());
    let _ = docs;
    doc.style
        .class_geometry
        .as_deref()
        .map_or(1, |g| g.frame.columns.len())
}

#[test]
fn geometrys_own_twocolumn_option_starts_two_column() {
    let src = "\\documentclass{article}\n\\usepackage[twocolumn]{geometry}\n\\begin{document}\nBody.\n\\end{document}\n";
    assert_eq!(columns(src), 2, "usepackage[twocolumn]{{geometry}} must start the document two-column");
}

/// Control: the plain class option still works (this direction was never
/// broken).
#[test]
fn the_class_option_still_starts_two_column() {
    let src = "\\documentclass[twocolumn]{article}\n\\begin{document}\nBody.\n\\end{document}\n";
    assert_eq!(columns(src), 2);
}

/// Control: neither the class option nor `geometry` asks for two columns.
#[test]
fn plain_article_stays_one_column() {
    let src = "\\documentclass{article}\n\\begin{document}\nBody.\n\\end{document}\n";
    assert_eq!(columns(src), 1);
}

/// `geometry` loaded, but without the `twocolumn` key: no effect.
#[test]
fn geometry_without_the_key_stays_one_column() {
    let src = "\\documentclass{article}\n\\usepackage[margin=1in]{geometry}\n\\begin{document}\nBody.\n\\end{document}\n";
    assert_eq!(columns(src), 1);
}
