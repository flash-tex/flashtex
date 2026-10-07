% changes/boxmemo.ch -- BOX-MEMO: guarded replay of macro calls that typeset
% only what they throw away (docs/design/engine-v2/BOX-MEMO.md, lane
% P6-INFDESC-PAGE). Applied last; its @x lines match the text the earlier
% change files leave.
%
% Nothing here changes what the program computes. Every hook is behind
% |bm_on| (the feature is switched on: |FLASHTEX_BOXMEMO|) or |bm_rec_on|
% (a recording is in progress), both false unless the driver switched the
% feature on, and calls the hand-written module src/boxmemo.rs:
%
% * |macro_call|, once the body and its arguments are fed to the scanner:
%   a registered macro (|bm_cand|) that |big_switch|'s |get_x_token| expands
%   (D9's |intr_at_switch|) is offered to |flashtex_bm_call|, which either
%   replays a recorded call whose key holds -- it pops the body with
%   |end_token_list| and makes the recorded assignments through
%   |eq_define| & co., exactly what |main_control| executing the body would
%   leave -- or starts a recording, or does nothing.
%
% * |bm_rec_on|: the recording's observers. The control sequences read
%   (|get_next|, |\csname|, |\ifcsname|), the assignments and groups
%   (|eq_define|, |eq_word_define|, |geq_define|, |geq_word_define|, e-TeX's
%   |sa_def| family, |new_save_level|, |unsave|), the box registers fetched,
%   the commands, expansions and internal quantities, conditionals popped,
%   and |prepare_mag|; src/boxmemo.rs abandons the recording at anything
%   outside its model.
%
% * |bm_font_version|, |bm_hyph_version|: a fresh number whenever a loaded
%   font's parameters or the hyphenation exceptions change, so that a key
%   naming a version names one state of those arrays.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.6585 (after changes/readset.ch) - a name looked up is a read, found or not
found: if rs_on then flashtex_id_read(j,l,p);
@y
found: if rs_on then flashtex_id_read(j,l,p);
if bm_rec_on then flashtex_bm_id(j,l,p);
@z

@x pdftex.web l.6601 (after changes/intrinsics.ch) - a new control sequence may be registered
if intr_on then flashtex_intr_new_cs(p);
@y
if intr_on then flashtex_intr_new_cs(p);
if bm_on then flashtex_bm_new_cs(p);
@z

@x pdftex.web l.7122 (after changes/intrinsics.ch) - a recorded run opens a group
begin if intr_rec_on then flashtex_intr_group(c);
@y
begin if intr_rec_on then flashtex_intr_group(c);
if bm_rec_on then flashtex_bm_group(c);
@z

@x pdftex.web l.7183 (after changes/intrinsics.ch) - |eq_define| is recorded
begin if intr_rec_on then flashtex_intr_def(p,t,e,0);
@y
begin if intr_rec_on then flashtex_intr_def(p,t,e,0);
if bm_rec_on then flashtex_bm_def(p,t,e,0);
@z

@x pdftex.web l.7200 (after changes/intrinsics.ch) - |eq_word_define| is recorded
begin if intr_rec_on then flashtex_intr_def(p,0,w,1);
@y
begin if intr_rec_on then flashtex_intr_def(p,0,w,1);
if bm_rec_on then flashtex_bm_def(p,0,w,1);
@z

@x pdftex.web l.7219 (after changes/intrinsics.ch) - |geq_define| is recorded
begin if intr_rec_on then flashtex_intr_def(p,t,e,2);
@y
begin if intr_rec_on then flashtex_intr_def(p,t,e,2);
if bm_rec_on then flashtex_bm_def(p,t,e,2);
@z

@x pdftex.web l.7227 (after changes/intrinsics.ch) - |geq_word_define| is recorded
begin if intr_rec_on then flashtex_intr_def(p,0,w,3);
@y
begin if intr_rec_on then flashtex_intr_def(p,0,w,3);
if bm_rec_on then flashtex_bm_def(p,0,w,3);
@z

@x pdftex.web l.7257 (after changes/intrinsics.ch) - a recorded run closes a group
begin if intr_rec_on then flashtex_intr_unsave;
@y
begin if intr_rec_on then flashtex_intr_unsave;
if bm_rec_on then flashtex_bm_unsave;
@z

@x pdftex.web l.7404 - \.{true} dimensions read and set |mag_set|
@p procedure prepare_mag;
begin if (mag_set>0)and(mag<>mag_set) then
@y
@p procedure prepare_mag;
begin if bm_rec_on then flashtex_bm_abort(1);
if (mag_set>0)and(mag<>mag_set) then
@z

@x pdftex.web l.8518 (after changes/throughput.ch) - every token a recording reads is reported
exit: if intr_rec_on then flashtex_intr_next;
end;
@y
exit: if intr_rec_on then flashtex_intr_next;
if bm_rec_on then flashtex_bm_next;
end;
@z

@x changes/throughput.ch [3] - ... on the fast path too
if c<0 then get_next_slow
else if intr_rec_on then flashtex_intr_next;
@y
if c<0 then get_next_slow
else begin if intr_rec_on then flashtex_intr_next;
  if bm_rec_on then flashtex_bm_next;
  end;
@z

@x pdftex.web l.8961 (after changes/intrinsics.ch) - every expansion a recording makes is checked
reswitch: intr_at_switch:=false;
if intr_rec_on then flashtex_intr_expand;
@y
reswitch: intr_at_switch:=false;
if intr_rec_on then flashtex_intr_expand;
if bm_rec_on then flashtex_bm_expand;
@z

@x pdftex.web l.9117 (after changes/intrinsics.ch) - \.{\\csname} reads the meaning it tests
if intr_rec_on then flashtex_intr_read(cur_cs);
if eq_type(cur_cs)=undefined_cs then
@y
if intr_rec_on then flashtex_intr_read(cur_cs);
if bm_rec_on then flashtex_bm_read(cur_cs);
if eq_type(cur_cs)=undefined_cs then
@z

@x pdftex.web l.9349 - a registered macro may be replayed once its body is fed
@<Feed the macro body and its parameters to the scanner@>;
exit:scanner_status:=save_scanner_status; warning_index:=save_warning_index;
@y
@<Feed the macro body and its parameters to the scanner@>;
if bm_on then if intr_at_switch then if bm_cand[warning_index] then
  flashtex_bm_call(n,save_scanner_status);
exit:scanner_status:=save_scanner_status; warning_index:=save_warning_index;
@z

@x pdftex.web l.9740 (after changes/intrinsics.ch) - a recording fetches an internal quantity
if intr_rec_on then flashtex_intr_internal;
@y
if intr_rec_on then flashtex_intr_internal;
if bm_rec_on then flashtex_bm_internal;
@z

@x pdftex.web l.11842 (after changes/intrinsics.ch) - a recording never ends a conditional it did not begin
begin if intr_rec_on then flashtex_intr_pop_cond;
@y
begin if intr_rec_on then flashtex_intr_pop_cond;
if bm_rec_on then flashtex_bm_pop_cond;
@z

@x pdftex.web l.28752 (after changes/intrinsics.ch) - |big_switch| ends recordings
big_switch: if ckpt_request<>0 then flashtex_checkpoint_hook;
@y
big_switch: if ckpt_request<>0 then flashtex_checkpoint_hook;
if bm_rec_on then flashtex_bm_switch;
@z

@x pdftex.web l.28755 (after changes/intrinsics.ch) - every command a recording runs is checked
reswitch: @<Give diagnostic information, if requested@>;
if intr_rec_on then flashtex_intr_command;
@y
reswitch: @<Give diagnostic information, if requested@>;
if intr_rec_on then flashtex_intr_command;
if bm_rec_on then flashtex_bm_command;
@z

@x pdftex.web l.31632 (after changes/intrinsics.ch) - ... and every assignment
if intr_rec_on then flashtex_intr_command;
case cur_cmd of
@t\4@>@<Assignments@>@;
@y
if intr_rec_on then flashtex_intr_command;
if bm_rec_on then flashtex_bm_command;
case cur_cmd of
@t\4@>@<Assignments@>@;
@z

@x pdftex.web l.32879 - new hyphenation exceptions are a new version of them
  else  begin new_hyph_exceptions; goto done;
@y
  else  begin if bm_on then flashtex_bm_hyph_changed;
    new_hyph_exceptions; goto done;
@z

@x pdftex.web l.32894 - changed font parameters are a new version of the fonts
assign_font_dimen: begin find_font_dimen(true); k:=cur_val;
@y
assign_font_dimen: begin if bm_on then flashtex_bm_font_changed;
  find_font_dimen(true); k:=cur_val;
@z

@x pdftex.web l.32897
assign_font_int: begin n:=cur_chr; scan_font_ident; f:=cur_val;
@y
assign_font_int: begin if bm_on then flashtex_bm_font_changed;
  n:=cur_chr; scan_font_ident; f:=cur_val;
@z

@x pdftex.web l.33519 (after changes/intrinsics.ch) - registered names are found once the format is loaded
  if intr_on then flashtex_intr_loaded;
@y
  if intr_on then flashtex_intr_loaded;
  if bm_on then flashtex_bm_loaded;
@z

@x pdftex.web l.34658 - so is \.{\\pdffontexpand}
  pdf_font_expand_code: @<Implement \.{\\pdffontexpand}@>;
@y
  pdf_font_expand_code: begin if bm_on then flashtex_bm_font_changed;
    @<Implement \.{\\pdffontexpand}@>;
    end;
@z

@x pdftex.web l.38695 (after changes/intrinsics.ch) - \.{\\ifcsname} reads the meaning it tests
  if intr_rec_on then flashtex_intr_read(cur_cs);
  b:=(eq_type(cur_cs)<>undefined_cs);
@y
  if intr_rec_on then flashtex_intr_read(cur_cs);
  if bm_rec_on then flashtex_bm_read(cur_cs);
  b:=(eq_type(cur_cs)<>undefined_cs);
@z

@x pdftex.web l.39516 - a recording that fetches a box register is checked
@d fetch_box(#)== {fetch |box(cur_val)|}
  if cur_val<256 then #:=box(cur_val)
  else  begin find_sa_element(box_val,cur_val,false);
    if cur_ptr=null then #:=null@+else #:=sa_ptr(cur_ptr);
    end
@y
@d fetch_box(#)== {fetch |box(cur_val)|}
  begin if bm_rec_on then flashtex_bm_box(cur_val);
  if cur_val<256 then #:=box(cur_val)
  else  begin find_sa_element(box_val,cur_val,false);
    if cur_ptr=null then #:=null@+else #:=sa_ptr(cur_ptr);
    end;
  end
@z

@x pdftex.web l.39955 (after changes/intrinsics.ch) - e-TeX's sparse assignments are recorded
procedure sa_def(@!p:pointer;@!e:halfword);
  {new data for sparse array elements}
begin if intr_rec_on then flashtex_intr_abort(3);
@y
procedure sa_def(@!p:pointer;@!e:halfword);
  {new data for sparse array elements}
begin if intr_rec_on then flashtex_intr_abort(3);
if bm_rec_on then flashtex_bm_sa_def(p,e,0);
@z

@x pdftex.web l.39969 (after changes/intrinsics.ch)
procedure sa_w_def(@!p:pointer;@!w:integer);
begin if intr_rec_on then flashtex_intr_abort(3);
@y
procedure sa_w_def(@!p:pointer;@!w:integer);
begin if intr_rec_on then flashtex_intr_abort(3);
if bm_rec_on then flashtex_bm_sa_def(p,w,1);
@z

@x pdftex.web l.39988 (after changes/intrinsics.ch)
procedure gsa_def(@!p:pointer;@!e:halfword); {global |sa_def|}
begin if intr_rec_on then flashtex_intr_abort(3);
@y
procedure gsa_def(@!p:pointer;@!e:halfword); {global |sa_def|}
begin if intr_rec_on then flashtex_intr_abort(3);
if bm_rec_on then flashtex_bm_sa_def(p,e,2);
@z

@x pdftex.web l.39996 (after changes/intrinsics.ch)
procedure gsa_w_def(@!p:pointer;@!w:integer); {global |sa_w_def|}
begin if intr_rec_on then flashtex_intr_abort(3);
@y
procedure gsa_w_def(@!p:pointer;@!w:integer); {global |sa_w_def|}
begin if intr_rec_on then flashtex_intr_abort(3);
if bm_rec_on then flashtex_bm_sa_def(p,w,3);
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ BOX-MEMO's state (docs/design/engine-v2/BOX-MEMO.md). The recordings
themselves live in src/boxmemo.rs, outside the word space, so that an edit's
restore keeps them; the guard checks every value they depend on.

@<Glob...@>=
@!bm_on:boolean; {BOX-MEMO is switched on}
@!bm_rec_on:boolean; {a BOX-MEMO recording is in progress}
@!bm_cand:array[0..eqtb_top] of boolean; {a registered macro}
@!bm_font_version:integer; {names the state of the loaded fonts' parameters}
@!bm_hyph_version:integer; {names the state of the hyphenation exceptions}

@ @<Set init...@>=
bm_rec_on:=false; bm_font_version:=0; bm_hyph_version:=0;
bm_on:=flashtex_bm_enabled;

@ @<Declare the routines of pdf\TeX's C parts@>=
function flashtex_bm_enabled:boolean; external;
procedure flashtex_bm_loaded; external;
procedure flashtex_bm_new_cs(@!p:pointer); external;
procedure flashtex_bm_call(@!n:integer;@!s:integer); external;
procedure flashtex_bm_group(@!c:group_code); external;
procedure flashtex_bm_unsave; external;
procedure flashtex_bm_def(@!p:pointer;@!t:quarterword;@!e:integer;@!k:integer); external;
procedure flashtex_bm_sa_def(@!p:pointer;@!e:integer;@!k:integer); external;
procedure flashtex_bm_next; external;
procedure flashtex_bm_expand; external;
procedure flashtex_bm_read(@!p:pointer); external;
procedure flashtex_bm_id(@!j:integer;@!l:integer;@!p:pointer); external;
procedure flashtex_bm_internal; external;
procedure flashtex_bm_pop_cond; external;
procedure flashtex_bm_switch; external;
procedure flashtex_bm_command; external;
procedure flashtex_bm_box(@!n:integer); external;
procedure flashtex_bm_abort(@!r:integer); external;
procedure flashtex_bm_font_changed; external;
procedure flashtex_bm_hyph_changed; external;

@* \[55] Index.
@z
