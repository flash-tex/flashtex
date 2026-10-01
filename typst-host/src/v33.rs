//! The `display-list-v3.3` additions this host sends (DESIGN.md §15.4), as a
//! **draft** for the protocol owner. DESIGN.md §15.4 drafted them as "3.2";
//! the shared specification has since given 3.2 to external tools (#1296), so
//! this host negotiates them as **3.3**, and a 3.2 client (the reference
//! `Client` included) gets none of them. They are additive and capability-gated,
//! so a 3.1 or 3.2 reader decodes every frame (it skips the new section tags) and
//! the LaTeX host is never required to emit any of it:
//!
//! * **E1** `FONT.format: "opentype"` (a value §5.1 already reserves): the
//!   program is the font file Typst read (OpenType CFF or TrueType outlines,
//!   or a collection), `code` is the **glyph id**, and the JSON carries
//!   `face_index`, `units_per_em`, `variations`, `file` and
//!   `program_sha256`. The glyph's `FontMatrix` is `1/units_per_em`.
//! * **E2** section [`tag::ORIGINS_F64`]: every GLYPH item's origin in f64,
//!   stream space (bp, y up), in item order; the i32 `x`/`y` stay the
//!   §4.2 rounding of the same numbers.
//! * **E7** section [`tag::PAGE_META`]: JSON, `{"engine", "number",
//!   "bleed"}`; `counts` stays TeX-only (zeros).
//!
//! Nothing here changes `crates/display-list-v3` or its specification: the
//! sections are appended to a body that crate encoded, and read back by
//! [`sections`]. When the protocol owner lands v3.3, this module moves there.

use flashtex_display_list::sha256::Sha256;

/// The minor version this host speaks at most.
pub const MINOR: u32 = 3;

/// Page section tags added by 3.3 (after 3.1's 1..=6; 3.2 adds none).
pub mod tag {
    /// E2: `u32 n`, then `n × f64[2]`: each GLYPH item's origin (X, Y) in
    /// stream space (bp, origin bottom-left, y up), in item order.
    pub const ORIGINS_F64: u32 = 7;
    /// E7: UTF-8 JSON object (page number, bleed, engine).
    pub const PAGE_META: u32 = 8;
}

/// Offset of the section count `n` in a PAGE/FORM body (§4.1: index, flags,
/// width, height, counts[10], box[4], hash[32]).
const SECTION_COUNT_AT: usize = 4 + 4 + 4 + 4 + 40 + 32 + 32;

/// Append `sections` to an encoded PAGE/FORM body, updating its count.
pub fn append_sections(body: &mut Vec<u8>, sections: &[(u32, Vec<u8>)]) {
    let at = SECTION_COUNT_AT;
    let n = u32::from_le_bytes(body[at..at + 4].try_into().unwrap());
    body[at..at + 4].copy_from_slice(&(n + sections.len() as u32).to_le_bytes());
    for (t, data) in sections {
        body.extend_from_slice(&t.to_le_bytes());
        body.extend_from_slice(&(data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
    }
}

/// Every section of a PAGE/FORM body: (tag, data), in order.
pub fn sections(body: &[u8]) -> Result<Vec<(u32, &[u8])>, String> {
    let at = SECTION_COUNT_AT;
    if body.len() < at + 4 {
        return Err("page body shorter than its header".into());
    }
    let n = u32::from_le_bytes(body[at..at + 4].try_into().unwrap()) as usize;
    let mut out = Vec::with_capacity(n);
    let mut p = at + 4;
    for _ in 0..n {
        if body.len() < p + 8 {
            return Err("truncated section header".into());
        }
        let t = u32::from_le_bytes(body[p..p + 4].try_into().unwrap());
        let len = u32::from_le_bytes(body[p + 4..p + 8].try_into().unwrap()) as usize;
        p += 8;
        if body.len() < p + len {
            return Err(format!("section {t} longer than the body"));
        }
        out.push((t, &body[p..p + len]));
        p += len;
    }
    if p != body.len() {
        return Err("bytes after the last section".into());
    }
    Ok(out)
}

pub fn encode_origins(origins: &[(f64, f64)]) -> Vec<u8> {
    let mut o = Vec::with_capacity(4 + origins.len() * 16);
    o.extend_from_slice(&(origins.len() as u32).to_le_bytes());
    for (x, y) in origins {
        o.extend_from_slice(&x.to_le_bytes());
        o.extend_from_slice(&y.to_le_bytes());
    }
    o
}

pub fn decode_origins(d: &[u8]) -> Result<Vec<(f64, f64)>, String> {
    if d.len() < 4 {
        return Err("ORIGINS_F64 without a count".into());
    }
    let n = u32::from_le_bytes(d[..4].try_into().unwrap()) as usize;
    if d.len() != 4 + n * 16 {
        return Err(format!("ORIGINS_F64: {} bytes for {n} origins", d.len()));
    }
    let f = |i: usize| f64::from_le_bytes(d[i..i + 8].try_into().unwrap());
    Ok((0..n).map(|i| (f(4 + 16 * i), f(12 + 16 * i))).collect())
}

/// The 3.3 content hash: the v3 hash (§4.6) extended by the 3.3 sections
/// that change what is drawn (E2 origins; E7 is metadata but its bleed moves
/// the trim box, so it counts too).
pub fn extended_hash(v3: [u8; 32], sections: &[(u32, Vec<u8>)]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"display-list-v3.3 content\0");
    h.update(&v3);
    for (t, data) in sections {
        h.update(&t.to_le_bytes());
        h.update(&(data.len() as u64).to_le_bytes());
        h.update(data);
    }
    h.finish()
}

/// E1 font key: SHA-256 over the format, the program's hash, the face index
/// and the variation coordinates (§5.1's key, with the fields an OpenType
/// face needs in place of the Type 1 encoding and transform).
pub fn opentype_font_key(
    program_sha256: &[u8; 32],
    face_index: u32,
    variations: &[(String, f32)],
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(b"display-list-v3 font\0");
    h.update(b"opentype");
    h.update(&[0]);
    h.update(program_sha256);
    h.update(b"\0face\0");
    h.update(&face_index.to_le_bytes());
    for (tag, v) in variations {
        h.update(b"\0var\0");
        h.update(tag.as_bytes());
        h.update(&v.to_le_bytes());
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use flashtex_display_list::page::{Item, Page, StreamKind};

    #[test]
    fn appended_sections_round_trip_and_a_31_decoder_skips_them() {
        let mut p = Page::new(StreamKind::Page, 3);
        p.width = 100;
        p.height = 200;
        p.items.push(Item::Glyph {
            font: 1,
            code: 42,
            x: 7,
            y: 9,
            col: 0,
        });
        let mut body = p.encode();
        let origins = vec![(1.25, -3.5), (1e-9, 841.889763)];
        let meta = br#"{"engine":"typst","number":4}"#.to_vec();
        append_sections(
            &mut body,
            &[
                (tag::ORIGINS_F64, encode_origins(&origins)),
                (tag::PAGE_META, meta.clone()),
            ],
        );
        let secs = sections(&body).unwrap();
        let o = secs.iter().find(|(t, _)| *t == tag::ORIGINS_F64).unwrap().1;
        assert_eq!(decode_origins(o).unwrap(), origins);
        assert_eq!(
            secs.iter().find(|(t, _)| *t == tag::PAGE_META).unwrap().1,
            &meta[..]
        );
        // The reference (3.2) decoder still reads the page, unchanged.
        assert_eq!(Page::decode(StreamKind::Page, &body).unwrap(), p);
    }

    #[test]
    fn origins_reject_bad_lengths() {
        assert!(decode_origins(&[1, 0, 0, 0]).is_err());
        assert!(decode_origins(&[]).is_err());
        assert_eq!(decode_origins(&[0, 0, 0, 0]).unwrap(), vec![]);
    }

    #[test]
    fn keys_depend_on_face_and_variations() {
        let sha = [7u8; 32];
        let a = opentype_font_key(&sha, 0, &[]);
        assert_ne!(a, opentype_font_key(&sha, 1, &[]));
        assert_ne!(a, opentype_font_key(&sha, 0, &[("wght".into(), 700.0)]));
        assert_eq!(a, opentype_font_key(&sha, 0, &[]));
    }
}
