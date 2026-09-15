//! Prints a TeX `\showbox`-style dump and the flattened runs of every fixture
//! (or of the fixture index named on the command line) in text or display
//! style, using the Computer Modern metrics.
//!
//! Run: `cargo run --example dump -- [--display] [fixture-index]`

use flashtex_math_layout::{
    BoxKind, CmMathMetrics, MathBox, MathFontMetrics, Style, fixtures, layout_with_report,
    positioned_runs,
};

fn dump(b: &MathBox, depth: usize, m: &dyn MathFontMetrics) {
    let pad = ".".repeat(depth);
    let dims = format!("({:.5}+{:.5})x{:.5}", b.height, b.depth, b.width);
    match &b.kind {
        BoxKind::Glyph {
            font_id,
            gid,
            ch,
            size,
        } => println!(
            "{pad}\\{} {:?} (gid {gid}, {size}pt) {dims}",
            m.font_name(*font_id),
            ch
        ),
        BoxKind::Rule => println!("{pad}\\rule{dims}"),
        BoxKind::Kern => println!("{pad}\\kern{:.5}", b.width),
        BoxKind::Glue { mu, .. } => println!("{pad}\\glue{:.5} ({mu}mu)", b.width),
        BoxKind::HBox(children) | BoxKind::VBox(children) => {
            let kind = if matches!(b.kind, BoxKind::HBox(_)) {
                "hbox"
            } else {
                "vbox"
            };
            println!("{pad}\\{kind}{dims}");
            for c in children {
                if c.dx != 0.0 || c.dy != 0.0 {
                    println!("{pad}. @ dx {:.5} dy {:.5}", c.dx, c.dy);
                }
                dump(&c.content, depth + 1, m);
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let display = args.iter().any(|a| a == "--display");
    let only: Option<usize> = args.iter().find_map(|a| a.parse().ok());
    let style = if display { Style::DISPLAY } else { Style::TEXT };
    let m = CmMathMetrics::latex_10pt();
    for (i, (name, list)) in fixtures::all().into_iter().enumerate() {
        if only.is_some_and(|o| o != i) {
            continue;
        }
        let out = layout_with_report(&list, style, &m);
        println!("== [{i}] {name} ({style:?})");
        dump(&out.root, 0, &m);
        let runs = positioned_runs(&out.root, (0.0, 0.0));
        for g in &runs.glyphs {
            println!(
                "glyph {:?} {} gid {} x {:.5} baseline {:.5} size {}",
                g.ch,
                m.font_name(g.font_id),
                g.gid,
                g.x,
                g.baseline_y,
                g.size
            );
        }
        for r in &runs.rules {
            println!("rule x {:.5} y {:.5} w {:.5} h {:.5}", r.x, r.y, r.w, r.h);
        }
        for l in &out.limitations {
            println!("limitation: {l:?}");
        }
        println!();
    }
}
