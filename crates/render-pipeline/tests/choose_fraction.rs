//! Plain TeX's `\choose` is `\atopwithdelims()`: a generalized fraction, so
//! in a text formula its two parts are set in script style (7pt) with
//! `\delim2` parentheses around them. The compiler (ef53bbdfa) hands
//! `{n \choose k}` over as a one-column parenthesised grid at the `\choose`
//! token; the pipeline used to set that grid at text size, which pushed the
//! whole line 4.48 bp down and everything after it 16.55 bp right. It is now
//! the same Rule 15e fraction `\atopwithdelims()` gives, while a `pmatrix`
//! of the same shape stays a grid.
//!
//! The table is pdfTeX 3.141592653 (TeX Live 2026) glyph origins in bp (x,
//! baseline from the page top) for this exact source, `article` 10pt on US
//! letter; pdflatex never runs here. The parentheses are left out: pdfTeX
//! paints cmex pieces hung from their top, this pipeline Latin Modern Math
//! glyphs on the baseline, at the same x.

mod common;

use common::*;

#[test]
fn choose_is_a_script_size_fraction_like_atop() {
    if !lm_available() {
        return;
    }
    let src = r"\documentclass{article}\usepackage{amsmath}\begin{document}Q $a \atop b$ Z ${a+1 \above 1pt b}$ Y ${n \choose k}$ X $m \choose j$ W $\begin{pmatrix}c\\ d\end{pmatrix}$ V\[{r \choose s} + t\]\end{document}";
    assert_pdftex_glyphs(
        src,
        &[
            ("Q", "CMR10", 148.712, 139.248),
            ("a", "CMMI7", 160.977, 134.827),
            ("b", "CMMI7", 161.386, 142.683),
            ("Z", "CMR10", 169.815, 139.248),
            ("a", "CMMI7", 180.420, 134.433),
            ("+", "CMR7", 184.744, 134.433),
            ("1", "CMR7", 190.860, 134.433),
            ("b", "CMMI7", 185.873, 143.095),
            ("Y", "CMR10", 199.345, 139.248),
            // `{n \choose k}` in braces.
            ("n", "CMMI7", 214.704, 134.827),
            ("k", "CMMI7", 214.965, 142.683),
            ("X", "CMR10", 227.516, 139.248),
            // `$m \choose j$`: the whole formula is the fraction.
            ("m", "CMMI7", 242.875, 134.827),
            ("j", "CMMI7", 244.560, 142.683),
            ("W", "CMR10", 257.832, 139.248),
            // The control: a `pmatrix` is a text-style grid.
            ("c", "CMMI10", 279.163, 133.171),
            ("d", "CMMI10", 278.726, 145.126),
            ("V", "CMR10", 294.566, 139.248),
            // Display style: the parts are text style (10pt).
            ("r", "CMMI10", 295.351, 167.378),
            ("s", "CMMI10", 295.402, 180.951),
            ("+", "CMR10", 309.670, 174.117),
            ("t", "CMMI10", 319.633, 174.117),
        ],
        0.1,
    );
}
