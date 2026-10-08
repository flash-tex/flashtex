% changes/lineshift.ch -- where the line-shift journal observes the engine
% (DESIGN.md section 5.3, rule (c): input line numbers are shifted by an
% edit's line delta, and every read of a line number into output or state
% stays a barrier).
%
% An edit that adds or removes line breaks moves every later line of its file
% by the same number, delta. The incremental engine compares the new run's
% state with the old run's with that shift allowed for (src/lineshift.rs,
% src/incr.rs), and keeps the old run's later pages only if nothing the old
% run did after the convergence point printed or computed with a moved line
% number. This file adds the calls that let it know, and nothing else: every
% routine is hand-written in src/lineshift.rs, only reads TeX's variables,
% never prints and never changes what the program computes.
%
% * |ls_line_read|, as \.{\\inputlineno} is read (|scan_something_internal|):
%   the value and the file it counts lines of go into the run's journal.
% * |ls_the_begin|, |ls_the_take| and |ls_the_direct|: the one read that is
%   not a journal entry is \.{\\the\\inputlineno} met by |scan_toks|'s own
%   \.{\\the} while it scans the body of a definition (LaTeX's \.{\\begin}:
%   \.{\\edef\\@currenvline\{\\on@line\}}). Its digits go into that
%   definition's token list and nowhere else, so |ls_toks_done| marks the
%   list (a taint) instead.
% * |ls_use(p)| and |ls_show(p)|, as a token list is expanded
%   (|macro_call|), compared (\.{\\ifx}) or shown (|show_token_list|): a
%   tainted list's reads become journal entries then. |ls_free(p)|, as a
%   token list is freed (|delete_token_ref|): its taint goes.
% * |ls_print_level|, |ls_print_unknown| and |ls_box_lines|, as a line
%   number is printed: |show_context|'s \.{l.<n>}, \.{-file-line-error}'s
%   \.{file:<n>:}, the box reports, \.{\\showlists}, \.{\\showgroups} and
%   |print_group|, |print_if_line|, the incomplete \.{\\if} messages, the
%   editor's line.
% * |ls_nest_tag|, |ls_grp_tag|, |ls_cond_tag|: as a semantic level, a group
%   or a conditional begins, the SyncTeX tag of the input level its line
%   is of (|push_nest|, |new_save_level|, the condition stack's push; with
%   |ls_cond_depth| kept by the pushes and pops), so that the engine can tell
%   a line of the edited file from a line of another. These are the only
%   variables this file adds, and nothing reads them but src/; with
%   |ls_tag_file|, the full name of the file each tag was given to.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x tex.ch [6.84] as web2c.ch has it - "E": the editor's line
  edit_line:=line;
@y
  edit_line:=line; ls_print_level(in_open);
@z

@x pdftex.web l.4513 - delete_token_ref: a list freed loses its taint
begin if token_ref_count(p)=null then flush_list(p)
@y
begin if token_ref_count(p)=null then
  begin ls_free(p); flush_list(p);
  end
@z

@x pdftex.web l.5034 - push_nest: the new level's line is of the current file
incr(nest_ptr); head:=get_avail; tail:=head; prev_graf:=0; mode_line:=line;
@y
incr(nest_ptr); head:=get_avail; tail:=head; prev_graf:=0; mode_line:=line;
ls_nest_tag[nest_ptr]:=synctex_tag;
@z

@x pdftex.web l.5061 - show_activities: a mode's line
  print(" entered at line "); print_int(abs(nest[p].ml_field));
@y
  ls_print_unknown(nest[p].ml_field);
  print(" entered at line "); print_int(abs(nest[p].ml_field));
@z

@x pdftex.web l.7128 - new_save_level: the group's line is of the current file
incr(cur_level); incr(save_ptr);
end;
@y
incr(cur_level); incr(save_ptr);
ls_grp_tag[cur_level]:=synctex_tag;
end;
@z

@x pdftex.web l.7483 - show_token_list: a list shown
@!n:ASCII_code; {the highest parameter number, as an ASCII digit}
begin match_chr:="#"; n:="0"; tally:=0;
@y
@!n:ASCII_code; {the highest parameter number, as an ASCII digit}
begin ls_show(p); match_chr:="#"; n:="0"; tally:=0;
@z

@x pdftex.web l.8070 - show_context: a file level's line
  if index=in_open then print_int(line)
@y
  ls_print_level(index);
  if index=in_open then print_int(line)
@z

@x pdftex.web l.8423 - check_outer_validity: where skipping began
    print("; all text was ignored after line "); print_int(skip_line);
@y
    ls_print_unknown(skip_line);
    print("; all text was ignored after line "); print_int(skip_line);
@z

@x pdftex.web l.9343 - macro_call: a list expanded
warning_index:=cur_cs; ref_count:=cur_chr; r:=link(ref_count); n:=0;
@y
warning_index:=cur_cs; ref_count:=cur_chr; ls_use(ref_count);
r:=link(ref_count); n:=0;
@z

@x pdftex.web l.10015 - scan_something_internal: \inputlineno
  input_line_no_code: cur_val:=line;
@y
  input_line_no_code: begin cur_val:=line; ls_line_read;
    end;
@z

@x pdftex.web l.10758 - the_toks: scan_toks's own \the of a definition
@!c:small_number; {value of |cur_chr|}
begin @<Handle \.{\\unexpanded} or \.{\\detokenize} and |return|@>;@/
get_x_token; scan_something_internal(tok_val,false);
@y
@!c:small_number; {value of |cur_chr|}
@!ls_mine:boolean; {this is |scan_toks|'s \.{\\the} in a definition's body}
begin ls_mine:=ls_the_take;
@<Handle \.{\\unexpanded} or \.{\\detokenize} and |return|@>;@/
get_x_token; if ls_mine then ls_the_direct;
scan_something_internal(tok_val,false);
@z

@x pdftex.web l.11470 - scan_toks: the list is complete
found: scanner_status:=normal;
@y
found: scanner_status:=normal; ls_toks_done(macro_def);
@z

@x pdftex.web l.11545 - scan_toks: its own \the
  else  begin q:=the_toks;
@y
  else  begin ls_the_begin(macro_def); q:=the_toks;
@z

@x pdftex.web l.11838 - push the condition stack: the line is of the current file
cond_ptr:=p; cur_if:=cur_chr; if_limit:=if_code; if_line:=line;
end
@y
cond_ptr:=p; cur_if:=cur_chr; if_limit:=if_code; if_line:=line;
incr(ls_cond_depth);
if ls_cond_depth<=ls_cond_size then ls_cond_tag[ls_cond_depth]:=synctex_tag;
end
@z

@x pdftex.web l.11845 - pop the condition stack
cur_if:=subtype(p); if_limit:=type(p); cond_ptr:=link(p);
@y
cur_if:=subtype(p); if_limit:=type(p); cond_ptr:=link(p); decr(ls_cond_depth);
@z

@x pdftex.web l.12041 - \ifx: two lists compared
begin p:=link(cur_chr); q:=link(equiv(n)); {omit reference counts}
@y
begin ls_use(cur_chr); ls_use(equiv(n));
p:=link(cur_chr); q:=link(equiv(n)); {omit reference counts}
@z

@x pdftex.web l.21199 - hpack: a box report's lines
dg_box_begin(r); if output_active then print(") has occurred while \output is active")
else  begin if pack_begin_line<>0 then
@y
dg_box_begin(r); if output_active then print(") has occurred while \output is active")
else  begin ls_box_lines; if pack_begin_line<>0 then
@z

@x pdftex.web l.21376 - vpackage: a box report's lines
dg_box_begin(r); if output_active then print(") has occurred while \output is active")
else  begin if pack_begin_line<>0 then {it's actually negative}
@y
dg_box_begin(r); if output_active then print(") has occurred while \output is active")
else  begin ls_box_lines; if pack_begin_line<>0 then {it's actually negative}
@z

@x pdftex.web l.33536 - final_cleanup: an incomplete \if's line
    begin print(" on line "); print_int(if_line);
@y
    begin ls_print_unknown(if_line); print(" on line "); print_int(if_line);
@z

@x pdftex.web l.33541 - final_cleanup: the condition stack popped
  cond_ptr:=link(cond_ptr); free_node(temp_ptr,if_node_size);
@y
  cond_ptr:=link(cond_ptr); free_node(temp_ptr,if_node_size); decr(ls_cond_depth);
@z

@x pdftex.web l.37336 - print_group: a group's line
  print_int(saved(-1));
@y
  ls_print_unknown(saved(-1)); print_int(saved(-1));
@z

@x pdftex.web l.37710 - print_if_line: a conditional's line
@d print_if_line(#)==if #<>0 then
  begin print(" entered on line "); print_int(#);
@y
@d print_if_line(#)==if #<>0 then
  begin ls_print_unknown(#); print(" entered on line "); print_int(#);
@z

@x changes/synctex.ch - a file opened: which one its tag is
begin incr(synctex_tag_counter); synctex_tag:=synctex_tag_counter;
end
@y
begin incr(synctex_tag_counter); synctex_tag:=synctex_tag_counter;
if synctex_tag<=ls_tag_size then
  ls_tag_file[synctex_tag]:=full_source_filename_stack[in_open];
end
@z

@x changes/web2c-run.ch - print_file_line: a file level's line
    if level=in_open then print_int (line)
@y
    ls_print_level(level);
    if level=in_open then print_int (line)
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The line-shift journal's routines (src/lineshift.rs, see the top of
changes/lineshift.ch).

@ Which file each open semantic level, group and conditional took its line
from: the SyncTeX tag of the input level that was being read when it began
(a file's, which token lists inherit; 0 in a \.{\\scantokens} pseudo
file). Nothing \TeX\ computes reads them; the incremental engine does, to
tell a line an edit moved from one it did not. Conditionals nested deeper
than |ls_cond_size| have none. |ls_tag_file| names the file of each tag, so
that a level, group or conditional begun in a file that is closed again is
still known to be that file's.

@d ls_cond_size=1000 {conditionals whose file is known}
@d ls_tag_size=65535 {files opened whose name is kept by their tag}

@<Glob...@>=
@!ls_nest_tag:array[0..nest_size] of integer; {|mode_line|'s file}
@!ls_grp_tag:array[0..max_quarterword] of integer; {a group's line's file}
@!ls_cond_tag:array[0..ls_cond_size] of integer; {a conditional's line's file}
@!ls_cond_depth:integer; {conditionals open}
@!ls_taints:integer; {definitions holding confined reads (the hooks' fast test)}
@!ls_the_def:boolean; {|scan_toks|'s \.{\\the} in a definition is next}
@!ls_def_reads:boolean; {the definition being scanned holds confined reads}
@!ls_tag_file:array[0..ls_tag_size] of str_number;
  {the full name of the file each SyncTeX tag was given to}

@ @<Set init...@>=
ls_cond_depth:=0;

@ @<Declare the routines of pdf\TeX's C parts@>=
procedure ls_line_read; external;
procedure ls_the_begin(@!d:boolean); external;
function ls_the_take:boolean; external;
procedure ls_the_direct; external;
procedure ls_toks_done(@!d:boolean); external;
procedure ls_use(@!p:pointer); external;
procedure ls_show(@!p:integer); external;
procedure ls_free(@!p:pointer); external;
procedure ls_print_level(@!j:integer); external;
procedure ls_print_unknown(@!v:integer); external;
procedure ls_box_lines; external;

@* \[55] Index.
@z
