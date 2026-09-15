//! GH-TEXT-PHANTOM (issue #494): `\phantom`, `\hphantom` and `\vphantom` in
//! text mode reserve the argument's geometry and paint nothing. Math mode
//! already implements all three (`math.rs` `Nucleus::Phantom`, covered by the
//! `amsmath_corpus` 39/40 fixtures); these tests cover the text-mode path:
//! no diagnostics in isolation and combined, and numeric geometry against a
//! sibling real-text render (positions must match what the same text would
//! occupy if actually typeset).

use flashtex_compiler::incremental::{compile_full, CompileOutput, LayoutConstraints};
use flashtex_compiler::layout::{text_width, Font, BODY_SIZE_PT};
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
fn math_mode_phantoms_are_unaffected() {
    let source =
        "\\begin{document}$\\phantom{bbb} + \\hphantom{cc} + \\vphantom{d}$\\end{document}";
    assert!(messages(source).is_empty(), "{:?}", messages(source));
    let blocks = blocks_debug(source);
    assert!(blocks.contains("Phantom"), "{blocks}");
}
