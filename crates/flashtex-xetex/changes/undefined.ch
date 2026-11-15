% undefined.ch: inputs on which TeX Live's xetex reads through a null pointer
% (undefined behaviour; TeX Live 2026 crashes with signal 11, on macOS arm64
% and on x86_64 Linux alike: exit 139, an empty log, no output file).
% DESIGN.md section 13: FlashTeX never crashes. Each case below stops the run
% as a fatal error instead, which keeps the log written so far and the pages
% already shipped, and exits non-zero, like the crash.
%
% No new string literal: a new pool string would renumber the pool and change
% the string statistics of every log against TeX Live's (#2458). So the stop
% is |fatal_error| spelled out with |help0| and the strings xetex.web already
% has ("! ", "Emergency stop", ""). The line naming the case,
% "FlashTeX: internal error avoided (TeX Live xetex crashes here)", is printed
% by |undefined_null_read_note| (src/undefined.rs), on the terminal and in the
% log, just before "! Emergency stop." (not after the error's context: with
% -halt-on-error, |error| itself ends the run).
%
% This file is GPL-2.0-or-later, like the rest of the crate.

@x [47.1034] l.24646 - #2391: \XeTeXinterwordspaceshaping before the font has a space glue
            if t <> width(font_glue[main_f]) then begin
@y
            { |font_glue[main_f]| is |null| until a space has been typed in
              this font (an explicit \.{\\hskip} between its first two words);
              TeX Live reads |width(null)| here and crashes. }
            if font_glue[main_f]=null then begin
              normalize_selector; print_nl(""); undefined_null_read_note;
              print_err("Emergency stop"); help0; succumb;
              end;
            if t <> width(font_glue[main_f]) then begin
@z
