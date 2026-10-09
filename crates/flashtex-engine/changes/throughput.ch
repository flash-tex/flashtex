% changes/throughput.ch -- faster routines that compute exactly what
% pdftex.web's compute (DESIGN.md section 4.2, "allowed optimisations, each
% proven identical by the lockstep harness"; section 5.6 L6; lane
% P6-THROUGHPUT, docs/evidence/p6-throughput-2026-09-30/).
%
% Each change carries its proof of identity in the comment above it. None
% changes a result, a side effect or an error; the lockstep, trip, etrip
% and parity gates check that.
%
% [1] get_next (section 363): the external-file part out of line.
%
% get_next runs once per token, and nearly all tokens of a LaTeX run come
% from token lists. Its external-file part (reading characters, scanning
% control-sequence names, moving to the next line) is most of its code:
% compiled together, get_next was 1,269 instructions, 246 without it, and
% the smaller routine retires 3% fewer instructions on the full-300
% benchmark (docs/evidence/p6-throughput-2026-09-30/). So that part becomes
% the function get_next_file, with the same statements in the same order,
% and get_next calls it. Its three ways out are its result: 0 for its
% `goto restart' (get_next goes to restart), 1 for its `return' (a \read
% line has finished: get_next returns), 2 for falling through (get_next
% goes on to the alignment test). Its locals k, cat, c, cc, d are the ones
% get_next declared for it; the token-list part uses only t; both parts
% assign each local before reading it, so where they live changes nothing.
% Nothing else refers to the labels switch, reswitch, start_cs and found.
%
% [3] get_next (section 363): a fast path for a token list's next token.
%
% Measured first by hand on the generated code (P6-HYPEROPT,
% docs/evidence/p6-hyperopt-2026-10-04/): most tokens of a LaTeX run come
% from a token list, and get_next's prologue and epilogue (seven register
% pairs, the constants its restart loop keeps) cost as much as its token-list
% part. So get_next becomes a small routine, inlined into its callers
% (web2rust-*.args: --inline get_next=always), that handles the common cases
% itself and leaves the rest, unchanged, to get_next_slow (the routine of [1],
% renamed; --inline get_next_slow=never). The fast path handles a token from
% a token list (state=token_list, loc<>null) that is
%   - a control sequence whose eq_type is below outer_call (so not outer and
%     not dont_expand) and is not tab_mark..car_ret while align_state=0, or
%   - a character token whose command is not out_param (which expands a
%     parameter) and is not tab_mark while align_state=0 (a character token's
%     command is at most car_ret=out_param only for those two, section 364).
% It decides from mem[loc] and eqtb[t-cs_token_flag] alone (each read once,
% as one word: the same reads [1]'s routine makes, before the stores),
% changing nothing, and in those
% cases does exactly what [1]'s routine does for them, in its order: for a
% control sequence cur_cs, loc, cur_cmd, cur_chr, then the read-set hook
% (changes/readset.ch); for a character cur_cs:=0, loc, cur_cmd, cur_chr and
% the align_state step of a brace (section 357); section 364's alignment test
% does not apply (its condition is excluded above); then the exit's
% intrinsics hook (changes/intrinsics.ch). In every other case it calls
% get_next_slow on the unchanged state, which then does exactly what the
% routine did. So the two compute the same in every case.
%
% [4] pass_text (section 494): skipped tokens of a token list, read in place.
%
% pass_text skips the text of a false conditional: for each token it calls
% get_next and looks at cur_cmd (and at cur_chr for fi_or_else). Most of
% that text comes from token lists (macro bodies), and get_next's fast
% path [3] stores cur_cs, cur_cmd and cur_chr for every token. Here, while
% no intrinsic is being recorded, a token of a token list that [3]'s fast
% path would take (loc<>null; a control sequence whose eq_type is below
% outer_call and not tab_mark..car_ret while align_state=0; a character
% token that is not out_param and not tab_mark while align_state=0) is
% read in place: loc moves on, the read-set hook runs for a control
% sequence as in [3], a brace steps align_state as in [3], and cur_cs,
% cur_cmd and cur_chr are not stored. A control sequence whose command is
% fi_or_else or if_test, and every other token, goes to get_next, as
% before. So cur_cmd and cur_chr hold what pass_text's own get_next would
% have left whenever they are read: pass_text reads them only right after
% get_next, and it leaves with the three set by the get_next of the
% fi_or_else that ends it. Nothing between two tokens reads them: the loop
% runs no other code, and get_next's exit hook (flashtex_intr_next) runs
% only while a recording is in progress, when every token goes through
% get_next. A token not read in place leaves the state as get_next found
% it.
%
% [5] macro_call (section 392): a delimited argument's tokens, read in place.
%
% Section 392 ends a stored token with incr(m) and, when info(r) is not a
% match or end_match token (the parameter is delimited), goto continue.
% Only a fall-through of section 397 reaches that point with a token
% stored (397 aborts when s=null and s<>r, and otherwise leaves r=s), so
% there s<>null and r=s: no partial match of the delimiter is in effect.
% From continue, a token get_token returns that is not the delimiter's first
% token d=info(r), not par_token and not a brace takes the same steps
% again: 397 does nothing (s=r), 392's \par and brace tests fail, 398 stores
% it (its space test needs info(r) to be a match token), incr(m), and back
% to continue. So here, at that point and while no intrinsic is being
% recorded, the following tokens of a token list that [3]'s fast path would
% take (loc<>null; a control sequence whose eq_type is below outer_call and
% not tab_mark..car_ret while align_state=0; a character token that is not
% out_param and not tab_mark while align_state=0) are read in place:
%   - one that is neither d, par_token nor a brace: loc moves on, the
%     read-set hook runs for a control sequence as in [3], and the token is
%     stored and counted as 398 and 392 do (fast_store_new_token takes the
%     same node as store_new_token: both are get_avail's code, display-list
%     hook included, and link(q) is null after either), incr(m);
%   - d, par_token or a brace: [3]'s fast path is done here, with its stores
%     in its order (cur_cs, loc, cur_cmd, cur_chr, the read-set hook; a
%     brace's align_state step), and get_token's last step, cur_tok:=tt
%     (cs_token_flag+cur_cs for a control sequence, cur_cmd*@'400+cur_chr
%     for a character: tt in both cases); then the scan goes on right after
%     continue's get_token (found1).
% Any other token, and the end of a list, goes to continue, as before:
% get_token reads it from the unchanged state. For a token stored in place,
% cur_cs, cur_cmd, cur_chr and cur_tok are not stored: nothing reads them
% before the next token sets all four (the loop runs no other code; [3]'s
% exit hook runs only while a recording is in progress; get_token sets them
% before it reads them). get_token's no_new_control_sequence:=false ...
% true is not done for a token read in place; it is true outside get_token
% and \csname's own lookup, so the pair changes nothing here. An undelimited
% parameter never reaches the loop, so its cost is unchanged.
%
% [6] macro_call (section 425): a group's tokens read in place.
%
% The profile of the edited page on Infinite Descent (P6-ENGINE-SPEED) put
% macro_call first (20 %), most of it the loop of section 425 that copies
% a braced argument token by token: per token fast_store_new_token, then
% get_token, which runs get_next and packs cur_tok. A token of a token list
% that [3]'s fast path would take (state=token_list, loc<>null, no intrinsic
% recording; a control sequence whose eq_type is below outer_call and not
% tab_mark..car_ret while align_state=0; a character token that is not
% out_param and not tab_mark while align_state=0) is here read in place,
% except par_token, which goes to get_token as before: loc moves on, the
% read-set hook runs for a control sequence, a brace steps align_state, as
% in [3]; cur_tok is the token itself, which is what get_token computes
% from cur_cs (t-cs_token_flag) or from cur_cmd and cur_chr (t div 256,
% t mod 256); no_new_control_sequence is true, as get_token leaves it. The
% loop's own tests (par_token, the brace counts) then see the same cur_tok.
% cur_cs, cur_cmd and cur_chr are not stored for a token read in place:
% nothing reads them in the loop (fast_store_new_token and the tests read
% cur_tok), and when the loop ends on a token read in place, which is then
% the right brace that closes the group, they are set to what get_token
% would have left (cur_cs=0, the brace's command and character). Every
% other token goes to get_token with the state get_token would have found.
%
% [7] scan_toks (section 477): a body's tokens, read in place.
%
% The same for the body of a definition or a token list that is not
% expanded: each token get_token returns that is not a brace (cur_tok <
% right_brace_limit) and whose command is not mac_param is stored
% (store_new_token(cur_tok)) and the loop goes on. While no intrinsic is
% being recorded, such a token of a token list that [3]'s fast path would
% take is read in place (loc moves on, the read-set hook runs for a control
% sequence, fast_store_new_token as in [5]); a brace or a mac_param token
% (a character, or a control sequence whose eq_type is mac_param) that [3]
% would take is read as [3] and get_token read it, and the loop goes on
% after get_token (found1). intr_weak (changes/intrinsics.ch) is read only
% while a recording is in progress, so not setting it here changes nothing.
% Every other token and the end of a list go to get_token, as before.
%
% [2] divide_scaled (section 689): one 64-bit division, not a digit loop.
%
% Precondition: m > 0 when the division runs. The sign handling makes m
% non-negative and pdf_error stops m = 0, except for m = -2^31, which
% m := -m leaves negative and the m >= max_integer div 10 test lets
% through; there the original's 10*r can overflow and the two versions
% may differ. The proof below assumes m > 0; the review of this change
% (#1309) found m = -2^31 unreachable from pdfTeX's callers.
%
% pdftex.web computes q and r by long division, one decimal digit per step:
% q := s div m; r := s mod m, then dd times q := 10*q + (10*r) div m;
% r := (10*r) mod m. By induction each step leaves 10^k*s = q*m + r with r
% of the sign of s and |r| < m (Pascal's div and mod truncate, as C's and
% Rust's do), which is the truncating division of 10^k*s by m; that holds
% too when s is still negative after s := -s (s = -2^31). So after dd steps
% q and r are the quotient and remainder of one truncating division of
% s*ten_pow[dd] by m, which is computed here in 64 bits, exactly: |s| <=
% 2^31 and ten_pow[dd] <= 10^9, so |t| < 2^61. Where the 32-bit original
% overflows it wraps q modulo 2^32; here q is reduced modulo 2^32 where
% divide_scaled's result is stored (web2rust's 64-to-32-bit conversion is
% `as i32`), and every later operation on q or on the result (incr, sign*q)
% is an addition or a multiplication, which commute with that reduction.
% r is the same value in both (|r| < m < 2^31/10, so 2*r does not overflow
% in either), and so is scaled_out. A dd outside 0..9 stops at ten_pow[dd]
% in both: the original's loop has no effect before it gets there. The
% errors (m = 0, m too big) are tested first in both.

@x [24] m.363 l.8501 - get_next: the external-file part out of line.
@p procedure get_next; {sets |cur_cmd|, |cur_chr|, |cur_cs| to next token}
label restart, {go here to get the next input token}
  switch, {go here to eat the next character from a file}
  reswitch, {go here to digest it again}
  start_cs, {go here to start looking for a control sequence}
  found, {go here when a control sequence has been found}
  exit; {go here when the next input token has been got}
var k:0..buf_size; {an index into |buffer|}
@!t:halfword; {a token}
@!cat:0..max_char_code; {|cat_code(cur_chr)|, usually}
@!c,@!cc:ASCII_code; {constituents of a possible expanded code}
@!d:2..3; {number of excess characters in an expanded code}
begin restart: cur_cs:=0;
if state<>token_list then
@<Input from external file, |goto restart| if no input found@>
else @<Input from token list, |goto restart| if end of list or
  if a parameter needs to be expanded@>;
@y
@p function get_next_file:integer; {|get_next|'s external-file part:
  0 means |goto restart|, 1 means |return|, 2 means go on}
label restart, {go here to get the next input token}
  switch, {go here to eat the next character from a file}
  reswitch, {go here to digest it again}
  start_cs, {go here to start looking for a control sequence}
  found, {go here when a control sequence has been found}
  exit; {go here when the next input token has been got}
var k:0..buf_size; {an index into |buffer|}
@!cat:0..max_char_code; {|cat_code(cur_chr)|, usually}
@!c,@!cc:ASCII_code; {constituents of a possible expanded code}
@!d:2..3; {number of excess characters in an expanded code}
begin get_next_file:=1;
@<Input from external file, |goto restart| if no input found@>;
get_next_file:=2; return;
restart: get_next_file:=0;
exit:end;
@#
procedure get_next_slow; {|get_next| where its fast path [3] does not apply}
label restart, {go here to get the next input token}
  exit; {go here when the next input token has been got}
var @!t:halfword; {a token}
begin restart: cur_cs:=0;
if state<>token_list then
  case get_next_file of
  0: goto restart;
  1: return;
  othercases do_nothing
  endcases
else @<Input from token list, |goto restart| if end of list or
  if a parameter needs to be expanded@>;
@z

@x [24] m.363 - get_next: a fast path for a token list's next token.
exit: if intr_rec_on then flashtex_intr_next;
end;
@y
exit: if intr_rec_on then flashtex_intr_next;
end;
@#
procedure get_next; {sets |cur_cmd|, |cur_chr|, |cur_cs| to next token}
var @!t:halfword; {a token}
@!n:pointer; {the rest of the list}
@!q:pointer; {the control sequence of |t|}
@!c:integer; {its command code, or $-1$: |get_next_slow| reads it}
@!e:halfword; {its |equiv|}
begin c:=-1;
if state=token_list then if loc<>null then
  begin t:=info(loc); n:=link(loc);
  if t>=cs_token_flag then
    begin q:=t-cs_token_flag; c:=eq_type(q); e:=equiv(q);
    if (c>=outer_call)or((c<=car_ret)and(c>=tab_mark)and(align_state=0)) then
      c:=-1
    else  begin cur_cs:=q; loc:=n; cur_cmd:=c; cur_chr:=e;
      if rs_on then if not rs_seen[q] then flashtex_cs_read(q);
      end;
    end
  else  begin c:=t div @'400;
    if (c=out_param)or((c=tab_mark)and(align_state=0)) then c:=-1
    else  begin cur_cs:=0; loc:=n; cur_cmd:=c; cur_chr:=t mod @'400;
      if c=left_brace then incr(align_state)
      else if c=right_brace then decr(align_state);
      end;
    end;
  end;
if c<0 then get_next_slow
else if intr_rec_on then flashtex_intr_next;
end;
@z

@x [25] m.389 l.9326 - macro_call: a delimited argument's tokens, read in place [5]
procedure macro_call; {invokes a user-defined control sequence}
label exit, continue, done, done1, found;
@y
procedure macro_call; {invokes a user-defined control sequence}
label exit, continue, done, done1, found, found1;
@z

@x [25] m.389 l.9341 - macro_call: tokens read in place [5] [6]
@!match_chr:ASCII_code; {character used in parameter}
@y
@!match_chr:ASCII_code; {character used in parameter}
@!tt:halfword; {a token read in place}
@!c:integer; {its command code}
@!d:halfword; {the first token of the delimiter}
@!ft:halfword; {a token read in place [6]}
@!fq:pointer; {the control sequence of |ft|}
@!fc:integer; {its command code}
@!in_place:boolean; {the last token was read in place}
@z

@x [25] m.392 l.9402 - macro_call: a delimited argument's tokens, read in place [5]
continue: get_token; {set |cur_tok| to the next token of input}
if cur_tok=info(r) then
@y
continue: get_token; {set |cur_tok| to the next token of input}
found1: if cur_tok=info(r) then
@z

@x [25] m.392 l.9418 - macro_call: a delimited argument's tokens, read in place [5]
incr(m);
if info(r)>end_match_token then goto continue;
if info(r)<match_token then goto continue;
@y
incr(m);
if (info(r)>end_match_token)or(info(r)<match_token) then
  begin if not intr_rec_on then
    begin d:=info(r);
    loop@+  begin if state<>token_list then goto continue;
      if loc=null then goto continue;
      tt:=info(loc);
      if tt>=cs_token_flag then
        begin c:=eq_type(tt-cs_token_flag);
        if c>=outer_call then goto continue;
        if (c<=car_ret)and(c>=tab_mark)and(align_state=0) then goto continue;
        if (tt=d)or(tt=par_token) then
          begin cur_cs:=tt-cs_token_flag; loc:=link(loc); cur_cmd:=c;
          cur_chr:=equiv(cur_cs);
          if rs_on then if not rs_seen[cur_cs] then flashtex_cs_read(cur_cs);
          cur_tok:=tt; goto found1;
          end;
        loc:=link(loc);
        if rs_on then if not rs_seen[tt-cs_token_flag] then
          flashtex_cs_read(tt-cs_token_flag);
        end
      else  begin c:=tt div @'400;
        if c=out_param then goto continue;
        if (c=tab_mark)and(align_state=0) then goto continue;
        if (tt=d)or(tt<right_brace_limit) then
          begin cur_cs:=0; loc:=link(loc); cur_cmd:=c; cur_chr:=tt mod @'400;
          if c=left_brace then incr(align_state)
          else if c=right_brace then decr(align_state);
          cur_tok:=tt; goto found1;
          end;
        loc:=link(loc);
        end;
      fast_store_new_token(tt); incr(m);
      end;
    end;
  goto continue;
  end;
@z

@x [25] m.425 l.9519 - macro_call: a group's tokens read in place [6]
begin unbalance:=1;
@^inner loop@>
loop@+  begin fast_store_new_token(cur_tok); get_token;
  if cur_tok=par_token then if long_state<>long_call then
    @<Report a runaway argument and abort@>;
  if cur_tok<right_brace_limit then
    if cur_tok<left_brace_limit then incr(unbalance)
    else  begin decr(unbalance);
      if unbalance=0 then goto done1;
      end;
  end;
done1: rbrace_ptr:=p; store_new_token(cur_tok);
end
@y
begin unbalance:=1;
@^inner loop@>
loop@+  begin fast_store_new_token(cur_tok);
  in_place:=false;
  if state=token_list then if loc<>null then if not intr_rec_on then
    begin ft:=info(loc);
    if ft>=cs_token_flag then
      begin if ft<>par_token then
        begin fq:=ft-cs_token_flag; fc:=eq_type(fq);
        if (fc<outer_call)and((fc>car_ret)or(fc<tab_mark)or(align_state<>0)) then
          begin loc:=link(loc); in_place:=true;
          if rs_on then if not rs_seen[fq] then flashtex_cs_read(fq);
          end;
        end;
      end
    else  begin fc:=ft div @'400;
      if (fc<>out_param)and((fc<>tab_mark)or(align_state<>0)) then
        begin loc:=link(loc); in_place:=true;
        if fc=left_brace then incr(align_state)
        else if fc=right_brace then decr(align_state);
        end;
      end;
    end;
  if in_place then
    begin cur_tok:=ft; no_new_control_sequence:=true;
    end
  else  begin get_token;
    if cur_tok=par_token then if long_state<>long_call then
      @<Report a runaway argument and abort@>;
    end;
  if cur_tok<right_brace_limit then
    if cur_tok<left_brace_limit then incr(unbalance)
    else  begin decr(unbalance);
      if unbalance=0 then goto done1;
      end;
  end;
done1: if in_place then
  begin cur_cs:=0; cur_cmd:=cur_tok div @'400; cur_chr:=cur_tok mod @'400;
  end;
rbrace_ptr:=p; store_new_token(cur_tok);
end
@z

@x [27] m.473 l.11456 - scan_toks: a body's tokens, read in place [7]
label found,continue,done,done1,done2;
@y
label found,continue,done,done1,done2,done3,found1;
@z

@x [27] m.473 l.11462 - scan_toks: a body's tokens, read in place [7]
@!hash_brace:halfword; {possible `\.{\#\{}' token}
@y
@!hash_brace:halfword; {possible `\.{\#\{}' token}
@!tt:halfword; {a token read in place}
@!c:integer; {its command code}
@z

@x [27] m.477 - scan_toks: a body's tokens, read in place [7]
  else begin intr_weak:=true; get_token; intr_weak:=false;
    end;
@y
  else begin if not intr_rec_on then
      loop@+  begin if state<>token_list then goto done3;
        if loc=null then goto done3;
        tt:=info(loc);
        if tt>=cs_token_flag then
          begin c:=eq_type(tt-cs_token_flag);
          if c>=outer_call then goto done3;
          if (c<=car_ret)and(c>=tab_mark)and(align_state=0) then goto done3;
          if c=mac_param then
            begin cur_cs:=tt-cs_token_flag; loc:=link(loc); cur_cmd:=c;
            cur_chr:=equiv(cur_cs);
            if rs_on then if not rs_seen[cur_cs] then flashtex_cs_read(cur_cs);
            cur_tok:=tt; goto found1;
            end;
          loc:=link(loc);
          if rs_on then if not rs_seen[tt-cs_token_flag] then
            flashtex_cs_read(tt-cs_token_flag);
          end
        else  begin c:=tt div @'400;
          if c=out_param then goto done3;
          if (c=tab_mark)and(align_state=0) then goto done3;
          if (tt<right_brace_limit)or(c=mac_param) then
            begin cur_cs:=0; loc:=link(loc); cur_cmd:=c; cur_chr:=tt mod @'400;
            if c=left_brace then incr(align_state)
            else if c=right_brace then decr(align_state);
            cur_tok:=tt; goto found1;
            end;
          loc:=link(loc);
          end;
        fast_store_new_token(tt);
        end;
    done3: intr_weak:=true; get_token; intr_weak:=false;
    found1: end;
@z

@x [28] m.494 l.11812 - pass_text: skipped tokens of a token list, read in place [4]
@p procedure pass_text;
label done;
var l:integer; {level of $\.{\\if}\ldots\.{\\fi}$ nesting}
@!save_scanner_status:small_number; {|scanner_status| upon entry}
begin save_scanner_status:=scanner_status; scanner_status:=skipping; l:=0;
skip_line:=line;
loop@+  begin get_next;
@y
@p procedure pass_text;
label done, continue;
var l:integer; {level of $\.{\\if}\ldots\.{\\fi}$ nesting}
@!save_scanner_status:small_number; {|scanner_status| upon entry}
@!t:halfword; {a token}
@!q:pointer; {the control sequence of |t|}
@!c:integer; {its command code}
begin save_scanner_status:=scanner_status; scanner_status:=skipping; l:=0;
skip_line:=line;
loop@+  begin continue:
  if state=token_list then if loc<>null then if not intr_rec_on then
    begin t:=info(loc);
    if t>=cs_token_flag then
      begin q:=t-cs_token_flag; c:=eq_type(q);
      if (c<outer_call)and((c>car_ret)or(c<tab_mark)or(align_state<>0)) then
        begin if (c<>fi_or_else)and(c<>if_test) then
          begin loc:=link(loc);
          if rs_on then if not rs_seen[q] then flashtex_cs_read(q);
          goto continue;
          end;
        end;
      end
    else  begin c:=t div @'400;
      if (c<>out_param)and((c<>tab_mark)or(align_state<>0)) then
        begin loc:=link(loc);
        if c=left_brace then incr(align_state)
        else if c=right_brace then decr(align_state);
        goto continue;
        end;
      end;
    end;
  get_next;
@z

@x [42] m.689 l.15826 - divide_scaled: one 64-bit division.
var q, r: scaled;
    sign, i: integer;
@y
var q, r, t: longinteger;
    sign: integer;
@z

@x [42] m.689 l.15842 - divide_scaled: one 64-bit division.
    q := s div m;
    r := s mod m;
    for i := 1 to dd do begin
        q := 10*q + (10*r) div m;
        r := (10*r) mod m;
    end;
@y
    t := s;
    t := t * ten_pow[dd];
    q := t div m;
    r := t mod m;
@z
