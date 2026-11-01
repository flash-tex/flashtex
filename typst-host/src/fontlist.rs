//! The per-project font list (DESIGN.md §15.2 "Fonts"): which font files
//! the document's text was set with, recorded in the project's lock
//! (`[fonts]`, [`crate::lock`]) on its first successful compile and checked
//! on every later one.
//!
//! An unknown family is only a warning in Typst, and a different file under
//! the same name (another version, another foundry's cut) silently reflows
//! the document (Track C §2.5). So each recorded font is looked up the way
//! Typst selects it (family and variant in the host's font book): missing,
//! substituted by another variant, or a file with another SHA-256 is a
//! prominent `DIAGNOSTIC` (`"kind": "font"`, spec §11.8) on every compile
//! until it is fixed or accepted (`COMPILE.lock` `update`).

use std::collections::BTreeMap;

use flashtex_display_list::sha256::{hex, sha256};
use typst::layout::{Frame, FrameItem, Ratio};
use typst::text::{Font, FontBook, FontInfo, FontStretch, FontStyle, FontVariant, FontWeight};
use typst_layout::PagedDocument;

/// The lock key of a font: `family|style|weight|stretch` (stretch in
/// thousandths), what Typst selects a font by.
pub fn key(info: &FontInfo) -> String {
    let v = info.variant;
    format!(
        "{}|{}|{}|{}",
        info.family,
        style_name(v.style),
        v.weight.to_number(),
        (v.stretch.to_ratio().get() * 1000.0).round() as u16
    )
}

fn style_name(s: FontStyle) -> &'static str {
    match s {
        FontStyle::Normal => "normal",
        FontStyle::Italic => "italic",
        FontStyle::Oblique => "oblique",
    }
}

/// The family and variant of a lock key.
pub fn parse_key(k: &str) -> Option<(String, FontVariant)> {
    // The family may itself contain '|': split the last three fields.
    let mut it = k.rsplitn(4, '|');
    let stretch: u16 = it.next()?.parse().ok()?;
    let weight: u16 = it.next()?.parse().ok()?;
    let style = match it.next()? {
        "normal" => FontStyle::Normal,
        "italic" => FontStyle::Italic,
        "oblique" => FontStyle::Oblique,
        _ => return None,
    };
    let family = it.next()?.to_string();
    Some((
        family,
        FontVariant::new(
            style,
            FontWeight::from_number(weight),
            FontStretch::from_ratio(Ratio::new(stretch as f64 / 1000.0)),
        ),
    ))
}

/// The SHA-256 (hex) of a font's file.
pub fn file_sha(font: &Font) -> String {
    hex(&sha256(font.data().as_slice()))
}

/// The distinct fonts the document's text uses.
pub fn used_fonts(doc: &PagedDocument) -> Vec<Font> {
    // A document uses a handful of fonts: a list is the right set.
    fn walk(frame: &Frame, out: &mut Vec<Font>) {
        for (_, item) in frame.items() {
            match item {
                FrameItem::Group(g) => walk(&g.frame, out),
                FrameItem::Text(t) => {
                    let f = t.font.font();
                    if !out.contains(f) {
                        out.push(f.clone());
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = vec![];
    for p in doc.pages() {
        walk(&p.frame, &mut out);
    }
    out
}

/// One problem with a recorded font.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub key: String,
    pub message: String,
}

/// Check the recorded fonts against the font book. `load(i)` loads font
/// `i` of the book; `sha(i, font)` hashes it (callers cache it).
pub fn check(
    recorded: &BTreeMap<String, (String, String)>,
    book: &FontBook,
    load: &dyn Fn(usize) -> Option<Font>,
    sha: &dyn Fn(usize, &Font) -> String,
) -> Vec<Problem> {
    let mut out = vec![];
    for (k, (want, file)) in recorded {
        let Some((family, variant)) = parse_key(k) else {
            continue;
        };
        let shown = format!(
            "\"{family}\" ({} {})",
            style_name(variant.style),
            variant.weight.to_number()
        );
        let was = if file.is_empty() {
            String::new()
        } else {
            format!(" ({file})")
        };
        let found = book
            .select(&family.to_lowercase(), variant)
            .and_then(|i| Some((i, book.info(i)?.clone())));
        let message = match found {
            None => Some(format!(
                "font {shown} that this project was set with{was} is not installed: its text \
                 falls back to another font and the document reflows (recorded in {})",
                crate::lock::FILE
            )),
            Some((_, info)) if key(&info) != *k => Some(format!(
                "font {shown} that this project was set with{was} is not installed; Typst uses \
                 {} instead and the document reflows (recorded in {})",
                key(&info).replace('|', " "),
                crate::lock::FILE
            )),
            Some((i, _)) => match load(i) {
                None => Some(format!("font {shown} could not be loaded")),
                Some(font) => {
                    let got = sha(i, &font);
                    (got != *want).then(|| {
                        format!(
                            "font {shown} is not the file this project was set with{was}: \
                             SHA-256 {}… in {}, {}… here; its text may reflow. Compile with \
                             \"lock\": \"update\" to accept this file",
                            &want[..12],
                            crate::lock::FILE,
                            &got[..12]
                        )
                    })
                }
            },
        };
        if let Some(message) = message {
            out.push(Problem {
                key: k.clone(),
                message,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        let v = FontVariant::new(
            FontStyle::Italic,
            FontWeight::from_number(700),
            FontStretch::from_ratio(Ratio::new(0.875)),
        );
        let k = format!("A|B|{}|700|875", "italic");
        let (f, w) = parse_key(&k).unwrap();
        assert_eq!((f.as_str(), w), ("A|B", v));
        assert!(parse_key("x|bold|700|1000").is_none());
    }
}
