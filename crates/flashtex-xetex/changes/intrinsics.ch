% Verification-only candidate observation. No macro body is skipped or replayed.
% The entry flag is cleared by expand, so only big_switch's get_x_token
% can offer a macro; nested expansions remain ordinary XeTeX.

@x xetex.web - expansion other than big_switch's direct macro
reswitch:
if cur_cmd<call then @<Expand a nonmacro@>
@y
reswitch: intr_at_switch:=false;
if cur_cmd<call then @<Expand a nonmacro@>
@z

@x xetex.web - discover a candidate before any macro scanning
begin flashtex_poll;
save_scanner_status:=scanner_status; save_warning_index:=warning_index;
@y
begin flashtex_poll;
save_scanner_status:=scanner_status; save_warning_index:=warning_index;
if intr_verify_on then flashtex_intr_observe;
intr_at_switch:=false;
@z

@x after changes/checkpoint.ch - observe the main-control entry
big_switch: flashtex_poll; if ckpt_request<>0 then flashtex_checkpoint_hook;
get_x_token;@/
@y
big_switch: flashtex_poll; if ckpt_request<>0 then flashtex_checkpoint_hook;
intr_at_switch:=intr_verify_on; get_x_token; intr_at_switch:=false;@/
@z

@x xetex.web - verification state and external observer
@* \[55] Index.
@y
@ Verification-only observer state.
@<Glob...@>=
@!intr_verify_on:boolean; {set only for verify/report modes; replay is absent}
@!intr_at_switch:boolean; {this macro came directly from |big_switch|}

@ @<Declare the routines of \XeTeX's C parts@>=
procedure flashtex_intr_observe; external;

@* \[55] Index.
@z
