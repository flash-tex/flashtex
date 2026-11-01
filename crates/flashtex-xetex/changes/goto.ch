% changes/goto.ch -- jumps the Rust translation cannot express as written.
%
% The XeTeX counterpart of crates/flashtex-engine/changes/goto.ch. web2c
% compiles xetex.web to C, where `goto' may enter any statement of the
% routine. tools/web2rust translates `goto' into labelled blocks and loops
% (DESIGN.md section 4.1); the jumps below cannot be expressed that way, so
% they are rewritten here into equivalent structured code. Each rewrite
% executes exactly the same statements in the same order as the original.
%
% In |line_break|, the code that prepares a native word for hyphenation
% jumps to |done3|, a label at the end of the other branch of the |if|
% (the one for a list of characters), after whose statements nothing else
% is in that branch: so the jump leaves the native branch for the code after
% the |if|. Here it goes to a new label |done7| at the end of the native
% branch's own code instead, which continues at the same statement.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.19716 - line_break: the label of the native branch's end
label done,done1,done2,done3,done4,done5,done6,continue, restart;
@y
label done,done1,done2,done3,done4,done5,done6,done7,continue, restart;
@z

@x xetex.web l.21562 - line_break: `goto done3' leaves the native branch
      goto done3;
@y
      goto done7;
@z

@x xetex.web l.21571 - line_break: `goto done3' leaves the native branch
    goto done3
@y
    goto done7
@z

@x xetex.web l.21586 - line_break: the end of the native branch
    hyf_bchar:=non_char;
  end
end;
@y
    hyf_bchar:=non_char;
  end
end;
done7:
@z

@x xetex.web l.34414 - new sections at the end of part 54
@* \[55] Index.
@y
@ @d done7=37 {the end of the native branch of |line_break|'s hyphenation}

@* \[55] Index.
@z
