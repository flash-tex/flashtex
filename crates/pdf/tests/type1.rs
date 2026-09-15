//! Type 1 subsetting (`flashtex_pdf::type1`): identity of retained
//! charstrings, blanked subroutines, `Length1/2/3`, and (oracle only) the
//! apples-to-apples comparison with pdfTeX's own Latin Modern Type 1 subset.
//! The oracle and `lmr12.pfb` are located by `tests/common/mod.rs` --
//! `$FLASHTEX_PDFLATEX` / `PATH` / MacTeX for the binary, `kpsewhich` for the
//! font -- and their absence is announced, never silently swallowed.

mod common;

use flashtex_pdf::compare;
use flashtex_pdf::exact::{
    self, Content, Decimal, ExactDocument, ExactFont, ExactPage, FontProgram,
};
use flashtex_pdf::reader::PdfFile;
use flashtex_pdf::type1::{Type1Font, builtin_encoding, decrypt, encrypt};
use flashtex_pdf::verify;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A tiny Type 1 program in PFB form: three glyphs, four subroutines
/// (Subrs 0–3 as the Flex/hint-replacement convention, plus one used only
/// by /b through hint replacement and one unused), a seac composite.
fn synthetic_pfb() -> Vec<u8> {
    fn cs(plain: &[u8]) -> Vec<u8> {
        encrypt(plain, 4330, &[0, 0, 0, 0])
    }
    // Type 1 charstring numbers: v+139 for -107..=107.
    let n = |v: i32| (v + 139) as u8;
    let notdef = cs(&[n(0), n(100), 13, 14]); // 0 100 hsbw endchar
    // a: 0 500 hsbw 10 10 rmoveto 4 callsubr closepath endchar
    let a = cs(&[n(0), n(100), 13, n(10), n(10), 21, n(4), 10, 9, 14]);
    // b: 0 600 hsbw  5 1 3 callothersubr pop callsubr  endchar (hint replacement via Subrs 5)
    let b = cs(&[n(0), n(105), 13, n(5), n(1), n(3), 12, 16, 12, 17, 10, 14]);
    // c: seac composing a (code 97) with grave (code 193):
    // 0 100 hsbw 0 0 0 97 193 seac (193 in the 32-bit form).
    let mut c_plain = vec![n(0), n(100), 13, n(0), n(0), n(0), n(97)];
    c_plain.extend_from_slice(&[255, 0, 0, 0, 193]); // 32-bit 193
    c_plain.extend_from_slice(&[12, 6]);
    let c = cs(&c_plain);
    let grave = cs(&[n(0), n(100), 13, n(1), n(1), 21, 9, 14]);
    let subrs: Vec<Vec<u8>> = vec![
        cs(&[n(3), n(0), 12, 16, 12, 17, 12, 17, 12, 17, 11]), // Subrs 0 (flex convention)
        cs(&[n(0), n(1), 12, 16, 11]),
        cs(&[n(0), n(2), 12, 16, 11]),
        cs(&[n(3), n(0), 12, 16, 12, 17, 11]),
        cs(&[n(50), n(0), 5, n(0), n(50), 5, 11]), // Subrs 4: rlineto rlineto return
        cs(&[n(1), n(2), 1, 11]),                  // Subrs 5: hstem return (hint replacement)
        cs(&[n(9), n(9), 5, 11]),                  // Subrs 6: unused
    ];
    let mut private = Vec::new();
    private.extend_from_slice(b"dup /Private 8 dict dup begin\n/RD{string currentfile exch readstring pop}executeonly def\n/ND{noaccess def}executeonly def\n/NP{noaccess put}executeonly def\n/StdVW[70]def\n/password 5839 def\n/Subrs 7 array\n");
    for (i, s) in subrs.iter().enumerate() {
        private.extend_from_slice(format!("dup {i} {} RD ", s.len()).as_bytes());
        private.extend_from_slice(s);
        private.extend_from_slice(b" NP\n");
    }
    private.extend_from_slice(b"ND\n2 index\n/CharStrings 5 dict dup begin\n");
    for (name, g) in [
        (".notdef", &notdef),
        ("a", &a),
        ("b", &b),
        ("c", &c),
        ("grave", &grave),
    ] {
        private.extend_from_slice(format!("/{name} {} RD ", g.len()).as_bytes());
        private.extend_from_slice(g);
        private.extend_from_slice(b" ND\n");
    }
    private.extend_from_slice(b"end end readonly put put\ndup/FontName get exch definefont pop\nmark currentfile closefile\n");
    let binary = encrypt(&private, 55665, &[0, 0, 0, 0]);
    let clear = b"%!PS-AdobeFont-1.0: Synth 001.000\n/FontName /Synth def\n/FontMatrix[0.001 0 0 0.001 0 0]readonly def\n/FontBBox{-10 -20 600 700}readonly def\n/ItalicAngle 0 def\n/Encoding 256 array\n0 1 255 {1 index exch /.notdef put} for\ndup 97/a put\ndup 98 /b put\ndup 99/c put\nreadonly def\ncurrentdict end\ncurrentfile eexec\n".to_vec();
    let trailer: Vec<u8> = b"0000000000000000\ncleartomark\n".to_vec();
    let mut pfb = Vec::new();
    for (kind, seg) in [(1u8, &clear), (2, &binary), (1, &trailer)] {
        pfb.extend_from_slice(&[0x80, kind]);
        pfb.extend_from_slice(&(seg.len() as u32).to_le_bytes());
        pfb.extend_from_slice(seg);
    }
    pfb.extend_from_slice(&[0x80, 3]);
    pfb
}

#[test]
fn subset_keeps_retained_charstrings_and_needed_subrs_identically() {
    let pfb = synthetic_pfb();
    let font = Type1Font::parse_pfb(&pfb).unwrap();
    assert_eq!(font.font_name(), Some("Synth"));
    assert_eq!(font.font_bbox(), Some([-10, -20, 600, 700]));
    assert_eq!(font.std_vw(), Some(70));
    assert_eq!(font.subr_count(), 7);
    assert_eq!(font.glyph_names().count(), 5);
    assert_eq!(font.advance_width("b").unwrap(), exact::Ratio::int(105));
    let enc = builtin_encoding(&pfb[6..]);
    assert_eq!(enc.get(&97).map(String::as_str), Some("a"));
    assert_eq!(enc.get(&98).map(String::as_str), Some("b"));

    // /c composes /a and /grave through seac; /b reaches Subrs 5 through
    // hint replacement; Subrs 0-3 are always kept; Subrs 6 never.
    let (glyphs, subrs) = font.closure(&BTreeSet::from(["c".to_string()])).unwrap();
    assert_eq!(
        glyphs,
        BTreeSet::from([".notdef".into(), "a".into(), "c".into(), "grave".into()])
    );
    assert_eq!(subrs, BTreeSet::from([0, 1, 2, 3, 4]));
    let (_, subrs_b) = font.closure(&BTreeSet::from(["b".to_string()])).unwrap();
    assert_eq!(subrs_b, BTreeSet::from([0, 1, 2, 3, 5]));

    let sub = font.subset(&BTreeSet::from(["c".to_string()])).unwrap();
    assert_eq!(sub.glyphs, vec![".notdef", "a", "c", "grave"]);
    assert_eq!((sub.subrs_retained, sub.subrs_total), (5, 7));
    let FontProgram::Type1 {
        bytes,
        length1,
        length2,
        length3,
    } = &sub.program
    else {
        panic!()
    };
    assert_eq!(*length1, pfb_clear_len(&pfb), "clear text verbatim");
    assert_eq!(*length3, 0, "no trailer, as pdfTeX writes");
    assert_eq!(bytes.len(), length1 + length2);
    let again = Type1Font::parse_program(bytes, *length1, *length2, *length3).unwrap();
    for g in &sub.glyphs {
        assert_eq!(
            again.decrypted_charstring(g),
            font.decrypted_charstring(g),
            "/{g}"
        );
        assert_eq!(
            again.encrypted_charstring(g),
            font.encrypted_charstring(g),
            "/{g} encrypted bytes verbatim"
        );
    }
    assert_eq!(again.glyph_names().count(), 4);
    for i in 0..7 {
        if subrs.contains(&i) {
            assert_eq!(
                again.decrypted_subr(i),
                font.decrypted_subr(i),
                "Subrs[{i}]"
            );
        } else {
            assert_eq!(
                again.decrypted_subr(i),
                Some(vec![11]),
                "Subrs[{i}] blanked to return"
            );
        }
    }
    // Deterministic, and a different glyph set is a different program.
    assert_eq!(
        font.subset(&BTreeSet::from(["c".to_string()])).unwrap(),
        sub
    );
    assert_ne!(
        font.subset(&BTreeSet::from(["b".to_string()]))
            .unwrap()
            .program,
        sub.program
    );
    // Missing glyphs are errors.
    assert!(font.subset(&BTreeSet::from(["zz".to_string()])).is_err());
    // The four eexec prefix bytes are zero and the decryption round-trips.
    assert_eq!(&decrypt(&bytes[*length1..], 55665, 0)[..4], &[0, 0, 0, 0]);
}

fn pfb_clear_len(pfb: &[u8]) -> usize {
    u32::from_le_bytes([pfb[2], pfb[3], pfb[4], pfb[5]]) as usize
}

#[test]
fn type1_subset_embeds_as_a_simple_font_and_reads_back() {
    let pfb = synthetic_pfb();
    let font = Type1Font::parse_pfb(&pfb).unwrap();
    let encoding = vec![(99u8, "c".to_string()), (97, "a".to_string())];
    let (exact_font, subset) = ExactFont::type1_subset(&font, &encoding, &|name| {
        let w = font.advance_width(name).ok()?;
        Decimal::from_ratio(w.num, w.den as u128, 6)
    })
    .unwrap();
    let ExactFont::Simple(s) = &exact_font else {
        panic!()
    };
    assert_eq!(s.first_char, 97);
    assert_eq!(s.widths.len(), 3, "codes 97..=99");
    assert_eq!(s.widths[1].as_str(), "0", "unencoded code 98 has width 0");
    assert!(s.base_font.ends_with("+Synth"));
    assert_eq!(
        s.descriptor.as_ref().unwrap().char_set.as_deref(),
        Some("/.notdef/a/c/grave")
    );
    assert_eq!(subset.glyphs, vec![".notdef", "a", "c", "grave"]);
    let doc = ExactDocument {
        images: Default::default(),
        pages: vec![ExactPage {
            width: Decimal::from_i64(100),
            height: Decimal::from_i64(100),
            content: Content::Verbatim(b"BT /F1 10 Tf 10 50 Td (ac) Tj ET".to_vec()),
            fonts: None,
        }],
        fonts: BTreeMap::from([("F1".to_string(), exact_font.clone())]),
    };
    let out = exact::render_exact(&doc).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    let file = PdfFile::parse(&out.bytes).unwrap();
    let back = compare::reemit(&file).unwrap();
    assert_eq!(
        back.fonts["F1"], exact_font,
        "everything read back verbatim"
    );
    let text = String::from_utf8_lossy(&out.bytes);
    assert!(text.contains("/Length1 "), "{text}");
    assert!(text.contains("/Differences [ 97 /a 99 /c ]"), "{text}");
}

fn lm_type1(name: &str) -> Option<PathBuf> {
    common::lm_type1(name)
}

fn pdflatex() -> Option<PathBuf> {
    common::pdflatex()
}

/// Oracle only: pdfTeX's Latin Modern subset and this crate's subset of the
/// same `lmr12.pfb` for the same glyphs must decrypt to the same charstrings.
#[test]
fn latin_modern_subset_matches_pdftex_charstring_for_charstring() {
    const TEST: &str = "latin_modern_subset_matches_pdftex_charstring_for_charstring";
    let (Some(tex), Some(pfb_path)) = (pdflatex(), lm_type1("lmr12.pfb")) else {
        common::skip(
            TEST,
            "no pdflatex found, or lmr12.pfb is not resolvable via \
             FLASHTEX_LM_TYPE1_DIR / kpsewhich / the fixed TeX Live directories",
        );
        return;
    };
    common::announce(&format!(
        "ORACLE {TEST}: pdflatex = {}, lmr12.pfb = {}",
        tex.display(),
        pfb_path.display()
    ));
    let dir = std::env::temp_dir().join(format!("flashtex-pdf-type1-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("main.tex"),
        "\\documentclass[12pt]{article}\\usepackage[T1]{fontenc}\\usepackage{lmodern}\\usepackage[margin=1in]{geometry}\\setlength{\\parindent}{0pt}\\pagestyle{empty}\\begin{document}Hello world. Efficient caf\\'e na\\\"ive fjord.\\end{document}\n",
    )
    .unwrap();
    let status = std::process::Command::new(&tex)
        .current_dir(&dir)
        .args(["-interaction=batchmode", "-halt-on-error", "main.tex"])
        .output()
        .unwrap();
    if !status.status.success() || !dir.join("main.pdf").is_file() {
        let detail = format!(
            "exit {:?}\nstdout:\n{}\nstderr:\n{}\nlog:\n{}",
            status.status.code(),
            String::from_utf8_lossy(&status.stdout),
            String::from_utf8_lossy(&status.stderr),
            std::fs::read_to_string(dir.join("main.log")).unwrap_or_default()
        );
        common::oracle_failed(TEST, "pdflatex did not produce main.pdf", &detail);
    }
    let reference = PdfFile::parse(&std::fs::read(dir.join("main.pdf")).unwrap()).unwrap();
    let doc = compare::reemit(&reference).unwrap();
    // pdfTeX's font: its subset program, encoding and widths.
    let (res, theirs) = doc
        .fonts
        .iter()
        .find_map(|(k, f)| match f {
            ExactFont::Simple(s) if s.base_font.ends_with("LMRoman12-Regular") => {
                Some((k.clone(), s.clone()))
            }
            _ => None,
        })
        .expect("pdfTeX embedded LMRoman12-Regular");
    let Some(FontProgram::Type1 {
        bytes,
        length1,
        length2,
        length3,
    }) = &theirs.program
    else {
        panic!("pdfTeX embeds Type 1")
    };
    let pdftex_font = Type1Font::parse_program(bytes, *length1, *length2, *length3).unwrap();
    let Some(exact::Encoding::Differences { differences, .. }) = &theirs.encoding else {
        panic!("pdfTeX writes /Differences")
    };
    // Ours, from the installed lmr12.pfb, for exactly pdfTeX's codes and
    // with pdfTeX's widths (TFM-derived) so only the program differs.
    let ours_font = Type1Font::parse_pfb(&std::fs::read(&pfb_path).unwrap()).unwrap();
    let width_of = |name: &str| -> Option<Decimal> {
        let (code, _) = differences.iter().find(|(_, n)| n == name)?;
        theirs
            .widths
            .get((*code - theirs.first_char) as usize)
            .cloned()
    };
    let (ours, subset) = ExactFont::type1_subset(&ours_font, differences, &width_of).unwrap();
    eprintln!(
        "pdfTeX program {} bytes ({} glyphs); ours {} bytes ({} glyphs, {}/{} subrs)",
        bytes.len(),
        pdftex_font.glyph_names().count(),
        subset.program.bytes().len(),
        subset.glyphs.len(),
        subset.subrs_retained,
        subset.subrs_total
    );
    // Every glyph pdfTeX embedded decrypts to the same bytes in our subset.
    let ours_parsed = {
        let FontProgram::Type1 {
            bytes,
            length1,
            length2,
            length3,
        } = &subset.program
        else {
            panic!()
        };
        Type1Font::parse_program(bytes, *length1, *length2, *length3).unwrap()
    };
    let mut compared = 0;
    for name in pdftex_font.glyph_names() {
        let theirs_cs = pdftex_font.decrypted_charstring(name).unwrap();
        let ours_cs = ours_parsed
            .decrypted_charstring(name)
            .unwrap_or_else(|| panic!("pdfTeX embeds /{name}, our subset lacks it"));
        assert_eq!(theirs_cs, ours_cs, "/{name}");
        compared += 1;
    }
    assert!(compared >= 15, "compared {compared} glyphs");
    // And the subroutines the common glyphs reach are the same bytes.
    let names: BTreeSet<String> = pdftex_font.glyph_names().map(String::from).collect();
    let (_, subrs_theirs) = pdftex_font.closure(&names).unwrap();
    let (_, subrs_ours) = ours_parsed.closure(&names).unwrap();
    assert_eq!(subrs_theirs, subrs_ours);
    for i in subrs_theirs {
        assert_eq!(
            pdftex_font.decrypted_subr(i),
            ours_parsed.decrypted_subr(i),
            "Subrs[{i}]"
        );
    }
    // Swap our font into the re-emitted document and classify against pdfTeX:
    // bytes differ (FontProgram) but the classifier reports the charstrings
    // identical, which is the apples-to-apples verdict.
    let mut swapped = doc.clone();
    swapped.fonts.insert(res.clone(), ours);
    let out = exact::render_exact(&swapped).unwrap();
    verify::check_structure(&out.bytes).unwrap();
    let report = compare::classify(
        &reference,
        &PdfFile::parse(&out.bytes).unwrap(),
        "pdftex",
        "ours",
    );
    eprintln!("{}", report.text());
    assert!(report.content_byte_identical.iter().all(|&x| x));
    let key = report
        .font_charstrings_identical
        .keys()
        .find(|k| k.contains(&res))
        .cloned()
        .expect("Type 1 charstrings were compared");
    assert!(report.font_charstrings_identical[&key], "{}", report.text());
    assert!(
        report
            .lines
            .iter()
            .any(|l| l.starts_with("[same]") && l.contains("Type 1 charstrings")),
        "{}",
        report.text()
    );
    // Raster: our program draws the same ink as pdfTeX's for these glyphs.
    if Path::new("/usr/bin/sips").exists() {
        let mine = dir.join("ours.pdf");
        std::fs::write(&mine, &out.bytes).unwrap();
        for (src, png) in [
            (dir.join("main.pdf"), "ref.png"),
            (mine.clone(), "ours.png"),
        ] {
            let st = std::process::Command::new("/usr/bin/sips")
                .args(["-s", "format", "png", "--resampleWidth", "1224"])
                .arg(&src)
                .arg("--out")
                .arg(dir.join(png))
                .output()
                .unwrap();
            assert!(st.status.success());
        }
        let a = std::fs::read(dir.join("ref.png")).unwrap();
        let b = std::fs::read(dir.join("ours.png")).unwrap();
        assert!(
            (a.len() as i64 - b.len() as i64).abs() < 64,
            "PNG sizes {} vs {}: rasters should be near-identical",
            a.len(),
            b.len()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}
