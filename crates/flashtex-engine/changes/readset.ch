% changes/readset.ch -- L5 read-sets (DESIGN.md section 5.5): which control
% sequences a run looked at, in the order it first did.
%
% The previous run's .aux is fixed input: when a pass finds that an entry
% changed (\.{\\r@...}, \.{\\b@...}, anything the .aux defines), it re-runs
% only from the first place that read a changed entry, so every place the
% engine looks at a control sequence's meaning or name must be seen:
% |get_next| reading |eq_type| and |equiv| of a control sequence (from a
% file, a token list, an active character, the \.{\\par} of an empty line,
% a \.{\\noexpand}ed one), and |id_lookup|, which \.{\\csname} and e-TeX's
% \.{\\ifcsname} use (a name looked up and not found is a read too: a
% changed .aux may define it).
%
% While |rs_on| (the host turns it on at the .aux point, before the .aux is
% read) each site calls the hand-written |flashtex_cs_read| or
% |flashtex_id_read| (src/readset.rs) the first time a control sequence is
% seen: |rs_seen| remembers which were, in the word space, so that a
% restore puts it back with the rest of the state. None of this changes what
% the program computes: the new variables are read only by these tests and
% by the host.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.6585 - a looked-up name is a read (section 5.5)
found: id_lookup:=p;
@y
found: if rs_on then flashtex_id_read(j,l,p);
id_lookup:=p;
@z

@x pdftex.web l.8630 - the \par of an empty line is a read
begin loc:=limit+1; cur_cs:=par_loc; cur_cmd:=eq_type(cur_cs);
cur_chr:=equiv(cur_cs);
@y
begin loc:=limit+1; cur_cs:=par_loc; cur_cmd:=eq_type(cur_cs);
cur_chr:=equiv(cur_cs);
if rs_on then if not rs_seen[cur_cs] then flashtex_cs_read(cur_cs);
@z

@x pdftex.web l.8661 - an active character is a read
cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs); state:=mid_line;
@y
cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs); state:=mid_line;
if rs_on then if not rs_seen[cur_cs] then flashtex_cs_read(cur_cs);
@z

@x pdftex.web l.8696 - a control sequence scanned from a file is a read
found: cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
@y
found: cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
if rs_on then if not rs_seen[cur_cs] then flashtex_cs_read(cur_cs);
@z

@x pdftex.web l.8748 - a control sequence token of a list is a read
    begin cur_cs:=t-cs_token_flag;
    cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
@y
    begin cur_cs:=t-cs_token_flag;
    cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
    if rs_on then if not rs_seen[cur_cs] then flashtex_cs_read(cur_cs);
@z

@x pdftex.web l.8775 - so is a \noexpand'ed one
begin cur_cs:=info(loc)-cs_token_flag; loc:=null;@/
cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
@y
begin cur_cs:=info(loc)-cs_token_flag; loc:=null;@/
cur_cmd:=eq_type(cur_cs); cur_chr:=equiv(cur_cs);
if rs_on then if not rs_seen[cur_cs] then flashtex_cs_read(cur_cs);
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The read-set's variables. They are zero when the engine is made; only
the host turns |rs_on| on.

@<Glob...@>=
@!rs_on:boolean; {note the control sequences read}
@!rs_seen:array[0..eqtb_top] of boolean;
  {read since the read-set began}

@ @<Declare the routines of pdf\TeX's C parts@>=
procedure flashtex_cs_read(@!p:pointer); external;
procedure flashtex_id_read(@!j,@!l:integer;@!p:pointer); external;

@* \[55] Index.
@z
