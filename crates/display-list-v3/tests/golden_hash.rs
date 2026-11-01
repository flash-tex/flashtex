//! A page without 3.3 sections hashes exactly as in 3.2: the hash below was
//! computed by the 3.2 crate (origin/main before the 3.3 change) and must
//! never move, or every cached raster of a LaTeX page would be invalid.

use flashtex_display_list::page::{Color, Item, Page, RuleKind, StreamKind, NO_COLUMN};
use flashtex_display_list::sha256::hex;

#[test]
fn a_32_page_hashes_as_the_32_crate_did() {
    let mut p = Page::new(StreamKind::Page, 2);
    p.width = 39_158_276;
    p.height = 55_380_603;
    p.pdf_box = [0.0, 0.0, 595.2756, 841.8898];
    p.matrices.push([9.9626, 0.0, 0.0, 9.9626, 0.0, 0.0]);
    p.items = vec![
        Item::Span(3),
        Item::Matrix(1),
        Item::FillColor(Color(vec![0.0, 0.0, 1.0])),
        Item::Glyph {
            font: 4,
            code: 65,
            x: 4_736_287,
            y: 5_920_358,
            col: 7,
        },
        Item::Glyph {
            font: 4,
            code: 66,
            x: 5_000_000,
            y: 5_920_358,
            col: NO_COLUMN,
        },
        Item::Rule {
            kind: RuleKind::StrokeH,
            x: 4_736_287,
            y: 6_000_000,
            w: 30_000_000,
            h: 26_214,
        },
    ];
    p.origins = vec![[72.0001, 700.5], [76.0, 700.5]];
    let h = p.content_hash(&|_| [1u8; 32], &|_| [2u8; 32]);
    assert_eq!(hex(&h), GOLDEN, "the 3.2 page hash moved");
}

const GOLDEN: &str = "b8369c63e8e7e51a2312f11bb6b3240fd0436f27005b1b59163208139ce411f4";
