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
