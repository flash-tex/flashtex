% changes/lazyflush.ch -- |flush_list| without walking the list it frees
% (lane CONTIG-TOKENS, docs/design/engine-v2/CONTIG-TOKENS.md; DESIGN.md
% section 4.2's "allowed optimisations" and section 5.6 L6). Always on.
%
% pdftex.web's |flush_list(p)| walks the whole list to find its last node
% and to count it out of |dyn_used|, then links that node to |avail|. Every
% macro's parameters, every backed-up token and every token list a
% definition replaces go through it. Here it does not walk: the free list
% becomes a stack of segments. |avail| heads the first; each segment ends
% in a null link, and |seg_stack[seg_ptr-1]|, ..., |seg_stack[0]| say where
% the free list continues after it. So |flush_list(p)|, for a list of more
% than one node, pushes |avail| (when it is not null) and sets |avail:=p|;
% |get_avail|, finding |avail| null, pops the next segment before it
% considers |mem_end| and |hi_mem_min|.
%
% (1) Nothing pdfTeX allocates moves. Let the free list be what pdftex.web
% keeps: |avail|, then |link| to null. Here it is the concatenation of
% |avail|'s chain and the chains of |seg_stack| from the top down. Every
% operation keeps the two equal as sequences:
%   - pdftex.web's |flush_list(p)| makes the sequence p's nodes, then the
%     old sequence; here |avail:=p| starts it with p's nodes, whose chain
%     ends in null, and the old sequence's head (|avail|) is now the top of
%     the stack: the same sequence. (A null |avail| is not pushed: the old
%     sequence is then the stack's, which follows p's anyway.) A list of
%     one node (most are a backed-up token) is linked to |avail|, which is
%     what the walk does to it; a longer list while the stack is full is
%     walked and linked as before: the same sequence.
%   - |free_avail|, |fast_get_avail| and |get_avail| (displaylist.ch's
%     |dl_new_node| included) take or put one node at the head through
%     |avail|; when |avail| is null, the head is the top segment's, which
%     |get_avail| pops first, and |fast_get_avail| calls |get_avail| when
%     |avail| is null. Every other use of |avail| in pdftex.web is the
%     initialisation (|avail:=null|, |seg_ptr:=0|) or the format's dump and
%     undump (below), and the hand-written code only calls |get_avail|.
% So every one-word node is allocated at the address pdfTeX gives it, in
% the same order, and |mem_end|, |hi_mem_min| and the "main memory size"
% overflow move exactly as before (they change only when the sequence is
% empty). The words that differ are the links of a pending segment's last
% node (null, not the next segment), which nothing reads: a free node is
% read only by the allocation that takes it, which gives it |link:=null|.
%
% (2) |dyn_used| is worked out where it is printed. It is printed in two
% places: the memory-usage lines of |\tracingstats>1| at every shipout
% (section 639; |ship_out|'s and |pdf_ship_out|'s copies) and the format's
% dump (section 1311). The end-of-job block (section 1334) prints
% |lo_mem_max-mem_min+mem_end-hi_mem_min+2|, which (1) leaves alone.
% pdfTeX's |dyn_used| always equals D = |mem_end+1-hi_mem_min| less the
% length of the free list:
%   - at the start: the initialisation sets |dyn_used:=hi_mem_stat_usage|
%     (15), |hi_mem_min:=hi_mem_stat_min| (|mem_top-14|), |mem_end:=mem_top|
%     and |avail:=null|, so D=15; undump sets it to what the dump
%     recomputed as D (section 1311, "so that INITEX dumps valid
%     information even when it has not been gathering statistics") from
%     the hi memory and |avail| it also restores;
%   - after it: |avail|, |hi_mem_min| and |mem_end| are assigned only by
%     |get_avail| (a pop, or |hi_mem_min| or |mem_end| one further: D goes
%     up by one, and so does |dyn_used|), |fast_get_avail| (a pop: +1),
%     |free_avail| (a push: -1) and |flush_list| (n nodes pushed: -n), the
%     initialisation and undump -- in pdftex.web, in every change file and
%     in the hand-written code, which only calls |get_avail| (the hosts'
%     restore puts back a whole state, and |flashtex_merge_free| relinks
%     segments without changing the sequence). A free node's |link| is
%     written only by those routines: a write through a pointer to a node
%     already freed would corrupt pdfTeX's own free list too.
% A |flush_list| that does not walk cannot count, so no routine keeps the
% running count any more: both places print D, |flashtex_dyn_used|
% (src/lazyflush.rs) at the shipouts and the dump's own recomputation, on
% a state whose free list, |hi_mem_min| and |mem_end| are pdfTeX's (1).
% |dyn_used| itself keeps the value the initialisation or the format gave
% it; nothing else reads it. (etrip compares those lines only normalised:
% TeX Live's pdfTeX gives several kinds of node SyncTeX's two extra words,
% which moves |var_used| and |lo_mem_max|. PR #2016 compared the lines of
% every shipout of an arXiv paper and a beamer deck run with
% |\tracingstats=2| with pdfTeX's and with the running count's.)
%
% (3) The hosts see pdftex.web's free list. Every reading of the word
% space (a checkpoint, a restore, a hash, the convergence test, the
% format image) starts with |spill_scalars| (src/checkpoint.rs), which
% first calls |flashtex_merge_free|: each pending segment's last node is
% linked to the next segment, the stack's entries are cleared and
% |seg_ptr:=0|. A popped entry is cleared too, so |seg_stack| is zero above
% |seg_ptr| at all times. After the merge every word is the one the walking
% |flush_list| would have left, but |dyn_used| (2), which every comparison
% of states treats as a statistic (|crate::iso::scalar_covered|). The dump
% merges first too, so the format's words are pdfTeX's; undump is
% unchanged (no list is freed before the format is loaded: |seg_ptr=0|).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.3119 - get_avail: the next segment of the free list
begin p:=avail; {get top location in the |avail| stack}
if p<>null then avail:=link(avail) {and pop it off}
@y
begin p:=avail; {get top location in the |avail| stack}
if p=null then if seg_ptr>0 then
  begin decr(seg_ptr); p:=seg_stack[seg_ptr]; seg_stack[seg_ptr]:=null;
  end; {the free list's next segment}
if p<>null then avail:=link(p) {and pop it off}
@z

@x pdftex.web l.3133 - get_avail: no running count of the nodes in use
@!stat incr(dyn_used);@+tats@;{maintain statistics}
@y
@z

@x pdftex.web l.3143 - free_avail: no running count of the nodes in use
  begin link(#):=avail; avail:=#;
  @!stat decr(dyn_used);@+tats@/
  end
@y
  begin link(#):=avail; avail:=#;
  end
@z

@x pdftex.web l.3155 (as displaylist.ch leaves it) - fast_get_avail: no running count
  else  begin avail:=link(#); link(#):=null;
    @!stat incr(dyn_used);@+tats@/
    dl_new_node(#);
@y
  else  begin avail:=link(#); link(#):=null;
    dl_new_node(#);
@z

@x pdftex.web l.3166 - flush_list: the list freed without walking it
begin if p<>null then
  begin r:=p;
  repeat q:=r; r:=link(r); @!stat decr(dyn_used);@+tats@/
  until r=null; {now |q| is the last node on the list}
  link(q):=avail; avail:=p;
  end;
@y
begin if p<>null then
  if link(p)=null then
    begin link(p):=avail; avail:=p; {a one-node list is its own last node}
    end
  else if seg_ptr<seg_size then
    begin if avail<>null then
      begin seg_stack[seg_ptr]:=avail; incr(seg_ptr);
      end;
    avail:=p;
    end
  else  begin r:=p;
    repeat q:=r; r:=link(r);
    until r=null; {now |q| is the last node on the list}
    link(q):=avail; avail:=p;
    end;
@z

@x pdftex.web l.15029 - the memory-usage lines: pdfTeX's |dyn_used|
  print_int(dyn_used); print_char(";");
@y
  print_int(flashtex_dyn_used); print_char(";");
@z

@x pdftex.web l.15036 - the memory-usage lines: pdfTeX's |dyn_used|
  print_int(dyn_used); print("; still untouched: ");
@y
  print_int(flashtex_dyn_used); print("; still untouched: ");
@z

@x pdftex.web l.32946 - the format's dump: the free list as one chain
sort_avail; var_used:=0;
@y
flashtex_merge_free; sort_avail; var_used:=0;
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The free list's segments (see the top of changes/lazyflush.ch). A list
of one node is linked to |avail|, as walking it would; every longer list
is walked and linked as pdftex.web's |flush_list| does while |seg_size|
segments are pending.

@d seg_size=16383 {the most free-list segments pending}

@<Glob...@>=
@!seg_stack:array[0..seg_size] of pointer;
  {where the free list goes on after each pending segment; null above
  |seg_ptr|}
@!seg_ptr:integer; {the segments pending}

@ @<Set init...@>=
seg_ptr:=0;

@ @<Declare the routines of pdf\TeX's C parts@>=
function flashtex_dyn_used:integer; external;
procedure flashtex_merge_free; external;

@* \[55] Index.
@z
