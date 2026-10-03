//! Malformed TrueType fonts (writettf.c, src/pdftex/writettf.rs) end in a
//! TeX fatal error, never a panic (DESIGN.md §4.5). Found by the T6
//! TrueType/OpenType fuzzer (tools/fuzz/parsers/ttf.py, README-ttf.md).
//!
//! Each case is a font `fuzz` (cmr10's TFM under another name) mapped to a
//! small TrueType font built here, `fuzz.ttf`, subset through `8r.enc`. A
//! control case, the font as built, must embed cleanly, so the malformed
//! cases test the one field they change. Where pdfTeX itself crashes or gets
//! to its error only through undefined behaviour, the case names the error
//! wanted. Skips where there is no TeX Live, unless
//! `FLASHTEX_REQUIRE_TEXLIVE=1`.
#![cfg(feature = "kpathsea")]

mod common;

use flashtex_engine::resolver::find_texlive_bin;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The document: -ini, PDF output, `H` in the fuzz font, subset by name.
const DOC: &str = "\\catcode`\\{=1 \\catcode`\\}=2 \\pdfoutput=1\n\
    \\pdfmapline{+fuzz FuzzFont <8r.enc <fuzz.ttf}\n\
    \\font\\x=fuzz \\setbox0\\hbox{\\x H}\\shipout\\box0 \\end\n";

/// A simple glyph: one closed rectangle.
fn rect_glyph(x0: i16, y0: i16, x1: i16, y1: i16) -> Vec<u8> {
    let mut v = Vec::new();
    for x in [1i16, x0, y0, x1, y1] {
        v.extend_from_slice(&x.to_be_bytes()); // contours, bbox
    }
    v.extend_from_slice(&3u16.to_be_bytes()); // endPtsOfContours[0]
    v.extend_from_slice(&0u16.to_be_bytes()); // instructionLength
    v.extend_from_slice(&[1; 4]); // flags: on curve, 16-bit deltas
    let pts = [(x0, y0), (x0, y1), (x1, y1), (x1, y0)];
    let (mut px, mut py) = (0i16, 0i16);
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for (x, y) in pts {
        xs.extend_from_slice(&(x - px).to_be_bytes());
        ys.extend_from_slice(&(y - py).to_be_bytes());
        (px, py) = (x, y);
    }
    v.extend(xs);
    v.extend(ys);
    v
}

fn be16(v: &mut Vec<u8>, xs: &[u16]) {
    for x in xs {
        v.extend_from_slice(&x.to_be_bytes());
    }
}

fn be32(v: &mut Vec<u8>, xs: &[u32]) {
    for x in xs {
        v.extend_from_slice(&x.to_be_bytes());
    }
}

/// A TrueType font of three glyphs (`.notdef`, `.null`, `H`) whose `maxp`
/// says `num_glyphs`. Glyph names come from `post` format 2 with standard
/// Macintosh indices only (0, 1, 43); `cmap` format 4 maps U+0048 to `H`.
/// Checksums are zero: pdfTeX recomputes them and never checks the input's.
fn font(num_glyphs: u16) -> Vec<u8> {
    let glyphs = [
        rect_glyph(50, 0, 450, 700),
        vec![],
        rect_glyph(60, 0, 540, 700),
    ];
    let mut glyf = Vec::new();
    let mut loca = Vec::new();
    for g in &glyphs {
        be32(&mut loca, &[glyf.len() as u32]);
        glyf.extend_from_slice(g);
        glyf.resize(glyf.len().next_multiple_of(4), 0);
    }
    be32(&mut loca, &[glyf.len() as u32]);

    let mut head = Vec::new();
    be32(&mut head, &[0x0001_0000, 0x0001_0000, 0, 0x5F0F_3CF5]);
    be16(&mut head, &[0x000B, 1000]);
    head.extend_from_slice(&[0; 16]); // created, modified
    for x in [0i16, -200, 1000, 800] {
        head.extend_from_slice(&x.to_be_bytes()); // bbox
    }
    be16(&mut head, &[0, 8, 2, 1, 0]); // macStyle .. indexToLocFormat (long), glyphDataFormat

    let mut hhea = Vec::new();
    be32(&mut hhea, &[0x0001_0000]);
    be16(&mut hhea, &[800, (-200i16) as u16, 0, 1000, 0, 0, 900, 1]);
    hhea.extend_from_slice(&[0; 14]); // caret run/offset, reserved, metricDataFormat
    be16(&mut hhea, &[3]); // numberOfHMetrics
    assert_eq!(hhea.len(), 36);

    let mut hmtx = Vec::new();
    be16(&mut hmtx, &[600, 50, 0, 0, 600, 60]);

    let mut maxp = Vec::new();
    be32(&mut maxp, &[0x0001_0000]);
    be16(
        &mut maxp,
        &[num_glyphs, 4, 1, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0],
    );

    let mut post = Vec::new();
    be32(&mut post, &[0x0002_0000, 0]);
    be16(&mut post, &[(-100i16) as u16, 50]);
    be32(&mut post, &[0, 0, 0, 0, 0]);
    be16(&mut post, &[3, 0, 1, 43]); // .notdef, .null, H

    // cmap: one (3,1) format 4 subtable: 'H' -> glyph 2, then 0xFFFF.
    let mut sub = Vec::new();
    be16(&mut sub, &[4, 0, 0, 4, 0, 0, 0]); // format, length, lang, segCountX2..
    be16(&mut sub, &[0x48, 0xFFFF, 0, 0x48, 0xFFFF]); // ends, pad, starts
    be16(&mut sub, &[(2i16 - 0x48) as u16, 1, 0, 0]); // deltas, rangeOffsets
    let len = sub.len() as u16;
    sub[2..4].copy_from_slice(&len.to_be_bytes());
    let mut cmap = Vec::new();
    be16(&mut cmap, &[0, 1, 3, 1]);
    be32(&mut cmap, &[12]);
    cmap.extend(sub);

    let family: Vec<u8> = "FuzzFont"
        .encode_utf16()
        .flat_map(u16::to_be_bytes)
        .collect();
    let mut name = Vec::new();
    be16(&mut name, &[0, 2, 6 + 24]);
    be16(&mut name, &[3, 1, 0x409, 1, family.len() as u16, 0]);
    be16(&mut name, &[3, 1, 0x409, 6, family.len() as u16, 0]);
    name.extend_from_slice(&family);

    let mut os2 = vec![0u8; 96];
    os2[1] = 3; // version 3
    os2[58..62].copy_from_slice(b"FUZZ");

    let mut tables: Vec<(&[u8; 4], Vec<u8>)> = vec![
        (b"OS/2", os2),
        (b"cmap", cmap),
        (b"glyf", glyf),
        (b"head", head),
        (b"hhea", hhea),
        (b"hmtx", hmtx),
        (b"loca", loca),
        (b"maxp", maxp),
        (b"name", name),
        (b"post", post),
    ];
    tables.sort_by_key(|t| *t.0);
    let n = tables.len() as u16;
    let mut out = Vec::new();
    be32(&mut out, &[0x0001_0000]);
    be16(&mut out, &[n, 128, 3, n * 16 - 128]);
    let mut offset = 12 + 16 * tables.len();
    let mut body = Vec::new();
    for (tag, data) in &tables {
        out.extend_from_slice(*tag);
        be32(&mut out, &[0, offset as u32, data.len() as u32]);
        body.extend_from_slice(data);
        body.resize(body.len().next_multiple_of(4), 0);
        offset = 12 + 16 * tables.len() + body.len();
    }
    out.extend(body);
    out
}

struct Run {
    code: Option<i32>,
    log: String,
}

fn run(bin: &Path, dir: &Path) -> Run {
    let st = Command::new(bin)
        .args(["-ini", "-interaction=nonstopmode", "doc.tex"])
        .current_dir(dir)
        .env("TEXMFVAR", dir.join("texmf-var"))
        .env("SOURCE_DATE_EPOCH", "0")
        .env("FORCE_SOURCE_DATE", "1")
        .env(
            "FLASHTEX_POOL",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("pdftex.pool"),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let mut log = std::fs::read_to_string(dir.join("doc.log")).unwrap_or_default();
    log.push_str(&String::from_utf8_lossy(&st.stderr));
    Run {
        code: st.status.code(),
        log,
    }
}

/// The text of the `!pdfTeX error:` after `(file fuzz.ttf): `, lines joined.
fn pdftex_error(log: &str) -> Option<String> {
    let text: String = log.lines().collect();
    let i = text.find("!pdfTeX error: ")?;
    let rest = &text[i..];
    let rest = &rest[rest.find("): ").map_or(0, |j| j + 3)..];
    Some(rest[..rest.find(" ==> Fatal").unwrap_or(rest.len())].to_string())
}

fn kpsewhich(texbin: &Path, name: &str) -> Option<PathBuf> {
    Command::new(texbin.join("kpsewhich"))
        .arg(name)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
}

fn setup(base: &Path, name: &str, tfm: &Path, enc: &Path, ttf: &[u8]) -> PathBuf {
    let d = base.join(name);
    std::fs::create_dir_all(d.join("texmf-var")).unwrap();
    std::fs::copy(tfm, d.join("fuzz.tfm")).unwrap();
    std::fs::copy(enc, d.join("8r.enc")).unwrap();
    std::fs::write(d.join("doc.tex"), DOC).unwrap();
    std::fs::write(d.join("fuzz.ttf"), ttf).unwrap();
    d
}

#[test]
fn malformed_truetype_fonts_end_in_a_tex_error() {
    let Some(texbin) = find_texlive_bin() else {
        common::no_texlive();
        return;
    };
    let (Some(tfm), Some(enc)) = (
        kpsewhich(&texbin, "cmr10.tfm"),
        kpsewhich(&texbin, "8r.enc"),
    ) else {
        common::no_texlive();
        return;
    };
    let ours = Path::new(env!("CARGO_BIN_EXE_flashtex-initex"));
    let base = common::fresh_dir("flashtex-ttf");

    // The font as built embeds cleanly, and so does one that declares just
    // the two glyphs every subset starts with: the cases below differ from
    // these in maxp.numGlyphs alone.
    for n in [3, 2] {
        let d = setup(&base, &format!("control-{n}"), &tfm, &enc, &font(n));
        let r = run(ours, &d);
        assert_eq!(
            r.code,
            Some(0),
            "control font (numGlyphs {n}) did not embed: {:?}",
            pdftex_error(&r.log)
        );
        assert!(!r.log.contains("panicked"), "{}", r.log);
    }

    // maxp.numGlyphs below 2: every subset starts with .notdef and .null,
    // and C reads glyph_tab[id + 1], past the table, for both (undefined
    // behaviour). pdfTeX 1.40.29 ends in "unexpected EOF" with no PDF
    // (TeX Live 2023's crashes), so the wording is not compared: what must
    // hold is exit 1 with a pdfTeX error about this font, no panic and no
    // PDF. Before, 0 panicked in write_glyf and 1 wrote a PDF with a
    // broken subset.
    let mut failures = vec![];
    for n in [0u16, 1] {
        let name = format!("num-glyphs-{n}");
        let d = setup(&base, &name, &tfm, &enc, &font(n));
        let r = run(ours, &d);
        let got = pdftex_error(&r.log);
        if r.log.contains("panicked") || r.code != Some(1) || got.is_none() {
            failures.push(format!(
                "{name}: exit {:?}, error {got:?} (want exit 1 and a pdfTeX error)",
                r.code
            ));
        } else if !r
            .log
            .lines()
            .collect::<String>()
            .contains("(file fuzz.ttf)")
        {
            failures.push(format!("{name}: the error does not name fuzz.ttf: {got:?}"));
        } else if d.join("doc.pdf").exists() {
            failures.push(format!("{name}: a PDF was written"));
        }
    }
    let _ = std::fs::remove_dir_all(&base);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
