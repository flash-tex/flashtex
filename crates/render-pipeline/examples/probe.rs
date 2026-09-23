//! Prints glyph bounds for a few characters (development probe).
use flashtex_render_pipeline::fonts::{Family, FontSet, Role};
use flashtex_render_pipeline::shape::Shaper;

fn main() {
    let fonts = FontSet::with_default_dirs(&[]);
    let face = fonts.resolve(Family::LatinModern, Role::Text { bold: false, italic: false }, 10.0).face;
    let shaper = Shaper::new();
    for w in ["office", "x", "H", "p", "fi", "ffi", "The"] {
        let s = shaper.shape(&face, w);
        println!("{w:?}: width {} height {} depth {}", s.width_units, s.height_units, s.depth_units);
        for c in &s.clusters {
            for g in &c.glyphs {
                println!("   {:?} gid {} adv {} ymax {} ymin {} empty {}", c.text, g.gid.0, g.advance, g.y_max, g.y_min, g.empty);
            }
        }
    }
    let m = fonts.resolve(Family::LatinModern, Role::Math, 10.0).face;
    for ch in ['x', '\u{1D465}', '\u{2211}', '(', 'a'] {
        let g = m.face().glyph_id(ch);
        println!("math {ch:?} -> {:?} bounds {:?}", g, g.map(|g| m.bounds(g, Some(ch))));
    }
}
