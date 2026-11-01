% changes/displaylist.ch -- where the display list's source map observes
% the engine (the pdfTeX engine's changes/displaylist.ch, for xetex.web).
%
% Unicode mode's display list (src/out) is read from the XDV stream
% `ship_out` writes. What the XDV stream cannot say is which node drew
% what, and where in the source that node came from. That is what this
% file adds, and nothing else: calls to routines of src/dlsource.rs that
% do nothing unless a display list was asked for, and that never change a
% variable of TeX's.
%
% * |dl_new_node(p)|, as a node is allocated (|get_avail|, |fast_get_avail|,
%   |get_node|): the node's source position (file, line, column), kept in
%   a side table indexed like |mem|.
% * |dl_copy(r,p)| in |copy_node_list|, and where XeTeX merges native words
%   into a new node at output (|hlist_out|) or packaging (|hpack|): the new
%   node keeps the position of the node it is made from.
% * |dl_hyph_begin(ha)| before a word is prepared for hyphenation (a native
%   word is split there, before |hyphenate|) and |dl_hyph_end| when the
%   attempt ends (|done1|): the nodes made meanwhile get the word's
%   position, not that of the paragraph's end.
% * |dl_token_begin| and |dl_token_end| around the allocations that never
%   become part of a list TeX ships (|back_input|'s token, the condition
%   stack's node).
% * |dl_node(p)| as |hlist_out| and |vlist_out| output node |p|: the
%   writer notes where the XDV stream is at that moment, so that what is
%   drawn from there on is attributed to |p|.
%
% XeTeX differences: the hyphenation hooks surround the native-word
% preparation as well as |hyphenate|; the two native-word merges; a native
% word's column is its first character's, found when it is measured
% (src/dlsource.rs, |set_native_metrics|).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-xetex.

@x xetex.web l.3232 - get_avail: the display list notes the new node
get_avail:=p;
@y
dl_new_node(p);
get_avail:=p;
@z

@x xetex.web l.3252 - fast_get_avail: the display list notes the new node
  else  begin avail:=link(#); link(#):=null;
    @!stat incr(dyn_used);@+tats@/
    end;
@y
  else  begin avail:=link(#); link(#):=null;
    @!stat incr(dyn_used);@+tats@/
    dl_new_node(#);
    end;
@z

@x xetex.web l.3325 - get_node: the display list notes the new node
found: link(r):=null; {this node is now nonempty}
@y
found: link(r):=null; {this node is now nonempty}
dl_new_node(r);
@z

@x xetex.web l.4838 - copy_node_list: a copy keeps its original's place
  begin @<Make a copy of node |p| in node |r|@>;
  link(q):=r; q:=r; p:=link(p);
@y
  begin @<Make a copy of node |p| in node |r|@>;
  dl_copy(r,p);
  link(q):=r; q:=r; p:=link(p);
@z

@x xetex.web l.8228 - back_input: a backed-up token is not a node to note
p:=get_avail; info(p):=cur_tok;
if cur_tok<right_brace_limit then
@y
dl_token_begin; p:=get_avail; dl_token_end; info(p):=cur_tok;
if cur_tok<right_brace_limit then
@z

@x xetex.web l.11817 - the condition stack's node is not a node to note
begin p:=get_node(if_node_size); link(p):=cond_ptr; type(p):=if_limit;
@y
begin dl_token_begin; p:=get_node(if_node_size); dl_token_end;
link(p):=cond_ptr; type(p):=if_limit;
@z

@x xetex.web l.14797 - hlist_out: a merged native word keeps the first one's place
        q:=new_native_word_node(native_font(r), cur_length);
@y
        q:=new_native_word_node(native_font(r), cur_length);
        dl_copy(q,r);
@z

@x xetex.web l.14845 - hlist_out: the display list notes each character
reswitch: if is_char_node(p) then
  begin synch_h; synch_v;
  repeat f:=font(p); c:=character(p);
@y
reswitch: if is_char_node(p) then
  begin synch_h; synch_v;
  repeat dl_node(p); f:=font(p); c:=character(p);
@z

@x xetex.web l.14872 - hlist_out: the display list notes each node
@ @<Output the non-|char_node| |p| for |hlist_out|...@>=
begin case type(p) of
@y
@ @<Output the non-|char_node| |p| for |hlist_out|...@>=
begin dl_node(p); case type(p) of
@z

@x xetex.web l.15061 - vlist_out: the display list notes each node
@ @<Output the non-|char_node| |p| for |vlist_out|@>=
begin case type(p) of
@y
@ @<Output the non-|char_node| |p| for |vlist_out|@>=
begin dl_node(p); case type(p) of
@z

@x xetex.web l.21505 - line_break: nodes made for hyphenation keep the word's position
  if l_hyf+r_hyf>max_hyphenatable_length then goto done1;
if is_native_word_node(ha) then begin
@y
  if l_hyf+r_hyf>max_hyphenatable_length then goto done1;
dl_hyph_begin(ha);
if is_native_word_node(ha) then begin
@z

@x xetex.web l.21515 - line_break: the hyphenation attempt ends
  hyphenate;
  end;
done1: end
@y
  hyphenate;
  end;
done1: dl_hyph_end; end
@z

@x xetex.web l.30087 - hpack: a merged native word keeps the first fragment's place
      pp:=new_native_word_node(native_font(p), total_chars); { make new node for merged word }
@y
      pp:=new_native_word_node(native_font(p), total_chars); { make new node for merged word }
      dl_copy(pp,p);
@z

@x xetex.web l.34414 - new sections at the end of part 54
@* \[55] Index.
@y
@ The display list's source map (src/dlsource.rs, see the top of
changes/displaylist.ch).

@<Declare the routines of \XeTeX's C parts@>=
procedure dl_new_node(@!p:pointer); external;
procedure dl_copy(@!r,@!p:pointer); external;
procedure dl_node(@!p:pointer); external;
procedure dl_hyph_begin(@!p:pointer); external;
procedure dl_hyph_end; external;
procedure dl_token_begin; external;
procedure dl_token_end; external;

@* \[55] Index.
@z
