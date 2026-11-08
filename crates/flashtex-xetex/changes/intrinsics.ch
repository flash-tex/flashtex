% Verification only: no macro body is skipped, and every output comes from
% the real path. The entry flag is cleared by expand, so only big_switch's
% get_x_token can offer a macro; nested expansions remain ordinary XeTeX.
%
% In verify mode (src/intrinsics.rs) the definitions, word definitions and
% group boundaries the real path makes are reported to |flashtex_intr_op|
% (kinds: 1 |eq_define|, 2 |eq_word_define|, plus 8 when global; 3
% |new_save_level|, 4 |unsave|; 5 a sparse-array definition, 6 a box
% dimension, 7 |out_what|, which a recording cannot hold), and |big_switch| asks |flashtex_intr_switch|
% whether an observed body is used up. Off and `on' never call either.
%
% The verifier compares every |mem| word either path wrote but those of the
% cells a path allocated or freed (|flashtex_intr_cell|, |flashtex_intr_flush|
% in |get_avail|, |fast_get_avail|, |free_avail|, |flush_list|, |get_node|,
% |free_node|): an in-place change of a node the call found (|\limits|'s
% |subtype(tail)|) is a difference.

@x after changes/displaylist.ch - get_avail: a cell the path allocates (verify)
dl_new_node(p);
get_avail:=p;
@y
dl_new_node(p);
if intr_verify_on then flashtex_intr_cell(p,1);
get_avail:=p;
@z

@x xetex.web - free_avail: a cell the path frees (verify)
@d free_avail(#)== {single-word node liberation}
  begin link(#):=avail; avail:=#;
@y
@d free_avail(#)== {single-word node liberation}
  begin if intr_verify_on then flashtex_intr_cell(#,1);
  link(#):=avail; avail:=#;
@z

@x after changes/displaylist.ch - fast_get_avail: a cell the path allocates (verify)
    dl_new_node(#);
    end;
@y
    dl_new_node(#);
    if intr_verify_on then flashtex_intr_cell(#,1);
    end;
@z

@x xetex.web - flush_list: the cells the path frees (verify)
begin if p<>null then
  begin r:=p;
  repeat q:=r; r:=link(r); @!stat decr(dyn_used);@+tats@/
@y
begin if p<>null then
  begin if intr_verify_on then flashtex_intr_flush(p);
  r:=p;
  repeat q:=r; r:=link(r); @!stat decr(dyn_used);@+tats@/
@z

@x after changes/displaylist.ch - get_node: the cells the path allocates (verify)
found: link(r):=null; {this node is now nonempty}
dl_new_node(r);
@y
found: link(r):=null; {this node is now nonempty}
dl_new_node(r);
if intr_verify_on then flashtex_intr_cell(r,s);
@z

@x xetex.web - free_node: the cells the path frees (verify)
begin node_size(p):=s; link(p):=empty_flag;
@y
begin if intr_verify_on then flashtex_intr_cell(p,s);
node_size(p):=s; link(p):=empty_flag;
@z

@x xetex.web - a group boundary, for the recording
@p procedure new_save_level(@!c:group_code); {begin a new level of grouping}
begin check_full_save_stack;
@y
@p procedure new_save_level(@!c:group_code); {begin a new level of grouping}
begin if intr_verify_on then flashtex_intr_op(3,c,0,0);
check_full_save_stack;
@z

@x xetex.web - a local definition, for the recording
@p procedure eq_define(@!p:pointer;@!t:quarterword;@!e:halfword);
  {new data for |eqtb|}
label exit;
begin if eTeX_ex and(eq_type(p)=t)and(equiv(p)=e) then
@y
@p procedure eq_define(@!p:pointer;@!t:quarterword;@!e:halfword);
  {new data for |eqtb|}
label exit;
begin if intr_verify_on then flashtex_intr_op(1,p,t,e);
if eTeX_ex and(eq_type(p)=t)and(equiv(p)=e) then
@z

@x xetex.web - a local word definition, for the recording
@p procedure eq_word_define(@!p:pointer;@!w:integer);
label exit;
begin if eTeX_ex and(eqtb[p].int=w) then
@y
@p procedure eq_word_define(@!p:pointer;@!w:integer);
label exit;
begin if intr_verify_on then flashtex_intr_op(2,p,w,0);
if eTeX_ex and(eqtb[p].int=w) then
@z

@x xetex.web - a global definition, for the recording
@p procedure geq_define(@!p:pointer;@!t:quarterword;@!e:halfword);
  {global |eq_define|}
begin assign_trace(p,"globally changing")@;@/
@y
@p procedure geq_define(@!p:pointer;@!t:quarterword;@!e:halfword);
  {global |eq_define|}
begin if intr_verify_on then flashtex_intr_op(9,p,t,e);
assign_trace(p,"globally changing")@;@/
@z

@x xetex.web - a global word definition, for the recording
procedure geq_word_define(@!p:pointer;@!w:integer); {global |eq_word_define|}
begin assign_trace(p,"globally changing")@;@/
@y
procedure geq_word_define(@!p:pointer;@!w:integer); {global |eq_word_define|}
begin if intr_verify_on then flashtex_intr_op(10,p,w,0);
assign_trace(p,"globally changing")@;@/
@z

@x xetex.web - a group's end, for the recording
@!a:boolean; {have we already processed an \.{\\aftergroup} ?}
begin a:=false;
@y
@!a:boolean; {have we already processed an \.{\\aftergroup} ?}
begin a:=false; if intr_verify_on then flashtex_intr_op(4,0,0,0);
@z

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
if intr_verify_on then flashtex_intr_switch;
intr_at_switch:=intr_verify_on; get_x_token; intr_at_switch:=false;@/
@z

@x xetex.web - \wd, \ht, \dp write |mem|: not replayable (kind 6)
begin c:=cur_chr; scan_register_num; fetch_box(b); scan_optional_equals;
@y
begin if intr_verify_on then flashtex_intr_op(6,0,0,0);
c:=cur_chr; scan_register_num; fetch_box(b); scan_optional_equals;
@z

@x after changes/filenames.ch - \immediate output: not replayable (kind 7)
    @!old_setting:0..max_selector;
begin case subtype(p) of
@y
    @!old_setting:0..max_selector;
begin if intr_verify_on then flashtex_intr_op(7,0,0,0);
case subtype(p) of
@z

@x xetex.web - a sparse-array definition ends a recording's replayability
procedure sa_def(@!p:pointer;@!e:halfword);
  {new data for sparse array elements}
begin add_sa_ref(p);
@y
procedure sa_def(@!p:pointer;@!e:halfword);
  {new data for sparse array elements}
begin if intr_verify_on then flashtex_intr_op(5,0,0,0);
add_sa_ref(p);
@z

@x xetex.web - (its word variant)
procedure sa_w_def(@!p:pointer;@!w:integer);
begin add_sa_ref(p);
@y
procedure sa_w_def(@!p:pointer;@!w:integer);
begin if intr_verify_on then flashtex_intr_op(5,0,0,0);
add_sa_ref(p);
@z

@x xetex.web - (global)
procedure gsa_def(@!p:pointer;@!e:halfword); {global |sa_def|}
begin add_sa_ref(p);
@y
procedure gsa_def(@!p:pointer;@!e:halfword); {global |sa_def|}
begin if intr_verify_on then flashtex_intr_op(5,0,0,0);
add_sa_ref(p);
@z

@x xetex.web - (global word)
procedure gsa_w_def(@!p:pointer;@!w:integer); {global |sa_w_def|}
begin add_sa_ref(p);
@y
procedure gsa_w_def(@!p:pointer;@!w:integer); {global |sa_w_def|}
begin if intr_verify_on then flashtex_intr_op(5,0,0,0);
add_sa_ref(p);
@z

@x xetex.web - verification state and external observer
@* \[55] Index.
@y
@ Verification-only observer state.
@<Glob...@>=
@!intr_verify_on:boolean; {verify/report mode; output stays the real path's}
@!intr_at_switch:boolean; {this macro came directly from |big_switch|}

@ @<Declare the routines of \XeTeX's C parts@>=
procedure flashtex_intr_observe; external;
procedure flashtex_intr_switch; external;
procedure flashtex_intr_op(@!k,@!p,@!t,@!e:integer); external;
procedure flashtex_intr_cell(@!p,@!s:integer); external;
procedure flashtex_intr_flush(@!p:integer); external;

@* \[55] Index.
@z
