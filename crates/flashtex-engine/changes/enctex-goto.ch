% changes/enctex-goto.ch -- a jump of enctex2.ch the Rust translation cannot
% express as written.
%
% enctex2.ch [24.343] makes get_next's reading of a character from a file
% (section 343) end, when \mubyte turned the bytes into a control sequence,
% with `goto found': into section 354, which is inside a sibling |case| arm
% of section 344 (|any_state_plus(escape)|). web2c's C may jump there;
% tools/web2rust may not (changes/goto.ch). So here the statements at
% |found| become a module, and section 343 uses it where it jumped, then
% skips what followed (the |reswitch| part, section 344), as the jump did:
% after the module the control reaches, in both, the end of section 343's
% first branch. The same statements run in the same order. Applied right
% after enctex2.ch; changes/readset.ch's hook at |found| is in the module.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x enctex2.ch [24.343] - get_next: \mubyte's control sequence, without `goto found'
    if (mubyte_token > 0) then
    begin
      state := mid_line;
      cur_cs := mubyte_token - cs_token_flag;
      goto found;
    end;
  reswitch: cur_cmd:=cat_code(cur_chr);
  @<Change state if necessary, and |goto switch| if the
    current character should be ignored,
    or |goto reswitch| if the current character
    changes to another@>;
  end
@y
    if (mubyte_token > 0) then
    begin
      state := mid_line;
      cur_cs := mubyte_token - cs_token_flag;
      @<Find the meaning of the control sequence |cur_cs| (|found|)@>;
    end
  else begin
  reswitch: cur_cmd:=cat_code(cur_chr);
  @<Change state if necessary, and |goto switch| if the
    current character should be ignored,
    or |goto reswitch| if the current character
    changes to another@>;
  end;
  end
@z

@x enctex2.ch [24.354] - get_next: the statements at |found| are a module
found: cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
if cur_cmd>=outer_call then check_outer_validity;
if write_noexpanding then
begin
  p := mubyte_cswrite [cur_cs mod 128];
  while p <> null do
    if info (p) = cur_cs then
    begin
      cur_cmd := relax; cur_chr := 256; p := null;
    end else p := link (link (p));
end;
end
@y
found: @<Find the meaning of the control sequence |cur_cs| (|found|)@>;
end

@ What section 354 does at |found|, and section 343 when \.{\\mubyte} made
the control sequence (changes/enctex-goto.ch).

@<Find the meaning of the control sequence |cur_cs| (|found|)@>=
begin cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
if cur_cmd>=outer_call then check_outer_validity;
if write_noexpanding then
begin
  p := mubyte_cswrite [cur_cs mod 128];
  while p <> null do
    if info (p) = cur_cs then
    begin
      cur_cmd := relax; cur_chr := 256; p := null;
    end else p := link (link (p));
end;
end
@z
