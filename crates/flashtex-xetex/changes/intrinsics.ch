% changes/intrinsics.ch -- XeTeX's guarded intrinsics (DESIGN.md section 5.6
% item 4, decision D9; the pdfTeX engine's changes/intrinsics.ch, ported to
% xetex.web with XeTeX's own command and chr codes).
%
% Nothing here changes what the program computes unless the driver switched
% the feature on (|intr_on|, `FLASHTEX_INTRINSICS=on|verify'); every hook
% calls src/intrinsics.rs and is behind a boolean:
%
% * |intr_at_switch|: |big_switch|'s own |get_x_token| is expanding (cleared
%   by |expand| and by every |macro_call|). A registered macro without
%   parameters reached so is offered to |flashtex_intr_call| before anything
%   of it is scanned; it either replays a recorded run whose every read still
%   holds (and |macro_call| returns at once, as if the body had been fed to
%   the scanner and |main_control| had executed all of it), or lets the macro
%   expand as usual, possibly recording it (`verify' mode: it is checked).
%
% * |intr_rec_on|: a recording is in progress. Reported are every state the
%   recorded run reads (the meaning of each control sequence |get_next|
%   delivers, |intr_weak| when its caller looks only at the token, |intr_let|
%   when it is a \.{\\let}'s source, whose meaning is copied; \.{\\csname}
%   and \.{\\ifcsname} look-ups with the name found; |scan_something_internal|'s
%   fetches; character codes; the register \.{\\advance} and kin read; the
%   category code and the name |print_cs| and |sprint_cs| print), every change
%   it makes (|eq_define|, |eq_word_define|, |geq_define|, |geq_word_define|,
%   |new_save_level|, |unsave|), every command and expandable primitive it
%   runs, and the points where it stops being a pure function of what it read
%   (dimensions, token-register copies, sparse arrays, box dimensions, file
%   output, leaving its own input levels or conditionals); src/intrinsics.rs
%   abandons the recording at any of those.
%
% The recordings are kept outside the word space (|Host|), so that they
% survive a restore; their guard compares every read value in full.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web - |print_cs| reads the category code of a one-character name
    else  begin print_esc(p-single_base);
      if cat_code(p-single_base)=letter then print_char(" ");
@y
    else  begin print_esc(p-single_base);
      if intr_rec_on then flashtex_intr_read(cat_code_base+p-single_base);
      if cat_code(p-single_base)=letter then print_char(" ");
@z

@x xetex.web - ... and the name at a location of the hash
  if (p>=prim_eqtb_base)and(p<frozen_null_font) then
    print_esc(prim_text(p-prim_eqtb_base)-1) else print_esc(text(p));
  print_char(" ");
@y
  if intr_rec_on then flashtex_intr_name(p);
  if (p>=prim_eqtb_base)and(p<frozen_null_font) then
    print_esc(prim_text(p-prim_eqtb_base)-1) else print_esc(text(p));
  print_char(" ");
@z

@x xetex.web - so does |sprint_cs|
else if (p>=prim_eqtb_base)and(p<frozen_null_font) then
    print_esc(prim_text(p-prim_eqtb_base)-1)
else print_esc(text(p));
@y
else if (p>=prim_eqtb_base)and(p<frozen_null_font) then
    print_esc(prim_text(p-prim_eqtb_base)-1)
else  begin if intr_rec_on then flashtex_intr_name(p);
  print_esc(text(p));
  end;
@z

@x xetex.web - a group boundary, for the recording
@p procedure new_save_level(@!c:group_code); {begin a new level of grouping}
begin check_full_save_stack;
@y
@p procedure new_save_level(@!c:group_code); {begin a new level of grouping}
begin if intr_rec_on then flashtex_intr_op(3,c,0,0);
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
begin if intr_rec_on then flashtex_intr_op(1,p,t,e);
if eTeX_ex and(eq_type(p)=t)and(equiv(p)=e) then
@z

@x xetex.web - a local word definition, for the recording
@p procedure eq_word_define(@!p:pointer;@!w:integer);
label exit;
begin if eTeX_ex and(eqtb[p].int=w) then
@y
@p procedure eq_word_define(@!p:pointer;@!w:integer);
label exit;
begin if intr_rec_on then flashtex_intr_op(2,p,w,0);
if eTeX_ex and(eqtb[p].int=w) then
@z

@x xetex.web - a global definition, for the recording
@p procedure geq_define(@!p:pointer;@!t:quarterword;@!e:halfword);
  {global |eq_define|}
begin assign_trace(p,"globally changing")@;@/
@y
@p procedure geq_define(@!p:pointer;@!t:quarterword;@!e:halfword);
  {global |eq_define|}
begin if intr_rec_on then flashtex_intr_op(9,p,t,e);
assign_trace(p,"globally changing")@;@/
@z

@x xetex.web - a global word definition, for the recording
procedure geq_word_define(@!p:pointer;@!w:integer); {global |eq_word_define|}
begin assign_trace(p,"globally changing")@;@/
@y
procedure geq_word_define(@!p:pointer;@!w:integer); {global |eq_word_define|}
begin if intr_rec_on then flashtex_intr_op(10,p,w,0);
assign_trace(p,"globally changing")@;@/
@z

@x xetex.web - a group's end, for the recording
@!a:boolean; {have we already processed an \.{\\aftergroup} ?}
begin a:=false;
@y
@!a:boolean; {have we already processed an \.{\\aftergroup} ?}
begin a:=false; if intr_rec_on then flashtex_intr_op(4,0,0,0);
@z

@x xetex.web - a meaning printed by a recorded run must not read fonts or sparse arrays
@p procedure print_meaning;
begin print_cmd_chr(cur_cmd,cur_chr);
@y
@p procedure print_meaning;
begin if intr_rec_on then flashtex_intr_meaning;
print_cmd_chr(cur_cmd,cur_chr);
@z

@x xetex.web - every control sequence a recorded run reads is reported
@<If an alignment entry has just ended, take appropriate action@>;
exit:end;
@y
@<If an alignment entry has just ended, take appropriate action@>;
exit: if intr_rec_on then flashtex_intr_next;
end;
@z

@x xetex.web - expansion other than big_switch's direct macro
reswitch:
if cur_cmd<call then @<Expand a nonmacro@>
@y
reswitch: intr_at_switch:=false;
if intr_rec_on then flashtex_intr_expand;
if cur_cmd<call then @<Expand a nonmacro@>
@z

@x xetex.web - \.{\\expandafter} only stores the token it skips
@<Expand the token after...@>=
begin get_token; t:=cur_tok; get_token;
@y
@<Expand the token after...@>=
begin intr_weak:=true; get_token; intr_weak:=false; t:=cur_tok; get_token;
@z

@x xetex.web - \.{\\csname} reads the meaning and the name it finds
flush_list(r);
if eq_type(cur_cs)=undefined_cs then
@y
flush_list(r);
if intr_rec_on then flashtex_intr_lookup(cur_cs);
if eq_type(cur_cs)=undefined_cs then
@z

@x xetex.web - a registered macro may be replayed instead of expanded
begin flashtex_poll;
save_scanner_status:=scanner_status; save_warning_index:=warning_index;
@y
begin flashtex_poll;
save_scanner_status:=scanner_status; save_warning_index:=warning_index;
if intr_at_switch then if flashtex_intr_call then return;
intr_at_switch:=false;
@z

@x xetex.web - a recorded run fetches an internal quantity
begin restart: m:=cur_chr;
@y
begin restart: m:=cur_chr;
if intr_rec_on then flashtex_intr_internal;
@z

@x xetex.web - ... or a character code
@ @<Fetch a character code from some table@>=
begin scan_usv_num;
@y
@ @<Fetch a character code from some table@>=
begin scan_usv_num;
if intr_rec_on then flashtex_intr_read(m+cur_val);
@z

@x xetex.web - a recorded run never scans a dimension
begin f:=0; arith_error:=false; cur_order:=normal; negative:=false;
@y
begin if intr_rec_on then flashtex_intr_abort(1);
f:=0; arith_error:=false; cur_order:=normal; negative:=false;
@z

@x xetex.web - the tokens of a parameter text are stored, not interpreted
begin loop begin continue: get_token; {set |cur_cmd|, |cur_chr|, |cur_tok|}
@y
begin loop begin continue: intr_weak:=true; get_token; intr_weak:=false;
  {set |cur_cmd|, |cur_chr|, |cur_tok|}
@z

@x xetex.web - ... and so are those of a body that is not expanded
loop@+  begin if xpand then @<Expand the next part of the input@>
  else get_token;
@y
loop@+  begin if xpand then @<Expand the next part of the input@>
  else  begin intr_weak:=true; get_token; intr_weak:=false;
    end;
@z

@x xetex.web - a recorded run never ends a conditional it did not begin
@ @<Pop the condition stack@>=
begin if if_stack[in_open]=cond_ptr then if_warning;
@y
@ @<Pop the condition stack@>=
begin if intr_rec_on then flashtex_intr_pop_cond;
if if_stack[in_open]=cond_ptr then if_warning;
@z

@x after changes/checkpoint.ch - big_switch ends recordings and offers its own expansions
big_switch: flashtex_poll; if ckpt_request<>0 then flashtex_checkpoint_hook;
get_x_token;@/
reswitch: @<Give diagnostic information, if requested@>;
@y
big_switch: flashtex_poll; if ckpt_request<>0 then flashtex_checkpoint_hook;
if intr_rec_on then flashtex_intr_switch;
intr_at_switch:=intr_on; get_x_token; intr_at_switch:=false;@/
reswitch: @<Give diagnostic information, if requested@>;
if intr_rec_on then flashtex_intr_command;
@z

@x xetex.web - the assignments a recorded run may make
@<Adjust \(f)for the setting of \.{\\globaldefs}@>;
case cur_cmd of
@y
@<Adjust \(f)for the setting of \.{\\globaldefs}@>;
if intr_rec_on then flashtex_intr_command;
case cur_cmd of
@z

@x xetex.web - the control sequence being defined is not interpreted
begin restart: repeat get_token;
@y
begin restart: repeat intr_weak:=true; get_token; intr_weak:=false;
@z

@x xetex.web - a \.{\\let} looks at its source's meaning only to copy it
  if n=normal then
    begin repeat get_token;
    until cur_cmd<>spacer;
    if cur_tok=other_token+"=" then
      begin get_token;
      if cur_cmd=spacer then get_token;
      end;
    end
@y
  if n=normal then
    begin intr_let:=true;
    repeat get_token;
    until cur_cmd<>spacer;
    if cur_tok=other_token+"=" then
      begin get_token;
      if cur_cmd=spacer then get_token;
      end;
    intr_let:=false;
    end
@z

@x xetex.web - a recorded run never copies a token register
  end else q:=equiv(cur_chr);
  if q=null then sa_define(p,null)(p,undefined_cs,null)
@y
  end else q:=equiv(cur_chr);
  if intr_rec_on then flashtex_intr_abort(2);
  if q=null then sa_define(p,null)(p,undefined_cs,null)
@z

@x xetex.web - \.{\\advance}, \.{\\multiply} and \.{\\divide} read the register
@<Compute the register location |l| and its type |p|; but |return| if invalid@>;
if q=register then scan_optional_equals
@y
@<Compute the register location |l| and its type |p|; but |return| if invalid@>;
if intr_rec_on then if not e then if q<>register then flashtex_intr_read(l);
if q=register then scan_optional_equals
@z

@x xetex.web - \wd, \ht, \dp write |mem|: never recorded
begin c:=cur_chr; scan_register_num; fetch_box(b); scan_optional_equals;
@y
begin if intr_rec_on then flashtex_intr_op(6,0,0,0);
c:=cur_chr; scan_register_num; fetch_box(b); scan_optional_equals;
@z

@x after changes/filenames.ch - \immediate output: never recorded
    @!old_setting:0..max_selector;
begin case subtype(p) of
@y
    @!old_setting:0..max_selector;
begin if intr_rec_on then flashtex_intr_op(7,0,0,0);
case subtype(p) of
@z

@x xetex.web - \.{\\ifcsname} reads the meaning and the name it finds
  flush_list(n);
  b:=(eq_type(cur_cs)<>undefined_cs);
@y
  flush_list(n);
  if intr_rec_on then flashtex_intr_lookup(cur_cs);
  b:=(eq_type(cur_cs)<>undefined_cs);
@z

@x xetex.web - a recorded run never uses a sparse array
begin cur_ptr:=sa_root[t];
@y
begin if intr_rec_on then flashtex_intr_abort(3);
cur_ptr:=sa_root[t];
@z

@x xetex.web - (its definitions)
procedure sa_def(@!p:pointer;@!e:halfword);
  {new data for sparse array elements}
begin add_sa_ref(p);
@y
procedure sa_def(@!p:pointer;@!e:halfword);
  {new data for sparse array elements}
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
@z

@x xetex.web - (its word variant)
procedure sa_w_def(@!p:pointer;@!w:integer);
begin add_sa_ref(p);
@y
procedure sa_w_def(@!p:pointer;@!w:integer);
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
@z

@x xetex.web - (global)
procedure gsa_def(@!p:pointer;@!e:halfword); {global |sa_def|}
begin add_sa_ref(p);
@y
procedure gsa_def(@!p:pointer;@!e:halfword); {global |sa_def|}
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
@z

@x xetex.web - (global word)
procedure gsa_w_def(@!p:pointer;@!w:integer); {global |sa_w_def|}
begin add_sa_ref(p);
@y
procedure gsa_w_def(@!p:pointer;@!w:integer); {global |sa_w_def|}
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
@z

@x xetex.web - the intrinsics' flags and external routines
@* \[55] Index.
@y
@ The guarded intrinsics' flags. They are false when the engine is made;
only the driver sets |intr_on|, and src/intrinsics.rs the others. The
recordings themselves live outside the word space.

@<Glob...@>=
@!intr_on:boolean; {a registered macro may be offered (modes `on' and `verify')}
@!intr_at_switch:boolean; {|big_switch|'s |get_x_token| is expanding}
@!intr_rec_on:boolean; {a recording is in progress}
@!intr_weak:boolean; {|get_next|'s caller looks only at the token, not its meaning}
@!intr_let:boolean; {|get_next|'s caller is \.{\\let}, reading its source}

@ @<Declare the routines of \XeTeX's C parts@>=
function flashtex_intr_call:boolean; external;
procedure flashtex_intr_op(@!k,@!p,@!t,@!e:integer); external;
procedure flashtex_intr_next; external;
procedure flashtex_intr_expand; external;
procedure flashtex_intr_read(@!p:integer); external;
procedure flashtex_intr_lookup(@!p:integer); external;
procedure flashtex_intr_name(@!p:integer); external;
procedure flashtex_intr_meaning; external;
procedure flashtex_intr_internal; external;
procedure flashtex_intr_abort(@!r:integer); external;
procedure flashtex_intr_pop_cond; external;
procedure flashtex_intr_switch; external;
procedure flashtex_intr_command; external;

@* \[55] Index.
@z
