//! The six commands `gen_amssymb.py` used to drop from the generated table.
//!
//! They were skipped as "kernel commands amsfonts only redefines", which is
//! false in two different ways. Checked against pdfTeX
//! 3.141592653-2.6-1.40.27 (TeX Live 2025):
//!
//! * `\mho`, `\sqsubset` and `\sqsupset` are not kernel commands at all --
//!   `latex.ltx` 14145/14150/14151 make them `\not@base` stubs, and base
//!   LaTeX2e answers `! LaTeX Error: Command \sqsubset not provided in base
//!   LaTeX2e.` `amsfonts.sty` 99-101 provides them.
//! * `\angle`, `\hbar` and `\rightleftharpoons` are kernel commands
//!   (`fontmath.ltx` 243, 241, 361) but kernel *composites*, and amsfonts
//!   replaces each with one msam/msbm glyph of different metrics.
//!
//! `\show` under amssymb gives the mathchars asserted below; the `width_em`
//! values are the msam10/msbm10 TFM widths, which agree with the widths
//! pdflatex reports for `\hbox{$\sym$}` at 10pt.

use flashtex_compiler::amssymb::{by_name, SymbolClass, SymbolFont};

#[test]
fn amsfonts_declarations_are_not_skipped() {
    // (command, \show mathchar, class, font, slot, msam/msbm width in em,
    //  pdflatex \hbox width at 10pt)
    let expected: [(&str, u32, SymbolClass, SymbolFont, u8, f64, f64); 6] = [
        ("rightleftharpoons", 0x340A, SymbolClass::Rel, SymbolFont::Msam, 0x0A, 1.000003, 10.00002),
        ("angle", 0x045C, SymbolClass::Ord, SymbolFont::Msam, 0x5C, 0.722224, 7.22223),
        ("hbar", 0x057E, SymbolClass::Ord, SymbolFont::Msbm, 0x7E, 0.540280, 5.40280),
        ("sqsubset", 0x3440, SymbolClass::Rel, SymbolFont::Msam, 0x40, 0.777781, 7.77780),
        ("sqsupset", 0x3441, SymbolClass::Rel, SymbolFont::Msam, 0x41, 0.777781, 7.77780),
        ("mho", 0x0566, SymbolClass::Ord, SymbolFont::Msbm, 0x66, 0.722224, 7.22223),
    ];
    for (name, mathchar, class, font, slot, width_em, pt_at_10) in expected {
        let s = by_name(name).unwrap_or_else(|| panic!("\\{name} is missing from the table"));
        assert_eq!(s.class, class, "\\{name} class");
        assert_eq!(s.font, font, "\\{name} font");
        assert_eq!(s.slot, slot, "\\{name} slot");
        // \show's mathchar encodes class, family and slot as "CFSS. The
        // family digit is the symbol font amssymb allocates (4 = AMSa/msam,
        // 5 = AMSb/msbm), so it must agree with `font` and `slot` above.
        let family = match font {
            SymbolFont::Msam => 4,
            SymbolFont::Msbm => 5,
        };
        let class_digit = match class {
            SymbolClass::Ord => 0,
            SymbolClass::Bin => 2,
            SymbolClass::Rel => 3,
            SymbolClass::Open => 4,
            SymbolClass::Close => 5,
        };
        assert_eq!(
            mathchar,
            class_digit * 0x1000 + family * 0x100 + u32::from(slot),
            "\\{name} does not reconstruct pdfTeX's \\mathchar"
        );
        // The TFM width pdflatex sets the glyph at, to the hundredth of a pt.
        assert!(
            (width_em * 10.0 - pt_at_10).abs() < 0.01,
            "\\{name}: table {width_em}em is {}pt at 10pt, pdflatex measured {pt_at_10}pt",
            width_em * 10.0
        );
        assert!(
            (s.width_em - width_em).abs() < 1e-6,
            "\\{name}: table carries {} em, expected {width_em}",
            s.width_em
        );
    }
}
