//! Writes a one-page PDF from this crate's embedding output so an external
//! reader (PDFKit via `tools/pdf_extract_check.swift`) can check that text
//! extraction returns the original Unicode and that the page renders.
//!
//!   cargo run --example pdf_roundtrip -- <font path> <size pt> <out.pdf> <text...>
//!
//! This is a test writer, not the product PDF writer: one Type0 font,
//! Identity-H, `/ToUnicode`, `TJ` runs carrying the shaper's kerning as
//! adjustments (offset marks placed with `Tm`), and `/Span << /ActualText >>` around every
//! cluster whose glyph set does not read back through ToUnicode alone
//! (ligatures, composed marks, multi-glyph clusters). It also prints the
//! positions it used, one `gid x y` line per glyph, so a renderer can draw
//! the same glyphs from the same file at the same places.

use std::fmt::Write as _;
use std::path::Path;

use flashtex_font_engine::embed::EmbedPlan;
use flashtex_font_engine::shape::{ShapeOptions, shape};
use flashtex_font_engine::{Face, load_from_path};

fn utf16_hex(s: &str) -> String {
    let mut h = String::from("FEFF");
    for u in s.encode_utf16() {
        let _ = write!(h, "{u:04X}");
    }
    h
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("usage: pdf_roundtrip <font> <size-pt> <out.pdf> <text...>");
        std::process::exit(2);
    }
    let face = load_from_path(Path::new(&args[0])).expect("font");
    let size: f64 = args[1].parse().expect("size");
    let out_path = &args[2];
    let text = args[3..].join(" ");
    let shaped = shape(&face, &text, &ShapeOptions::default()).expect("shape");
    let mut plan = EmbedPlan::new();
    plan.add_shaped(&shaped);
    let pdf = plan.finish(&face).expect("embed");

    // Content stream: `TJ` runs starting at the pen position, with the
    // kerning folded into the shaper's advances expressed as TJ adjustments
    // (thousandths of em), so a reader sees ordinary word spacing. Marks with
    // offsets are placed absolutely and the run then resumes at the pen.
    let (x0, y0) = (72.0f64, 700.0f64);
    let upem = f64::from(face.units_per_em());
    let mut content = String::from("BT\n");
    let _ = writeln!(content, "/F1 {size} Tf");
    let mut pen = 0.0f64;
    let mut positions = String::new();
    let mut run: Vec<String> = Vec::new();
    let mut run_open = false;
    let flush = |content: &mut String, run: &mut Vec<String>, run_open: &mut bool| {
        if !run.is_empty() {
            let _ = writeln!(content, "[{}] TJ", run.join(" "));
            run.clear();
        }
        *run_open = false;
    };
    for cluster in &shaped.clusters {
        let plain = cluster.glyphs.len() == 1
            && pdf
                .to_unicode
                .get(&pdf.cid(cluster.glyphs[0].gid).unwrap())
                .is_some_and(|t| *t == cluster.text);
        if !plain && !cluster.glyphs.is_empty() {
            flush(&mut content, &mut run, &mut run_open);
            let _ = writeln!(
                content,
                "/Span << /ActualText <{}> >> BDC",
                utf16_hex(&cluster.text)
            );
        }
        for g in &cluster.glyphs {
            let cid = pdf.cid(g.gid).expect("cid");
            let gx = x0 + pen + face.to_points(i64::from(g.x_offset), size);
            let gy = y0 + face.to_points(i64::from(g.y_offset), size);
            let _ = writeln!(positions, "{} {gx:.4} {gy:.4}", g.gid.0);
            if g.x_offset != 0 || g.y_offset != 0 {
                flush(&mut content, &mut run, &mut run_open);
                let _ = writeln!(content, "1 0 0 1 {gx:.4} {gy:.4} Tm <{cid:04X}> Tj");
            } else {
                if !run_open {
                    let _ = writeln!(content, "1 0 0 1 {:.4} {y0:.4} Tm", x0 + pen);
                    run_open = true;
                }
                run.push(format!("<{cid:04X}>"));
                let width_units = pdf.width(cid).unwrap_or(0) as f64 / 1000.0 * upem;
                let delta = f64::from(g.advance) - width_units; // kerning, font units
                if delta.abs() > 0.5 {
                    run.push(format!("{:.1}", -delta * 1000.0 / upem));
                }
            }
            pen += face.to_points(i64::from(g.advance), size);
        }
        if !plain && !cluster.glyphs.is_empty() {
            flush(&mut content, &mut run, &mut run_open);
            content.push_str("EMC\n");
        }
    }
    flush(&mut content, &mut run, &mut run_open);
    content.push_str("ET\n");

    let (file_key, subtype) = pdf.font_file.stream_key();
    let mut objects: Vec<Vec<u8>> = Vec::new();
    // 1 catalog, 2 pages, 3 page, 4 content, 5 Type0, 6 CIDFont, 7 descriptor,
    // 8 font file, 9 ToUnicode
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    objects.push(
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
            .to_vec(),
    );
    objects.push(stream(content.as_bytes(), ""));
    objects.push(
        format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /{} /Encoding /Identity-H /DescendantFonts [6 0 R] /ToUnicode 9 0 R >>",
            pdf.base_font
        )
        .into_bytes(),
    );
    objects.push(
        format!(
            "<< /Type /Font /Subtype /{} /BaseFont /{} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 7 0 R /DW 1000 /W {} /CIDToGIDMap /Identity >>",
            pdf.cid_font_subtype,
            pdf.base_font,
            pdf.w_array()
        )
        .into_bytes(),
    );
    let d = &pdf.descriptor;
    objects.push(
        format!(
            "<< /Type /FontDescriptor /FontName /{} /Flags {} /FontBBox [{} {} {} {}] /ItalicAngle {} /Ascent {} /Descent {} /CapHeight {} /StemV {} /{} 8 0 R >>",
            pdf.base_font, d.flags, d.bbox[0], d.bbox[1], d.bbox[2], d.bbox[3], d.italic_angle, d.ascent, d.descent, d.cap_height, d.stem_v, file_key
        )
        .into_bytes(),
    );
    let extra = match subtype {
        Some(s) => format!(" /Subtype /{s}"),
        None => format!(" /Length1 {}", pdf.font_file.bytes().len()),
    };
    objects.push(stream(pdf.font_file.bytes(), &extra));
    objects.push(stream(&pdf.to_unicode_cmap, ""));

    let mut out: Vec<u8> = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, obj) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    std::fs::write(out_path, out).expect("write pdf");
    print!("{positions}");
    eprintln!(
        "wrote {out_path}: {} glyphs, {} clusters, font {} ({file_key}{}), missing {}",
        shaped.glyphs().count(),
        shaped.clusters.len(),
        pdf.base_font,
        subtype.map(|s| format!(" /{s}")).unwrap_or_default(),
        shaped.missing.len()
    );
}

fn stream(data: &[u8], extra_dict: &str) -> Vec<u8> {
    let mut v = format!("<< /Length {}{extra_dict} >>\nstream\n", data.len()).into_bytes();
    v.extend_from_slice(data);
    v.extend_from_slice(b"\nendstream");
    v
}
