% changes/goto.ch -- jumps the Rust translation cannot express as written.
%
% web2c compiles pdftex.web to C, where `goto` may enter any statement of the
% routine. tools/web2rust translates `goto` into labelled blocks and loops
% (DESIGN.md section 4.1), which covers every jump in tex.web; the ones below
% jump into a sibling |case| arm, so they are rewritten here into equivalent
% structured code. Each rewrite executes exactly the same statements in the
% same order as the original.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.27521 - prune_page_top: `goto discard_or_move` enters the next case arm
@p function prune_page_top(@!p:pointer;@!s:boolean):pointer;
label discard_or_move;
  {adjust top after page break}
var prev_p:pointer; {lags one step behind |p|}
@!q,@!r:pointer; {temporary variables for list manipulation}
begin prev_p:=temp_head; link(temp_head):=p;
while p<>null do
  case type(p) of
  hlist_node,vlist_node,rule_node:@<Insert glue for |split_top_skip|
    and set~|p:=null|@>;
  whatsit_node,mark_node,ins_node: begin
    if (type(p) = whatsit_node) and
        ((subtype(p) = pdf_snapy_node) or
         (subtype(p) = pdf_snapy_comp_node)) then
      begin
        print("snap node being discarded");
        goto discard_or_move;
      end;
    prev_p:=p; p:=link(prev_p);
    end;
  glue_node,kern_node,penalty_node: begin
discard_or_move:
@y
@p function prune_page_top(@!p:pointer;@!s:boolean):pointer;
  {adjust top after page break}
var prev_p:pointer; {lags one step behind |p|}
@!q,@!r:pointer; {temporary variables for list manipulation}
begin prev_p:=temp_head; link(temp_head):=p;
while p<>null do
  case type(p) of
  hlist_node,vlist_node,rule_node:@<Insert glue for |split_top_skip|
    and set~|p:=null|@>;
  whatsit_node,mark_node,ins_node: begin
    if (type(p) = whatsit_node) and
        ((subtype(p) = pdf_snapy_node) or
         (subtype(p) = pdf_snapy_comp_node)) then
      begin
        print("snap node being discarded");
        {the |glue_node| arm's statements, where pdftex.web jumps}
        q:=p; p:=link(q); link(q):=null;
        link(prev_p):=p;
        if s then
          begin if split_disc=null then split_disc:=q@+else link(r):=q;
          r:=q;
          end
        else flush_node_list(q);
      end
    else begin
    prev_p:=p; p:=link(prev_p);
    end;
    end;
  glue_node,kern_node,penalty_node: begin
@z
