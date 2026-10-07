% changes/displaylist.ch -- where the display-list writer observes the engine.
%
% The engine writes `display-list-v3` (docs/protocol/display-list-v3.md) for
% the preview: at every \shipout, what the page's PDF content stream draws,
% with the source position of the node that drew it. The geometry comes from
% the page content stream itself, which the writer reads as pdfTeX's own
% traversal (|pdf_hlist_out|, |pdf_vlist_out|) produces it, between the C
% calls |pdfshipoutbegin| and |pdfshipoutend| (src/displaylist/). Nothing in
% the traversal is re-implemented.
%
% What the content stream cannot say is which node drew what, and where in
% the source that node came from. That is what this file adds, and nothing
% else: calls to routines of src/displaylist/ that do nothing unless a
% display list was asked for, and that never change a variable of TeX's.
%
% * |dl_new_node(p)|, as a node is allocated (|get_avail|, |fast_get_avail|,
%   |get_node|): the node's source position (file, line, column), which the
%   writer keeps in a side table indexed like |mem|. SyncTeX's file tag
%   and line, in the last two words of some nodes (changes/synctex.ch, kept
%   for TeX Live's memory layout), carry no column and are not used here.
% * |dl_free(p)| and |dl_free_node(p,s)|, as memory is given back
%   (|free_avail|, |flush_list|, |free_node|): the side table's entries of
%   the freed words go back to 0, so that only what is allocated has a
%   position. The incremental engine's convergence test compares the table
%   (src/incr.rs): a word that differs between two runs is then a live
%   node's (or token's) position, never what a freed word happened to keep.
% * |dl_copy(r,p)| in |copy_node_list|: the copy |r| of node |p| keeps |p|'s
%   position (LaTeX's output routine ships copies of what the paragraphs
%   made).
% * |dl_hyph_begin(ha)| and |dl_hyph_end| around |hyphenate|, which rebuilds
%   a word's nodes while the paragraph is broken: the new nodes get the
%   position of the word's first letter, not that of the paragraph's end.
% * |dl_token_begin| and |dl_token_end| around the allocations that never
%   become part of a list TeX ships: the token |back_input| backs up, and the
%   condition stack's node, most of the allocations made outside the scanner
%   in macro-heavy documents (docs/evidence/cold-speed-2026-10-04/). The side
%   table's entry of such a location is set again when it is next allocated
%   as a node.
% * |dl_node(p)| as |pdf_hlist_out| and |pdf_vlist_out| output node |p|: the
%   writer notes where the page stream is at that moment, so that what is
%   drawn from there on is attributed to |p|.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.3134 - get_avail: the display list notes the new node
get_avail:=p;
@y
dl_new_node(p);
get_avail:=p;
@z

@x pdftex.web l.3141 - free_avail: the freed word has no position
@d free_avail(#)== {single-word node liberation}
  begin link(#):=avail; avail:=#;
@y
@d free_avail(#)== {single-word node liberation}
  begin link(#):=avail; avail:=#; dl_free(#);
@z

@x pdftex.web l.3154 - fast_get_avail: the display list notes the new node
  else  begin avail:=link(#); link(#):=null;
    @!stat incr(dyn_used);@+tats@/
    end;
@y
  else  begin avail:=link(#); link(#):=null;
    @!stat incr(dyn_used);@+tats@/
    dl_new_node(#);
    end;
@z

@x pdftex.web l.3168 - flush_list: the freed words have no position
  repeat q:=r; r:=link(r); @!stat decr(dyn_used);@+tats@/
@y
  repeat q:=r; r:=link(r); dl_free(q); @!stat decr(dyn_used);@+tats@/
@z

@x pdftex.web l.3227 - get_node: the display list notes the new node
found: link(r):=null; {this node is now nonempty}
@y
found: link(r):=null; {this node is now nonempty}
dl_new_node(r);
@z

@x pdftex.web l.3293 - free_node: the freed words have no position
begin node_size(p):=s; link(p):=empty_flag;
@y
begin dl_free_node(p,s); node_size(p):=s; link(p):=empty_flag;
@z

@x pdftex.web l.4608 - copy_node_list: a copy keeps its original's place
  begin @<Make a copy of node |p| in node |r|@>;
  link(q):=r; q:=r; p:=link(p);
@y
  begin @<Make a copy of node |p| in node |r|@>;
  dl_copy(r,p);
  link(q):=r; q:=r; p:=link(p);
@z

@x pdftex.web l.8278 - back_input: a backed-up token is not a node to note
p:=get_avail; info(p):=cur_tok;
if cur_tok<right_brace_limit then
@y
dl_token_begin; p:=get_avail; dl_token_end; info(p):=cur_tok;
if cur_tok<right_brace_limit then
@z

@x pdftex.web l.11836 - the condition stack's node is not a node to note
begin p:=get_node(if_node_size); link(p):=cond_ptr; type(p):=if_limit;
@y
begin dl_token_begin; p:=get_node(if_node_size); dl_token_end;
link(p):=cond_ptr; type(p):=if_limit;
@z

@x pdftex.web l.18798 - pdf_hlist_out: the display list notes each character
@ @<Output node |p| for |pdf_hlist_out|...@>=
reswitch: if is_char_node(p) then
  begin
  repeat f:=font(p); c:=character(p);
@y
@ @<Output node |p| for |pdf_hlist_out|...@>=
reswitch: if is_char_node(p) then
  begin
  repeat dl_node(p); f:=font(p); c:=character(p);
@z

@x pdftex.web l.18815 - pdf_hlist_out: the display list notes each node
@ @<Output the non-|char_node| |p| for |pdf_hlist_out|...@>=
begin case type(p) of
@y
@ @<Output the non-|char_node| |p| for |pdf_hlist_out|...@>=
begin dl_node(p); case type(p) of
@z

@x pdftex.web l.18955 - pdf_vlist_out: the display list notes each node
@ @<Output the non-|char_node| |p| for |pdf_vlist_out|@>=
begin case type(p) of
@y
@ @<Output the non-|char_node| |p| for |pdf_vlist_out|@>=
begin dl_node(p); case type(p) of
@z

@x pdftex.web l.26183 - hyphenate: rebuilt nodes keep the word's position
  hyphenate;
@y
  dl_hyph_begin(ha); hyphenate; dl_hyph_end;
@z

@x pdftex.web l.40320 - new sections at the end of part 54
@* \[55] Index.
@y
@ The display-list writer's routines (src/displaylist/, see the top of
changes/displaylist.ch).

@<Declare the routines of pdf\TeX's C parts@>=
{\.{src/displaylist}}
procedure dl_new_node(@!p:pointer); external;
procedure dl_copy(@!r,@!p:pointer); external;
procedure dl_free(@!p:pointer); external;
procedure dl_free_node(@!p:pointer;@!s:halfword); external;
procedure dl_node(@!p:pointer); external;
procedure dl_hyph_begin(@!p:pointer); external;
procedure dl_hyph_end; external;
procedure dl_token_begin; external;
procedure dl_token_end; external;

@ The side table: the source position of the node at each |mem| location,
which only the display-list writer reads and writes. It is an array of the
word space like |mem| itself, so that a checkpoint keeps it as it keeps
|mem| (as the words that changed since the last checkpoint) and the undo
logs' memory budget covers it (DESIGN.md section 5.2). The writer packs a
position into the 64 bits of a |memory_word|.

@<Glob...@>=
@!dl_side:array[mem_bot..mem_max] of memory_word;
  {source positions of the nodes in |mem|}

@* \[55] Index.
@z
