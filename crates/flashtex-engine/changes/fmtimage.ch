% changes/fmtimage.ch -- the loaded format kept in memory (lane COLD-FIXED,
% DESIGN.md section 4.2's "allowed optimisations").
%
% A resident host runs from the format at every cold compile and every
% preamble edit, and undumping the 15 MB format word by word costs about
% 116 M instructions each time. |load_fmt_file| is a function of the state
% before it (the whole word space, the C parts' state) and of the format
% file's bytes: given the same, it leaves the same. So the first load in a
% process keeps an image of what it left (src/fmtimage.rs), and a later load
% of the same format file whose state before the load is the same, word for
% word, installs that image instead of reading the file. Nothing else
% changes: the format is still found and opened (|open_fmt_file|, the
% read-set), closed, and everything after the load runs as before.
%
% * |flashtex_fmt_restore|, after |open_fmt_file|: |true| when the image
%   was installed, so |load_fmt_file| is not called.
% * |flashtex_fmt_loaded|, after a load that succeeded: keep its image.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.33585 - a load the process has made before is installed
  if not open_fmt_file then goto final_end;
  if not load_fmt_file then
    begin w_close(fmt_file); goto final_end;
    end;
  w_close(fmt_file);
@y
  if not open_fmt_file then goto final_end;
  if not flashtex_fmt_restore then
    begin if not load_fmt_file then
      begin w_close(fmt_file); goto final_end;
      end;
    flashtex_fmt_loaded;
    end;
  w_close(fmt_file);
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The loaded format's image (src/fmtimage.rs, see the top of
changes/fmtimage.ch).

@<Declare the routines of pdf\TeX's C parts@>=
function flashtex_fmt_restore:boolean; external;
procedure flashtex_fmt_loaded; external;

@* \[55] Index.
@z
