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
fn capital_aliases_resolve_identically_to_their_canonical_accents() {
    // Each `\capital<name>` is the same accent as its lowercase-named
    // counterpart; over a capital base both must lay out exactly alike.
    for (alias, canonical, base, expected) in [
        ("capitalcaron", "v", "S", "Š"),
        ("capitalbreve", "u", "A", "Ă"),
        ("capitalring", "r", "A", "Å"),
        ("capitalogonek", "k", "A", "Ą"),
        ("capitalhungarumlaut", "H", "U", "Ű"),
        ("capitalcedilla", "c", "C", "Ç"),
    ] {
        // `\k` is T1-only, like its canonical; everything else is OT1-safe.
        let preamble = if canonical == "k" {
            "\\usepackage[T1]{fontenc}\n"
        } else {
            ""
        };
        same_as_typed(
            &format!("{preamble}x \\{alias}{{{base}}} end\n"),
            &format!("{preamble}x \\{canonical}{{{base}}} end\n"),
        );
        // And both are the precomposed character typed directly.
        same_as_typed(
            &format!("{preamble}x \\{alias}{{{base}}} end\n"),
            &format!("{preamble}x {expected} end\n"),
        );
    }
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
fn a_letter_without_a_declared_character_uses_a_combining_mark() {
    // No dfu table declares `\b a` or `\c x`: instead of the bare letter,
    // the base is followed by the accent's Unicode combining mark. The
    // command must lay out exactly like the combining mark typed directly,
    // proving the shaper handles the fallback generically; the only extra
    // diagnostics are the fallback warnings (the v1 Times face has no
    // combining-mark glyphs, so both versions share its missing-glyph
    // warnings, like any other character outside the face).
    let source = "\\b{a} and \\c{x} end.\n";
    let typed = "a\u{331} and x\u{327} end.\n";
    let (out, direct) = (compile(source), compile(typed));
    assert_eq!(words_in(&out, source), words_in(&direct, typed));
    let mut expected = vec![
        "\\b{a} has no precomposed character, so the accent is drawn with a combining mark",
        "\\c{x} has no precomposed character, so the accent is drawn with a combining mark",
    ];
    expected.extend(messages(&direct));
    assert_eq!(messages(&out), expected);
    let texts: Vec<&str> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.text.as_str())
        .collect();
    assert!(texts.iter().any(|t| *t == "a\u{331}"), "{texts:?}");
    assert!(texts.iter().any(|t| *t == "x\u{327}"), "{texts:?}");
}

#[test]
fn an_accent_over_a_digit_without_a_precomposed_form_uses_a_combining_mark() {
    // No dfu table declares a cedilla over a digit: the digit is followed
    // by U+0327 COMBINING CEDILLA rather than set bare with no accent, and
    // lays out exactly like the combining mark typed directly.
    let source = "x \\c{5} end\n";
    let typed = "x 5\u{327} end\n";
    let (out, direct) = (compile(source), compile(typed));
    assert_eq!(words_in(&out, source), words_in(&direct, typed));
    let mut expected =
        vec!["\\c{5} has no precomposed character, so the accent is drawn with a combining mark"];
    expected.extend(messages(&direct));
    assert_eq!(messages(&out), expected);
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
fn an_empty_base_with_no_spacing_mark_warns_and_draws_nothing() {
    // `\b{}` declares no spacing mark, so there is no base to attach the
    // combining mark to: the old bare warning stays and nothing is drawn.
    let source = "x \\b{} end\n";
    let out = compile(source);
    assert_eq!(
        messages(&out),
        [
            "\\b{} has no precomposed character and \\accent is not implemented; the accent is not drawn"
        ]
    );
    let texts: Vec<&str> = out
        .pages
        .iter()
        .flat_map(|p| p.items.iter())
        .map(|i| i.text.as_str())
        .collect();
    assert_eq!(texts, ["x", "end"]);
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
