% undefined.ch: inputs on which TeX Live's xetex reads through a null pointer
% (undefined behaviour; TeX Live 2026 on macOS arm64 crashes with signal 11).
% DESIGN.md section 13: FlashTeX never crashes. Each case below stops the run
% as a fatal error instead, which keeps the log written so far and the pages
% already shipped, and exits non-zero, like the crash.
%
% This file is GPL-2.0-or-later, like the rest of the crate.

@x [47.1034] l.24646 - #2391: \XeTeXinterwordspaceshaping before the font has a space glue
            if t <> width(font_glue[main_f]) then begin
@y
            { |font_glue[main_f]| is |null| until a space has been typed in
              this font (an explicit \.{\\hskip} between its first two words);
              TeX Live reads |width(null)| here and crashes. }
            if font_glue[main_f]=null then
              fatal_error("FlashTeX: internal error avoided (TeX Live xetex crashes here)");
            if t <> width(font_glue[main_f]) then begin
@z
