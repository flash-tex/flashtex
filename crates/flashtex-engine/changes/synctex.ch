% changes/synctex.ch -- SyncTeX's memory layout and the \synctex parameter,
% without SyncTeX's output.
%
% TeX Live's pdfTeX includes SyncTeX: pdftexdir/am/pdftex.am applies
% |pdftex_ch_synctex| (synctexdir/am/synctex.am), that is synctex-def.ch0,
% synctex-mem.ch0, synctex-e-mem.ch0, synctex-e-mem.ch1, synctex-rec.ch0,
% synctex-rec.ch1, synctex-e-rec.ch0 and synctex-pdf-rec.ch2, with synctex.c
% behind them. They add the integer parameter \synctex (after e-TeX's, as
% synctex-e-mem.ch1 places it) and two words of source position, the file
% tag and the line, at the end of every box, rule, glue, kern, math and
% penalty node; and |get_node| writes the current tag and line into the last
% two words of every node of |medium_node_size| words or more.
%
% That layout shows beyond \tracingstats memory accounting. |get_node| does
% not clear a node, and a box that |hpack| makes with |m=cal_expand_ratio|
% and |font_expand_ratio=0| (a line under \pdfadjustspacing that needs no
% expansion) keeps whatever |glue_set|, |glue_sign| and |glue_order| its
% words held before, which \showbox prints. So the node sizes, the copies of
% the source-position words and the values |get_node| writes into them are
% re-specified here, hunk for hunk as those files make them (each change
% names its origin). What SyncTeX does with them -- the .synctex file
% synctex.c writes as pages ship out, through the recording calls of
% synctex-rec.ch0, synctex-rec.ch1, synctex-e-rec.ch0 and synctex-pdf-rec.ch2
% -- is not here: those calls read |mem| and never write it, and `-synctex`
% is refused. synctex.c's |synctexstartinput| without a `-synctex` option is
% this file's ``Prepare new file'' section: it numbers the files \TeX\ opens
% with |start_input| 1, 2, ..., and that number is the file's tag (synctex.c
% gives tag 0 after $2^{32}-1$ files; this counter does not get there).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.3230 - synctex-rec.ch0: get_node writes the SyncTeX information
get_node:=r;
@y
@<Initialize bigger nodes with {\sl Sync\TeX} information@>;
get_node:=r;
@z

@x pdftex.web l.3409 - synctex-def.ch0, synctex-mem.ch0: box nodes are synchronized
@d hlist_node=0 {|type| of hlist nodes}
@d box_node_size=7 {number of words to allocate for a box node}
@y
@d synctex_field_size=2 {two integers for file tag and line}
@d sync_tag(#) == mem[#-synctex_field_size].int {the tag subfield}
@d sync_line(#) == mem[#-synctex_field_size+1].int {the line subfield}
@#
@d hlist_node=0 {|type| of hlist nodes}
@d box_node_size=7+synctex_field_size {number of words to allocate for a box node}
@z

@x pdftex.web l.3456 - synctex-mem.ch0: rule nodes are synchronized
@d rule_node_size=4 {number of words to allocate for a rule node}
@y
@d rule_node_size=4+synctex_field_size {number of words to allocate for a rule node}
@z

@x pdftex.web l.3495 - synctex-mem.ch0: math, glue, kern and penalty nodes are synchronized
@d small_node_size=2 {number of words to allocate for most node types}
@y
@d small_node_size=2 {number of words to allocate for most node types}
@d medium_node_size=small_node_size+synctex_field_size {number of words to
           allocate for synchronized node types like math, kern, glue and penalty nodes}
@z

@x pdftex.web l.3628 - synctex-mem.ch0
begin p:=get_node(small_node_size); type(p):=math_node;
@y
begin p:=get_node(medium_node_size); type(p):=math_node;
@z

@x pdftex.web l.3724 - synctex-mem.ch0
begin p:=get_node(small_node_size); type(p):=glue_node; subtype(p):=n+1;
@y
begin p:=get_node(medium_node_size); type(p):=glue_node; subtype(p):=n+1;
@z

@x pdftex.web l.3736 - synctex-mem.ch0
begin p:=get_node(small_node_size); type(p):=glue_node; subtype(p):=normal;
@y
begin p:=get_node(medium_node_size); type(p):=glue_node; subtype(p):=normal;
@z

@x pdftex.web l.3803 - synctex-mem.ch0
begin p:=get_node(small_node_size); type(p):=kern_node;
@y
begin p:=get_node(medium_node_size); type(p):=kern_node;
@z

@x pdftex.web l.3826 - synctex-mem.ch0
begin p:=get_node(small_node_size); type(p):=penalty_node;
@y
begin p:=get_node(medium_node_size); type(p):=penalty_node;
@z

@x pdftex.web l.4551 - synctex-mem.ch0: free nodes with proper size
    glue_node: begin fast_delete_glue_ref(glue_ptr(p));
      if leader_ptr(p)<>null then flush_node_list(leader_ptr(p));
      end;
    kern_node,math_node,penalty_node: do_nothing;
@y
    glue_node: begin fast_delete_glue_ref(glue_ptr(p));
      if leader_ptr(p)<>null then flush_node_list(leader_ptr(p));
        free_node(p, medium_node_size);
        goto done;
      end;
    kern_node,math_node,penalty_node:begin
        free_node(p, medium_node_size);
        goto done;
      end;
@z

@x pdftex.web l.4626 - synctex-rec.ch0: a copied box keeps its SyncTeX information
hlist_node,vlist_node,unset_node: begin r:=get_node(box_node_size);
@y
hlist_node,vlist_node,unset_node: begin r:=get_node(box_node_size);
  @<Copy the box {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.4631 - synctex-rec.ch0: a copied rule keeps what get_node wrote
rule_node: begin r:=get_node(rule_node_size); words:=rule_node_size;
@y
rule_node: begin r:=get_node(rule_node_size); words:=rule_node_size-synctex_field_size;
  {{\sl Sync\TeX}: do not let \TeX\ copy the {\sl Sync\TeX} information}
  @<Copy the rule {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.4640 - synctex-mem.ch0, synctex-rec.ch0
glue_node: begin r:=get_node(small_node_size); add_glue_ref(glue_ptr(p));
@y
glue_node: begin r:=get_node(medium_node_size); add_glue_ref(glue_ptr(p));
  @<Copy the medium sized node {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.4643 - synctex-mem.ch0
kern_node,math_node,penalty_node: begin r:=get_node(small_node_size);
  words:=small_node_size;
  end;
@y
kern_node,math_node,penalty_node: begin r:=get_node(medium_node_size);
  words:=medium_node_size;
  end;
@z

@x pdftex.web l.5736 - synctex-e-mem.ch1: \synctex is the last integer parameter
@d int_pars=etex_int_pars {total number of integer parameters}
@y
@d synctex_code=etex_int_pars
@d int_pars=synctex_code+1 {total number of integer parameters}
@z

@x pdftex.web l.7693 - synctex-mem.ch0: each input level has a SyncTeX tag
  @!start_field,@!loc_field, @!limit_field, @!name_field: halfword;
  end;
@y
  @!start_field,@!loc_field, @!limit_field, @!name_field: halfword;
  @!synctex_tag_field: integer; {stack the tag of the current file}
  end;
@z

@x pdftex.web l.7711 - synctex-mem.ch0
@d name==cur_input.name_field {name of the current file}
@y
@d name==cur_input.name_field {name of the current file}
@d synctex_tag==cur_input.synctex_tag_field {{\sl Sync\TeX} tag of the current file}
@z

@x pdftex.web l.8328 - synctex-rec.ch0: terminal input has tag 0
name:=0; {|terminal_input| is now |true|}
@y
name:=0; {|terminal_input| is now |true|}
@<Prepare terminal input {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.12578 - synctex-rec.ch0: a new file gets a new tag
@<Read the first line of the new file@>;
end;
@y
@<Prepare new file {\sl Sync\TeX} information@>;
@<Read the first line of the new file@>;
end;
@z

@x pdftex.web l.22327 - synctex-mem.ch0: the italic correction is a kern
    begin free_node(r,small_node_size); link(q):=null;
@y
    begin free_node(r,medium_node_size); link(q):=null;
@z

@x pdftex.web l.26545 - synctex-rec.ch0
  begin link(t):=new_kern(w); t:=link(t); w:=0;
@y
  begin link(t):=new_kern(w); t:=link(t); w:=0;
    sync_tag(t+medium_node_size):=0; {{\sl Sync\TeX}: do nothing, it is too late}
@z

@x pdftex.web l.38055 - synctex-e-rec.ch0
begin save_h:=cur_h; temp_ptr:=p; p:=new_kern(0); link(prev_p):=p;
@y
begin save_h:=cur_h; temp_ptr:=p; p:=new_kern(0);
sync_tag(p+medium_node_size):=0; {{\sl Sync\TeX}: do nothing, it is too late}
link(prev_p):=p;
@z

@x pdftex.web l.38067 - synctex-e-mem.ch0: |p| is a |math_node|
free_node(p,small_node_size);
cur_dir:=reflected; p:=new_edge(cur_dir,rule_wd); link(prev_p):=p;
@y
free_node(p,medium_node_size); {{\sl Sync\TeX}: p is a |math_node|}
cur_dir:=reflected; p:=new_edge(cur_dir,rule_wd); link(prev_p):=p;
@z

@x pdftex.web l.38126 - synctex-e-mem.ch0
  begin free_node(p,small_node_size); p:=l;
@y
  begin free_node(p,medium_node_size); p:=l;
@z

@x pdftex.web l.38178 - synctex-e-mem.ch0: |p| is a |kern_node|
begin free_node(p,small_node_size);
link(t):=q; width(t):=rule_wd; edge_dist(t):=-cur_h-rule_wd; goto done;
@y
begin free_node(p,medium_node_size); {{\sl Sync\TeX}: p is a |kern_node|}
link(t):=q; width(t):=rule_wd; edge_dist(t):=-cur_h-rule_wd; goto done;
@z

@x pdftex.web l.38217 - synctex-e-rec.ch0
  hlist_node,vlist_node: begin r:=get_node(box_node_size);
@y
  hlist_node,vlist_node: begin r:=get_node(box_node_size);
    @<Copy the box {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.38226 - synctex-e-mem.ch0, synctex-e-rec.ch0
  kern_node,math_node: begin r:=get_node(small_node_size);
    words:=small_node_size;
    end;
  glue_node: begin r:=get_node(small_node_size); add_glue_ref(glue_ptr(p));
@y
  kern_node,math_node: begin
      words:=medium_node_size; {{\sl Sync\TeX}: proper size for math and kern}
      r:=get_node(words);
    end;
  glue_node: begin r:=get_node(medium_node_size); add_glue_ref(glue_ptr(p));
    @<Copy the medium sized node {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.38301 - synctex-e-mem.ch0: drop unused label
label found,done;
@y
label done;
@z

@x pdftex.web l.38323 - synctex-e-mem.ch0
found:width(t):=width(p); link(t):=q; free_node(p,small_node_size);
@y
width(t):=width(p); link(t):=q; free_node(p,small_node_size);
@z

@x pdftex.web l.38334 - synctex-e-mem.ch0: no more |goto found|, and proper node size
    else  begin if m>min_halfword then decr(m)@+else goto found;
      type(p):=kern_node;
@y
    else  begin if m>min_halfword then decr(m)@+else begin
    width(t):=width(p); link(t):=q; free_node(p,medium_node_size);
    goto done;
  end;
      type(p):=kern_node;
@z

@x pdftex.web l.38553 - synctex-e-rec.ch0: a pseudo file has tag 0
else name:=18
@y
else begin
    name:=18;
    @<Prepare pseudo file {\sl Sync\TeX} information@>;
end
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ synctex-mem.ch0's parameter \.{\\synctex}.

@d synctex==int_par(synctex_code)

@<Put each of \TeX's primitives into the hash table@>=
primitive("synctex",assign_int,int_base+synctex_code);@/
@!@:synctex_}{\.{\\synctex} primitive@>

@ @<Cases for |print_param|@>=
synctex_code:    print_esc("synctex");

@ synctex-rec.ch0: ``every node with a sufficiently big size is initialized
at creation time in the |get_node| routine with the current {\sl Sync\TeX}
information, whether or not the node is synchronized.'' Only the |int| half
of each of the two words is written.

@<Initialize bigger nodes with {\sl Sync\TeX} information@>=
if s>=medium_node_size then
begin
  sync_tag(r+s):=synctex_tag;
  sync_line(r+s):=line;
end

@ synctex-rec.ch0 and synctex-e-rec.ch0: input from the terminal, from
\.{\\read} or from a pseudo file has tag 0.

@<Prepare terminal input {\sl Sync\TeX} information@>=
synctex_tag:=0

@ @<Prepare pseudo file {\sl Sync\TeX} information@>=
synctex_tag:=0

@ synctex.c's |synctexstartinput| when no \.{-synctex} option was given:
every file that |start_input| opens gets the next number.

@<Glob...@>=
@!synctex_tag_counter:integer; {the tag of the last file opened}

@ @<Set init...@>=
synctex_tag_counter:=0;

@ @<Prepare new file {\sl Sync\TeX} information@>=
begin incr(synctex_tag_counter); synctex_tag:=synctex_tag_counter;
end

@ synctex-rec.ch0: a copy of a box or of a glue node gets the original's
{\sl Sync\TeX} information (the |int| halves of its two words); a copy of a
kern, math or penalty node copies it with the rest of the node; a copy of a
rule keeps what |get_node| wrote.

@<Copy the box {\sl Sync\TeX} information@>=
sync_tag(r+box_node_size):=sync_tag(p+box_node_size);
sync_line(r+box_node_size):=sync_line(p+box_node_size)

@ @<Copy the rule {\sl Sync\TeX} information@>=
do_nothing

@ @<Copy the medium sized node {\sl Sync\TeX} information@>=
sync_tag(r+medium_node_size):=sync_tag(p+medium_node_size);
sync_line(r+medium_node_size):=sync_line(p+medium_node_size)

@* \[55] Index.
@z
