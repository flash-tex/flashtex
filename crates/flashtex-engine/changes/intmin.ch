% changes/intmin.ch -- the most negative integer, as TeX Live's build treats it.
%
% A \count or \dimen register can hold -2^31: TeX does not check \advance for
% overflow. web2c's C negates such a value in `negate` and `abs`, which is
% undefined in C. clang, which builds TeX Live's macOS binaries, assumes that
% the negation does not overflow, so the negated value behaves as +2^31 in
% the code that follows. tools/web2rust's `--exact-abs` covers an `abs` that
% is compared or divided in the same expression; the two places below keep
% the negated value in a variable, so they are written out here. Each gives
% what TeX Live 2026's pdftex prints (arm64 and x86-64 on macOS, measured
% 2026-11-01; crates/flashtex-engine/tests/initex.rs, `int_min_*`).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.2362 - print_scaled(-2^31): pdftex prints -32768.0
print_int(s div unity); {print the integer part}
print_char(".");
s:=10*(s mod unity)+5; delta:=10;
@y
if s<0 then {|s| was $-2^{31}$, which C's negation leaves as it was}
  begin print_int(32768); s:=0;
  end
else  begin print_int(s div unity); s:=s mod unity;
  end; {print the integer part}
print_char(".");
s:=10*s+5; delta:=10;
@z

@x pdftex.web l.2835 - init_randoms(-2^31): pdftex halves 2^31
begin j:=abs(seed);
@y
begin j:=abs(seed);
if j<0 then j:=fraction_four; {|seed| was $-2^{31}$; $2^{31}$ halves to this}
@z
