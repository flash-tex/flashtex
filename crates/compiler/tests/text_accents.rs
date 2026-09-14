//! Kernel text accents that take a letter (`\c`, `\v`, `\u`, `\H`, `\r`,
//! `\k`, `\d`, `\b`): the character LaTeX's dfu tables declare for
//! `\<accent> <letter>`, laid out exactly like that character typed directly.

use flashtex_compiler::incremental::{compile_full, CompileOutput};
use flashtex_compiler::layout::LayoutConstraints;

fn compile(text: &str) -> CompileOutput {
    compile_full(text, LayoutConstraints::default())
}

/// Words (items joined up to the next item that follows source whitespace)
/// with the position of their first item: an accent command splits a word
/// into items, but must not move the word or anything after it.
fn words_in(o: &CompileOutput, source: &str) -> Vec<(String, i64, i64, i64)> {
    let r = |v: f64| (v * 100.0).round() as i64;
    let mut out: Vec<(String, i64, i64, i64)> = Vec::new();
    for i in o.pages.iter().flat_map(|p| p.items.iter()) {
        let start = i.span.start;
        let word_start = start == 0 || source[..start].ends_with(char::is_whitespace);
        match out.last_mut() {
            Some(last) if !word_start && last.2 == r(i.baseline_y_pt) => last.0.push_str(&i.text),
            _ => out.push((
                i.text.clone(),
                r(i.x_pt),
                r(i.baseline_y_pt),
                r(i.font_size_pt),
            )),
        }
    }
    out
}

/// Compile both sources and require identical words, positions and
/// diagnostics (glyph coverage warnings included).
fn same_as_typed(commands: &str, typed: &str) {
    let (a, b) = (compile(commands), compile(typed));
    assert_eq!(
        words_in(&a, commands),
        words_in(&b, typed),
        "{commands:?} vs {typed:?}"
    );
    assert_eq!(messages(&a), messages(&b), "{commands:?} vs {typed:?}");
}

fn messages(o: &CompileOutput) -> Vec<&str> {
    o.diagnostics.iter().map(|d| d.message.as_str()).collect()
}

#[test]
fn accent_commands_lay_out_like_the_precomposed_characters() {
    let commands = "Fa\\c{c}ade, Ko\\v{s}ice, \\v{C}esky, Ro\\u{a}ta, Erd\\H{o}s, \\r{U}sti, na \\d{h}, ta\\v{\\i} end.\n";
    let typed = "Façade, Košice, Česky, Roăta, Erdős, Ůsti, na ḥ, taǐ end.\n";
    same_as_typed(commands, typed);
    // Only glyph coverage of the v1 Times layout may warn, never the accents.
    let out = compile(commands);
    assert!(
        messages(&out)
            .iter()
            .all(|m| m.contains("has no glyph for")),
        "{:?}",
        messages(&out)
    );
}

#[test]
fn an_unbraced_argument_is_the_next_letter_only() {
    same_as_typed("Ko\\v sice and \\c C\\v s end.\n", "Košice and Çš end.\n");
    same_as_typed(
        "Ko\\v sice and \\c C\\v s end.\n",
        "Ko\\v{s}ice and \\c{C}\\v{s} end.\n",
    );
    assert!(compile("Ko\\v sice and \\c C\\v s end.\n")
        .diagnostics
        .is_empty());
}

#[test]
fn every_accent_composes_a_declared_letter() {
    for (source, expected) in [
        ("\\c{c}", "ç"),
        ("\\v{z}", "ž"),
        ("\\u{g}", "ğ"),
        ("\\H{u}", "ű"),
        ("\\r{a}", "å"),
        ("\\d{s}", "ṣ"),
        ("\\v{\\j}", "ǰ"),
        ("\\v{\\i}", "ǐ"),
        ("\\d{h}", "ḥ"),
    ] {
        // A word of its own: the v1 layout does not kern across inline
        // boundaries (`x\o{}y` loses its kern the same way), so neighbours
        // inside the word would measure that, not the accent.
        same_as_typed(&format!("x {source} end\n"), &format!("x {expected} end\n"));
    }
}

#[test]
fn ogonek_needs_t1_like_latex() {
    same_as_typed(
        "\\usepackage[T1]{fontenc}\nx \\k{a} end\n",
        "\\usepackage[T1]{fontenc}\nx ą end\n",
    );
    let ot1 = compile("x\\k{a}y end\n");
    assert_eq!(
        messages(&ot1),
        ["LaTeX Error: Command \\k unavailable in encoding OT1."]
    );
    assert_eq!(
        words_in(&ot1, "x\\k{a}y end\n")[0].0,
        "xay",
        "the letter is still typeset"
    );
}

#[test]
fn a_letter_without_a_declared_character_is_set_bare_with_a_warning() {
    let source = "\\b{a} and \\c{x} end.\n";
    let out = compile(source);
    assert_eq!(
        messages(&out),
        [
            "\\b{a} has no precomposed character and \\accent is not implemented; the accent is not drawn",
            "\\c{x} has no precomposed character and \\accent is not implemented; the accent is not drawn",
        ]
    );
    let typed = "a and x end.\n";
    assert_eq!(words_in(&out, source), words_in(&compile(typed), typed));
}

#[test]
fn a_group_that_is_not_one_letter_is_typeset_without_the_accent() {
    let source = "\\v{cz} end.\n";
    let out = compile(source);
    assert_eq!(
        messages(&out),
        ["the argument to \\v is not a single letter, \\i or \\j; the accent is not drawn"]
    );
    assert_eq!(words_in(&out, source)[0].0, "cz");
}

#[test]
fn a_bare_accent_warns_and_draws_nothing() {
    let source = "x \\v\n\ny end\n";
    let out = compile(source);
    assert_eq!(messages(&out), ["\\v has no letter to accent"]);
}

#[test]
fn a_document_may_redefine_an_accent_name() {
    let renew = compile("\\renewcommand{\\v}[1]{\\textbf{#1}}\nx \\v{s} end\n");
    assert!(renew.diagnostics.is_empty(), "{:?}", messages(&renew));
    // The user's macro, not the caron: `s`, never `š`.
    let texts: Vec<&str> = renew
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.text.as_str())
        .collect();
    assert_eq!(texts, ["x", "s", "end"]);
    let def = compile("\\def\\b{bee}\nx \\b\\ end\n");
    assert!(def.diagnostics.is_empty(), "{:?}", messages(&def));
}
