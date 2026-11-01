% changes/checkpoint.ch -- the points where a resident engine may take a
% checkpoint (lane XETEX-INCR; DESIGN.md sections 5.1 and 5.2, and the
% pdfTeX engine's crates/flashtex-engine/changes/checkpoint.ch, whose
% mechanism this is, against xetex.web's text).
%
% A checkpoint captures the whole engine state, so it can only be taken where
% the Pascal program itself holds nothing on the (Rust) call stack that the
% state does not describe. That point is |big_switch| in |main_control|:
% between two commands, with every local of |main_control| dead. There the
% engine calls the hand-written |flashtex_checkpoint_hook| (src/ckpt.rs)
% whenever |ckpt_request| is nonzero, and a restored run re-enters
% |main_control| at |big_switch| (|ckpt_resuming| skips \.{\\everyjob},
% which the uninterrupted run inserted long before): |resume_main_control|.
%
% The begin-document snapshot $S_0$ (section 5.1) is armed without looking at
% any token: the host names a control sequence (\.{\\document}, whose
% expansion LaTeX's \.{\\begin\{document\}} ends in); when |macro_call| pushes
% its body, the input level is remembered, and when |pop_input| goes below
% that level the body -- with everything it expanded to -- has been
% consumed, and the next |big_switch| takes $S_0$. A body consumed while
% the scanner is not |normal| was not run but absorbed: a package that
% patches \.{\\document} with \.{\\expandafter} (bidi, loaded by
% polyglossia for a right-to-left language) expands it into a definition or
% an argument in the preamble. That arms the control sequence again rather
% than taking $S_0$ there, in the middle of the preamble. After each shipout, when
% the host asks (|ckpt_on_shipout|), the next |big_switch| takes a page
% checkpoint (section 5.2), from which a later run restarts near its edit.
% None of this changes what
% the program computes: the new variables are read only by these tests and by
% the hook, and the hook itself only reads the state. (XeTeX's |null| is not
% zero, so ``no control sequence armed'' is 0, which no control sequence is.)
%
% READ-TRACK (docs/design/engine-v2/READ-TRACK.md; the pdfTeX engine's
% checkpoint.ch has the same hunks): the test |pop_input| makes for the
% armed level also watches a dormant file level. At every page checkpoint
% the host sets |dorm_level| to one above the innermost level that reads a
% file (when every level above it is a token list), and |pop_watch| to the
% larger of the two levels; |pop_input| below |pop_watch| does what the test
% did for the armed level and counts the watched level's return in
% |dorm_fired|, inline (a call there costs every caller of |end_token_list|).
% |flashtex_level_opened| counts every new input level and terminal line
% (both read |first|), and |flashtex_level_shown| every input line
% |show_context| prints. The inline test is the same comparison as before.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x l.1946 of the merged WEB - a line read from the terminal is counted (READ-TRACK)
begin update_terminal; {now the user sees the prompt for sure}
@y
begin flashtex_level_opened; update_terminal; {now the user sees the prompt for sure}
@z

@x l.8019 of the merged WEB - so is a file level's line printed (READ-TRACK)
  if index=in_open then print_int(line)
@y
  flashtex_level_shown;
  if index=in_open then print_int(line)
@z

@x l.8240 of the merged WEB - leaving the armed input level requests the checkpoint
@d pop_input==@t@> {leave an input level, re-enter the old}
  begin decr(input_ptr); cur_input:=input_stack[input_ptr];
  end
@y
@d pop_input==@t@> {leave an input level, re-enter the old}
  begin decr(input_ptr); cur_input:=input_stack[input_ptr];
  if input_ptr<pop_watch then
    begin if input_ptr<ckpt_arm_level then
      begin ckpt_arm_level:=0;
      if scanner_status=normal then ckpt_request:=2
      else ckpt_arm_cs:=ckpt_arm_again;
      end;
    if input_ptr<dorm_level then
      begin dorm_level:=0; incr(dorm_fired);
      end;
    if dorm_level>ckpt_arm_level then pop_watch:=dorm_level else pop_watch:=ckpt_arm_level;
    end;
  end
@z

@x l.8274 (after changes/web2c-run.ch) - a new input level is counted (READ-TRACK)
incr(in_open); push_input; index:=in_open;
full_source_filename_stack[index]:=0;
@y
incr(in_open); push_input; index:=in_open;
full_source_filename_stack[index]:=0;
flashtex_level_opened;
@z

@x l.9475 of the merged WEB - expanding the armed control sequence arms the checkpoint
begin_token_list(ref_count,macro); name:=warning_index; loc:=link(r);
@y
begin_token_list(ref_count,macro); name:=warning_index; loc:=link(r);
if ckpt_arm_cs<>0 then if warning_index=ckpt_arm_cs then
  begin ckpt_arm_level:=input_ptr; ckpt_arm_again:=ckpt_arm_cs; ckpt_arm_cs:=0;
  if pop_watch<ckpt_arm_level then pop_watch:=ckpt_arm_level;
  end;
@z

@x l.15632 of the merged WEB - a checkpoint after every shipout, when asked (section 5.2)
update_terminal; {progress report}
@<Flush the box from memory, showing statistics if requested@>;
end;
@y
update_terminal; {progress report}
@<Flush the box from memory, showing statistics if requested@>;
if ckpt_on_shipout<>0 then ckpt_request:=ckpt_on_shipout;
end;
@z

@x l.24680 of the merged WEB - checkpoints are taken, and resumed, at |big_switch|
begin if every_job<>null then begin_token_list(every_job,every_job_text);
big_switch: flashtex_poll; get_x_token;@/
@y
begin if ckpt_resuming then ckpt_resuming:=false
else if every_job<>null then begin_token_list(every_job,every_job_text);
big_switch: flashtex_poll; if ckpt_request<>0 then flashtex_checkpoint_hook;
get_x_token;@/
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The checkpoint layer's variables. They are zero when the engine is made,
and only the host (through |flashtex_checkpoint_hook|) sets them otherwise,
so an engine that is not asked for checkpoints never calls the hook.

@<Glob...@>=
@!ckpt_request:integer; {nonzero: call |flashtex_checkpoint_hook| at |big_switch|}
@!ckpt_arm_cs:pointer; {expanding this control sequence arms the checkpoint (0: none)}
@!ckpt_arm_level:integer; {|input_ptr| with the armed body on top, or 0}
@!dorm_level:integer; {nonzero: |input_ptr| below it makes the watched file level current}
@!pop_watch:integer; {the larger of |ckpt_arm_level| and |dorm_level|}
@!dorm_fired:integer; {times the watched level became current again}
@!ckpt_arm_again:pointer; {the armed control sequence, armed again when its
  body was absorbed into a definition or an argument rather than run}
@!ckpt_resuming:boolean; {enter |main_control| at |big_switch|}
@!ckpt_on_shipout:integer; {nonzero: request a checkpoint after each shipout}

@ A run restored from a checkpoint taken at |big_switch| goes on from there
to the end, as the main program does after |main_control|.

@<Last-minute...@>=
procedure resume_main_control;
begin ckpt_resuming:=true;
main_control; {come to life}
final_cleanup; {prepare for death}
close_files_and_terminate;
ready_already:=0;
end;

@ @<Declare the routines of \XeTeX's C parts@>=
procedure flashtex_checkpoint_hook; external;
  {at |big_switch| when |ckpt_request| is nonzero (src/ckpt.rs)}
procedure flashtex_level_opened; external;
  {a new input level, or a terminal line}
procedure flashtex_level_shown; external;
  {|show_context| prints a file level's line}

@* \[55] Index.
@z
