% changes/intrinsics.ch -- the macro-level profiler and the guarded
% intrinsics (DESIGN.md section 5.6, items 1, 4 and 6; decision D9).
%
% Nothing here changes what the program computes. The hooks call the
% hand-written module src/intrinsics.rs (and src/macroprof.rs), and each is
% behind a boolean that is false unless the driver switched the feature on:
%
% * |macro_prof_on|: the macro-level profiler. |macro_call| reports each
%   macro body it feeds to the scanner, |end_token_list| each macro level it
%   leaves (|flashtex_prof_enter|, |flashtex_prof_leave|). They only read.
%
% * |intr_on|: guarded intrinsics. A registered macro without parameters
%   that |big_switch|'s |get_x_token| is about to expand (|intr_at_switch|)
%   is offered to |flashtex_intr_call| first, which either replays a
%   recorded, still valid run of it and returns |true| (|macro_call| then
%   returns at once, exactly as if the body had been fed to the scanner and
%   |main_control| had executed all of it), or returns |false|, and the macro
%   is expanded as usual -- possibly while |intr_rec_on| records it.
%
% * |intr_rec_on|: a recording is in progress. The hooks behind it report
%   every state the recorded run reads (the meaning of every control
%   sequence |get_next| delivers, |\csname| and |\ifcsname| look-ups, the
%   internal quantities |scan_something_internal| fetches), every change it
%   makes (|eq_define|, |geq_define|, |eq_word_define|, |geq_word_define|,
%   |new_save_level|, |unsave|), every command and expandable primitive it
%   runs, and the points where a run stops being a pure function of what it
%   read (sparse registers, dimensions, token-register copies, leaving its
%   own input levels, conditionals or groups); src/intrinsics.rs abandons
%   the recording at any of those.
%
% * |intr_watch[p]|, for the guard: nonzero when some recorded run read
%   |eqtb[p]|; every write to such an entry is reported
%   (|flashtex_intr_touch|), which keeps each recording's count of entries
%   that no longer hold the value it read. The writes are those of
%   |eq_define|, |geq_define| and |unsave|: every other routine that stores
%   into |eqtb| below |int_base| writes a box register or a font identifier,
%   which a recording never reads (docs/evidence/l6-intrinsics-2026-09-29/
%   lists them); recorded integer and dimension entries are compared by
%   value instead.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.6601 - a new control sequence may be a registered intrinsic
text(p):=make_string; pool_ptr:=pool_ptr+d;
@!stat incr(cs_count);@+tats@;@/
end
@y
text(p):=make_string; pool_ptr:=pool_ptr+d;
@!stat incr(cs_count);@+tats@;@/
if intr_on then flashtex_intr_new_cs(p);
end
@z

@x pdftex.web l.7122 - a recorded run opens a group
@p procedure new_save_level(@!c:group_code); {begin a new level of grouping}
begin check_full_save_stack;
@y
@p procedure new_save_level(@!c:group_code); {begin a new level of grouping}
begin if intr_rec_on then flashtex_intr_group(c);
check_full_save_stack;
@z

@x pdftex.web l.7183 - |eq_define| is recorded, and reported when watched
begin if eTeX_ex and(eq_type(p)=t)and(equiv(p)=e) then
  begin assign_trace(p,"reassigning")@;@/
  eq_destroy(eqtb[p]); return;
  end;
assign_trace(p,"changing")@;@/
if eq_level(p)=cur_level then eq_destroy(eqtb[p])
else if cur_level>level_one then eq_save(p,eq_level(p));
eq_level(p):=cur_level; eq_type(p):=t; equiv(p):=e;
@y
begin if intr_rec_on then flashtex_intr_def(p,t,e,0);
if eTeX_ex and(eq_type(p)=t)and(equiv(p)=e) then
  begin assign_trace(p,"reassigning")@;@/
  eq_destroy(eqtb[p]); return;
  end;
assign_trace(p,"changing")@;@/
if eq_level(p)=cur_level then eq_destroy(eqtb[p])
else if cur_level>level_one then eq_save(p,eq_level(p));
eq_level(p):=cur_level; eq_type(p):=t; equiv(p):=e;
if intr_watch[p]<>0 then flashtex_intr_touch(p);
@z

@x pdftex.web l.7200 - |eq_word_define| is recorded
begin if eTeX_ex and(eqtb[p].int=w) then
@y
begin if intr_rec_on then flashtex_intr_def(p,0,w,1);
if eTeX_ex and(eqtb[p].int=w) then
@z

@x pdftex.web l.7219 - |geq_define| is recorded, and reported when watched
begin assign_trace(p,"globally changing")@;@/
begin eq_destroy(eqtb[p]);
eq_level(p):=level_one; eq_type(p):=t; equiv(p):=e;
end;
@y
begin if intr_rec_on then flashtex_intr_def(p,t,e,2);
assign_trace(p,"globally changing")@;@/
begin eq_destroy(eqtb[p]);
eq_level(p):=level_one; eq_type(p):=t; equiv(p):=e;
end;
if intr_watch[p]<>0 then flashtex_intr_touch(p);
@z

@x pdftex.web l.7227 - |geq_word_define| is recorded
begin assign_trace(p,"globally changing")@;@/
begin eqtb[p].int:=w; xeq_level[p]:=level_one;
@y
begin if intr_rec_on then flashtex_intr_def(p,0,w,3);
assign_trace(p,"globally changing")@;@/
begin eqtb[p].int:=w; xeq_level[p]:=level_one;
@z

@x pdftex.web l.7257 - a recorded run closes a group
begin a:=false;
if cur_level>level_one then
@y
begin if intr_rec_on then flashtex_intr_unsave;
a:=false;
if cur_level>level_one then
@z

@x pdftex.web l.7300 - restoring a watched entry is reported
    eqtb[p]:=save_stack[save_ptr]; {restore the saved value}
@y
    eqtb[p]:=save_stack[save_ptr]; {restore the saved value}
    if intr_watch[p]<>0 then flashtex_intr_touch(p);
@z

@x pdftex.web l.8262 - leaving a macro level is reported to the profiler
else if token_type=u_template then
  if align_state>500000 then align_state:=0
  else fatal_error("(interwoven alignment preambles are not allowed)");
@.interwoven alignment preambles...@>
pop_input;
@y
else if token_type=u_template then
  if align_state>500000 then align_state:=0
  else fatal_error("(interwoven alignment preambles are not allowed)");
@.interwoven alignment preambles...@>
if macro_prof_on then if token_type=macro then flashtex_prof_leave;
pop_input;
@z

@x pdftex.web l.8518 - every token a recorded run reads is reported
@<If an alignment entry has just ended, take appropriate action@>;
exit:end;
@y
@<If an alignment entry has just ended, take appropriate action@>;
exit: if intr_rec_on then flashtex_intr_next;
end;
@z

@x pdftex.web l.8961 - an expansion that is not |big_switch|'s own ends |intr_at_switch|
reswitch:
if cur_cmd<call then @<Expand a nonmacro@>
@y
reswitch: intr_at_switch:=false;
if intr_rec_on then flashtex_intr_expand;
if cur_cmd<call then @<Expand a nonmacro@>
@z

@x pdftex.web l.9117 - \.{\\csname} reads the meaning it tests
if eq_type(cur_cs)=undefined_cs then
  begin eq_define(cur_cs,relax,256); {N.B.: The |save_stack| might change}
@y
if intr_rec_on then flashtex_intr_read(cur_cs);
if eq_type(cur_cs)=undefined_cs then
  begin eq_define(cur_cs,relax,256); {N.B.: The |save_stack| might change}
@z

@x pdftex.web l.9342 - a registered macro may be replayed instead of expanded
begin save_scanner_status:=scanner_status; save_warning_index:=warning_index;
@y
begin save_scanner_status:=scanner_status; save_warning_index:=warning_index;
if intr_at_switch then if (intr_cand[cur_cs]<>0)or intr_all then
  if flashtex_intr_call then return;
@z

@x pdftex.web l.9360 (after changes/checkpoint.ch) - entering a macro body is reported
begin_token_list(ref_count,macro); name:=warning_index; loc:=link(r);
if ckpt_arm_cs<>null then if warning_index=ckpt_arm_cs then
  begin ckpt_arm_level:=input_ptr; ckpt_arm_cs:=null;
  end;
@y
begin_token_list(ref_count,macro); name:=warning_index; loc:=link(r);
if ckpt_arm_cs<>null then if warning_index=ckpt_arm_cs then
  begin ckpt_arm_level:=input_ptr; ckpt_arm_cs:=null;
  end;
if macro_prof_on then flashtex_prof_enter(warning_index);
if intr_rec_on then flashtex_intr_fed;
@z

@x pdftex.web l.9740 - a recorded run fetches an internal quantity
begin restart: m:=cur_chr;
@y
begin restart: m:=cur_chr;
if intr_rec_on then flashtex_intr_internal;
@z

@x pdftex.web l.9769 - ... or a character code
begin scan_char_num;
if m=math_code_base then scanned_result(ho(math_code(cur_val)))(int_val)
@y
begin scan_char_num;
if intr_rec_on then flashtex_intr_read(m+cur_val);
if m=math_code_base then scanned_result(ho(math_code(cur_val)))(int_val)
@z

@x pdftex.web l.10446 - a recorded run never scans a dimension
begin f:=0; arith_error:=false; cur_order:=normal; negative:=false;
@y
begin if intr_rec_on then flashtex_intr_abort(1);
f:=0; arith_error:=false; cur_order:=normal; negative:=false;
@z

@x pdftex.web l.11518 - the tokens of a parameter text are stored, not interpreted
begin loop begin continue: get_token; {set |cur_cmd|, |cur_chr|, |cur_tok|}
@y
begin loop begin continue: intr_weak:=true; get_token; intr_weak:=false;
  {set |cur_cmd|, |cur_chr|, |cur_tok|}
@z

@x pdftex.web l.11565 - ... and so are those of a body that is not expanded
loop@+  begin if xpand then @<Expand the next part of the input@>
  else get_token;
@y
loop@+  begin if xpand then @<Expand the next part of the input@>
  else begin intr_weak:=true; get_token; intr_weak:=false;
    end;
@z

@x pdftex.web l.11842 - a recorded run never ends a conditional it did not begin
@ @<Pop the condition stack@>=
begin if if_stack[in_open]=cond_ptr then if_warning;
@y
@ @<Pop the condition stack@>=
begin if intr_rec_on then flashtex_intr_pop_cond;
if if_stack[in_open]=cond_ptr then if_warning;
@z

@x pdftex.web l.28752 (after changes/checkpoint.ch) - |big_switch| ends recordings and marks its own expansions
big_switch: if ckpt_request<>0 then flashtex_checkpoint_hook;
get_x_token;@/
reswitch: @<Give diagnostic information, if requested@>;
@y
big_switch: if ckpt_request<>0 then flashtex_checkpoint_hook;
if intr_on then
  begin if intr_rec_on then flashtex_intr_switch;
  intr_at_switch:=true;
  end;
get_x_token; intr_at_switch:=false;@/
reswitch: @<Give diagnostic information, if requested@>;
if intr_rec_on then flashtex_intr_command;
@z

@x pdftex.web l.31632 - the assignments a recorded run may make
@<Adjust \(f)for the setting of \.{\\globaldefs}@>;
case cur_cmd of
@y
@<Adjust \(f)for the setting of \.{\\globaldefs}@>;
if intr_rec_on then flashtex_intr_command;
case cur_cmd of
@z

@x pdftex.web l.31692 - the control sequence being defined is not interpreted
begin restart: repeat get_token;
until cur_tok<>space_token;
@y
begin restart: repeat intr_weak:=true; get_token; intr_weak:=false;
until cur_tok<>space_token;
@z

@x pdftex.web l.31906 - a recorded run never copies a token register
  else q:=equiv(cur_chr);
  if q=null then sa_define(p,null)(p,undefined_cs,null)
@y
  else q:=equiv(cur_chr);
  if intr_rec_on then flashtex_intr_abort(2);
  if q=null then sa_define(p,null)(p,undefined_cs,null)
@z

@x pdftex.web l.31990 - \.{\\advance}, \.{\\multiply} and \.{\\divide} read the register
@<Compute the register location |l| and its type |p|; but |return| if invalid@>;
if q=register then scan_optional_equals
@y
@<Compute the register location |l| and its type |p|; but |return| if invalid@>;
if intr_rec_on then if not e then if q<>register then flashtex_intr_read(l);
if q=register then scan_optional_equals
@z

@x pdftex.web l.33519 - intrinsics named in the format are found once it is loaded
  w_close(fmt_file);
  while (loc<limit)and(buffer[loc]=" ") do incr(loc);
@y
  w_close(fmt_file);
  if intr_on then flashtex_intr_loaded;
  while (loc<limit)and(buffer[loc]=" ") do incr(loc);
@z

@x pdftex.web l.38695 - \.{\\ifcsname} reads the meaning it tests
  b:=(eq_type(cur_cs)<>undefined_cs);
  is_in_csname := e;
@y
  if intr_rec_on then flashtex_intr_read(cur_cs);
  b:=(eq_type(cur_cs)<>undefined_cs);
  is_in_csname := e;
@z

@x pdftex.web l.39543 - a recorded run never uses a sparse register
label not_found,not_found1,not_found2,not_found3,not_found4,exit;
var q:pointer; {for list manipulations}
@!i:small_number; {a four bit index}
begin cur_ptr:=sa_root[t];
@y
label not_found,not_found1,not_found2,not_found3,not_found4,exit;
var q:pointer; {for list manipulations}
@!i:small_number; {a four bit index}
begin if intr_rec_on then flashtex_intr_abort(3);
cur_ptr:=sa_root[t];
@z

@x pdftex.web l.39955
begin add_sa_ref(p);
if sa_ptr(p)=e then
@y
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
if sa_ptr(p)=e then
@z

@x pdftex.web l.39969
begin add_sa_ref(p);
if sa_int(p)=w then
@y
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
if sa_int(p)=w then
@z

@x pdftex.web l.39988
begin add_sa_ref(p);
@!stat if tracing_assigns>0 then show_sa(p,"globally changing");@+tats@;@/
sa_destroy(p); sa_lev(p):=level_one; sa_ptr(p):=e;
@y
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
@!stat if tracing_assigns>0 then show_sa(p,"globally changing");@+tats@;@/
sa_destroy(p); sa_lev(p):=level_one; sa_ptr(p):=e;
@z

@x pdftex.web l.39996
begin add_sa_ref(p);
@!stat if tracing_assigns>0 then show_sa(p,"globally changing");@+tats@;@/
sa_lev(p):=level_one; sa_int(p):=w;
@y
begin if intr_rec_on then flashtex_intr_abort(3);
add_sa_ref(p);
@!stat if tracing_assigns>0 then show_sa(p,"globally changing");@+tats@;@/
sa_lev(p):=level_one; sa_int(p):=w;
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The profiler's switch. It is false when the engine is made; only the
driver sets it.

@<Glob...@>=
@!macro_prof_on:boolean; {report macro levels to the profiler}

@ The guarded intrinsics' state. Everything a recording or its guard
needs lives here, in the engine's word space, so that a checkpoint and
a restore (\S5.2 of DESIGN.md) carry it along with the |eqtb| and |mem|
it describes. |intr_state| and |intr_data| are laid out by
src/intrinsics.rs.

@d intr_state_size=4095 {scalars and per-slot records}
@d intr_data_size=8388607 {watch records, recorded reads, operations}

@<Glob...@>=
@!intr_on:boolean; {some intrinsics are registered}
@!intr_at_switch:boolean; {|big_switch|'s |get_x_token| is expanding}
@!intr_rec_on:boolean; {a recording is in progress}
@!intr_all:boolean; {every parameterless macro is a candidate (a stress test)}
@!intr_weak:boolean; {|get_next|'s caller looks only at the token, not its meaning}
@!intr_state:array[0..intr_state_size] of integer;
@!intr_cand:array[0..eqtb_top] of integer; {first slot of a registered macro, plus one}
@!intr_watch:array[0..eqtb_top] of integer; {first watch record of |eqtb[p]|, or 0}
@!intr_seen:array[0..eqtb_top] of integer; {how the current recording has used |eqtb[p]|}
@!intr_pre:array[0..eqtb_top] of memory_word; {what |eqtb[p]| held before the recording wrote it}
@!intr_data:array[0..intr_data_size] of integer;

@ The configuration-dependent layout of |eqtb| is handed to
src/intrinsics.rs through |intr_state|, so that the e-trip build, whose
hash is smaller, is served by the same code.

@<Set init...@>=
intr_state[100]:=hash_base; intr_state[101]:=frozen_control_sequence;
intr_state[102]:=font_id_base; intr_state[103]:=undefined_control_sequence;
intr_state[104]:=glue_base; intr_state[105]:=local_base;
intr_state[106]:=toks_base; intr_state[107]:=box_base;
intr_state[108]:=cur_font_loc; intr_state[109]:=cat_code_base;
intr_state[110]:=int_base; intr_state[111]:=eqtb_size;
intr_state[112]:=single_base; intr_state[113]:=null_cs;
intr_state[114]:=math_font_base; intr_state[115]:=lc_code_base;
intr_state[116]:=count_base; intr_state[117]:=dimen_base;
intr_state[118]:=del_code_base; intr_state[119]:=math_code_base;
intr_state[120]:=hash_prime; intr_state[121]:=eqtb_top;
intr_on:=flashtex_intr_enabled;

@ @<Declare the routines of pdf\TeX's C parts@>=
function flashtex_intr_enabled:boolean; external;
procedure flashtex_intr_loaded; external;
procedure flashtex_prof_enter(@!cs:pointer); external;
procedure flashtex_prof_leave; external;
function flashtex_intr_call:boolean; external;
procedure flashtex_intr_new_cs(@!p:pointer); external;
procedure flashtex_intr_group(@!c:group_code); external;
procedure flashtex_intr_unsave; external;
procedure flashtex_intr_def(@!p:pointer;@!t:quarterword;@!e:integer;@!k:integer); external;
procedure flashtex_intr_touch(@!p:pointer); external;
procedure flashtex_intr_next; external;
procedure flashtex_intr_expand; external;
procedure flashtex_intr_read(@!p:pointer); external;
procedure flashtex_intr_fed; external;
procedure flashtex_intr_internal; external;
procedure flashtex_intr_abort(@!r:integer); external;
procedure flashtex_intr_pop_cond; external;
procedure flashtex_intr_switch; external;
procedure flashtex_intr_command; external;

@* \[55] Index.
@z
