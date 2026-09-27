//! KOMA-Script `scrartcl` / `scrreprt` / `scrbook` (TeX Live 2026):
//! `typearea`'s DIV computation.
//!
//! Oracle: live pdflatex, `\number\<length>` probes on
//! `\documentclass[<opts>]{<cls>}` with `\makeatletter` (so `\ta@div` and
//! `\ta@bcor` are readable), no packages. Every length is asserted to the
//! scaled point. Paper sizes are typearea's own (e.g. letter is 612bp by
//! 792bp, A4 is the ISO halving chain): they differ from plain `8.5in` /
//! `210mm` conversions by a few sp, so they are asserted exactly too.

use flashtex_class_geometry::*;

fn resolved(class: &str, options: &str) -> ResolvedDocument {
    let setup = DocumentSetup::new(ClassKind::parse(class).unwrap(), options);
    resolve(&setup)
}

fn params(class: &str, options: &str) -> PageParams {
    resolved(class, options).params
}

#[test]
fn koma_classes_parse() {
    assert_eq!(ClassKind::parse("scrartcl"), Some(ClassKind::Scrartcl));
    assert_eq!(ClassKind::parse("scrreprt"), Some(ClassKind::Scrreprt));
    assert_eq!(ClassKind::parse("scrbook"), Some(ClassKind::Scrbook));
    // `scrarticle.cls` is a thin wrapper that loads `scrartcl`.
    assert_eq!(ClassKind::parse("scrarticle"), Some(ClassKind::Scrartcl));
    assert!(ClassKind::Scrartcl.is_koma());
    assert!(ClassKind::Scrreprt.is_koma());
    assert!(ClassKind::Scrbook.is_koma());
    assert!(!ClassKind::Article.is_koma());
    // Unknown classes still fall through to the article fallback.
    assert_eq!(ClassKind::parse("memoir"), None);
    assert!(DocumentSetup::from_preamble("\\documentclass{scrartcl}\n\\begin{document}")
        .is_some());
    // `beamer` is a known class with its own geometry (see
    // `tests/beamer_geometry.rs`), not an unknown one.
    assert_eq!(ClassKind::parse("beamer"), Some(ClassKind::Beamer));
    assert!(DocumentSetup::from_preamble("\\documentclass{beamer}\n\\begin{document}").is_some());
}

/// `\documentclass{scrartcl}`: 11pt on A4, DIV 10. Every value is a live
/// `\number` probe (see module docs).
#[test]
fn scrartcl_default_lengths() {
    let r = resolved("scrartcl", "");
    let p = r.params;
    assert_eq!(p.paperwidth, Sp(39_158_280));
    assert_eq!(p.paperheight, Sp(55_380_996));
    // `typearea` sets the PDF media from the paper (the part-1 paper size).
    assert_eq!(r.frame.pdf_page_width, Sp(39_158_280));
    assert_eq!(r.frame.pdf_page_height, Sp(55_380_996));
    assert_eq!(p.textwidth, Sp(27_410_796));
    assert_eq!(p.textheight, Sp(39_046_366));
    assert_eq!(p.oddsidemargin, Sp(1_137_456));
    assert_eq!(p.evensidemargin, Sp(1_137_456));
    assert_eq!(p.topmargin, Sp(-1_649_234));
    assert_eq!(p.headheight, Sp(1_114_112));
    assert_eq!(p.headsep, Sp(1_336_935));
    assert_eq!(p.footskip, Sp(3_119_514));
    assert_eq!(p.topskip, Sp(720_896));
    assert_eq!(p.baselineskip, Sp(891_290));
    assert_eq!(p.parindent, Sp(717_621));
    assert_eq!(p.parskip, Glue::new("0pt", "1pt", "0pt"));
    assert_eq!(p.marginparwidth, Sp(3_915_828));
    assert_eq!(p.marginparsep, Sp(841_489));
    assert_eq!(p.marginparpush, Sp(401_077));
    assert_eq!(p.columnsep, Sp(655_360));
    assert_eq!(p.columnseprule, Sp::ZERO);
    assert_eq!(p.maxdepth, Sp(360_448));
    assert_eq!(p.footnotesep, Sp(504_627));
    assert_eq!(p.skip_footins, Glue::new("10pt", "4pt", "2pt"));
    assert_eq!(p.overfullrule, Sp::ZERO);
    assert_eq!(p.leftmargini, Sp(1_794_052));
    assert_eq!(p.labelsep, Sp(358_810));
    assert_eq!(p.columnwidth(false), Sp(27_410_796));
    // Class shape: article-like, no chapters, plain page style.
    assert_eq!(r.options.size, BaseSize::Pt11);
    assert!(!r.options.twoside);
    assert!(!r.options.titlepage);
    assert!(!r.options.openright);
    assert_eq!((r.secnumdepth, r.tocdepth), (3, 3));
    assert!(r.chapter.is_none());
    assert!(!r.headings.is_empty());
    assert_eq!(r.pagestyle, PageStyle::Plain);
    assert!(r.options.unused.is_empty());
}

/// 10pt → DIV 8, 12pt → DIV 12 (the A4 `\ta@divlist` table).
#[test]
fn scrartcl_sizes() {
    let p = params("scrartcl", "10pt");
    assert_eq!(p.textwidth, Sp(24_473_925));
    assert_eq!(p.textheight, Sp(35_258_368));
    assert_eq!(p.oddsidemargin, Sp(2_605_891));
    assert_eq!(p.evensidemargin, Sp(2_605_891));
    assert_eq!(p.topmargin, Sp(23_650));
    assert_eq!(p.headheight, Sp(983_040));
    assert_eq!(p.headsep, Sp(1_179_648));
    assert_eq!(p.footskip, Sp(2_752_512));
    assert_eq!(p.topskip, Sp(655_360));
    assert_eq!(p.baselineskip, Sp(786_432));
    assert_eq!(p.parindent, Sp(655_361));
    assert_eq!(p.marginparwidth, Sp(4_894_785));
    assert_eq!(p.marginparpush, Sp(353_892));
    assert_eq!(p.footnotesep, Sp(435_814));
    assert_eq!(p.skip_footins, Glue::new("9pt", "4pt", "2pt"));
    assert_eq!(p.leftmargini, Sp(1_638_402));
    assert_eq!(p.labelsep, Sp(327_680));
    assert_eq!(p.maxdepth, Sp(327_680));

    let p = params("scrartcl", "12pt");
    assert_eq!(p.textwidth, Sp(29_368_710));
    assert_eq!(p.textheight, Sp(41_648_128));
    assert_eq!(p.oddsidemargin, Sp(158_499));
    assert_eq!(p.evensidemargin, Sp(158_499));
    assert_eq!(p.topmargin, Sp(-2_734_451));
    assert_eq!(p.headheight, Sp(1_187_840));
    assert_eq!(p.headsep, Sp(1_425_408));
    assert_eq!(p.footskip, Sp(3_325_952));
    assert_eq!(p.topskip, Sp(786_432));
    assert_eq!(p.baselineskip, Sp(950_272));
    assert_eq!(p.parindent, Sp(770_040));
    assert_eq!(p.marginparwidth, Sp(3_263_190));
    assert_eq!(p.marginparpush, Sp(427_619));
    assert_eq!(p.footnotesep, Sp(550_502));
    assert_eq!(p.skip_footins, Glue::new("10.8pt", "4pt", "2pt"));
    assert_eq!(p.leftmargini, Sp(1_925_100));
    assert_eq!(p.labelsep, Sp(385_020));
    assert_eq!(p.maxdepth, Sp(393_216));

    // `fontsize=` spells the same sizes.
    assert_eq!(params("scrartcl", "fontsize=10pt").textwidth, Sp(24_473_925));
    assert_eq!(params("scrartcl", "fontsize=12").textwidth, Sp(29_368_710));
}

/// Explicit `DIV=12` (DIV 12 at 11pt) and `DIV=calc` (DIV 8 on A4).
#[test]
fn scrartcl_div_options() {
    let p = params("scrartcl", "DIV=12");
    assert_eq!(p.textwidth, Sp(29_368_710));
    assert_eq!(p.textheight, Sp(41_720_236));
    assert_eq!(p.oddsidemargin, Sp(158_499));
    assert_eq!(p.topmargin, Sp(-2_572_250));
    assert_eq!(p.marginparwidth, Sp(3_263_190));

    // `calc` on A4/11pt resolves to DIV 8 (unlike the default table's 10).
    let p = params("scrartcl", "DIV=calc");
    assert_eq!(p.textwidth, Sp(24_473_925));
    assert_eq!(p.textheight, Sp(35_481_206));
    assert_eq!(p.oddsidemargin, Sp(2_605_891));
    assert_eq!(p.topmargin, Sp(-264_709));
    assert_eq!(p.marginparwidth, Sp(4_894_785));

    // `classic` on A4/11pt resolves to DIV 10: identical to the default.
    assert_eq!(params("scrartcl", "DIV=classic"), params("scrartcl", ""));
    // Deprecated `DIV12` spells `DIV=12`.
    assert_eq!(params("scrartcl", "DIV12"), params("scrartcl", "DIV=12"));
    // Bare `DIV` takes the key default, `calc`.
    assert_eq!(params("scrartcl", "DIV"), params("scrartcl", "DIV=calc"));
    // Small DIVs are typearea's own sentinels: 0 is default, 1–2 calculate.
    assert_eq!(params("scrartcl", "DIV=0"), params("scrartcl", ""));
    assert_eq!(resolved("scrartcl", "DIV=0").options.div, DivSpec::Default);
    assert_eq!(resolved("scrartcl", "DIV=1").options.div, DivSpec::Calc);
    assert_eq!(
        params("scrartcl", "DIV=2").textwidth,
        params("scrartcl", "DIV=calc").textwidth
    );
}

/// `BCOR=12mm` shifts the block right; the height is untouched (BCOR never
/// enters `\ta@vblk`).
#[test]
fn scrartcl_bcor() {
    let r = resolved("scrartcl", "BCOR=12mm");
    assert_eq!(r.options.bcor, Sp(2_237_615));
    let p = r.params;
    assert_eq!(p.textwidth, Sp(25_844_467));
    assert_eq!(p.textheight, Sp(39_046_366));
    assert_eq!(p.oddsidemargin, Sp(3_039_428));
    assert_eq!(p.evensidemargin, Sp(3_039_428));
    assert_eq!(p.marginparwidth, Sp(3_692_066));
    // Deprecated bare-dimension form.
    assert_eq!(
        resolved("scrartcl", "BCOR12mm").options.bcor,
        Sp(2_237_615)
    );
}

/// `twoside` (and `scrbook` by default): inner/outer margins, marginpars at
/// 1.5 blocks.
#[test]
fn scrartcl_twoside() {
    let r = resolved("scrartcl", "twoside");
    assert!(r.options.twoside);
    assert!(r.flags.twoside);
    let p = r.params;
    assert_eq!(p.textwidth, Sp(27_410_796));
    assert_eq!(p.textheight, Sp(39_046_366));
    assert_eq!(p.oddsidemargin, Sp(-820_458));
    assert_eq!(p.evensidemargin, Sp(3_095_370));
    assert_eq!(p.marginparwidth, Sp(5_873_742));

    // `twoside=semi` keeps one-sided margins (`\@twosidefalse`).
    let r = resolved("scrartcl", "twoside=semi");
    assert!(!r.options.twoside);
    assert_eq!(r.params, params("scrartcl", ""));
}

/// `mpinclude` pulls the marginpar into the text block; combined with
/// `twoside` the outer margin grows by marginpar width plus separation.
#[test]
fn scrartcl_mpinclude() {
    let p = params("scrartcl", "mpinclude=true");
    assert_eq!(p.marginparwidth, Sp(3_074_339));
    assert_eq!(p.textwidth, Sp(23_494_968));
    assert_eq!(p.textheight, Sp(39_046_366));
    assert_eq!(p.oddsidemargin, Sp(1_137_456));

    let p = params("scrartcl", "twoside,mpinclude=true");
    assert_eq!(p.marginparwidth, Sp(3_074_339));
    assert_eq!(p.textwidth, Sp(23_494_968));
    assert_eq!(p.oddsidemargin, Sp(-820_458));
    assert_eq!(p.evensidemargin, Sp(7_011_198));
}

/// `headinclude` / `footinclude` shorten the text block; `headinclude`
/// also moves the top margin up by head height plus separation.
#[test]
fn scrartcl_head_foot_include() {
    let p = params("scrartcl", "headinclude=true");
    assert_eq!(p.textheight, Sp(36_372_496));
    assert_eq!(p.topmargin, Sp(801_813));
    assert_eq!(p.textwidth, Sp(27_410_796));

    let p = params("scrartcl", "footinclude=true");
    assert_eq!(p.textheight, Sp(36_372_496));
    assert_eq!(p.topmargin, Sp(-1_649_234));
}

/// Non-A4 papers take the `calc` path: letter resolves to DIV 7.
#[test]
fn scrartcl_letter() {
    let r = resolved("scrartcl", "letterpaper");
    let p = r.params;
    assert_eq!(p.paperwidth, Sp(40_258_437));
    assert_eq!(p.paperheight, Sp(52_099_153));
    assert_eq!(r.frame.pdf_page_width, Sp(40_258_437));
    assert_eq!(r.frame.pdf_page_height, Sp(52_099_153));
    assert_eq!(p.textwidth, Sp(23_004_822));
    assert_eq!(p.textheight, Sp(30_133_466));
    assert_eq!(p.oddsidemargin, Sp(3_890_521));
    assert_eq!(p.evensidemargin, Sp(3_890_521));
    assert_eq!(p.topmargin, Sp(255_403));
    assert_eq!(p.marginparwidth, Sp(5_751_205));
    // `paper=letter` spells `letterpaper`.
    assert_eq!(params("scrartcl", "paper=letter"), p);

    // calc at 10pt resolves to DIV 6, at 12pt to DIV 8.
    let p = params("scrartcl", "letterpaper,10pt");
    assert_eq!(p.textwidth, Sp(20_129_220));
    assert_eq!(p.textheight, Sp(26_607_616));
    assert_eq!(p.oddsidemargin, Sp(5_328_322));
    assert_eq!(p.topmargin, Sp(1_784_218));
    assert_eq!(p.marginparwidth, Sp(6_709_739));
    let p = params("scrartcl", "letterpaper,12pt");
    assert_eq!(p.textwidth, Sp(25_161_525));
    assert_eq!(p.textheight, Sp(33_095_680));
    assert_eq!(p.oddsidemargin, Sp(2_812_170));
    assert_eq!(p.topmargin, Sp(-837_140));
    assert_eq!(p.marginparwidth, Sp(5_032_304));

    // BCOR composes with calc (DIV stays 7).
    let p = params("scrartcl", "letterpaper,BCOR=10mm");
    assert_eq!(p.textwidth, Sp(21_939_292));
    assert_eq!(p.textheight, Sp(30_133_466));
    assert_eq!(p.oddsidemargin, Sp(5_355_626));
    assert_eq!(p.marginparwidth, Sp(5_484_822));
}

/// A5 resolves through calc plus the too-short-page recompute (DIV 14);
/// B5, legal and executive exercise the plain calc path.
#[test]
fn scrartcl_other_papers() {
    let p = params("scrartcl", "a5paper");
    assert_eq!(p.paperwidth, Sp(27_597_264));
    assert_eq!(p.paperheight, Sp(39_158_280));
    assert_eq!(p.textwidth, Sp(21_683_565));
    assert_eq!(p.textheight, Sp(31_024_756));
    assert_eq!(p.oddsidemargin, Sp(-1_779_437));
    assert_eq!(p.topmargin, Sp(-4_390_313));
    assert_eq!(p.marginparwidth, Sp(1_971_233));

    let p = params("scrartcl", "b5paper");
    assert_eq!(p.paperwidth, Sp(32_818_368));
    assert_eq!(p.paperheight, Sp(46_617_000));
    assert_eq!(p.textwidth, Sp(24_613_776));
    assert_eq!(p.textheight, Sp(35_481_206));
    assert_eq!(p.oddsidemargin, Sp(-633_990));
    assert_eq!(p.topmargin, Sp(-3_302_583));

    let p = params("scrartcl", "legalpaper");
    assert_eq!(p.paperwidth, Sp(40_258_437));
    assert_eq!(p.paperheight, Sp(66_308_014));
    assert_eq!(p.textwidth, Sp(23_004_822));
    assert_eq!(p.textheight, Sp(38_155_076));
    assert_eq!(p.topmargin, Sp(2_285_240));

    let p = params("scrartcl", "executivepaper");
    assert_eq!(p.paperwidth, Sp(34_338_078));
    assert_eq!(p.paperheight, Sp(49_731_010));
    assert_eq!(p.textwidth, Sp(24_036_657));
    assert_eq!(p.textheight, Sp(35_481_206));
    assert_eq!(p.oddsidemargin, Sp(414_424));
    assert_eq!(p.topmargin, Sp(-2_214_232));
    assert_eq!(p.marginparwidth, Sp(3_433_807));

    // Landscape A5 swaps the ISO sizes and calculates (DIV 8).
    let p = params("scrartcl", "a5paper,landscape");
    assert_eq!(p.paperwidth, Sp(39_158_280));
    assert_eq!(p.paperheight, Sp(27_597_264));
    assert_eq!(p.textwidth, Sp(24_473_925));
    assert_eq!(p.textheight, Sp(17_655_406));
    assert_eq!(p.topmargin, Sp(-3_737_675));
}

/// `calc` with `twocolumn` doubles the good width (plus `\columnsep`), so
/// the block goes negative, clamps to 5mm, and the too-short-page recompute
/// re-derives DIV 19 from the height.
#[test]
fn scrartcl_calc_twocolumn() {
    let p = params("scrartcl", "DIV=calc,twocolumn");
    assert_eq!(p.textwidth, Sp(32_975_394));
    assert_eq!(p.textheight, Sp(47_067_976));
    assert_eq!(p.oddsidemargin, Sp(-1_644_843));
    assert_eq!(p.topmargin, Sp(-4_272_544));
    assert_eq!(p.marginparwidth, Sp(2_060_962));
}

/// `calc` with `mpinclude` narrows the probe block (DIV 10 here) and then
/// pulls the marginpar into the text block.
#[test]
fn scrartcl_calc_mpinclude() {
    let p = params("scrartcl", "DIV=calc,mpinclude=true");
    assert_eq!(p.textwidth, Sp(23_494_968));
    assert_eq!(p.textheight, Sp(39_046_366));
    assert_eq!(p.marginparwidth, Sp(3_074_339));
}

/// `classic` falls back to `calc` when the ISO block is taller than the
/// page (landscape A5): identical to the landscape numbers above.
#[test]
fn scrartcl_classic_fallback() {
    assert_eq!(
        params("scrartcl", "a5paper,landscape,DIV=classic"),
        params("scrartcl", "a5paper,landscape")
    );
}

/// Small and large explicit DIVs.
#[test]
fn scrartcl_div_extremes() {
    let p = params("scrartcl", "DIV=4");
    assert_eq!(p.textwidth, Sp(9_789_570));
    assert_eq!(p.textheight, Sp(14_090_246));
    assert_eq!(p.oddsidemargin, Sp(9_948_069));
    assert_eq!(p.topmargin, Sp(6_657_916));
    assert_eq!(p.marginparwidth, Sp(9_789_570));

    let p = params("scrartcl", "DIV=15");
    assert_eq!(p.textwidth, Sp(31_326_624));
    assert_eq!(p.textheight, Sp(44_394_106));
    assert_eq!(p.oddsidemargin, Sp(-820_458));
    assert_eq!(p.topmargin, Sp(-3_495_267));
    assert_eq!(p.marginparwidth, Sp(2_610_552));
}

/// `pagesize=false` leaves the engine-default (letter) media while the
/// paper stays A4.
#[test]
fn scrartcl_pagesize_false() {
    let setup = DocumentSetup::new(ClassKind::Scrartcl, "pagesize=false");
    let r = resolve(&setup);
    assert!(!r.options.pagesize_pdf);
    assert_eq!(r.params.paperwidth, Sp(39_158_280));
    assert_eq!(r.frame.pdf_page_width, Sp::parse("8.5in").unwrap());
    assert_eq!(r.frame.pdf_page_height, Sp::parse("11in").unwrap());
    assert_eq!(r.params.textheight, Sp(39_046_366));
}

/// `scrbook` is twoside with a title page and `openright`; `scrreprt` is
/// oneside with a title page. Both share scrartcl's A4/DIV-10 geometry
/// (oneside/twoside margins aside).
#[test]
fn scrbook_scrreprt_defaults() {
    let b = resolved("scrbook", "");
    assert!(b.options.twoside);
    assert!(b.options.titlepage);
    assert!(b.options.openright);
    assert_eq!((b.secnumdepth, b.tocdepth), (2, 2));
    assert!(b.chapter.is_some());
    assert_eq!(b.pagestyle, PageStyle::Plain);
    assert_eq!(b.frame.pdf_page_width, Sp(39_158_280));
    assert_eq!(b.params.textwidth, Sp(27_410_796));
    assert_eq!(b.params.textheight, Sp(39_046_366));
    assert_eq!(b.params.oddsidemargin, Sp(-820_458));
    assert_eq!(b.params.evensidemargin, Sp(3_095_370));

    let r = resolved("scrreprt", "");
    assert!(!r.options.twoside);
    assert!(r.options.titlepage);
    assert!(!r.options.openright);
    assert_eq!((r.secnumdepth, r.tocdepth), (2, 2));
    assert!(r.chapter.is_some());
    assert_eq!(r.params, params("scrartcl", ""));

    // `openany` clears `openright` without touching the frame.
    let o = resolved("scrbook", "openany");
    assert!(!o.options.openright);
    assert_eq!(o.params.oddsidemargin, Sp(-820_458));
    assert!(o.chapter.is_some());
}

/// KOMA processes options in given order: later wins.
#[test]
fn koma_options_given_order() {
    let o = resolved("scrartcl", "letterpaper,a4paper").options;
    assert_eq!(o.paper, Paper::A4);
    let o = resolved("scrartcl", "a4paper,letterpaper").options;
    assert_eq!(o.paper, Paper::Letter);
    let o = resolved("scrartcl", "twoside,oneside").options;
    assert!(!o.twoside);
    let o = resolved("scrbook", "twoside,oneside").options;
    assert!(!o.twoside);
}

/// Declared-but-unmodelled keys stay silent; unknown keys warn; unknown
/// font sizes keep the 11pt default and warn.
#[test]
fn koma_unused_and_ignored() {
    let o = resolved("scrartcl", "headings=small,version=3.12,toc=flat").options;
    assert!(o.unused.is_empty(), "ignored keys warn: {:?}", o.unused);
    let o = resolved("scrartcl", "frobnicate,paper=A3").options;
    assert_eq!(o.unused, vec!["frobnicate".to_string(), "paper=A3".to_string()]);
    let o = resolved("scrartcl", "fontsize=14pt").options;
    assert_eq!(o.unused, vec!["fontsize=14pt".to_string()]);
    assert_eq!(o.size, BaseSize::Pt11);
    // `parskip=false` is the default, so it stays silent (see
    // `koma_parskip` for the modelled values).
    let o = resolved("scrartcl", "parskip=false").options;
    assert!(o.unused.is_empty());
}

/// KOMA `parskip` (scrartcl.cls `scrkernel-paragraphs.dtx`): every `half…`
/// choice zeroes `\parindent` with `\parskip` half the body baselineskip
/// (rigid halves: natural = stretch), every `full…` choice zeroes
/// `\parindent` with `\parskip` one baselineskip plus a tenth.
///
/// Oracle: live pdflatex `\number` probes, TeX Live 2026, 11pt default
/// (`\baselineskip` 891290sp): `half` → 445645/445645, `full` → 891290/89134.
#[test]
fn koma_parskip() {
    let half = Glue {
        natural: Sp(445_645),
        stretch: Sp(445_645),
        shrink: Sp::ZERO,
    };
    let full = Glue {
        natural: Sp(891_290),
        stretch: Sp(89_134),
        shrink: Sp::ZERO,
    };
    for opts in ["parskip=half", "parskip=half-", "parskip=half+", "parskip=half*"] {
        let r = resolved("scrartcl", opts);
        assert_eq!(r.params.parindent, Sp::ZERO, "{opts}");
        assert_eq!(r.params.parskip, half, "{opts}");
        assert!(r.options.unused.is_empty(), "{opts}: {:?}", r.options.unused);
    }
    for opts in ["parskip=full", "parskip=full-", "parskip=full+", "parskip=full*", "parskip", "parskip=true"] {
        let r = resolved("scrartcl", opts);
        assert_eq!(r.params.parindent, Sp::ZERO, "{opts}");
        assert_eq!(r.params.parskip, full, "{opts}");
        assert!(r.options.unused.is_empty(), "{opts}: {:?}", r.options.unused);
    }
    // `false` (the default) and `never` keep the 1em indent.
    for opts in ["", "parskip=false", "parskip=never", "parskip=relative", "parskip=half,parskip=relative"] {
        let r = resolved("scrartcl", opts);
        assert!(r.options.unused.is_empty(), "{opts}: {:?}", r.options.unused);
    }
    assert_eq!(params("scrartcl", "").parindent, Sp(717_621));
    assert_eq!(params("scrartcl", "").parskip, Glue::new("0pt", "1pt", "0pt"));
    assert_eq!(params("scrartcl", "parskip=false").parindent, Sp(717_621));
    assert_eq!(params("scrartcl", "parskip=false").parskip, Glue::new("0pt", "1pt", "0pt"));
    assert_eq!(params("scrartcl", "parskip=never").parindent, Sp(717_621));
    assert_eq!(params("scrartcl", "parskip=never").parskip, Glue::fixed(Sp::ZERO));
    // `\parskip` tracks the body baselineskip whatever the size: 12pt half
    // is 475136sp natural and stretch (`\number` probe, TeX Live 2026).
    let p = params("scrartcl", "12pt,parskip=half");
    assert_eq!(p.parindent, Sp::ZERO);
    assert_eq!(
        p.parskip,
        Glue {
            natural: Sp(475_136),
            stretch: Sp(475_136),
            shrink: Sp::ZERO,
        }
    );
    // Unknown values still warn like pdflatex's `Unused global option(s)`.
    let o = resolved("scrartcl", "parskip=bogus").options;
    assert_eq!(o.unused, vec!["parskip=bogus".to_string()]);
    // Plain `article` does not declare `parskip`: it warns and keeps the
    // standard indent with `0pt plus 1pt`, like pdflatex.
    let r = resolved("article", "11pt,parskip=half");
    assert_eq!(r.options.unused, vec!["parskip=half".to_string()]);
    assert_eq!(r.params.parindent, Sp::pt(17));
    assert_eq!(r.params.parskip, Glue::new("0pt", "1pt", "0pt"));
}

/// `twocolumn` halves the column but (unlike the standard classes) never
/// widens the text block; the list indent drops to 2em.
#[test]
fn koma_twocolumn() {
    let r = resolved("scrartcl", "twocolumn");
    assert!(r.options.twocolumn);
    assert_eq!(r.params.textwidth, Sp(27_410_796));
    assert_eq!(r.params.leftmargini, Sp(1_435_242));
    assert_eq!(r.params.columnwidth(true), Sp(13_377_718));
}

/// `draft` shows the overfull rule, like the standard classes.
#[test]
fn koma_draft() {
    assert_eq!(params("scrartcl", "draft").overfullrule, Sp(327_680));
    assert_eq!(params("scrartcl", "").overfullrule, Sp::ZERO);
}

/// Regression controls: `[a4paper]{article}` keeps the engine-default
/// letter media (article sets `\paperwidth` but not the PDF page size),
/// and plain `article` geometry is untouched.
#[test]
fn article_controls_unchanged() {
    let setup = DocumentSetup::new(ClassKind::Article, "a4paper");
    let r = resolve(&setup);
    assert_eq!(r.params.paperwidth, Sp::parse("210mm").unwrap());
    assert_ne!(r.params.paperwidth, Sp(39_158_280));
    assert_eq!(r.frame.pdf_page_width, Sp::parse("8.5in").unwrap());
    assert_eq!(r.frame.pdf_page_height, Sp::parse("11in").unwrap());

    let a = params("article", "");
    assert_eq!(a.textheight, Sp(36_044_800));
    assert_eq!(a.textwidth, Sp::pt(345));
}
