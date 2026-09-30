% changes/synctex.ch -- the \synctex parameter and SyncTeX's memory layout,
% without SyncTeX output.
%
% TeX Live's pdfTeX includes SyncTeX: the change files synctex.am lists for
% pdftex (synctexdir/synctex-def.ch0, synctex-mem.ch0, synctex-e-mem.ch0,
% synctex-e-mem.ch1, synctex-rec.ch0, synctex-rec.ch1, synctex-e-rec.ch0,
% synctex-pdf-rec.ch2) and synctex.c. They add the integer parameter
% \synctex (after e-TeX's, as synctex-e-mem.ch1 places it) and two words of
% source position (file tag and line) at the end of box, rule, glue, kern,
% penalty and math nodes, which |get_node| fills in for every node of four
% words or more.
%
% Those two words change where every later node lands in |mem|, and
% pdftex.web reads memory it has not initialised in at least one place: an
% |hpack| with |m=cal_expand_ratio| that finds a zero |font_expand_ratio|
% returns without setting |glue_order|, |glue_sign| and |glue_set| (#1220).
% What such a box shows in \showbox and \tracingoutput is what the node
% that last used those words left there. So the node sizes, the words
% |get_node| writes, and the file tags are re-specified here exactly as
% those change files and |synctexstartinput| (synctex.c) make them, and the
% memory accounting of \tracingstats agrees with pdfTeX's as a consequence.
%
% Not re-specified: the .synctex file (the controller's |synctex_sheet|,
% |synctex_hlist|, ... messages write only to it, never to |mem| or to a
% variable of TeX's), the -synctex option (refused, src/cli.rs), and
% |synctex_init_command|'s reset of \synctex at the start of a run.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.3230 - synctex-rec.ch0: |get_node| records the source position
get_node:=r;
exit:end;
@y
@<Initialize bigger nodes with {\sl Sync\TeX} information@>;
get_node:=r;
exit:end;
@z

@x pdftex.web l.3409 - synctex-def.ch0, synctex-mem.ch0: two more words in a box node
@d hlist_node=0 {|type| of hlist nodes}
@d box_node_size=7 {number of words to allocate for a box node}
@y
@d synctex_field_size=2 {Declare the {\sl Sync\TeX} field size to store the {\sl Sync\TeX} information:
                          2 integers for file tag and line}
@d sync_tag(#) == mem[#-synctex_field_size].int {The tag subfield}
@d sync_line(#) == mem[#-synctex_field_size+1].int {The line subfield}
@#
@d hlist_node=0 {|type| of hlist nodes}
@d box_node_size=7+synctex_field_size {number of words to allocate for a box node}
@z

@x pdftex.web l.3456 - synctex-mem.ch0: two more words in a rule node
@d rule_node_size=4 {number of words to allocate for a rule node}
@y
@d rule_node_size=4+synctex_field_size {number of words to allocate for a rule node}
@z

@x pdftex.web l.3495 - synctex-mem.ch0: math, glue, kern and penalty nodes are medium-sized
@d small_node_size=2 {number of words to allocate for most node types}
@y
@d small_node_size=2 {number of words to allocate for most node types}
@d medium_node_size=small_node_size+synctex_field_size {number of words to
           allocate for synchronized node types like math, kern, glue and penalty nodes}
@z

@x pdftex.web l.3628 - synctex-mem.ch0: |new_math|
begin p:=get_node(small_node_size); type(p):=math_node;
@y
begin p:=get_node(medium_node_size); type(p):=math_node;
@z

@x pdftex.web l.3724 - synctex-mem.ch0: |new_param_glue|
begin p:=get_node(small_node_size); type(p):=glue_node; subtype(p):=n+1;
@y
begin p:=get_node(medium_node_size); type(p):=glue_node; subtype(p):=n+1;
@z

@x pdftex.web l.3736 - synctex-mem.ch0: |new_glue|
begin p:=get_node(small_node_size); type(p):=glue_node; subtype(p):=normal;
@y
begin p:=get_node(medium_node_size); type(p):=glue_node; subtype(p):=normal;
@z

@x pdftex.web l.3803 - synctex-mem.ch0: |new_kern|
begin p:=get_node(small_node_size); type(p):=kern_node;
@y
begin p:=get_node(medium_node_size); type(p):=kern_node;
@z

@x pdftex.web l.3826 - synctex-mem.ch0: |new_penalty|
begin p:=get_node(small_node_size); type(p):=penalty_node;
@y
begin p:=get_node(medium_node_size); type(p):=penalty_node;
@z

@x pdftex.web l.4551 - synctex-mem.ch0: |flush_node_list| frees nodes with their size
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

@x pdftex.web l.4626 - synctex-rec.ch0: |copy_node_list| copies a box's position
hlist_node,vlist_node,unset_node: begin r:=get_node(box_node_size);
@y
hlist_node,vlist_node,unset_node: begin r:=get_node(box_node_size);
  @<Copy the box {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.4631 - synctex-rec.ch0: but not a rule's
rule_node: begin r:=get_node(rule_node_size); words:=rule_node_size;
@y
rule_node: begin r:=get_node(rule_node_size); words:=rule_node_size-synctex_field_size;{{\sl Sync\TeX}: do not let \TeX\ copy the {\sl Sync\TeX} information}
  @<Copy the rule {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.4640 - synctex-mem.ch0, synctex-rec.ch0: medium-sized copies
glue_node: begin r:=get_node(small_node_size); add_glue_ref(glue_ptr(p));
@y
glue_node: begin r:=get_node(medium_node_size); add_glue_ref(glue_ptr(p));
  @<Copy the medium sized node {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.4643 - synctex-mem.ch0: medium-sized copies
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

@x pdftex.web l.7691 - synctex-mem.ch0: each input level has a file tag
@!in_state_record = record
  @!state_field, @!index_field: quarterword;
  @!start_field,@!loc_field, @!limit_field, @!name_field: halfword;
  end;
@y
@!in_state_record = record
  @!state_field, @!index_field: quarterword;
  @!start_field,@!loc_field, @!limit_field, @!name_field: halfword;
  @!synctex_tag_field: integer; {stack the tag of the current file}
  end;
@z

@x pdftex.web l.7711 - synctex-mem.ch0: |synctex_tag|
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

@x pdftex.web l.22327 - synctex-mem.ch0: an italic correction is a medium-sized node
    begin free_node(r,small_node_size); link(q):=null;
@y
    begin free_node(r,medium_node_size); link(q):=null;
@z

@x pdftex.web l.26545 - synctex-rec.ch0: a kern made while hyphenating
  begin link(t):=new_kern(w); t:=link(t); w:=0;
@y
  begin link(t):=new_kern(w); t:=link(t); w:=0;
    sync_tag(t+medium_node_size):=0; {{\sl Sync\TeX}: do nothing, it is too late}
@z

@x pdftex.web l.38053 - synctex-e-rec.ch0: the kern that starts a reversed hlist
begin save_h:=cur_h; temp_ptr:=p; p:=new_kern(0); link(prev_p):=p;
@y
begin save_h:=cur_h; temp_ptr:=p; p:=new_kern(0);
sync_tag(p+medium_node_size):=0; {{\sl Sync\TeX}: do nothing, it is too late}
link(prev_p):=p;
@z

@x pdftex.web l.38066 - synctex-e-mem.ch0: |p| is a |math_node|
begin save_h:=cur_h; temp_ptr:=link(p); rule_wd:=width(p);
free_node(p,small_node_size);
@y
begin save_h:=cur_h; temp_ptr:=link(p); rule_wd:=width(p);
free_node(p,medium_node_size); {{\sl Sync\TeX}: p is a |math_node|}
@z

@x pdftex.web l.38125 - synctex-e-mem.ch0: a kern node is medium-sized
if type(p)=kern_node then if (rule_wd=0)or(l=null) then
  begin free_node(p,small_node_size); p:=l;
  end;
@y
if type(p)=kern_node then if (rule_wd=0)or(l=null) then
  begin free_node(p,medium_node_size); p:=l;
  end;
@z

@x pdftex.web l.38178 - synctex-e-mem.ch0: |p| is a |kern_node|
begin free_node(p,small_node_size);
link(t):=q; width(t):=rule_wd; edge_dist(t):=-cur_h-rule_wd; goto done;
@y
begin free_node(p,medium_node_size); {{\sl Sync\TeX}: p is a |kern_node|}
link(t):=q; width(t):=rule_wd; edge_dist(t):=-cur_h-rule_wd; goto done;
@z

@x pdftex.web l.38217 - synctex-e-rec.ch0: |just_copy| copies a box's position
  hlist_node,vlist_node: begin r:=get_node(box_node_size);
@y
  hlist_node,vlist_node: begin r:=get_node(box_node_size);
    @<Copy the box {\sl Sync\TeX} information@>;
@z

@x pdftex.web l.38226 - synctex-e-mem.ch0, synctex-e-rec.ch0: medium-sized copies
  kern_node,math_node: begin r:=get_node(small_node_size);
    words:=small_node_size;
    end;
  glue_node: begin r:=get_node(small_node_size); add_glue_ref(glue_ptr(p));
    glue_ptr(r):=glue_ptr(p); leader_ptr(r):=null;
    end;
@y
  kern_node,math_node: begin
      words:=medium_node_size; {{\sl Sync\TeX}: proper size for math and kern}
      r:=get_node(words);
    end;
  glue_node: begin r:=get_node(medium_node_size); add_glue_ref(glue_ptr(p));
                                                 {{\sl Sync\TeX}: proper size for glue}
    @<Copy the medium sized node {\sl Sync\TeX} information@>;
    glue_ptr(r):=glue_ptr(p); leader_ptr(r):=null;
    end;
@z

@x pdftex.web l.38300 - synctex-e-mem.ch0: drop an unused label
procedure just_reverse(@!p:pointer);
label found,done;
@y
procedure just_reverse(@!p:pointer);
label done;
@z

@x pdftex.web l.38323 - synctex-e-mem.ch0
found:width(t):=width(p); link(t):=q; free_node(p,small_node_size);
@y
width(t):=width(p); link(t):=q; free_node(p,small_node_size);
@z

@x pdftex.web l.38330 - synctex-e-mem.ch0: the math node ending the segment is medium-sized
    begin type(p):=kern_node; incr(LR_problems);
    end
  else  begin pop_LR;
    if n>min_halfword then
      begin decr(n); decr(subtype(p)); {change |after| into |before|}
      end
    else  begin if m>min_halfword then decr(m)@+else goto found;
      type(p):=kern_node;
      end;
    end
@y
    begin type(p):=kern_node; incr(LR_problems);
        {{\sl Sync\TeX} node size watch point: |math_node| size == |kern_node| size}
    end
  else  begin pop_LR;
    if n>min_halfword then
      begin decr(n); decr(subtype(p)); {change |after| into |before|}
      end
    else  begin if m>min_halfword then decr(m)@+else begin
    width(t):=width(p); link(t):=q; free_node(p,medium_node_size);
{{\sl Sync\TeX}: no more "goto found", and proper node size}
    goto done;
  end;
  type(p):=kern_node;
            {{\sl Sync\TeX} node size watch point: |math_node| size == |kern_node| size}
      end;
    end
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

@ synctex-rec.ch0: every node of |medium_node_size| words or more gets the
current file tag and line in its last two words when it is allocated, whether
or not it is a synchronized node.

@<Initialize bigger nodes with {\sl Sync\TeX} information@>=
if s>=medium_node_size then
begin
  sync_tag(r+s):=synctex_tag;
  sync_line(r+s):=line;
end;

@ The file tags are |synctexstartinput|'s (synctex.c): a counter of the files
\.{\\input} so far, the first (normally \.{\\jobname.tex}) having tag~1. The
terminal and pseudo files have tag~0. (synctex.c stops counting at the largest
unsigned integer, and with \.{-synctex=0}; this engine refuses that option.)

@<Glob...@>=
@!synctex_tag_counter:integer; {the file tag given last}

@ @<Set init...@>=
synctex_tag_counter:=0;

@ @<Prepare new file {\sl Sync\TeX} information@>=
incr(synctex_tag_counter); synctex_tag:=synctex_tag_counter;

@ @<Prepare terminal input {\sl Sync\TeX} information@>=
synctex_tag:=0;

@ @<Prepare pseudo file {\sl Sync\TeX} information@>=
synctex_tag:=0;

@ synctex-rec.ch0: a copy of a box or a glue node keeps its original's
position; math and kern nodes keep the one they got when allocated (for them
|copy_node_list| copies all |medium_node_size| words anyway), and a rule's
copy keeps the position |get_node| gave it.

@<Copy the box {\sl Sync\TeX} information@>=
sync_tag(r+box_node_size):=sync_tag(p+box_node_size);
sync_line(r+box_node_size):=sync_line(p+box_node_size);

@ @<Copy the rule {\sl Sync\TeX} information@>=
{|sync_tag(r+rule_node_size):=sync_tag(p+rule_node_size);|
|sync_line(r+rule_node_size):=sync_line(p+rule_node_size);|}

@ @<Copy the medium sized node {\sl Sync\TeX} information@>=
sync_tag(r+medium_node_size):=sync_tag(p+medium_node_size);
sync_line(r+medium_node_size):=sync_line(p+medium_node_size);

@* \[55] Index.
@z
