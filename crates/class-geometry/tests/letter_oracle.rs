//! `letter.cls` page geometry against pdflatex, pinned.
//!
//! pdflatex is an ORACLE ONLY: it never runs here. Every number below was
//! read back out of pdflatex on the lane machine and is committed as a
//! literal, exactly like `tests/data/oracle.txt` — but in this file rather
//! than in that one, so that adding a `letter` case never requires
//! regenerating the shared oracle data.
//!
//! Method, per fixture: `\the<length>` for every length after the class,
//! `geometry` and `\begin{document}` have run; `\pdfsavepos` plus a shipout
//! `\write16` for the first baseline of pages 1 and 2, in scaled points from
//! the page's bottom-left corner. The same probe reproduces
//! `tests/data/oracle.txt`'s `article-10pt` `b1` position exactly (8865054sp
//! from the top of the media), so the method is the committed one.
//!
//! Two facts about the generating machine are captured rather than assumed:
//!
//! * its `pdftexconfig.tex` defaults to **A4**, not the US Letter that
//!   [`letter_media`] documents for MacTeX 2026. Without `geometry` the
//!   MediaBox is therefore A4 even for `letterpaper` — which is exactly the
//!   engine-default behaviour [`PageFrame`] already models — so each fixture
//!   pins the media it was measured with and the test feeds it back in.
//!   Nothing here depends on which paper the engine happens to default to.
//! * `letter.cls` line 405 defines
//!   `\@texttop{\ifnum\c@page=1\vskip \z@ plus.00006fil\relax\fi}`: on
//!   **page 1 only**, a fil glue at the top of the text block takes a share
//!   of the page's leftover space and pushes the first baseline down (2160sp
//!   at 10pt in this probe). That is page building, not a frame length, so
//!   the frame is checked on page 2, where `\@texttop` expands to nothing
//!   and the model must be exact to the scaled point; page 1 is checked only
//!   for direction and bound. `letter.cls` line 404 is a plain
//!   `\raggedbottom`, with no `\if@twoside` guard — unlike article's.

use flashtex_class_geometry::*;

struct Fixture {
    id: &'static str,
    options: &'static str,
    geometry: Option<&'static str>,
    dims: &'static [(&'static str, &'static str)],
    /// (`\pdfpagewidth`, `\pdfpageheight`) in sp, as the engine had them.
    media_sp: (i64, i64),
    /// (x from the left edge, y from the bottom edge) in sp.
    page1_first_baseline_sp: (i64, i64),
    page2_first_baseline_sp: (i64, i64),
}

/// Measured with pdfTeX 3.141592653-2.6-1.40.27 (TeX Live 2025/nixos.org).
const FIXTURES: &[Fixture] = &[
    Fixture {
        id: "letter-10pt",
        options: "10pt",
        geometry: None,
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "794.96999pt"),
            ("textwidth", "345.0pt"),
            ("textheight", "550.0pt"),
            ("oddsidemargin", "62.3775pt"),
            ("evensidemargin", "62.3775pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "10.0pt"),
            ("baselineskip", "12.0pt"),
            ("parindent", "0.0pt"),
            ("parskip", "6.99997pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.0pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "25.00003pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "172.5pt"),
            ("indentedwidth", "172.5pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (8824258, 44482160),
        page2_first_baseline_sp: (8824258, 44484320),
    },
    Fixture {
        id: "letter-11pt",
        options: "11pt",
        geometry: None,
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "794.96999pt"),
            ("textwidth", "360.0pt"),
            ("textheight", "541.40024pt"),
            ("oddsidemargin", "54.8775pt"),
            ("evensidemargin", "54.8775pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "11.0pt"),
            ("baselineskip", "13.6pt"),
            ("parindent", "0.0pt"),
            ("parskip", "7.66498pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.5pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "27.37506pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "180.0pt"),
            ("indentedwidth", "180.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (8332738, 44416663),
        page2_first_baseline_sp: (8332738, 44418784),
    },
    Fixture {
        id: "letter-12pt",
        options: "12pt",
        geometry: None,
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "794.96999pt"),
            ("textwidth", "390.0pt"),
            ("textheight", "548.5pt"),
            ("oddsidemargin", "39.8775pt"),
            ("evensidemargin", "39.8775pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "12.0pt"),
            ("baselineskip", "14.5pt"),
            ("parindent", "0.0pt"),
            ("parskip", "8.22487pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "6.0pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "29.3747pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "195.0pt"),
            ("indentedwidth", "195.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (7349698, 44351102),
        page2_first_baseline_sp: (7349698, 44353248),
    },
    Fixture {
        id: "letter-11pt-twoside",
        options: "11pt,twoside",
        geometry: None,
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "794.96999pt"),
            ("textwidth", "360.0pt"),
            ("textheight", "541.40024pt"),
            ("oddsidemargin", "54.8775pt"),
            ("evensidemargin", "54.8775pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "11.0pt"),
            ("baselineskip", "13.6pt"),
            ("parindent", "0.0pt"),
            ("parskip", "7.66498pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.5pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "27.37506pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "180.0pt"),
            ("indentedwidth", "180.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (8332738, 44416663),
        page2_first_baseline_sp: (8332738, 44418784),
    },
    Fixture {
        id: "letter-10pt-a4",
        options: "10pt,a4paper",
        geometry: None,
        dims: &[
            ("paperwidth", "597.50787pt"),
            ("paperheight", "845.04684pt"),
            ("textwidth", "345.0pt"),
            ("textheight", "598.0pt"),
            ("oddsidemargin", "53.98393pt"),
            ("evensidemargin", "53.98393pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "10.0pt"),
            ("baselineskip", "12.0pt"),
            ("parindent", "0.0pt"),
            ("parskip", "6.99997pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.0pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "25.00003pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "172.5pt"),
            ("indentedwidth", "172.5pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (8274177, 44481968),
        page2_first_baseline_sp: (8274177, 44484320),
    },
    Fixture {
        id: "letter-12pt-a4",
        options: "12pt,a4paper",
        geometry: None,
        dims: &[
            ("paperwidth", "597.50787pt"),
            ("paperheight", "845.04684pt"),
            ("textwidth", "390.0pt"),
            ("textheight", "592.0pt"),
            ("oddsidemargin", "31.48393pt"),
            ("evensidemargin", "31.48393pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "12.0pt"),
            ("baselineskip", "14.5pt"),
            ("parindent", "0.0pt"),
            ("parskip", "8.22487pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "6.0pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "29.3747pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "195.0pt"),
            ("indentedwidth", "195.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (6799617, 44350928),
        page2_first_baseline_sp: (6799617, 44353248),
    },
    Fixture {
        id: "letter-11pt-legal",
        options: "11pt,legalpaper",
        geometry: None,
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "1011.78pt"),
            ("textwidth", "360.0pt"),
            ("textheight", "759.00034pt"),
            ("oddsidemargin", "54.8775pt"),
            ("evensidemargin", "54.8775pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "11.0pt"),
            ("baselineskip", "13.6pt"),
            ("parindent", "0.0pt"),
            ("parskip", "7.66498pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.5pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "27.37506pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "180.0pt"),
            ("indentedwidth", "180.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (8332738, 44415793),
        page2_first_baseline_sp: (8332738, 44418784),
    },
    Fixture {
        id: "letter-11pt-landscape",
        options: "11pt,landscape",
        geometry: None,
        dims: &[
            ("paperwidth", "794.96999pt"),
            ("paperheight", "614.295pt"),
            ("textwidth", "360.0pt"),
            ("textheight", "364.60016pt"),
            ("oddsidemargin", "145.215pt"),
            ("evensidemargin", "145.215pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "11.0pt"),
            ("baselineskip", "13.6pt"),
            ("parindent", "0.0pt"),
            ("parskip", "7.66498pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.5pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "27.37506pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "180.0pt"),
            ("indentedwidth", "180.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (14253096, 44417370),
        page2_first_baseline_sp: (14253096, 44418784),
    },
    Fixture {
        id: "letter-11pt-draft",
        options: "11pt,draft",
        geometry: None,
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "794.96999pt"),
            ("textwidth", "360.0pt"),
            ("textheight", "541.40024pt"),
            ("oddsidemargin", "54.8775pt"),
            ("evensidemargin", "54.8775pt"),
            ("topmargin", "27.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "11.0pt"),
            ("baselineskip", "13.6pt"),
            ("parindent", "0.0pt"),
            ("parskip", "7.66498pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.5pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "5.0pt"),
            ("leftmargini", "27.37506pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "180.0pt"),
            ("indentedwidth", "180.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (8332738, 44416663),
        page2_first_baseline_sp: (8332738, 44418784),
    },
    Fixture {
        id: "letter-11pt-geometry-1in",
        options: "11pt",
        geometry: Some("margin=1in"),
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "794.96999pt"),
            ("textwidth", "469.75502pt"),
            ("textheight", "650.43001pt"),
            ("oddsidemargin", "0.0pt"),
            ("evensidemargin", "0.0pt"),
            ("topmargin", "-57.0pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "11.0pt"),
            ("baselineskip", "13.6pt"),
            ("parindent", "0.0pt"),
            ("parskip", "7.66498pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.5pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "27.37506pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "180.0pt"),
            ("indentedwidth", "180.0pt"),
            ("pdfpagewidth", "614.295pt"),
            ("pdfpageheight", "794.96999pt"),
        ],
        media_sp: (40258437, 52099153),
        page1_first_baseline_sp: (4736286, 46639414),
        page2_first_baseline_sp: (4736286, 46641971),
    },
    Fixture {
        id: "letter-10pt-geometry-default",
        options: "10pt",
        geometry: Some(""),
        dims: &[
            ("paperwidth", "614.295pt"),
            ("paperheight", "794.96999pt"),
            ("textwidth", "430.00462pt"),
            ("textheight", "556.47656pt"),
            ("oddsidemargin", "19.8752pt"),
            ("evensidemargin", "19.8752pt"),
            ("topmargin", "-33.87262pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "10.0pt"),
            ("baselineskip", "12.0pt"),
            ("parindent", "0.0pt"),
            ("parskip", "6.99997pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "5.0pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "25.00003pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "172.5pt"),
            ("indentedwidth", "172.5pt"),
            ("pdfpagewidth", "614.295pt"),
            ("pdfpageheight", "794.96999pt"),
        ],
        media_sp: (40258437, 52099153),
        page1_first_baseline_sp: (6038827, 45189645),
        page2_first_baseline_sp: (6038827, 45191831),
    },
    Fixture {
        id: "letter-12pt-geometry-a4-2cm",
        options: "12pt",
        geometry: Some("a4paper,margin=2cm"),
        dims: &[
            ("paperwidth", "597.50787pt"),
            ("paperheight", "845.04684pt"),
            ("textwidth", "483.69687pt"),
            ("textheight", "731.23584pt"),
            ("oddsidemargin", "-15.36449pt"),
            ("evensidemargin", "-15.36449pt"),
            ("topmargin", "-72.36449pt"),
            ("headheight", "12.0pt"),
            ("headsep", "45.0pt"),
            ("footskip", "25.0pt"),
            ("topskip", "12.0pt"),
            ("baselineskip", "14.5pt"),
            ("parindent", "0.0pt"),
            ("parskip", "8.22487pt"),
            ("marginparwidth", "90.0pt"),
            ("marginparsep", "11.0pt"),
            ("marginparpush", "5.0pt"),
            ("columnsep", "10.0pt"),
            ("maxdepth", "6.0pt"),
            ("footnotesep", "12.0pt"),
            ("overfullrule", "0.0pt"),
            ("leftmargini", "29.3747pt"),
            ("labelsep", "5.0pt"),
            ("longindentation", "195.0pt"),
            ("indentedwidth", "195.0pt"),
            ("pdfpagewidth", "597.50787pt"),
            ("pdfpageheight", "845.04684pt"),
        ],
        media_sp: (39158276, 55380990),
        page1_first_baseline_sp: (3729359, 50862323),
        page2_first_baseline_sp: (3729359, 50865199),
    },
];

fn resolved(f: &Fixture) -> ResolvedDocument {
    let mut setup = DocumentSetup::new(ClassKind::Letter, f.options);
    setup.geometry = f.geometry.map(|g| GeometryInput {
        package_options: g.to_string(),
        calls: Vec::new(),
    });
    setup.engine_default_media = (Sp(f.media_sp.0), Sp(f.media_sp.1));
    resolve(&setup)
}

fn model_dim(r: &ResolvedDocument, name: &str) -> String {
    let p = &r.params;
    match name {
        "paperwidth" => p.paperwidth.to_string(),
        "paperheight" => p.paperheight.to_string(),
        "textwidth" => p.textwidth.to_string(),
        "textheight" => p.textheight.to_string(),
        "oddsidemargin" => p.oddsidemargin.to_string(),
        "evensidemargin" => p.evensidemargin.to_string(),
        "topmargin" => p.topmargin.to_string(),
        "headheight" => p.headheight.to_string(),
        "headsep" => p.headsep.to_string(),
        "footskip" => p.footskip.to_string(),
        "topskip" => p.topskip.to_string(),
        "baselineskip" => p.baselineskip.to_string(),
        "parindent" => p.parindent.to_string(),
        "parskip" => p.parskip.to_string(),
        "marginparwidth" => p.marginparwidth.to_string(),
        "marginparsep" => p.marginparsep.to_string(),
        "marginparpush" => p.marginparpush.to_string(),
        "columnsep" => p.columnsep.to_string(),
        "maxdepth" => p.maxdepth.to_string(),
        "footnotesep" => p.footnotesep.to_string(),
        "overfullrule" => p.overfullrule.to_string(),
        "leftmargini" => p.leftmargini.to_string(),
        "labelsep" => p.labelsep.to_string(),
        "longindentation" => letter_indentation(&r.options).0.to_string(),
        "indentedwidth" => letter_indentation(&r.options).1.to_string(),
        "pdfpagewidth" => r.frame.pdf_page_width.to_string(),
        "pdfpageheight" => r.frame.pdf_page_height.to_string(),
        other => panic!("no model value for \\{other}"),
    }
}

#[test]
fn letter_lengths_match_pdflatex() {
    let mut failures = Vec::new();
    let mut checked = 0;
    for f in FIXTURES {
        let r = resolved(f);
        for (name, want) in f.dims {
            checked += 1;
            let got = model_dim(&r, name);
            if got != *want {
                failures.push(format!("{}: \\{name}: pdflatex {want} model {got}", f.id));
            }
        }
    }
    assert!(checked >= 300, "only {checked} lengths checked");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn letter_first_baseline_matches_pdflatex_to_the_scaled_point() {
    let mut failures = Vec::new();
    for f in FIXTURES {
        let r = resolved(f);
        let frame = &r.frame;
        let media_h = frame.pdf_page_height;
        // Page 2: `\@texttop` is empty, so the frame formula must be exact.
        let (x, y) = f.page2_first_baseline_sp;
        let from_top = media_h - Sp(y);
        if Sp(x) != frame.text_left(2) {
            failures.push(format!(
                "{}: page 2 x: pdflatex {} model {}",
                f.id,
                Sp(x),
                frame.text_left(2)
            ));
        }
        if from_top != frame.first_baseline {
            failures.push(format!(
                "{}: page 2 first baseline: pdflatex {} model {} ({}sp)",
                f.id,
                from_top,
                frame.first_baseline,
                from_top.0 - frame.first_baseline.0
            ));
        }
        // Page 1: letter.cls's own `\@texttop` fil glue only ever pushes the
        // first baseline DOWN, and by well under a point here. The page
        // builder, not the frame, owns that share.
        let (x1, y1) = f.page1_first_baseline_sp;
        let from_top1 = media_h - Sp(y1);
        if Sp(x1) != frame.text_left(1) {
            failures.push(format!("{}: page 1 x moved: {}", f.id, Sp(x1)));
        }
        let extra = from_top1.0 - frame.first_baseline.0;
        if !(0..65_536).contains(&extra) {
            failures.push(format!(
                "{}: page 1 \\@texttop share {extra}sp outside [0pt, 1pt)",
                f.id
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The class defines no sectioning commands, no `\maketitle` and no
/// `\tableofcontents` (`\@ifundefined` for each, TeX Live 2025), and sets
/// neither `secnumdepth` nor `tocdepth`, so both keep latex.ltx's zero.
/// `letter.cls` ends with `\pagestyle{plain}` and `\pagenumbering{arabic}`.
#[test]
fn letter_has_no_sectioning_and_plain_pages() {
    let r = resolved(&FIXTURES[1]);
    assert!(r.headings.is_empty(), "{:?}", r.headings);
    assert!(r.chapter.is_none());
    assert_eq!((r.secnumdepth, r.tocdepth), (0, 0));
    assert_eq!(r.pagestyle, PageStyle::Plain);
    assert_eq!(r.numbering, Numbering::Arabic);
    assert!(!r.options.kind.has_chapters());
    assert!(!r.options.kind.has_sections());
    // letter.cls's own `\ps@headings` issues no marks, and there are no
    // sectioning commands to issue them from.
    let mut headings = DocumentSetup::new(ClassKind::Letter, "11pt");
    headings.pagestyle = Some(PageStyle::Headings);
    assert!(resolve(&headings).mark_rules.is_empty());
}

/// letter.cls declares neither `onecolumn`/`twocolumn` nor
/// `titlepage`/`notitlepage` nor `openbib`; pdflatex reports all of them as
/// unused global options for a `letter` document.
#[test]
fn letter_does_not_declare_the_column_or_titlepage_options() {
    let r = resolve(&DocumentSetup::new(
        ClassKind::Letter,
        "onecolumn,twocolumn,titlepage,notitlepage,openbib,openright,12pt",
    ));
    assert_eq!(
        r.options.unused,
        vec![
            "onecolumn".to_string(),
            "twocolumn".to_string(),
            "titlepage".to_string(),
            "notitlepage".to_string(),
            "openbib".to_string(),
            "openright".to_string(),
        ]
    );
    assert!(!r.options.twocolumn, "twocolumn is not a letter option");
    assert!(!r.options.titlepage);
    assert_eq!(r.params.textwidth.to_string(), "390.0pt");
    assert_eq!(r.frame.columns.len(), 1);
}

/// The regression this class exists to stop: before `ClassKind::Letter`,
/// `DocumentSetup::from_preamble` returned `None` for a `letter` preamble
/// and the render pipeline fell back to *article* geometry. Every length
/// below differs, so that fallback cannot be mistaken for a match.
#[test]
fn letter_geometry_is_not_article_geometry() {
    let letter = resolve(&DocumentSetup::new(ClassKind::Letter, "11pt"));
    let article = resolve(&DocumentSetup::new(ClassKind::Article, "11pt"));
    let l = &letter.params;
    let a = &article.params;
    assert_ne!(l.topmargin, a.topmargin);
    assert_ne!(l.headsep, a.headsep);
    assert_ne!(l.footskip, a.footskip);
    assert_ne!(l.oddsidemargin, a.oddsidemargin);
    assert_ne!(l.parindent, a.parindent);
    assert_ne!(l.parskip, a.parskip);
    assert_ne!(l.marginparwidth, a.marginparwidth);
    assert_ne!(l.labelsep, a.labelsep);
    assert_ne!(l.footnotesep, a.footnotesep);
    assert_ne!(l.skip_footins, a.skip_footins);
    assert_ne!(letter.frame.text_top, article.frame.text_top);
    // The lengths letter really does inherit from size1x.clo must NOT move.
    assert_eq!(l.textwidth, a.textwidth);
    assert_eq!(l.textheight, a.textheight);
    assert_eq!(l.baselineskip, a.baselineskip);
    assert_eq!(l.topskip, a.topskip);
}

/// `\longindentation` is `.5\textwidth` and `\indentedwidth` the rest
/// (letter.cls 219-222); `\closing` sets its parbox at exactly these.
#[test]
fn letter_indentation_splits_the_measure() {
    for f in FIXTURES {
        let r = resolved(f);
        let (long, indented) = letter_indentation(&r.options);
        assert_eq!(long + indented, class_params(&r.options).textwidth, "{}", f.id);
    }
}

/// A `letter` preamble must resolve as `letter` rather than falling through
/// to the article default.
#[test]
fn from_preamble_recognises_the_letter_class() {
    let setup = DocumentSetup::from_preamble(
        "\\documentclass[11pt]{letter}\n\\usepackage[margin=1in]{geometry}\n\\begin{document}",
    )
    .expect("letter is a modelled class");
    assert_eq!(setup.class, ClassKind::Letter);
    assert_eq!(setup.class_options, "11pt");
    assert!(setup.geometry.is_some());
    assert_eq!(ClassKind::parse("letter"), Some(ClassKind::Letter));
    assert_eq!(ClassKind::Letter.name(), "letter");
}
