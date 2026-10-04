//! Property-based fuzzing of what the host reads from typst-pdf's output:
//! the PDF reader and the content-stream interpreter (`pdf.rs`,
//! `pdfpos.rs`), the image decoders and the bounded inflate. Whatever the
//! bytes, they return a value or an error -- never a panic, an unbounded
//! allocation or an overflow.
//!
//! Dependency-free on purpose: proptest's random-number stack pulls
//! `r-efi`, whose licence expression lists LGPL, which the Typst host's
//! licence boundary refuses (scripts/check-license-boundary.sh, D3). The
//! generator is a seeded xorshift, so every failure is reproducible from
//! the case number it prints; `FLASHTEX_FUZZ_CASES` raises the count.

mod common;

use common::*;
use flashtex_typst_host::pdf::inflate_limited;
use flashtex_typst_host::pdfpos::{derive_pdf, ImageEncoding, PdfImage};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
    /// Up to `max - 1` random bytes.
    fn some_bytes(&mut self, max: usize) -> Vec<u8> {
        let n = self.below(max);
        self.bytes(n)
    }
}

fn cases(default: usize) -> usize {
    std::env::var("FLASHTEX_FUZZ_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Run `f` on `n` seeded cases; a panic names the case.
fn each(n: usize, seed: u64, f: impl Fn(usize, &mut Rng) + std::panic::RefUnwindSafe) {
    for i in 0..n {
        let r = std::panic::catch_unwind(|| {
            let mut rng = Rng(seed ^ (i as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
            f(i, &mut rng)
        });
        assert!(r.is_ok(), "case {i} (seed {seed:#x}) panicked");
    }
}

/// A real typst-pdf export to mutate: text, shapes, colours, alpha, a
/// spot colour, stroked text, an image.
fn sample_pdf() -> Vec<u8> {
    use flashtex_typst_host::world::{FontOptions, Fonts, HostWorld};
    let root = project(
        "fuzz-sample",
        "#set page(width: 120pt, height: 120pt)\n\
         #text(fill: rgb(255, 0, 0, 128))[Alpha] #text(stroke: 0.3pt + blue)[Stroke]\n\
         #rect(width: 20pt, height: 10pt, fill: color.spot(\"S\", cmyk(0%, 50%, 0%, 0%)).tint(60%))\n\
         #circle(radius: 5pt, fill: gradient.linear(red, blue))\n\
         #image(bytes((137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 0, 0, 0, 0, 58, 126, 155, 85, 0, 0, 0, 10, 73, 68, 65, 84, 120, 156, 99, 96, 0, 0, 0, 2, 0, 1, 72, 175, 164, 113, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130)), width: 10pt)\n",
    );
    let fonts = Fonts::load(&FontOptions {
        paths: vec![font_dir().to_path_buf()],
        system: false,
    });
    let world = HostWorld::new(&root, "main.typ", &fonts).unwrap();
    let doc = typst::compile::<typst_layout::PagedDocument>(&world)
        .output
        .unwrap_or_else(|e| panic!("{:?}", e.first().map(|d| &d.message)));
    typst_pdf::pdf(&doc, &typst_pdf::PdfOptions::default()).unwrap()
}

/// Byte-level mutations of a real export: flips, overwrites, insertions,
/// deletions, truncation. The reader and the interpreter must not panic.
#[test]
fn mutated_pdfs_never_panic() {
    let base = sample_pdf();
    assert!(derive_pdf(&base).is_ok());
    each(cases(2000), 0x5eed_0001, |_, rng| {
        let mut b = base.clone();
        for _ in 0..1 + rng.below(8) {
            if b.is_empty() {
                break;
            }
            let at = rng.below(b.len());
            match rng.below(5) {
                0 => b[at] ^= 1 << rng.below(8),
                1 => b[at] = rng.next() as u8,
                2 => {
                    let ins = {
                        let n = 1 + rng.below(16);
                        rng.bytes(n)
                    };
                    b.splice(at..at, ins);
                }
                3 => {
                    let end = (at + 1 + rng.below(32)).min(b.len());
                    b.drain(at..end);
                }
                _ => b.truncate(at.max(1)),
            }
        }
        let _ = derive_pdf(&b);
    });
}

/// A one-page PDF around a content stream (uncompressed), with fonts,
/// colour spaces, an ExtGState and an image the operators may name.
fn wrap(content: &[u8]) -> Vec<u8> {
    let objs: Vec<Vec<u8>> = vec![
        b"<</Type/Catalog/Pages 2 0 R>>".to_vec(),
        b"<</Type/Pages/Kids[3 0 R]/Count 1>>".to_vec(),
        b"<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]/Contents 4 0 R/Resources<</Font<</F0 5 0 R/F1 6 0 R>>/ColorSpace<</C0[/ICCBased 8 0 R]/C1[/Separation/S/DeviceGray 9 0 R]>>/ExtGState<</G0<</ca 0.5/BM/Multiply>>>>/XObject<</X0 10 0 R>>>>>>".to_vec(),
        [format!("<</Length {}>>\nstream\n", content.len()).into_bytes(), content.to_vec(), b"\nendstream".to_vec()].concat(),
        b"<</Type/Font/Subtype/Type0/Encoding/Identity-H/DescendantFonts[7 0 R]>>".to_vec(),
        b"<</Type/Font/Subtype/Type3/FirstChar 32/Widths[250 300 400]/FontMatrix[0.001 0 0 0.001 0 0]>>".to_vec(),
        b"<</Type/Font/Subtype/CIDFontType0/DW 1000/W[1[500 600]3 9 700]>>".to_vec(),
        b"<</N 3/Length 4>>\nstream\nABCD\nendstream".to_vec(),
        b"<</FunctionType 2/Domain[0 1]/C0[0]/C1[1]/N 1>>".to_vec(),
        b"<</Type/XObject/Subtype/Image/Width 2/Height 2/BitsPerComponent 8/ColorSpace/DeviceGray/Length 4>>\nstream\nabcd\nendstream".to_vec(),
    ];
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offs = vec![];
    for (i, o) in objs.iter().enumerate() {
        offs.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).into_bytes());
        out.extend(o);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).into_bytes());
    for o in offs {
        out.extend(format!("{o:010} 00000 n \n").into_bytes());
    }
    out.extend(
        format!(
            "trailer\n<</Size {}/Root 1 0 R>>\nstartxref\n{xref}\n%%EOF\n",
            objs.len() + 1
        )
        .into_bytes(),
    );
    out
}

/// Random operator sequences over the interpreter's whole vocabulary,
/// with random operands of every kind.
#[test]
fn random_content_streams_never_panic() {
    const OPS: &[&str] = &[
        "q", "Q", "cm", "BT", "ET", "Tf", "Tc", "Tw", "Tz", "TL", "Ts", "Tr", "Tm", "Td", "TD",
        "T*", "Tj", "TJ", "'", "\"", "cs", "CS", "sc", "scn", "SC", "SCN", "g", "G", "rg", "RG",
        "k", "K", "gs", "w", "J", "j", "M", "d", "m", "l", "c", "v", "y", "h", "re", "W", "W*",
        "f", "F", "f*", "S", "s", "B", "B*", "b", "b*", "n", "Do", "BI", "BDC", "EMC", "sh",
    ];
    // The wrapper itself is a PDF the interpreter reads.
    let ok = derive_pdf(&wrap(
        b"q 1 0 0 1 5 5 cm /C0 cs 0.1 0.2 0.3 scn 0 0 5 5 re f /X0 Do Q BT /F1 10 Tf ( !) Tj ET",
    ))
    .expect("the wrapper parses");
    assert_eq!(
        (ok[0].paths.len(), ok[0].images.len(), ok[0].glyphs.len()),
        (1, 1, 2)
    );
    const NAMES: &[&str] = &[
        "/F0",
        "/F1",
        "/C0",
        "/C1",
        "/G0",
        "/X0",
        "/Pattern",
        "/DeviceRGB",
        "/Nope",
    ];
    each(cases(3000), 0x5eed_0002, |_, rng| {
        let mut c = String::new();
        for _ in 0..rng.below(40) {
            for _ in 0..rng.below(7) {
                match rng.below(7) {
                    0 | 1 => {
                        c.push_str(&format!("{} ", (rng.next() % 20_000) as f64 / 7.0 - 1000.0))
                    }
                    2 => c.push_str(&format!("{} ", rng.below(4))),
                    3 => c.push_str(NAMES[rng.below(NAMES.len())]),
                    4 => c.push_str("(\\001\\002ab) "),
                    5 => c.push_str("[(a) -120 <0001> 33] "),
                    _ => c.push_str("<</MCID 3>> "),
                }
                c.push(' ');
            }
            c.push_str(OPS[rng.below(OPS.len())]);
            c.push('\n');
        }
        let _ = derive_pdf(&wrap(c.as_bytes()));
    });
}

/// Image decoding: any sizes, bit depths, encodings and data; the sizes
/// never overflow, a Flate stream never inflates past the image's size.
#[test]
fn image_decoding_never_panics_or_overallocates() {
    each(cases(3000), 0x5eed_0003, |_, rng| {
        let encoding = [
            ImageEncoding::Flate,
            ImageEncoding::Plain,
            ImageEncoding::Jpeg,
        ][rng.below(3)];
        let (w, h) = if rng.below(4) == 0 {
            (rng.next() as u32, rng.next() as u32)
        } else {
            (1 + rng.below(40) as u32, 1 + rng.below(40) as u32)
        };
        let components = [1u8, 3, 4][rng.below(3)];
        let bits = [8u8, 16][rng.below(2)];
        let raw_len =
            (w as u64 * h as u64 * components as u64 * bits as u64 / 8).min(4096) as usize;
        let samples = rng.bytes(raw_len);
        let data = match (encoding, rng.below(3)) {
            (ImageEncoding::Flate, 0) => rng.some_bytes(64),
            (ImageEncoding::Flate, _) => miniz_oxide::deflate::compress_to_vec_zlib(&samples, 1),
            _ => samples.clone(),
        };
        let mask = (rng.below(2) == 0).then(|| {
            let m = rng.bytes((w as u64 * h as u64).min(4096) as usize);
            (
                ImageEncoding::Flate,
                miniz_oxide::deflate::compress_to_vec_zlib(&m, 1),
            )
        });
        let img = PdfImage {
            width: w,
            height: h,
            bits,
            components,
            icc: None,
            interpolate: false,
            encoding,
            data,
            mask,
        };
        let _ = img.data_len();
        if let Ok(d) = img.data_part() {
            if encoding != ImageEncoding::Jpeg {
                assert_eq!(Some(d.len() as u64), img.samples_len());
            }
        }
        let _ = img.mask_part();
    });
}

/// The bounded inflate: random and crafted streams never return more than
/// the limit, and a stream that inflates past it is an error.
#[test]
fn inflate_respects_its_limit() {
    let bomb = miniz_oxide::deflate::compress_to_vec_zlib(&vec![0u8; 1 << 20], 10);
    assert!(inflate_limited(&bomb, 1000).is_err());
    assert_eq!(inflate_limited(&bomb, 1 << 20).unwrap().len(), 1 << 20);
    each(cases(3000), 0x5eed_0004, |_, rng| {
        let limit = rng.below(512);
        let input = if rng.below(2) == 0 {
            rng.some_bytes(256)
        } else {
            let mut z = miniz_oxide::deflate::compress_to_vec_zlib(&rng.some_bytes(1024), 6);
            if !z.is_empty() && rng.below(2) == 0 {
                let at = rng.below(z.len());
                z[at] ^= 0x55;
            }
            z
        };
        if let Ok(v) = inflate_limited(&input, limit) {
            assert!(v.len() <= limit);
        }
    });
}
