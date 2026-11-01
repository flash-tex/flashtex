% changes/lazyflush.ch -- |flush_list| without walking the list it frees
% (lane CONTIG-TOKENS, docs/design/engine-v2/CONTIG-TOKENS.md; DESIGN.md
% section 4.2's "allowed optimisations" and section 5.6 L6). Off unless the
% command-line driver switches it on (FLASHTEX_LAZY_FLUSH=1, src/main.rs and
% src/lazyflush.rs); the hosts never do.
%
% pdftex.web's |flush_list(p)| walks the whole list to find its last node
% and to count it out of |dyn_used|, then links that node to |avail|. Every
% macro's parameters, every backed-up token and every token list a
% definition replaces go through it. With |lazy_flush| it does not walk:
% the free list becomes a stack of segments. |avail| heads the first; each
% segment ends in a null link, and |seg_stack[seg_ptr-1]|, ...,
% |seg_stack[0]| say where the free list continues after it. So
% |flush_list(p)|, for a list of more than one node, pushes |avail| (when
% it is not null) and sets |avail:=p|;
% |get_avail|, finding |avail| null, pops the next segment before it
% considers |mem_end| and |hi_mem_min|.
%
% Proof that nothing pdfTeX computes, prints or allocates changes. Let the
% free list be what pdftex.web keeps: |avail|, then |link| to null. Here it
% is the concatenation of |avail|'s chain and the chains of |seg_stack|
% from the top down. Every operation keeps the two equal as sequences:
%   - pdftex.web's |flush_list(p)| makes the sequence p's nodes, then the
%     old sequence; here |avail:=p| starts it with p's nodes, whose chain
%     ends in null, and the old sequence's head (|avail|) is now the top of
%     the stack: the same sequence. (A null |avail| is not pushed: the old
%     sequence is then the stack's, which follows p's anyway.) A list of
%     one node (most are a backed-up token), or any list while the stack
%     is full, is walked and linked as before: the same sequence.
%   - |free_avail|, |fast_get_avail| and |get_avail| (displaylist.ch's
%     |dl_new_node| included) take or put one node at the head through
%     |avail|; when |avail| is null, the head is the top segment's, which
%     |get_avail| pops first, and |fast_get_avail| calls |get_avail| when
%     |avail| is null. Every other use of |avail| in pdftex.web is one of
%     these macros, the initialisation (|avail:=null|, |seg_ptr:=0|) or
%     the format's dump and undump (below).
% So every one-word node is allocated at the address pdfTeX gives it, in
% the same order, and |mem_end|, |hi_mem_min| and the "main memory size"
% overflow move exactly as before (they change only when the sequence is
% empty). The words that differ are the links of a pending segment's last
% node (null, not the next segment), which nothing reads: a free node is
% read only by the allocation that takes it, and it is given |link:=null|.
%
% |dyn_used| is printed in the memory-usage lines of |\tracingstats>1| at
% each shipout and by the format's dump. pdftex.web changes it by one at
% every allocation and every node freed, so it always equals
% |mem_end+1-hi_mem_min| less the length of the free list (it starts as
% |hi_mem_stat_usage|, all of hi mem then; section 1311 recomputes it that
% way). Here |flush_list| does not count, so the lines print
% |flashtex_dyn_used|, which is |dyn_used| when the switch is off and that
% difference when it is on (src/lazyflush.rs). The dump first makes the
% free list one chain again (|flashtex_merge_free|: each segment's last
% node is linked to the next segment), so the format's words are pdfTeX's.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.3119 - get_avail: the next segment of the free list
begin p:=avail; {get top location in the |avail| stack}
if p<>null then avail:=link(avail) {and pop it off}
@y
begin p:=avail; {get top location in the |avail| stack}
if p=null then if seg_ptr>0 then
  begin decr(seg_ptr); p:=seg_stack[seg_ptr]; {the free list's next segment}
  end;
if p<>null then avail:=link(p) {and pop it off}
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
  if lazy_flush and(seg_ptr<seg_size)and(link(p)<>null) then
    begin if avail<>null then
      begin seg_stack[seg_ptr]:=avail; incr(seg_ptr);
      end;
    avail:=p;
    end
  else  begin r:=p;
    repeat q:=r; r:=link(r); @!stat decr(dyn_used);@+tats@/
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
of one node, and every list while |seg_size| segments are pending, is
walked and linked as pdftex.web's |flush_list| does.

@d seg_size=16383 {the most free-list segments pending}

@<Glob...@>=
@!lazy_flush:boolean; {|flush_list| does not walk the list it frees}
@!seg_stack:array[0..seg_size] of pointer;
  {where the free list goes on after each pending segment}
@!seg_ptr:integer; {the segments pending}

@ @<Set init...@>=
lazy_flush:=flashtex_lazy_flush_enabled; seg_ptr:=0;

@ @<Declare the routines of pdf\TeX's C parts@>=
function flashtex_lazy_flush_enabled:boolean; external;
function flashtex_dyn_used:integer; external;
procedure flashtex_merge_free; external;

@* \[55] Index.
@z
