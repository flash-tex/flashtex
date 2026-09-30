//! The canonical text of decoded frames: every field a decoder produces,
//! one line per element, floats as their IEEE 754 bit patterns. Two
//! decoders agree on a stream exactly when their canonical texts are equal;
//! the Mac app's Swift decoder (apps/mac `FlashTeXDisplayListV3`) is tested
//! against this one on every parity fixture (`dl3-dump --canonical`).
//!
//! Lines (fields separated by one space; `b(x)` = `{:016x}` of `x.to_bits()`,
//! `h(bytes)` = lowercase hex, `-` for empty):
//!
//! ```text
//! page|form INDEX FLAGS WIDTH HEIGHT C0..C9 b(BOX0)..b(BOX3) h(HASH)
//!   mat b(a) b(b) b(c) b(d) b(e) b(f)
//!   path PAINT MATRIX [stroke b(w) CAP JOIN b(miter) N b(dash).. b(phase)] SEGS
//!     seg m|l|c b(..)..   or   seg h
//!   g FONT CODE X Y COL | r KIND X Y W H | p N | clip N | img ID M | form ID M
//!   q | Q | fill N b(..).. | stroke N b(..).. | m N | span N | tr MODE | u N
//!   link L T R B SPAN KIND h(FILE) h(DATA)
//!   dest NAMED h(NAME) KIND L T R B ZOOM
//!   unsupported h(UTF-8 TEXT)
//! font ID h(KEY) PROGRAM_BYTES h(sha256(PROGRAM)) FORMAT
//!   enc NAME0 .. NAME255     (only when the font has an `encoding`; `-` for a missing name)
//! sources
//!   file ID h(PATH)
//!   span ID FILE LINE
//! json KIND h(sha256(BODY))  (every other JSON message: started, image, …)
//! other KIND LEN             (a kind this version does not know)
//! ```

use crate::client::{decode_event, Event};
use crate::kind;
use crate::page::{Item, Page, Seg};
use crate::sha256::{hex, sha256};
use std::fmt::Write;

fn b(x: f64) -> String {
    format!("{:016x}", x.to_bits())
}

fn h(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        "-".into()
    } else {
        hex(bytes)
    }
}

fn page(o: &mut String, tag: &str, p: &Page) {
    let _ = write!(o, "{tag} {} {} {} {}", p.index, p.flags, p.width, p.height);
    for c in p.counts {
        let _ = write!(o, " {c}");
    }
    for v in p.pdf_box {
        let _ = write!(o, " {}", b(v));
    }
    let _ = writeln!(o, " {}", h(&p.hash));
    for m in &p.matrices {
        let _ = writeln!(
            o,
            "mat {} {} {} {} {} {}",
            b(m[0]),
            b(m[1]),
            b(m[2]),
            b(m[3]),
            b(m[4]),
            b(m[5])
        );
    }
    for path in &p.paths {
        let _ = write!(o, "path {} {}", path.paint, path.matrix);
        if let Some(s) = &path.stroke {
            let _ = write!(
                o,
                " stroke {} {} {} {} {}",
                b(s.width),
                s.cap,
                s.join,
                b(s.miter),
                s.dash.len()
            );
            for d in &s.dash {
                let _ = write!(o, " {}", b(*d));
            }
            let _ = write!(o, " {}", b(s.phase));
        }
        let _ = writeln!(o, " {}", path.segs.len());
        for seg in &path.segs {
            let _ = match seg {
                Seg::Move(x, y) => writeln!(o, "seg m {} {}", b(*x), b(*y)),
                Seg::Line(x, y) => writeln!(o, "seg l {} {}", b(*x), b(*y)),
                Seg::Curve(a, c, d, e, f, g) => writeln!(
                    o,
                    "seg c {} {} {} {} {} {}",
                    b(*a),
                    b(*c),
                    b(*d),
                    b(*e),
                    b(*f),
                    b(*g)
                ),
                Seg::Close => writeln!(o, "seg h"),
            };
        }
    }
    for it in &p.items {
        let _ = match it {
            Item::Glyph {
                font,
                code,
                x,
                y,
                col,
            } => writeln!(o, "g {font} {code} {x} {y} {col}"),
            Item::Rule { kind, x, y, w, h } => {
                writeln!(o, "r {} {x} {y} {w} {h}", *kind as u8)
            }
            Item::Path(n) => writeln!(o, "p {n}"),
            Item::Clip(n) => writeln!(o, "clip {n}"),
            Item::Image { id, matrix } => writeln!(o, "img {id} {matrix}"),
            Item::Form { id, matrix } => writeln!(o, "form {id} {matrix}"),
            Item::Save => writeln!(o, "q"),
            Item::Restore => writeln!(o, "Q"),
            Item::FillColor(c) | Item::StrokeColor(c) => {
                let name = if matches!(it, Item::FillColor(_)) {
                    "fill"
                } else {
                    "stroke"
                };
                let _ = write!(o, "{name} {}", c.0.len());
                for v in &c.0 {
                    let _ = write!(o, " {}", b(*v));
                }
                writeln!(o)
            }
            Item::Matrix(n) => writeln!(o, "m {n}"),
            Item::Span(n) => writeln!(o, "span {n}"),
            Item::TextRender(m) => writeln!(o, "tr {m}"),
            Item::Unsupported(n) => writeln!(o, "u {n}"),
        };
    }
    for l in &p.links {
        let _ = writeln!(
            o,
            "link {} {} {} {} {} {} {} {}",
            l.rect[0],
            l.rect[1],
            l.rect[2],
            l.rect[3],
            l.span,
            l.kind.clone() as u8,
            h(&l.file),
            h(&l.data)
        );
    }
    for d in &p.dests {
        let _ = writeln!(
            o,
            "dest {} {} {} {} {} {} {} {}",
            d.named as u8,
            h(&d.name),
            d.kind,
            d.rect[0],
            d.rect[1],
            d.rect[2],
            d.rect[3],
            d.zoom
        );
    }
    for u in &p.unsupported {
        let _ = writeln!(o, "unsupported {}", h(u.as_bytes()));
    }
}

/// The canonical text of one frame (`k`, `body`), or the decoder's error.
pub fn canonical(k: u8, body: Vec<u8>) -> Result<String, String> {
    let digest = hex(&sha256(&body));
    let len = body.len();
    let mut o = String::new();
    match decode_event(k, body)? {
        Event::Page(p) => page(&mut o, "page", &p),
        Event::Form(p) => page(&mut o, "form", &p),
        Event::Font(f) => {
            let format = f.info.str_field("format").unwrap_or("-");
            let _ = writeln!(
                o,
                "font {} {} {} {} {}",
                f.id,
                h(&f.key),
                f.program.len(),
                hex(&sha256(&f.program)),
                format
            );
            if f.info.get("encoding").and_then(|e| e.as_array()).is_some() {
                o.push_str("enc");
                for c in 0..256u16 {
                    o.push(' ');
                    o.push_str(f.glyph_name(c).filter(|n| !n.is_empty()).unwrap_or("-"));
                }
                o.push('\n');
            }
        }
        Event::Sources(s) => {
            o.push_str("sources\n");
            for (id, path) in &s.files {
                let _ = writeln!(o, "file {id} {}", h(path.as_bytes()));
            }
            for (id, file, line) in &s.spans {
                let _ = writeln!(o, "span {id} {file} {line}");
            }
        }
        Event::Other(k, _) => {
            let _ = writeln!(o, "other {k} {len}");
        }
        _ => {
            let _ = writeln!(o, "json {} {digest}", kind::name(k));
        }
    }
    Ok(o)
}
