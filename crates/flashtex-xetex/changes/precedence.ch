% changes/precedence.ch -- expressions whose C reading differs from Pascal's.
%
% The XeTeX counterpart of crates/flashtex-engine/changes/precedence.ch.
% web2c prints Pascal's `and'/`or' as C's `&&'/`||' without adding
% parentheses, and C binds those more loosely than a comparison, so TeX
% Live's XeTeX computes these three expressions the C way. web2c also prints
% a unary minus before its factor, so `-a and b' is C's `(-a) && b' where
% Pascal reads `-(a and b)'; that happens where a macro is a negative number
% (xetex.web's |null| is |min_halfword|, -"FFFFFFF), in |is_glyph_node|. And
% that `- (integer)' truncates a negated real first (the last two changes;
% xetex.web's picture code is the only place). tools/web2rust
% refuses any expression whose two readings could differ, so this list is
% complete; each change makes the C reading explicit.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.14704 - C: ((p) != -268435455) && ... (|null| is negative)
@d is_glyph_node(#) == (((#)<>null and (not is_char_node(#)) and (type(#) = whatsit_node) and (subtype(#) = glyph_node)))
@y
@d is_glyph_node(#) == ((((#)<>null) and (not is_char_node(#)) and (type(#) = whatsit_node) and (subtype(#) = glyph_node)))
@z

@x xetex.web l.16889 - C: (XeTeX_linebreak_penalty != 0) || !use_skip
    use_penalty:=XeTeX_linebreak_penalty <> 0 or not use_skip;
@y
    use_penalty:=(XeTeX_linebreak_penalty <> 0) or not use_skip;
@z

@x xetex.web l.18558 - C: is_valid_pointer(script_head) && (math_type(script_head) == math_char)
if is_valid_pointer(script_head) and math_type(script_head)=math_char then
@y
if is_valid_pointer(script_head) and (math_type(script_head)=math_char) then
@z

@x xetex.web l.24347 - C: XeTeX_inter_char_tokens_en && (space_class != char_class_ignored)
  if XeTeX_inter_char_tokens_en and space_class <> char_class_ignored then begin {class 4096 = ignored (for combining marks etc)}
@y
  if XeTeX_inter_char_tokens_en and (space_class <> char_class_ignored) then begin {class 4096 = ignored (for combining marks etc)}
@z

@x xetex.web l.30552 - C: xmax = - (integer) xmin
    xmax:=-xmin;
@y
    xmax:=-trunc(xmin);
@z

@x xetex.web l.30699 - C: - (integer) xmin * 72 / ((double) 72.27)
  make_translation(addressof(t2), -xmin * 72 / 72.27, -ymin * 72 / 72.27);
@y
  make_translation(addressof(t2), (-trunc(xmin)) * 72 / 72.27, (-trunc(ymin)) * 72 / 72.27);
@z
