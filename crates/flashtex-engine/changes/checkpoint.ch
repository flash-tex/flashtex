% changes/checkpoint.ch -- the points where the incremental engine may take a
% checkpoint (DESIGN.md sections 5.1 and 5.2).
%
% A checkpoint captures the whole engine state, so it can only be taken where
% the Pascal program itself holds nothing on the (Rust) call stack that the
% state does not describe. That point is |big_switch| in |main_control|:
% between two commands, with every local of |main_control| dead. There the
% engine calls the hand-written |flashtex_checkpoint_hook| (src/checkpoint.rs)
% whenever |ckpt_request| is nonzero, and a restored run re-enters
% |main_control| at |big_switch| (|ckpt_resuming| skips \.{\\everyjob}, which
% the uninterrupted run inserted long before).
%
% The begin-document snapshot $S_0$ (section 5.1) is armed without looking at
% any token: the host names a control sequence (\.{\\document}, whose
% expansion LaTeX's \.{\\begin\{document\}} ends in); when |macro_call| pushes
% its body, the input level is remembered, and when |pop_input| goes below
% that level the body -- with everything it expanded to -- has been
% consumed, and the next |big_switch| takes $S_0$. None of this changes what
% the program computes: the new variables are read only by these tests and
% by the hook, and the hook itself only reads the state.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.8211 - leaving the armed input level requests the checkpoint
@d pop_input==@t@> {leave an input level, re-enter the old}
  begin decr(input_ptr); cur_input:=input_stack[input_ptr];
  end
@y
@d pop_input==@t@> {leave an input level, re-enter the old}
  begin decr(input_ptr); cur_input:=input_stack[input_ptr];
  if input_ptr<ckpt_arm_level then
    begin ckpt_arm_level:=0; ckpt_request:=2;
    end;
  end
@z

@x pdftex.web l.9360 - expanding the armed control sequence arms the checkpoint
begin_token_list(ref_count,macro); name:=warning_index; loc:=link(r);
@y
begin_token_list(ref_count,macro); name:=warning_index; loc:=link(r);
if ckpt_arm_cs<>null then if warning_index=ckpt_arm_cs then
  begin ckpt_arm_level:=input_ptr; ckpt_arm_cs:=null;
  end;
@z

@x pdftex.web l.19817 - a checkpoint after every shipout, when asked (section 5.2)
@p procedure ship_out(p:pointer); {output the box |p|}
begin
    fix_pdfoutput;
    if pdf_output > 0 then
        pdf_ship_out(p, true)
    else
        dvi_ship_out(p);
end;
@y
@p procedure ship_out(p:pointer); {output the box |p|}
begin
    fix_pdfoutput;
    if pdf_output > 0 then
        pdf_ship_out(p, true)
    else
        dvi_ship_out(p);
    if ckpt_on_shipout<>0 then ckpt_request:=ckpt_on_shipout;
end;
@z

@x pdftex.web l.28751 - checkpoints are taken, and resumed, at |big_switch|
begin if every_job<>null then begin_token_list(every_job,every_job_text);
big_switch: get_x_token;@/
@y
begin if ckpt_resuming then ckpt_resuming:=false
else if every_job<>null then begin_token_list(every_job,every_job_text);
big_switch: if ckpt_request<>0 then flashtex_checkpoint_hook;
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
@!ckpt_arm_cs:pointer; {expanding this control sequence arms the checkpoint}
@!ckpt_arm_level:integer; {|input_ptr| with the armed body on top, or 0}
@!ckpt_resuming:boolean; {enter |main_control| at |big_switch|}
@!ckpt_on_shipout:integer; {nonzero: request a checkpoint after each shipout}

@ @<Declare the routines of pdf\TeX's C parts@>=
procedure flashtex_checkpoint_hook; external;

@* \[55] Index.
@z
