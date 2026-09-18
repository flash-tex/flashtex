//! `beamer.cls` page geometry: default 4:3 slide size plus the
//! `aspectratio` class option.
//!
//! Provenance (see also `CHECKIN.md` and the docs on
//! [`beamer_paper_size`](flashtex_class_geometry::beamer_paper_size)):
//! supervisor-measured real pdflatex output (`\the\<dimen>`, TeX Live 2026)
//! for the default, `aspectratio=32` and `aspectratio=141` documents: paper
//! sizes (128×96mm, 135×90mm, 148.5×105mm), 1cm side margins only, zero
//! vertical margins (`\topmargin` −1in, `\headheight`/`\headsep` 0pt,
//! `\textheight` = paper − 4pt `\footskip`), `\footskip`/`\marginparwidth`
//! 4pt, and the 11pt / 13.6pt body size. The `aspectratio` value set
//! {32, 43, 54, 141, 149, 169, 1610} is beamer's documented set; the
//! 1610/169/149/54 dimensions are quoted `beamer.cls` fragments.
//! Frame pagination (`\frame` starting a new page, `\pause`, overlays) is
//! NOT covered here — geometry only.

use flashtex_class_geometry::*;

fn resolved(class: &str, options: &str) -> ResolvedDocument {
    let setup = DocumentSetup::new(ClassKind::parse(class).unwrap(), options);
    resolve(&setup)
}

fn params(class: &str, options: &str) -> PageParams {
    resolved(class, options).params
}

fn mm(s: &str) -> Sp {
    Sp::parse(s).unwrap()
}

#[test]
fn beamer_parses_as_its_own_class() {
    assert_eq!(ClassKind::parse("beamer"), Some(ClassKind::Beamer));
    assert_eq!(ClassKind::Beamer.name(), "beamer");
    assert!(!ClassKind::Beamer.is_koma());
    assert!(!ClassKind::Beamer.has_chapters());
    assert!(ClassKind::Beamer.has_sections());
    assert!(DocumentSetup::from_preamble("\\documentclass{beamer}\n\\begin{document}").is_some());
    assert!(DocumentSetup::from_preamble(
        "\\documentclass[aspectratio=169]{beamer}\n\\begin{document}"
    )
    .is_some());
}

/// `\documentclass{beamer}`: 128mm × 96mm paper, 1cm side margins, zero
/// vertical margins (full-bleed slide), 11pt body with a 13.6pt baseline
/// skip. Pins every supervisor-measured default-column value.
#[test]
fn beamer_default_lengths() {
    let r = resolved("beamer", "");
    let p = r.params;
    // Paper: beamer's default `\mode<presentation>` size.
    assert_eq!(p.paperwidth, mm("128mm"));
    assert_eq!(p.paperheight, mm("96mm"));
    // Text block: 1cm in from the left/right edges, full height minus the
    // 4pt footskip reservation.
    assert_eq!(p.textwidth, mm("128mm") - mm("2cm"));
    assert_eq!(p.textheight, mm("96mm") - mm("4pt"));
    assert_eq!(p.oddsidemargin, mm("1cm") - mm("1in"));
    assert_eq!(p.evensidemargin, mm("1cm") - mm("1in"));
    // Vertical: exactly −1in, cancelling TeX's 1in origin (measured
    // −72.26999pt); no head boxes; the whole paper−text gap is footskip.
    assert_eq!(p.topmargin, Sp::ZERO - mm("1in"));
    assert_eq!(p.headheight, Sp::ZERO);
    assert_eq!(p.headsep, Sp::ZERO);
    assert_eq!(p.footskip, mm("4pt"));
    assert_eq!(p.marginparwidth, mm("4pt"));
    assert_eq!(p.parindent, Sp::ZERO);
    // 11pt body sizing (`size11.clo`'s `\normalsize`).
    assert_eq!(r.options.size, BaseSize::Pt11);
    assert_eq!(p.topskip, Sp::pt(11));
    assert_eq!(p.baselineskip, mm("13.6pt"));
    assert_eq!(p.maxdepth, Sp::pt(11).scaled(".5").unwrap());
    assert_eq!(r.font, body_font(BaseSize::Pt11));
    // Frame: text starts 1cm from the side edges and at the paper top
    // edge; first baseline one topskip below that; foot baseline at the
    // paper bottom edge; the PDF media is the slide, not Letter.
    assert_eq!(r.frame.pdf_page_width, mm("128mm"));
    assert_eq!(r.frame.pdf_page_height, mm("96mm"));
    assert_eq!(r.frame.text_left(1), mm("1cm"));
    assert_eq!(r.frame.text_top, Sp::ZERO);
    assert_eq!(r.frame.first_baseline, Sp::pt(11));
    assert_eq!(r.frame.foot_baseline, mm("96mm"));
    // Class shape: no chapters, plain page style, article-like depths.
    assert_eq!((r.secnumdepth, r.tocdepth), (3, 3));
    assert!(r.chapter.is_none());
    assert!(!r.headings.is_empty());
    assert_eq!(r.pagestyle, PageStyle::Plain);
    assert!(r.options.unused.is_empty());
}

/// Absolute scaled-point pins for the default geometry, so a regression in
/// the length arithmetic shows up even if the spec strings above stay put.
#[test]
fn beamer_default_absolute_sp() {
    let p = params("beamer", "");
    assert_eq!(p.paperwidth, Sp(23_867_901));
    assert_eq!(p.paperheight, Sp(17_900_926));
    assert_eq!(p.textwidth, Sp(20_138_542));
    assert_eq!(p.textheight, Sp(17_638_782));
    assert_eq!(p.oddsidemargin, Sp(-2_871_607));
    assert_eq!(p.topmargin, Sp(-4_736_286));
    assert_eq!(p.footskip, Sp(262_144));
    assert_eq!(p.marginparwidth, Sp(262_144));
    assert_eq!(p.baselineskip, Sp(891_290));
}

/// Every documented `aspectratio` value resolves to its slide size; the
/// 32 and 141 rows pin the supervisor's pdflatex-measured sizes (135×90mm
/// exact 3:2; 148.5×105mm ISO √2 — not the old 140×100 inference).
#[test]
fn beamer_aspectratio_table() {
    let cases = [
        ("32", "135mm", "90mm"),
        ("43", "128mm", "96mm"),
        ("54", "125mm", "100mm"),
        ("141", "148.5mm", "105mm"),
        ("149", "140mm", "90mm"),
        ("169", "160mm", "90mm"),
        ("1610", "160mm", "100mm"),
    ];
    for (aspect, w, h) in cases {
        let p = params("beamer", &format!("aspectratio={aspect}"));
        assert_eq!(p.paperwidth, mm(w), "aspectratio={aspect} width");
        assert_eq!(p.paperheight, mm(h), "aspectratio={aspect} height");
        // Horizontal margins stay 1cm; vertical margins stay zero: the
        // text block tracks the paper in both directions.
        assert_eq!(p.textwidth, mm(w) - mm("2cm"), "aspectratio={aspect}");
        assert_eq!(p.textheight, mm(h) - mm("4pt"), "aspectratio={aspect}");
        assert_eq!(p.oddsidemargin, mm("1cm") - mm("1in"));
        assert_eq!(p.evensidemargin, mm("1cm") - mm("1in"));
        assert_eq!(p.topmargin, Sp::ZERO - mm("1in"));
        assert_eq!(p.headheight, Sp::ZERO);
        assert_eq!(p.headsep, Sp::ZERO);
        assert_eq!(p.footskip, mm("4pt"));
        assert_eq!(p.marginparwidth, mm("4pt"));
        // The PDF media follows the slide size, and the font stays 11pt.
        let r = resolved("beamer", &format!("aspectratio={aspect}"));
        assert_eq!(r.frame.pdf_page_width, mm(w));
        assert_eq!(r.frame.pdf_page_height, mm(h));
        assert_eq!(r.frame.text_top, Sp::ZERO);
        assert_eq!(r.params.baselineskip, mm("13.6pt"));
        assert!(r.options.unused.is_empty(), "{:?}", r.options.unused);
    }
}

#[test]
fn beamer_aspectratio_spelling_and_fallbacks() {
    // Bare `aspectratio` means 43; explicit 43 is the same slide.
    assert_eq!(params("beamer", "aspectratio").paperwidth, mm("128mm"));
    assert_eq!(params("beamer", "aspectratio=43").paperheight, mm("96mm"));
    // Later options win, like keyval keys.
    let p = params("beamer", "aspectratio=43,aspectratio=169");
    assert_eq!((p.paperwidth, p.paperheight), (mm("160mm"), mm("90mm")));
    // Spaces around the key are tolerated.
    assert_eq!(
        params("beamer", "aspectratio = 169").paperwidth,
        mm("160mm")
    );
    // Unlisted numbers keep the default size without an "unused" warning:
    // beamer's own `\ifnum` chain simply matches nothing.
    let r = resolved("beamer", "aspectratio=999");
    assert_eq!(
        (r.params.paperwidth, r.params.paperheight),
        (mm("128mm"), mm("96mm"))
    );
    assert!(r.options.unused.is_empty());
}

/// beamer ignores the standard paper/size/side options (they warn like
/// pdflatex's `Unused global option(s)`) and accepts its own modes and
/// font sizes without geometry effects.
#[test]
fn beamer_option_handling() {
    let r = resolved("beamer", "a4paper,twoside,landscape");
    assert_eq!(r.options.unused, vec!["a4paper", "twoside", "landscape"]);
    // ... and they change nothing: the paper is still beamer's own.
    assert_eq!(
        (r.params.paperwidth, r.params.paperheight),
        (mm("128mm"), mm("96mm"))
    );

    // Modes, alignment keys and font sizes are accepted silently.
    for opts in [
        "handout",
        "trans",
        "presentation",
        "article",
        "book",
        "notes=show",
        "compress",
        "t",
        "10pt",
        "12pt",
        "14pt",
        "17pt",
        "20pt",
        "aspectratio=169,handout,t",
    ] {
        let r = resolved("beamer", opts);
        assert!(
            r.options.unused.is_empty(),
            "{opts}: {:?}",
            r.options.unused
        );
        // Only `aspectratio` moves the frame; the body stays 11pt.
        assert_eq!(r.options.size, BaseSize::Pt11, "{opts}");
        assert_eq!(r.params.baselineskip, mm("13.6pt"), "{opts}");
    }

    // `draft` still raises the overfull rule; `fleqn` still sets
    // `\mathindent`, exactly like the other classes.
    assert_eq!(params("beamer", "draft").overfullrule, Sp::pt(5));
    assert_eq!(params("beamer", "").overfullrule, Sp::ZERO);
    assert!(params("beamer", "fleqn").mathindent.is_some());
    assert!(params("beamer", "").mathindent.is_none());
}

/// A beamer document through the `geometry` package still resolves (the
/// class paper survives when geometry sets no paper of its own).
#[test]
fn beamer_with_geometry_package() {
    let mut setup = DocumentSetup::new(ClassKind::Beamer, "aspectratio=169");
    setup.geometry = Some(GeometryInput {
        package_options: String::new(),
        calls: Vec::new(),
    });
    let r = resolve(&setup);
    assert_eq!(
        (r.params.paperwidth, r.params.paperheight),
        (mm("160mm"), mm("90mm"))
    );
}
