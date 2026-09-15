//! Exact export route: original-GID glyph runs, verbatim decimals, typed
//! operators, bounded font subset identity, deterministic serialisation,
//! and (oracle only) the pdfTeX/xdvipdfmx reference round trip with
//! difference classification. The oracle is located by `common::pdflatex`
//! and friends -- `$FLASHTEX_PDFLATEX`, then `PATH`, then MacTeX -- and its
//! absence is announced rather than swallowed; see `tests/common/mod.rs`.

mod common;

use flashtex_pdf::cff::CffFont;
use flashtex_pdf::compare::{self, Category};
use flashtex_pdf::exact::*;
use flashtex_pdf::reader::PdfFile;
use flashtex_pdf::truetype::TrueTypeFont;
use flashtex_pdf::verify;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn d(s: &str) -> Decimal {
    Decimal::new(s).unwrap()
}

/// A minimal single-font CFF: `.notdef` plus `n` box glyphs of increasing
/// width, with a Private DICT and one local subroutine, so the subsetter's
/// identity guarantees can be checked without a font file.
fn synthetic_cff(n: usize) -> Vec<u8> {
    fn index(items: &[Vec<u8>]) -> Vec<u8> {
        let mut out = (items.len() as u16).to_be_bytes().to_vec();
        if items.is_empty() {
            return out;
        }
        out.push(2);
        let mut off = 1u16;
        out.extend_from_slice(&off.to_be_bytes());
        for i in items {
            off += i.len() as u16;
            out.extend_from_slice(&off.to_be_bytes());
        }
        for i in items {
            out.extend_from_slice(i);
        }
        out
    }
    fn int5(v: i32) -> Vec<u8> {
        let b = v.to_be_bytes();
        vec![29, b[0], b[1], b[2], b[3]]
    }
    // Charstrings: glyph k: "(50+10k) 0 rmoveto 100 hlineto (60) vlineto -100 hlineto endchar"
    let mut charstrings = vec![vec![14u8]];
    for k in 0..n {
        let w = 50 + 10 * k as i32;
        // width-ish operand, 0, rmoveto, subr index -107 (bias 107 -> 0),
        // callsubr, endchar
        charstrings.push(vec![(w + 139) as u8, 139, 21, 139 - 107, 10, 14]);
    }
    let lsubr = vec![139 + 100, 6, 139 + 60, 7, 139u8.wrapping_sub(100), 6, 11]; // hlineto vlineto hlineto return
    let name = index(&[b"SynthFont".to_vec()]);
    let strings = index(&[]);
    let gsubrs = index(&[]);
    let lsubrs = index(&[lsubr]);
    let cs = index(&charstrings);
    let mut charset = vec![0u8];
    for i in 1..=n {
        charset.extend_from_slice(&(i as u16).to_be_bytes());
    }
    let mut private = vec![139u8, 139, 6]; // BlueValues 0 0
    let plen = private.len() + 5 + 1;
    private.extend_from_slice(&int5(plen as i32));
    private.push(19); // Subrs
    let top = |charset_off: i32, cs_off: i32, priv_off: i32| {
        let mut t = vec![139, 139, 139 + 100, 139 + 100, 5]; // FontBBox
        t.extend_from_slice(&int5(charset_off));
        t.push(15);
        t.extend_from_slice(&int5(cs_off));
        t.push(17);
        t.extend_from_slice(&int5(private.len() as i32));
        t.extend_from_slice(&int5(priv_off));
        t.push(18);
        index(&[t])
    };
    let header = [1u8, 0, 4, 4];
    let top_len = top(0, 0, 0).len();
    let charset_off = header.len() + name.len() + top_len + strings.len() + gsubrs.len();
    let cs_off = charset_off + charset.len();
    let priv_off = cs_off + cs.len();
    let mut out = header.to_vec();
    out.extend(name);
    out.extend(top(charset_off as i32, cs_off as i32, priv_off as i32));
    out.extend(strings);
    out.extend(gsubrs);
    out.extend(charset);
    out.extend(cs);
    out.extend(private);
    out.extend(lsubrs);
    out
}

fn cid_font_from_synthetic(gids: &[u16]) -> ExactFont {
    let cff = CffFont::parse(&synthetic_cff(8)).unwrap();
    let set: BTreeSet<u16> = gids.iter().copied().collect();
    let sub = cff.subset(&set).unwrap();
    let mut widths = BTreeMap::new();
    for &g in gids {
        widths.insert(g, Decimal::from_i64(500 + 10 * g as i64));
    }
    ExactFont::CidCff(CidFont {
        base_font: format!("{}+SynthFont", subset_tag(&set, "synthetic")),
        descendant_base_font: None,
        program: FontProgram::Cff(sub.bytes),
        widths,
        default_width: d("1000"),
        descriptor: FontDescriptor {
            flags: 4,
            bbox: [d("0"), d("0"), d("100"), d("100")],
            italic_angle: d("0"),
            ascent: d("100"),
            descent: d("0"),
            cap_height: d("60"),
            stem_v: d("80"),
            x_height: None,
            char_set: None,
            extra: Vec::new(),
        },
        to_unicode: BTreeMap::from([(1u16, "a".to_string()), (3, "ffi".to_string())]),
        to_unicode_verbatim: None,
        cid_set: None,
        glyphs: set,
    })
}

fn sample_document() -> ExactDocument {
    let run = GlyphRun {
        font: "F1".into(),
        size: d("12.000"),
        glyphs: vec![
            PlacedGlyph {
                gid: 1,
                origin: Some((d("72"), d("700.12345"))),
                adjust: None,
            },
            PlacedGlyph {
                gid: 3,
                origin: None,
                adjust: None,
            },
            PlacedGlyph {
                gid: 5,
                origin: Some((d("100.5"), d("700.12345"))),
                adjust: None,
            },
        ],
    };
    let mut ops = run.to_ops().unwrap();
    ops.extend(Op::rule(d("72"), d("690.25"), d("28.500"), d("0.398")));
    // A rendering-core style clipped filled path with exact colour.
    ops.extend([
        Op::Save,
        Op::Rect([d("0"), d("0"), d("612"), d("792")]),
        Op::ClipNonZero,
        Op::EndPath,
        Op::FillRgb([d("0.125"), d("0"), d("1")]),
        Op::Move(d("200"), d("600")),
        Op::Line(d("210.0625"), d("600")),
        Op::Cubic([
            d("212"),
            d("602"),
            d("214"),
            d("604"),
            d("216.00000000001"),
            d("606"),
        ]),
        Op::Close,
        Op::Fill,
        Op::Restore,
    ]);
    ExactDocument {
        images: Default::default(),
        pages: vec![
            ExactPage {
                width: d("612"),
                height: d("792"),
                content: Content::Ops(ops),
                fonts: None,
            },
            ExactPage {
                width: d("595.276"),
                height: d("841.89"),
                content: Content::Verbatim(
                    b"BT\n/F1 9.5 Tf\n1 0 0 1 50 800 Tm\n(\\000\\001) Tj\nET\n".to_vec(),
                ),
                fonts: Some(vec!["F1".into()]),
            },
        ],
        fonts: BTreeMap::from([("F1".to_string(), cid_font_from_synthetic(&[1, 3, 5]))]),
    }
}

#[test]
fn same_input_renders_byte_identical_output_twice_and_verifies() {
    let doc = sample_document();
    let a = render_exact(&doc).unwrap();
    let b = render_exact(&doc).unwrap();
    assert_eq!(a.bytes, b.bytes, "deterministic serialization");
    assert!(
        a.warnings.is_empty(),
        "the exact route has no silent substitutions"
    );
    let s = verify::check_structure(&a.bytes).unwrap();
    assert_eq!(
        s.object_count,
        3 + 2 * 2 + 5,
        "catalog, pages, info, 2×(page, content), 5 font objects"
    );
    assert!(a.bytes.starts_with(b"%PDF-1.4\n"));
    assert!(
        !a.bytes.windows(3).any(|w| w == b"/ID"),
        "no /ID: output must not vary"
    );
    assert!(
        !a.bytes.windows(13).any(|w| w == b"/CreationDate"),
        "no timestamps"
    );
}

#[test]
fn decimals_and_codes_are_written_verbatim() {
    let doc = sample_document();
    let out = render_exact(&doc).unwrap();
    let content = verify::stream_data(&out.bytes, 5).unwrap();
    let text = String::from_utf8_lossy(&content);
    assert!(
        text.contains("/F1 12.000 Tf\n"),
        "font size trailing zeros kept: {text}"
    );
    assert!(
        text.contains("1 0 0 1 72 700.12345 Tm\n"),
        "five decimals kept: {text}"
    );
    assert!(
        text.contains("(\\000\\001\\000\\003) Tj\n"),
        "two-byte GIDs, no re-encoding: {text}"
    );
    assert!(
        text.contains("72 690.25 28.500 0.398 re\nf\n"),
        "typed rule verbatim: {text}"
    );
    assert!(
        text.contains("216.00000000001 606 c\n"),
        "long decimal kept, no f64 round trip: {text}"
    );
    assert!(text.contains("0.125 0 1 rg\n"));
    let reparsed = parse(&content).unwrap();
    match &doc.pages[0].content {
        Content::Ops(ops) => assert_eq!(
            &reparsed, ops,
            "our own output parses back to the same operators"
        ),
        _ => unreachable!(),
    }
    // Page 2 was verbatim: inserted byte for byte.
    let page2 = verify::stream_data(&out.bytes, 7).unwrap();
    assert_eq!(
        page2,
        b"BT\n/F1 9.5 Tf\n1 0 0 1 50 800 Tm\n(\\000\\001) Tj\nET\n"
    );
}

#[test]
fn cff_subset_keeps_original_gids_as_cids_and_glyph_bytes() {
    let src = synthetic_cff(8);
    let font = CffFont::parse(&src).unwrap();
    let set = BTreeSet::from([2u16, 5, 7]);
    let sub = font.subset(&set).unwrap();
    assert_eq!(sub.glyphs, vec![0, 2, 5, 7]);
    let parsed = CffFont::parse(&sub.bytes).unwrap();
    assert!(parsed.is_cid_keyed());
    assert_eq!(parsed.glyph_count(), 4);
    for (new, &old) in sub.glyphs.iter().enumerate() {
        assert_eq!(
            parsed.charset_entry(new as u16),
            Some(old),
            "CID of subset glyph {new} is original GID {old}"
        );
        assert_eq!(
            parsed.charstring(new as u16),
            font.charstring(old),
            "charstring bytes of glyph {old} unchanged"
        );
    }
    assert_eq!(
        font.subset(&set).unwrap(),
        sub,
        "same glyph set twice gives byte-identical program"
    );
    assert_ne!(
        font.subset(&BTreeSet::from([2u16, 5])).unwrap().bytes,
        sub.bytes
    );
    // Bounded: an out-of-range glyph is an error, not silently dropped.
    assert!(font.subset(&BTreeSet::from([200u16])).is_err());
    // A CID-keyed result is not subset again (explicit, not guessed).
    assert!(matches!(
        parsed.subset(&set),
        Err(flashtex_pdf::cff::CffError::CidKeyedSource)
    ));
}

#[test]
fn subset_identity_is_bounded_by_the_declared_glyph_set() {
    let mut doc = sample_document();
    // Showing a GID the font was not subset for is an error naming the glyph.
    doc.pages[0].content = Content::Ops(vec![
        Op::BeginText,
        Op::Font("F1".into(), d("10")),
        Op::TextMatrix([d("1"), d("0"), d("0"), d("1"), d("10"), d("10")]),
        Op::ShowText(vec![0, 4]),
        Op::EndText,
    ]);
    let e = render_exact(&doc).unwrap_err().to_string();
    assert!(e.contains("glyph id 4 is not in the font's subset"), "{e}");
    // Odd byte count for a two-byte font.
    doc.pages[0].content = Content::Verbatim(b"BT /F1 10 Tf 1 0 0 1 0 0 Tm (\\000) Tj ET".to_vec());
    let e = render_exact(&doc).unwrap_err().to_string();
    assert!(e.contains("odd byte count"), "{e}");
}

#[test]
fn validation_rejects_unbalanced_state_unknown_fonts_and_foreign_operators() {
    let mut doc = sample_document();
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "font resource /F9 is not declared",
            b"BT /F9 10 Tf ET".to_vec(),
        ),
        ("Q without matching q", b"Q".to_vec()),
        ("unmatched q", b"q q Q".to_vec()),
        ("unterminated text object", b"BT".to_vec()),
        ("Tj outside BT/ET", b"(\\000\\001) Tj".to_vec()),
        ("without a path", b"f".to_vec()),
        ("path segment without a current point", b"1 2 l".to_vec()),
        (
            "outside the exact export's bounded set",
            b"/GS0 gs".to_vec(),
        ),
        (
            "outside the exact export's bounded set",
            b"q 0.5 0.5 0.5 sc Q".to_vec(),
        ),
        ("not a PDF number", b"1e3 0 0 1 0 0 cm".to_vec()),
        ("text shown before Tf", b"BT (\\000\\001) Tj ET".to_vec()),
        ("page ends with an unpainted path", b"0 0 m 1 1 l".to_vec()),
    ];
    for (expect, content) in cases {
        doc.pages[0].content = Content::Verbatim(content.clone());
        let e = render_exact(&doc).unwrap_err().to_string();
        assert!(
            e.contains(expect),
            "{:?}: {e}",
            String::from_utf8_lossy(&content)
        );
    }
    doc.pages[0].content = Content::Verbatim(b"BT /F1 10 Tf ET".to_vec());
    doc.pages[0].fonts = Some(vec![]);
    let e = render_exact(&doc).unwrap_err().to_string();
    assert!(e.contains("not in this page's resources"), "{e}");
    doc.pages[0].fonts = Some(vec!["F2".into()]);
    let e = render_exact(&doc).unwrap_err().to_string();
    assert!(e.contains("/F2 is not declared in the document"), "{e}");
    doc.pages[0].fonts = None;
    doc.pages[0].content = Content::Verbatim(b"0 0 m 1 1 l S".to_vec());
    doc.pages[0].width = d("0");
    assert!(
        render_exact(&doc)
            .unwrap_err()
            .to_string()
            .contains("width 0 is not positive")
    );
    doc.pages.clear();
    assert!(
        render_exact(&doc)
            .unwrap_err()
            .to_string()
            .contains("at least one page")
    );
}

#[test]
fn pdftex_style_simple_type1_font_round_trips_through_reader_and_reemit() {
    // A fake Type 1 program: the bytes are arbitrary but the lengths are real.
    let clear =
        b"%!PS-AdobeFont-1.0: Fake 001.000\n/FontName /Fake def\ncurrentfile eexec\n".to_vec();
    let binary: Vec<u8> = (0..600u32).map(|i| (i * 7 % 251) as u8).collect();
    let mut bytes = clear.clone();
    bytes.extend_from_slice(&binary);
    let font = ExactFont::Simple(SimpleFont {
        subtype: "Type1".into(),
        base_font: "ABCDEF+Fake".into(),
        program: Some(FontProgram::Type1 {
            length1: clear.len(),
            length2: binary.len(),
            length3: 0,
            bytes,
        }),
        first_char: 2,
        widths: vec![d("556"), d("556"), d("167"), d("333.5")],
        encoding: Some(Encoding::Differences {
            base: None,
            differences: vec![(2, "fi".into()), (3, "fl".into()), (5, "T".into())],
        }),
        descriptor: Some(FontDescriptor {
            flags: 4,
            bbox: [d("-168"), d("-281"), d("1000"), d("924")],
            italic_angle: d("0"),
            ascent: d("678"),
            descent: d("-216"),
            cap_height: d("651"),
            stem_v: d("85"),
            x_height: Some(d("450")),
            char_set: Some("/T/fi/fl".into()),
            extra: vec![
                ("AvgWidth".into(), "537".into()),
                (
                    "Style".into(),
                    "<</Panose (\\000\\000\\000\\000\\005\\000\\000\\000\\000\\000\\000\\000)>>"
                        .into(),
                ),
            ],
        }),
        to_unicode: Some(b"%!PS-Adobe-3.0 Resource-CMap\n(fake cmap body)\n".to_vec()),
    });
    let content = b"BT\n/F44 11.9552 Tf 72 708.045 Td [(\\002\\003)-250(\\005)10(\\004)]TJ\nET\nq\n1 0 0 1 135.015 711.034 cm\n[]0 d 0 J 0.398 w 0 0 m 4.498 0 l S\nQ\n".to_vec();
    let doc = ExactDocument {
        images: Default::default(),
        pages: vec![ExactPage {
            width: d("612"),
            height: d("792"),
            content: Content::Verbatim(content.clone()),
            fonts: None,
        }],
        fonts: BTreeMap::from([("F44".to_string(), font.clone())]),
    };
    let out = render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    let text = String::from_utf8_lossy(&out.bytes);
    assert!(text.contains("/Length1 "), "{text}");
    assert!(text.contains(&format!(
        "/Length1 {} /Length2 {} /Length3 0",
        clear.len(),
        binary.len()
    )));
    assert!(
        text.contains("/Differences [ 2 /fi /fl 5 /T ] >>"),
        "consecutive codes collapse as pdfTeX writes them"
    );
    assert!(text.contains("/FirstChar 2 /LastChar 5 /Widths [ 556 556 167 333.5 ]"));
    assert!(text.contains("/CharSet (/T/fi/fl)"));
    assert!(text.contains("/XHeight 450"));

    // Read it back with the independent reader and rebuild the document.
    let file = PdfFile::parse(&out.bytes).unwrap();
    assert_eq!(
        file.features.filters.len(),
        0,
        "no compression on this route"
    );
    let back = compare::reemit(&file).unwrap();
    assert_eq!(back.pages[0].content, Content::Verbatim(content));
    assert_eq!(
        back.fonts["F44"], font,
        "every font value survives verbatim"
    );
    // Rendering the rebuilt document is a fixed point.
    let again = render_exact(&back).unwrap();
    assert_eq!(
        again.bytes, out.bytes,
        "reemit of our own output is byte-identical"
    );
    let report = compare::classify(&file, &PdfFile::parse(&again.bytes).unwrap(), "a", "b");
    assert!(report.categories.is_empty(), "{}", report.text());

    // A code outside FirstChar..LastChar is refused, naming the code.
    let mut bad = doc.clone();
    bad.pages[0].content = Content::Verbatim(b"BT /F44 10 Tf 0 0 Td (\\007) Tj ET".to_vec());
    let e = render_exact(&bad).unwrap_err().to_string();
    assert!(e.contains("code 7 has no width"), "{e}");
}

#[test]
fn pfb_segments_become_length1_2_3() {
    let mut pfb = vec![0x80, 1, 5, 0, 0, 0];
    pfb.extend_from_slice(b"clear");
    pfb.extend_from_slice(&[0x80, 2, 3, 0, 0, 0, 1, 2, 3]);
    pfb.extend_from_slice(&[0x80, 1, 2, 0, 0, 0]);
    pfb.extend_from_slice(b"00");
    pfb.extend_from_slice(&[0x80, 3]);
    match FontProgram::type1_from_pfb(&pfb).unwrap() {
        FontProgram::Type1 {
            bytes,
            length1,
            length2,
            length3,
        } => {
            assert_eq!((length1, length2, length3), (5, 3, 2));
            assert_eq!(bytes, b"clear\x01\x02\x0300");
        }
        other => panic!("{other:?}"),
    }
    assert!(
        FontProgram::type1_from_pfb(b"%!PS-AdobeFont").is_err(),
        "PFA is reported, not guessed"
    );
}

#[test]
fn classify_reports_operand_and_program_differences_by_category() {
    let doc = sample_document();
    let a = render_exact(&doc).unwrap();
    let mut changed = doc.clone();
    if let Content::Ops(ops) = &mut changed.pages[0].content {
        ops[2] = Op::TextMatrix([d("1"), d("0"), d("0"), d("1"), d("72.0"), d("700.12345")]);
    }
    changed
        .fonts
        .insert("F1".into(), cid_font_from_synthetic(&[1, 3, 5, 6]));
    let b = render_exact(&changed).unwrap();
    let report = compare::classify(
        &PdfFile::parse(&a.bytes).unwrap(),
        &PdfFile::parse(&b.bytes).unwrap(),
        "a",
        "b",
    );
    assert!(
        report.categories.contains(&Category::ContentOperands),
        "{}",
        report.text()
    );
    assert!(
        report.categories.contains(&Category::FontProgram),
        "{}",
        report.text()
    );
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("72 700.12345 Tm | 1 0 0 1 72.0 700.12345 Tm")),
        "{}",
        report.text()
    );
    assert_eq!(report.content_ops_identical, vec![false, true]);
}

fn latin_modern() -> Option<PathBuf> {
    common::latin_modern_otf()
}

#[test]
fn latin_modern_cff_subset_preserves_gids_and_renders_in_coregraphics() {
    let Some(path) = latin_modern() else {
        common::skip(
            "latin_modern_cff_subset_preserves_gids_and_renders_in_coregraphics",
            "Latin Modern lmroman10-regular.otf is not resolvable (FLASHTEX_LM_DIR, \
             a TeX Live root or kpsewhich)",
        );
        return;
    };
    let font = TrueTypeFont::load(&path).unwrap();
    let cff = CffFont::parse(font.cff_table().unwrap()).unwrap();
    let text = "Hello world. fi";
    let gids: Vec<u16> = text.chars().map(|c| font.glyph_id(c).unwrap()).collect();
    let set: BTreeSet<u16> = gids.iter().copied().collect();
    let mut tu = BTreeMap::new();
    for (c, g) in text.chars().zip(&gids) {
        tu.insert(*g, c.to_string());
    }
    let (exact, outcome, note) = ExactFont::cid_from_opentype(&font, &set, tu).unwrap();
    assert_eq!(outcome, SubsetOutcome::CffSubset, "{note:?}");
    let ExactFont::CidCff(cid) = &exact else {
        panic!()
    };
    assert!(
        cid.base_font.ends_with("+LMRoman10-Regular"),
        "{}",
        cid.base_font
    );
    let sub = CffFont::parse(cid.program.bytes()).unwrap();
    assert!(sub.is_cid_keyed());
    assert_eq!(sub.glyph_count() as usize, set.len() + 1);
    for (new, &old) in std::iter::once(&0u16).chain(set.iter()).enumerate() {
        assert_eq!(sub.charset_entry(new as u16), Some(old));
        // Subroutines are pruned and renumbered, so the raw charstring may
        // differ in its call operands; the inlined outline program may not.
        assert_eq!(
            sub.expanded_charstring(new as u16).unwrap(),
            cff.expanded_charstring(old).unwrap(),
            "glyph {old} outline identical after subsetting"
        );
    }
    // Every retained glyph's raw charstring is identical to the source when
    // the subroutine INDEXes are copied whole (the unpruned form).
    let whole = cff
        .subset_with(
            &set,
            flashtex_pdf::cff::SubsetOptions { prune_subrs: false },
        )
        .unwrap();
    let whole_parsed = CffFont::parse(&whole.bytes).unwrap();
    for (new, &old) in std::iter::once(&0u16).chain(set.iter()).enumerate() {
        assert_eq!(whole_parsed.charstring(new as u16), cff.charstring(old));
    }
    assert!(
        cid.program.bytes().len() < whole.bytes.len() / 2
            && whole.bytes.len() < font.cff_table().unwrap().len() / 2,
        "pruned {} < whole-subr {} < table {}",
        cid.program.bytes().len(),
        whole.bytes.len(),
        font.cff_table().unwrap().len()
    );
    // Identity is bounded and deterministic.
    let (again, _, _) = ExactFont::cid_from_opentype(&font, &set, BTreeMap::new()).unwrap();
    assert_eq!(again.program_identity(), exact.program_identity());

    let run = GlyphRun {
        font: "F1".into(),
        size: d("24"),
        glyphs: gids
            .iter()
            .enumerate()
            .map(|(i, g)| PlacedGlyph {
                gid: *g,
                origin: if i == 0 {
                    Some((d("72"), d("700")))
                } else {
                    None
                },
                adjust: None,
            })
            .collect(),
    };
    let doc = ExactDocument {
        images: Default::default(),
        pages: vec![ExactPage {
            width: d("612"),
            height: d("792"),
            content: Content::Ops(run.to_ops().unwrap()),
            fonts: None,
        }],
        fonts: BTreeMap::from([("F1".to_string(), exact)]),
    };
    let out = render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    if !cfg!(target_os = "macos") || !Path::new("/usr/bin/sips").exists() {
        common::announce(
            "NOTE latin_modern_cff_subset_preserves_gids_and_renders_in_coregraphics: \
             CoreGraphics raster check skipped, /usr/bin/sips is macOS-only. The \
             subsetting assertions above did run.",
        );
        return;
    }
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-exact-lm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pdf = dir.join("lm.pdf");
    std::fs::write(&pdf, &out.bytes).unwrap();
    let png = dir.join("lm.png");
    let sips = std::process::Command::new("/usr/bin/sips")
        .env("CG_PDF_VERBOSE", "1")
        .args(["-s", "format", "png", "--resampleWidth", "612"])
        .arg(&pdf)
        .arg("--out")
        .arg(&png)
        .output()
        .unwrap();
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&sips.stdout),
        String::from_utf8_lossy(&sips.stderr)
    );
    assert!(sips.status.success(), "{log}");
    assert!(
        !log.to_lowercase().contains("unsupported") && !log.to_lowercase().contains("error"),
        "CoreGraphics complained: {log}"
    );
    // Ink must exist where the run was placed and nowhere else; check via a
    // second raster of an empty page to compare file sizes coarsely.
    let ink = std::fs::metadata(&png).unwrap().len();
    let mut empty = doc.clone();
    empty.pages[0].content = Content::Ops(vec![]);
    std::fs::write(dir.join("empty.pdf"), render_exact(&empty).unwrap().bytes).unwrap();
    let empty_png = dir.join("empty.png");
    std::process::Command::new("/usr/bin/sips")
        .args(["-s", "format", "png", "--resampleWidth", "612"])
        .arg(dir.join("empty.pdf"))
        .arg("--out")
        .arg(&empty_png)
        .output()
        .unwrap();
    assert!(
        ink > std::fs::metadata(&empty_png).unwrap().len() + 500,
        "glyphs painted"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Oracle only: pdflatex, never part of the product path.
fn pdflatex() -> Option<PathBuf> {
    common::pdflatex()
}

#[test]
fn pdflatex_reference_reemits_with_identical_content_and_font_programs() {
    const TEST: &str = "pdflatex_reference_reemits_with_identical_content_and_font_programs";
    let Some(tex) = pdflatex() else {
        common::skip(TEST, "no pdflatex found");
        return;
    };
    common::announce(&format!("ORACLE {TEST}: pdflatex = {}", tex.display()));
    let dir =
        std::env::temp_dir().join(format!("flashtex-pdf-exact-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("main.tex"),
        "\\documentclass[12pt]{article}\\usepackage[T1]{fontenc}\\usepackage{times}\\usepackage[margin=1in]{geometry}\\setlength{\\parindent}{0pt}\\pagestyle{empty}\\begin{document}Inline math: $\\frac{a}{b}$, $\\alpha+\\beta$ and \\textbf{bold} text.\\end{document}\n",
    )
    .unwrap();
    let status = std::process::Command::new(&tex)
        .current_dir(&dir)
        .args(["-interaction=batchmode", "-halt-on-error", "main.tex"])
        .output()
        .unwrap();
    let reference = dir.join("main.pdf");
    if !status.status.success() || !reference.is_file() {
        let log = dir.join("main.log");
        let detail = format!(
            "exit {:?}\nstdout:\n{}\nstderr:\n{}\nlog:\n{}",
            status.status.code(),
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr),
            std::fs::read_to_string(&log).unwrap_or_default()
        );
        common::oracle_failed(TEST, "pdflatex did not produce main.pdf", &detail);
    }
    let bytes = std::fs::read(&reference).unwrap();
    let file = PdfFile::parse(&bytes).unwrap();
    assert!(
        file.features.xref_stream || file.features.filters.contains("FlateDecode"),
        "reference uses pdfTeX's compressed layout"
    );
    let doc = compare::reemit(&file).unwrap();
    assert!(doc.fonts.len() >= 3, "text, bold and math fonts");
    let out = render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    let ours = PdfFile::parse(&out.bytes).unwrap();
    let report = compare::classify(&file, &ours, "pdflatex", "exact");
    eprintln!("{}", report.text());
    assert!(
        report.content_byte_identical.iter().all(|&x| x),
        "content streams byte-identical"
    );
    assert!(
        !report.font_program_identical.is_empty()
            && report.font_program_identical.values().all(|&x| x),
        "font programs byte-identical"
    );
    let allowed = BTreeSet::from([
        Category::ObjectLayout,
        Category::Compression,
        Category::DocumentIdentity,
    ]);
    assert!(
        report.categories.is_subset(&allowed),
        "only layout/compression/identity may differ: {}",
        report.text()
    );
    assert!(
        report.categories.contains(&Category::Compression),
        "pdfTeX compresses, this route does not"
    );
    // Fixed point: re-emitting our own output changes nothing.
    let again = render_exact(&compare::reemit(&ours).unwrap()).unwrap();
    assert_eq!(again.bytes, out.bytes);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Oracle only: xelatex (xdvipdfmx) writes CID-keyed CFF subsets under
/// `Type0`/`Identity-H`, the same font form this crate's exact route
/// produces, so its output exercises the CID side of the reader/re-emit.
fn xelatex() -> Option<PathBuf> {
    common::xelatex()
}

#[test]
fn xelatex_reference_with_cid_keyed_cff_reemits_identically() {
    const TEST: &str = "xelatex_reference_with_cid_keyed_cff_reemits_identically";
    let (Some(tex), Some(lm)) = (xelatex(), latin_modern()) else {
        common::skip(
            TEST,
            "no xelatex on PATH, or Latin Modern lmroman10-regular.otf is not resolvable",
        );
        return;
    };
    common::announce(&format!(
        "ORACLE {TEST}: xelatex = {}, Latin Modern = {}",
        tex.display(),
        lm.display()
    ));
    let lm_dir = lm.parent().unwrap().to_string_lossy().into_owned();
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-exact-xe-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("main.tex"),
        format!(
            "\\documentclass[12pt]{{article}}\\usepackage{{fontspec}}\\setmainfont{{lmroman10-regular.otf}}[Path={lm_dir}/,BoldFont=lmroman10-bold.otf]\\usepackage[margin=1in]{{geometry}}\\setlength{{\\parindent}}{{0pt}}\\pagestyle{{empty}}\\begin{{document}}Efficient \\textbf{{bold}} text: $\\alpha+\\frac{{a}}{{b}}$.\\end{{document}}\n"
        ),
    )
    .unwrap();
    let status = std::process::Command::new(&tex)
        .current_dir(&dir)
        .args(["-interaction=batchmode", "-halt-on-error", "main.tex"])
        .output()
        .unwrap();
    let reference = dir.join("main.pdf");
    if !status.status.success() || !reference.is_file() {
        let detail = format!(
            "exit {:?}\nstdout:\n{}\nstderr:\n{}\nlog:\n{}",
            status.status.code(),
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr),
            std::fs::read_to_string(dir.join("main.log")).unwrap_or_default()
        );
        common::oracle_failed(TEST, "xelatex did not produce main.pdf", &detail);
    }
    let file = PdfFile::parse(&std::fs::read(&reference).unwrap()).unwrap();
    let doc = compare::reemit(&file).unwrap();
    let cid_fonts: Vec<&CidFont> = doc
        .fonts
        .values()
        .filter_map(|f| match f {
            ExactFont::CidCff(c) => Some(c),
            _ => None,
        })
        .collect();
    assert!(
        cid_fonts.len() >= 2,
        "regular and bold Latin Modern as CIDFontType0C"
    );
    for c in &cid_fonts {
        let program = CffFont::parse(c.program.bytes()).unwrap();
        assert!(
            program.is_cid_keyed(),
            "xdvipdfmx embeds CID-keyed CFF subsets"
        );
        assert!(c.to_unicode_verbatim.is_some() && c.cid_set.is_some());
        assert!(
            c.descendant_base_font.is_some(),
            "Type0 name carries -Identity-H"
        );
    }
    let out = render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    let ours = PdfFile::parse(&out.bytes).unwrap();
    let report = compare::classify(&file, &ours, "xelatex", "exact");
    eprintln!("{}", report.text());
    assert!(report.content_byte_identical.iter().all(|&x| x));
    assert!(
        !report.font_program_identical.is_empty()
            && report.font_program_identical.values().all(|&x| x)
    );
    let allowed = BTreeSet::from([
        Category::ObjectLayout,
        Category::Compression,
        Category::DocumentIdentity,
    ]);
    assert!(report.categories.is_subset(&allowed), "{}", report.text());
    let again = render_exact(&compare::reemit(&ours).unwrap()).unwrap();
    assert_eq!(again.bytes, out.bytes, "fixed point");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Issue #28: identical bytes must not imply operator equality when the
/// stream is outside the bounded set.
#[test]
fn classify_reports_identical_unsupported_streams_as_unsupported_not_identical() {
    let doc = sample_document();
    let a = render_exact(&doc).unwrap();
    // Same byte length, same container: turn the ` rg` in page 1's content
    // into ` gs`, which the exact parser refuses.
    let needle = b" rg\n";
    let at = a
        .bytes
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("page 1 content has an rg operator");
    let mut bytes = a.bytes.clone();
    bytes[at + 1..at + 3].copy_from_slice(b"gs");
    let file = PdfFile::parse(&bytes).unwrap();
    verify::check_structure(&bytes).unwrap();
    let report = compare::classify(&file, &file, "a", "b");
    assert_eq!(
        report.content_byte_identical,
        vec![true, true],
        "bytes really are identical"
    );
    assert_eq!(
        report.content_ops_identical,
        vec![false, true],
        "page 1 is unknown (unsupported), page 2 is parsed-identical"
    );
    assert!(
        report.categories.contains(&Category::ContentUnsupported),
        "{}",
        report.text()
    );
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.contains("outside the bounded operator set")),
        "{}",
        report.text()
    );
    // The unmodified file compared with itself is parsed-identical on both pages.
    let good = PdfFile::parse(&a.bytes).unwrap();
    let report = compare::classify(&good, &good, "a", "b");
    assert_eq!(report.content_ops_identical, vec![true, true]);
    assert!(report.categories.is_empty(), "{}", report.text());
}
