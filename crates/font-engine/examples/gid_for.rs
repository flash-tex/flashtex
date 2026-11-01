//! Prints the original glyph id for each Unicode scalar via this crate's
//! cmap parser: `gid_for <font path> <U+XXXX or hex>...`, one `hex gid` line
//! per input (`-` when absent). Used by tools/gen_tfm_fixture.py as an
//! independent cross-check of glyph identities obtained from the CFF charset.
use std::path::Path;

use flashtex_font_engine::{Face, load_from_path};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let face = load_from_path(Path::new(&args[0])).expect("font");
    for a in &args[1..] {
        let hex = a.trim_start_matches("U+").trim_start_matches("0x");
        let cp = u32::from_str_radix(hex, 16).expect("hex code point");
        match char::from_u32(cp).and_then(|c| face.glyph_id(c)) {
            Some(g) => println!("{hex} {}", g.0),
            None => println!("{hex} -"),
        }
    }
}
