% changes/resident.ch -- the restart point of a resident engine (lane
% XETEX-INCR; docs/design/xetex/INCREMENTAL.md section 5 step 2, DESIGN.md
% section 5.1).
%
% The Unicode host runs the engine in its own process (src/resident.rs),
% one run after another. Everything a run does before its format is loaded
% -- |initialize| and |init_prim| over XeTeX's Unicode-sized |eqtb|, the
% start of the terminal and the first line, the undump of the format -- is
% the same at every run of the same command line from the same format
% file. So the first run calls |flashtex_format_ready| right after the
% format is loaded and closed, where the host keeps the engine's state (a
% checkpoint of the word space and the state outside it), and a later run of
% the same command line starts from that state: it calls
% |run_after_format|, which is the rest of the main program, unchanged.
%
% Nothing else changes: the uninterrupted run reaches the same code in the
% same state, |flashtex_format_ready| only reads the state, and the code
% moved into |start_after_format| and |run_after_format| is the code that
% followed it in the section that gets the first line and in the main
% program.
%
% A resident engine's run cannot be killed as a process can, so it stops
% itself when its host asks (a newer compile, a cancel): |flashtex_poll|
% at |big_switch| (between two commands) and at every macro call (so that a
% loop of expansions with no command between, \.{\\def\\a\{\\a\}\\a}, stops
% too). It reads only the host's request, and returns at once in a run that
% is a process of its own.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x l.9455 of the merged WEB - a macro call: the host may stop the run
begin save_scanner_status:=scanner_status; save_warning_index:=warning_index;
warning_index:=cur_cs; ref_count:=cur_chr; r:=link(ref_count); n:=0;
@y
begin flashtex_poll;
save_scanner_status:=scanner_status; save_warning_index:=warning_index;
warning_index:=cur_cs; ref_count:=cur_chr; r:=link(ref_count); n:=0;
@z

@x l.24680 of the merged WEB - between two commands: the host may stop the run
big_switch: get_x_token;@/
@y
big_switch: flashtex_poll; get_x_token;@/
@z

@x l.29996 of the merged WEB - the format is loaded: the restart point
  w_close(fmt_file);
  while (loc<limit)and(buffer[loc]=" ") do incr(loc);
  end;
if eTeX_ex then wterm_ln('entering extended mode');
if end_line_char_inactive then decr(limit)
else  buffer[limit]:=end_line_char;
fix_date_and_time;@/
random_seed :=(microseconds*1000)+(epochseconds mod 1000000);@/
init_randoms(random_seed);@/
@<Compute the magic offset@>;
@<Initialize the print |selector|...@>;
if (loc<limit)and(cat_code(buffer[loc])<>escape) then start_input;
  {\.{\\input} assumed}
end
@y
  w_close(fmt_file);
  while (loc<limit)and(buffer[loc]=" ") do incr(loc);
  end;
flashtex_format_ready; {a resident engine's restart point}
start_after_format;
end
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The rest of the section that gets the first line of input and prepares
to start, after the format is loaded (see changes/resident.ch): the code
that followed the load, moved here so that a resident engine restarting
from the state after the load can run it.

@<Last-minute...@>=
procedure start_after_format;
begin if eTeX_ex then wterm_ln('entering extended mode');
if end_line_char_inactive then decr(limit)
else  buffer[limit]:=end_line_char;
fix_date_and_time;@/
random_seed :=(microseconds*1000)+(epochseconds mod 1000000);@/
init_randoms(random_seed);@/
@<Compute the magic offset@>;
@<Initialize the print |selector|...@>;
if (loc<limit)and(cat_code(buffer[loc])<>escape) then start_input;
  {\.{\\input} assumed}
end;

@ A resident engine's run from the state after the format is loaded: the
main program from that point on.

@<Last-minute...@>=
procedure run_after_format;
begin start_after_format;
history:=spotless; {ready to go!}
main_control; {come to life}
final_cleanup; {prepare for death}
close_files_and_terminate;
ready_already:=0;
end;

@ @<Declare the routines of \XeTeX's C parts@>=
procedure flashtex_format_ready; external;
  {the format is loaded: a resident engine keeps the state here}
procedure flashtex_poll; external;
  {a resident engine's run stops here when its host asks}

@* \[55] Index.
@z
