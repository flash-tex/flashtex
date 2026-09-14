//! Click-to-source inside formulas (feature `math-glyph-spans`): every math
//! glyph cluster and rule maps to the source bytes of the atom that produced
//! it, not to the whole formula.

#![cfg(feature = "math-glyph-spans")]

mod common;

use common::*;
use flashtex_render_pipeline::display::{Item, RunRole};

/// (cluster text, source bytes) of every math glyph, and the source bytes of
/// every rule, in display-list order.
fn math_map(doc: &str) -> (Vec<(String, String)>, Vec<String>) {
    let r = render_one(doc);
    let mut glyphs = Vec::new();
    let mut rules = Vec::new();
    for page in &r.v2.pages {
        for item in &page.to_items() {
            match item {
                Item::GlyphRun(run) if run.role == RunRole::Math => {
                    for c in &run.clusters {
                        let [s] = c.provenance.sources() else {
                            panic!("one source per math cluster");
                        };
                        glyphs.push((run.text[c.text_start_byte..c.text_end_byte].to_string(), doc[s.start_byte..s.end_byte].to_string()));
                    }
                }
                Item::Rule(rule) => {
                    let [s] = rule.provenance.sources() else {
                        panic!("one source per rule");
                    };
                    rules.push(doc[s.start_byte..s.end_byte].to_string());
                }
                _ => {}
            }
        }
    }
    (glyphs, rules)
}

/// The distinct sources of every cluster whose text is `text`.
fn sources<'a>(glyphs: &'a [(String, String)], text: &str) -> Vec<&'a str> {
    let mut out: Vec<&str> = glyphs.iter().filter(|(t, _)| t == text).map(|(_, s)| s.as_str()).collect();
    out.dedup();
    out
}

#[test]
fn scripts_fractions_radicals_accents_and_text_map_to_their_atoms() {
    if !lm_available() {
        return;
    }
    let doc = "\\begin{document}\nInline $x_i^2 + \\frac{a}{b} + \\sqrt{z} + \\hat{w} + \\text{if } q$ here.\n\\end{document}\n";
    let (g, rules) = math_map(doc);
    for letter in ["x", "2", "a", "b", "z", "w", "q"] {
        assert_eq!(sources(&g, letter), [letter], "{letter}: {g:?}");
    }
    // The subscript i and the i of `\text{if }` map to different bytes.
    assert_eq!(sources(&g, "i"), ["i", "\\text{if }"]);
    assert_eq!(sources(&g, "+"), ["+"]);
    assert_eq!(sources(&g, "\u{221A}"), ["\\sqrt"]);
    assert_eq!(sources(&g, "\u{02C6}"), ["\\hat"]);
    // `\text{if }`: the run's glyphs map to the `\text` command.
    assert_eq!(sources(&g, "f"), ["\\text{if }"]);
    // The fraction bar and the vinculum map to their commands.
    assert_eq!(rules, ["\\frac", "\\sqrt"]);
}

#[test]
fn left_right_delimiters_map_to_their_own_commands_at_every_size() {
    if !lm_available() {
        return;
    }
    // A display tall enough for extensible delimiters, and an inline pair.
    let doc = "\\begin{document}\n\\[ \\left( \\frac{\\frac{a}{b}}{\\frac{c}{\\frac{d}{e}}} \\right] \\]\nand $\\left\\{ y \\right.$\n\\end{document}\n";
    let (g, rules) = math_map(doc);
    let opens: Vec<&(String, String)> = g.iter().filter(|(t, _)| t == "(" || t == "{").collect();
    let closes: Vec<&(String, String)> = g.iter().filter(|(t, _)| t == "]").collect();
    assert_eq!(opens.len(), 2, "{g:?}");
    assert_eq!(closes.len(), 1, "{g:?}");
    assert!(opens.iter().all(|(t, s)| s == &format!("\\left{}", if t == "{" { "\\{" } else { "(" })), "{opens:?}");
    assert_eq!(closes[0].1, "\\right]");
    // Bodies keep their own atoms; nothing maps to a whole formula.
    for letter in ["a", "b", "c", "d", "e", "y"] {
        assert_eq!(sources(&g, letter), [letter]);
    }
    // Before this feature every cluster mapped to the whole formula, which
    // spans a `\frac{`; now no glyph does.
    assert!(g.iter().all(|(_, s)| !s.contains("\\frac{") && s.len() <= "\\right]".len()), "{g:?}");
    assert!(rules.iter().all(|r| r == "\\frac"), "{rules:?}");
}

#[test]
fn big_operators_limits_and_grids() {
    if !lm_available() {
        return;
    }
    let doc = "\\begin{document}\n\\[ \\sum_{k=0}^{n} k \\]\n\\[ f = \\begin{cases} u & v \\\\ s & t \\end{cases} \\]\n\\end{document}\n";
    let (g, _) = math_map(doc);
    assert_eq!(sources(&g, "\u{2211}"), ["\\sum"]);
    for letter in ["k", "0", "n", "f", "u", "v", "s", "t"] {
        assert_eq!(sources(&g, letter), [letter], "{letter}: {g:?}");
    }
    // The cases brace was built by the grid, so it maps to the environment.
    let brace = sources(&g, "{");
    assert_eq!(brace.len(), 1, "{g:?}");
    assert!(brace[0].starts_with("\\begin"), "{brace:?}");
}
