% changes/virtex.ch -- INITEX and production runs in one program, as in web2c.
%
% tex.web is compiled twice, once as INITEX and once as a production
% program ("VIRTEX") that loads a format before reading the first line. web2c
% makes that a run-time switch (tex.ch [50.1301], `-ini'), and TeX Live's
% `pdflatex hello' is a production run of pdftex whose default format is
% named after the program (`-fmt', else the program name). Re-specified
% here, with the switches in system.rs (set by the driver, src/main.rs):
%
%   * |ini_version|: INITEX, or a production run;
%   * |etex_p|: `-etex', extended mode without a `*' (pdftex.ch);
%   * the default format, |dump_name| with `.fmt', which a production run
%     loads when the first line does not start with `&', and which its
%     banner names: ` (preloaded format=pdflatex)' (tex.ch [5.61]);
%   * kpathsea finds formats itself, so there is no `TeXformats:' area to
%     try second (tex.ch [29.524]).
%
%   * tex.ch's terminal messages when a format cannot be found name the
%     format files (tex.ch [29.524]);
%   * "init...tini is dynamic" (tex.ch [1.8], its |Init|...|Tini|): the
%     table entries that INITEX sets up (|mem|'s static part, all of
%     |eqtb|, the hash's frozen entries, the trie's) are set only when
%     |ini_version|, and so are the main program's string pool and
%     primitives (tex.ch [51.1332]), which need them. A production run gets
%     every one of them from the format it loads (|load_fmt_file|), as
%     web2c's does, and reads the clock after the load ([51.1337]); for
%     XeTeX's Unicode-sized |eqtb| this is most of a run's start-up.
%     |initex_tables| is |ini_version|, or true when
%     FLASHTEX_INITEX_TABLES=1 asks for the tables anyway (system.rs), which
%     is how the state after the format load is shown to be the same.
%
% web2c's version string after the banner is in web2c-run.ch.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x xetex.web l.497 - tex.ch [1.8]: init...tini is dynamic
@!init @<Initialize table entries (done by \.{INITEX} only)@>@;@+tini
@y
@!init if initex_tables then
  begin @<Initialize table entries (done by \.{INITEX} only)@>@;
  end;@+tini
@z

@x xetex.web l.1809 - tex.ch [5.61]: a production run names its default format
if format_ident=0 then wterm_ln(' (no format preloaded)')
@y
if format_ident=0 then
  begin wterm(' (preloaded format='); wterm_dump_name; wterm_ln(')');
  end
@z

@x xetex.web l.12308 - tex.ch [29.524]: kpathsea does everything
  pack_buffered_name(0,loc,j-1); {try first without the system file area}
  if w_open_in(fmt_file) then goto found;
  pack_buffered_name(format_area_length,loc,j-1);
    {now try the system format file area}
  if w_open_in(fmt_file) then goto found;
@y
  pack_buffered_name(0,loc,j-1); {Kpathsea does everything}
  if w_open_in(fmt_file) then goto found;
@z

@x xetex.web l.12314 - tex.ch [29.524]: the message names the format
  wterm_ln('Sorry, I can''t find that format;',' will try PLAIN.');
@y
  wterm ('Sorry, I can''t find the format `');
  wterm_name_of_file;
  wterm ('''; will try `');
  wterm_format_default;
  wterm_ln ('''.');
@z

@x xetex.web l.12319 - tex.ch [29.524]: the default format is |dump_name|
pack_buffered_name(format_default_length-format_ext_length,1,0);
@y
pack_default_format_name; {|dump_name| with \.{.fmt} (system.rs)}
@z

@x xetex.web l.12322 - tex.ch [29.524]: the message names the format
  wterm_ln('I can''t find the PLAIN format file!');
@y
  wterm ('I can''t find the format file `');
  wterm_format_default;
  wterm_ln ('''!');
@z

@x xetex.web l.28766 - tex.ch [50.1301]: INITEX is a run-time switch
format_ident:=" (INITEX)";
@y
if ini_version then format_ident:=" (INITEX)";
@z

@x xetex.web l.29312 - tex.ch [51.1332]: init...tini is dynamic
@!init if not get_strings_started then goto final_end;
init_prim; {call |primitive| for each primitive}
init_str_ptr:=str_ptr; init_pool_ptr:=pool_ptr; fix_date_and_time;
tini@/
@y
@!init if initex_tables then
  begin if not get_strings_started then goto final_end;
  init_prim; {call |primitive| for each primitive}
  init_str_ptr:=str_ptr; init_pool_ptr:=pool_ptr; fix_date_and_time;
  end;
tini@/
@z

@x xetex.web l.30797 - pdftex.ch: `-etex' enters extended mode without `*'
@!init if (buffer[loc]="*")and(format_ident=" (INITEX)") then
  begin no_new_control_sequence:=false;
  @<Generate all \eTeX\ primitives@>@;
  incr(loc); eTeX_mode:=1; {enter extended mode}
@y
@!init if (etex_p or(buffer[loc]="*"))and(format_ident=" (INITEX)") then
  begin no_new_control_sequence:=false;
  @<Generate all \eTeX\ primitives@>@;
  if (buffer[loc]="*") then incr(loc);
  eTeX_mode:=1; {enter extended mode}
@z

@x xetex.web l.34414 - new sections at the end of part 54
@* \[55] Index.
@y
@ The run's switches, which web2c takes from the command line.

@<Declare web2c's file-name procedures@>=
function ini_version:boolean; external; {is this INITEX?}
function initex_tables:boolean; external;
  {set up INITEX's table entries? (|ini_version|)}
function etex_p:boolean; external; {was \.{-etex} given?}
procedure wterm_dump_name; external; {write the default format's name}
procedure pack_default_format_name; external;
  {put the default format's file name into |name_of_file|}
procedure wterm_name_of_file; external; {write |name_of_file|}
procedure wterm_format_default; external;
  {write the default format's file name}

@* \[55] Index.
@z
