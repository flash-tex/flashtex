//! GH-TEXT-PHANTOM (issue #494): `\phantom`, `\hphantom` and `\vphantom` in
//! text mode reserve the argument's geometry and paint nothing. Math mode
//! already implements all three (`math.rs` `Nucleus::Phantom`, covered by the
//! `amsmath_corpus` 39/40 fixtures); these tests cover the text-mode path:
//! no diagnostics in isolation and combined, and numeric geometry against a
//! sibling real-text render (positions must match what the same text would
//! occupy if actually typeset).

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};
use flashtex_compiler::layout::{text_width, word_space, Font, BODY_SIZE_PT};
use flashtex_compiler::parser::parse;

fn messages(source: &str) -> Vec<String> {
    parse(source)
        .diagnostics
        .into_iter()
        .map(|d| d.message)
        .collect()
}

fn blocks_debug(source: &str) -> String {
    format!("{:?}", parse(source).blocks)
}

fn compile(source: &str) -> CompileOutput {
    compile_full(source, LayoutConstraints::default())
}

/// Every laid-out word, in order.
fn page_texts(source: &str) -> Vec<String> {
    compile(source)
        .pages
        .iter()
        .flat_map(|page| page.items.iter().map(|item| item.text.clone()))
        .collect()
}

/// `(x_pt, baseline_y_pt)` of the first laid-out item whose text is `word`.
fn position_of(source: &str, word: &str) -> (f64, f64) {
    compile(source).pages[0]
        .items
        .iter()
        .find(|item| item.text == word)
        .map(|item| (item.x_pt, item.baseline_y_pt))
        .unwrap_or_else(|| panic!("{word:?} must be laid out in {source:?}"))
}

#[test]
fn phantom_in_text_mode_has_no_diagnostics() {
    for source in [
        "\\begin{document}A\\phantom{X}B\\end{document}",
        "\\begin{document}A\\hphantom{X}B\\end{document}",
        "\\begin{document}A\\vphantom{X}B\\end{document}",
        // Issue #494's shape: all three in one paragraph.
        "\\begin{document}Text \\phantom{X} more \\hphantom{Y} end \\vphantom{Z} done\\end{document}",
    ] {
        assert!(messages(source).is_empty(), "{source:?}: {:?}", messages(source));
    }
}

#[test]
fn phantom_nodes_carry_the_math_mode_flags() {
    let full = blocks_debug("\\begin{document}P\\phantom{X}Q\\end{document}");
    assert!(full.contains("horizontal: true, vertical: true"), "{full}");
    let horizontal = blocks_debug("\\begin{document}P\\hphantom{X}Q\\end{document}");
    assert!(
        horizontal.contains("horizontal: true, vertical: false"),
        "{horizontal}"
    );
    let vertical = blocks_debug("\\begin{document}P\\vphantom{X}Q\\end{document}");
    assert!(
        vertical.contains("horizontal: false, vertical: true"),
        "{vertical}"
    );
}

#[test]
fn phantom_reserves_the_full_width_and_paints_nothing() {
    let real = "\\begin{document}P WORD Q\\end{document}";
    let ghost = "\\begin{document}P \\phantom{WORD} Q\\end{document}";
    // The word after the phantom sits exactly where it would if WORD were
    // really typeset ...
    assert_eq!(position_of(ghost, "Q"), position_of(real, "Q"));
    // ... but WORD itself leaves no ink.
    assert_eq!(page_texts(ghost), ["P", "Q"]);
    assert_eq!(page_texts(real), ["P", "WORD", "Q"]);
}

#[test]
fn hphantom_reserves_width_but_no_height_or_depth() {
    let real = "\\begin{document}P WORD Q\\end{document}";
    let ghost = "\\begin{document}P \\hphantom{WORD} Q\\end{document}";
    assert_eq!(position_of(ghost, "Q"), position_of(real, "Q"));
    assert_eq!(page_texts(ghost), ["P", "Q"]);
    // Even oversized content reserves no vertical space: the next baseline
    // lands exactly where it would with no phantom at all.
    let plain = "\\begin{document}A\\hphantom{{\\large Xy}}b\\\\cd\\end{document}";
    let bare = "\\begin{document}Ab\\\\cd\\end{document}";
    assert!(messages(plain).is_empty(), "{:?}", messages(plain));
    assert_eq!(position_of(plain, "cd"), position_of(bare, "cd"));
}

#[test]
fn vphantom_reserves_height_and_depth_but_no_width() {
    // Zero width against two oracles: an empty `\hphantom{}` (a known-zero
    // box with identical glue handling) and the shaped width of `A` itself.
    let ghost = "\\begin{document}A \\vphantom{WORD} B\\end{document}";
    let empty = "\\begin{document}A \\hphantom{} B\\end{document}";
    assert_eq!(position_of(ghost, "B"), position_of(empty, "B"));
    let glued = "\\begin{document}A\\vphantom{WORD}B\\end{document}";
    let (ax, _) = position_of(glued, "A");
    let (bx, _) = position_of(glued, "B");
    assert!(
        (bx - ax - text_width("A", BODY_SIZE_PT, Font::TimesRoman)).abs() < 0.011,
        "B must start exactly one A-width after A: {ax} -> {bx}"
    );
    assert_eq!(page_texts(glued), ["A", "B"]);
    // Full vertical reservation: oversized content pushes the next baseline
    // exactly as far as really typesetting it would, while contributing no
    // width of its own (`ab` starts at the margin, as in the bare document).
    let real = "\\begin{document}{\\large Xy}ab\\\\cd\\end{document}";
    let bare = "\\begin{document}ab\\\\cd\\end{document}";
    let ghost = "\\begin{document}\\vphantom{{\\large Xy}}ab\\\\cd\\end{document}";
    assert!(messages(ghost).is_empty(), "{:?}", messages(ghost));
    // No width: `ab` starts exactly where the bare document starts it ...
    let (ghost_x, _) = position_of(ghost, "ab");
    let (bare_x, _) = position_of(bare, "ab");
    assert_eq!(ghost_x, bare_x);
    // ... but the full height: `cd` sits exactly where the tall render puts it.
    assert_eq!(position_of(ghost, "cd"), position_of(real, "cd"));
    assert_eq!(page_texts(ghost), ["ab", "cd"]);
}

#[test]
fn phantom_with_tall_content_matches_the_real_render() {
    let real = "\\begin{document}{\\large Xy}ab\\\\cd\\end{document}";
    let ghost = "\\begin{document}\\phantom{{\\large Xy}}ab\\\\cd\\end{document}";
    assert!(messages(ghost).is_empty(), "{:?}", messages(ghost));
    // Width (ab starts past the full large-Xy advance) and height/depth
    // (cd's baseline is pushed down) both match the real typesetting.
    assert_eq!(position_of(ghost, "ab"), position_of(real, "ab"));
    assert_eq!(position_of(ghost, "cd"), position_of(real, "cd"));
    assert_eq!(page_texts(ghost), ["ab", "cd"]);
}

#[test]
fn phantom_breaks_the_line_before_an_overwide_box() {
    // Review finding 2, verbatim fixture: the phantom is an unbreakable box,
    // so TeX breaks at the preceding space instead of leaving it after `A`.
    let ghost = "\\begin{document}A \\hphantom{\\rule{500pt}{1pt}} B\\end{document}";
    let real = "\\begin{document}A \\rule{500pt}{1pt} B\\end{document}";
    assert!(messages(ghost).is_empty(), "{:?}", messages(ghost));
    assert!(messages(real).is_empty(), "{:?}", messages(real));
    let (_, ay) = position_of(ghost, "A");
    let (_, by) = position_of(ghost, "B");
    assert!(by > ay, "B must wrap to the next line: A y={ay}, B y={by}");
    // Exactly where the really-typeset rule leaves it.
    assert_eq!(position_of(ghost, "B"), position_of(real, "B"));
    assert_eq!(page_texts(ghost), ["A", "B"]);
}

#[test]
fn phantom_reserves_trailing_explicit_glue() {
    // Review finding 3, verbatim fixture: `A\phantom{\quad}B` must leave a
    // full 1em gap, not butt `B` against `A`.
    let source = "\\begin{document}A\\phantom{\\quad}B\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    let (ax, _) = position_of(source, "A");
    let (bx, _) = position_of(source, "B");
    let gap = bx - ax - text_width("A", BODY_SIZE_PT, Font::TimesRoman);
    assert!(
        (gap - BODY_SIZE_PT).abs() < 0.02,
        "B must start 1em past A's end: A x={ax}, B x={bx}, gap={gap}"
    );
    assert_eq!(page_texts(source), ["A", "B"]);
}

#[test]
fn phantom_in_a_section_heading_reserves_width_and_paints_nothing() {
    // Review finding 4, verbatim shape: the flattened heading parser must
    // build the same `Inline::Phantom` as the main loop, not visibly typeset
    // the group contents with no width reserved.
    let ghost = "\\begin{document}\\section{A\\phantom{X}B}\\end{document}";
    assert!(messages(ghost).is_empty(), "{:?}", messages(ghost));
    let texts = page_texts(ghost);
    assert!(
        !texts.iter().any(|t| t.contains('X')),
        "X must stay invisible: {texts:?}"
    );
    // Same width reservation as really typesetting the heading text: with
    // spaces around it the word items line up exactly with the visible oracle.
    let spaced_ghost = "\\begin{document}\\section{A \\phantom{X} B}\\end{document}";
    let spaced_real = "\\begin{document}\\section{A X B}\\end{document}";
    assert!(messages(spaced_ghost).is_empty(), "{:?}", messages(spaced_ghost));
    assert_eq!(position_of(spaced_ghost, "A"), position_of(spaced_real, "A"));
    assert_eq!(position_of(spaced_ghost, "B"), position_of(spaced_real, "B"));
    let spaced_texts = page_texts(spaced_ghost);
    assert!(
        !spaced_texts.iter().any(|t| t.contains('X')),
        "X must stay invisible: {spaced_texts:?}"
    );
}

#[test]
fn phantom_in_a_figure_caption_reserves_width_and_paints_nothing() {
    // Review finding 4, second shape: captions share the flattened
    // `inlines_from_tokens` path, so `\caption{A\phantom{X}B}` must behave
    // the same way (captions are centred, so this pins the relative A/B
    // geometry and invisibility rather than absolute positions).
    let ghost = "\\begin{document}\\begin{figure}\\caption{A\\phantom{X}B}\\end{figure}\\end{document}";
    let spaced = "\\begin{document}\\begin{figure}\\caption{A \\phantom{X} B}\\end{figure}\\end{document}";
    assert!(messages(ghost).is_empty(), "{:?}", messages(ghost));
    assert!(messages(spaced).is_empty(), "{:?}", messages(spaced));
    for (label, source) in [("ghost", ghost), ("spaced", spaced)] {
        let texts = page_texts(source);
        assert!(
            !texts.iter().any(|t| t.contains('X')),
            "{label}: X must stay invisible: {texts:?}"
        );
    }
    // Unspaced B sits exactly where the spaced B sits minus the two
    // inter-word gaps: the phantom reserved precisely X's width.
    let (ax, _) = position_of(ghost, "A");
    let (bx, _) = position_of(ghost, "B");
    let (sax, _) = position_of(spaced, "A");
    let (sbx, _) = position_of(spaced, "B");
    let expected = (sbx - sax) - 2.0 * word_space(BODY_SIZE_PT, Font::TimesRoman);
    assert!(
        ((bx - ax) - expected).abs() < 0.02,
        "caption phantom must reserve X's width: gap={} expected={}",
        bx - ax,
        expected
    );
}

#[test]
fn phantom_around_underline_reserves_the_rule_depth() {
    // Review finding 5: the underline rule hangs below the content box, so
    // the following baseline must land where the real render puts it, not
    // where size-based text extents alone would put it.
    let real = "\\begin{document}\\underline{g}ab\\\\cd\\end{document}";
    let ghost = "\\begin{document}\\phantom{\\underline{g}}ab\\\\cd\\end{document}";
    let bare = "\\begin{document}ab\\\\cd\\end{document}";
    assert!(messages(ghost).is_empty(), "{:?}", messages(ghost));
    assert_eq!(page_texts(ghost), ["ab", "cd"]);
    assert_eq!(position_of(ghost, "cd"), position_of(real, "cd"));
    assert_eq!(position_of(ghost, "ab"), position_of(real, "ab"));
    // The rule genuinely deepens the line: past where no phantom sits.
    assert!(position_of(ghost, "cd").1 > position_of(bare, "cd").1);
}

#[test]
fn math_mode_phantoms_are_unaffected() {
    let source =
        "\\begin{document}$\\phantom{bbb} + \\hphantom{cc} + \\vphantom{d}$\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    let blocks = blocks_debug(source);
    assert!(blocks.contains("Phantom"), "{blocks}");
}
