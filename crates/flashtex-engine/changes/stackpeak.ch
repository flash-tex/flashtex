% changes/stackpeak.ch -- the stack high-water marks a recording reaches.
%
% The end-of-job statistics (tex.web section 1334, "stack positions") print
% the high-water marks |max_in_stack|, |max_nest_stack|,
% |max_param_stack|, |max_buf_stack| and |max_save_stack|. A replayed
% macro (the intrinsics, src/intrinsics.rs; BOX-MEMO, src/boxmemo.rs) does
% not push what its normal expansion pushes, so a replay at a deeper place
% than its recording left the marks lower than pdfTeX's. Each mark gets a
% shadow |pk_...|, raised where the mark is raised, which a recording lowers
% to 0 when it begins and reads, relative to where it began, when it ends:
% the peak its normal path reached. A replay raises the marks (and the
% shadows) to where the normal path would have taken them.
%
% Always |pk_x<=max_x| (a shadow is raised only with its mark, lowered only
% by a recording, and a recording's end leaves it at most the mark), so
% testing the shadow first and the mark inside skips no raise of the mark,
% and outside recordings, where the two are equal, it costs the one
% comparison it replaces. The overflow tests are where they were. Applied
% last: the |@x| lines are the text after every other change file.
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.5016 (after the change files) - the shadow of |max_nest_stack|
nest_ptr:=0; max_nest_stack:=0;
@y
nest_ptr:=0; max_nest_stack:=0; pk_nest_stack:=0;
@z

@x pdftex.web l.5028 (after the change files) - push_nest
begin if nest_ptr>max_nest_stack then
  begin max_nest_stack:=nest_ptr;
  if nest_ptr=nest_size then overflow("semantic nest size",nest_size);
@:TeX capacity exceeded semantic nest size}{\quad semantic nest size@>
  end;
@y
begin if nest_ptr>pk_nest_stack then
  begin pk_nest_stack:=nest_ptr;
  if nest_ptr>max_nest_stack then
  begin max_nest_stack:=nest_ptr;
  if nest_ptr=nest_size then overflow("semantic nest size",nest_size);
@:TeX capacity exceeded semantic nest size}{\quad semantic nest size@>
  end;
  end;
@z

@x pdftex.web l.7092 (after the change files) - the shadow of |max_save_stack|
max_save_stack:=0;
@y
max_save_stack:=0; pk_save_stack:=0;
@z

@x pdftex.web l.7098 (after the change files) - check_full_save_stack
@d check_full_save_stack==if save_ptr>max_save_stack then
  begin max_save_stack:=save_ptr;
  if max_save_stack>save_size-7 then overflow("save size",save_size);
@:TeX capacity exceeded save size}{\quad save size@>
  end
@y
@d check_full_save_stack==if save_ptr>pk_save_stack then
  begin pk_save_stack:=save_ptr;
  if save_ptr>max_save_stack then
  begin max_save_stack:=save_ptr;
  if max_save_stack>save_size-7 then overflow("save size",save_size);
@:TeX capacity exceeded save size}{\quad save size@>
  end;
  end
@z

@x pdftex.web l.8200 (after the change files) - push_input
  begin if input_ptr>max_in_stack then
    begin max_in_stack:=input_ptr;
    if input_ptr=stack_size then overflow("input stack size",stack_size);
@:TeX capacity exceeded input stack size}{\quad input stack size@>
    end;
@y
  begin if input_ptr>pk_in_stack then
    begin pk_in_stack:=input_ptr;
    if input_ptr>max_in_stack then
    begin max_in_stack:=input_ptr;
    if input_ptr=stack_size then overflow("input stack size",stack_size);
@:TeX capacity exceeded input stack size}{\quad input stack size@>
    end;
    end;
@z

@x pdftex.web l.8355 (after the change files) - the shadows of the input marks
begin input_ptr:=0; max_in_stack:=0;
@y
begin input_ptr:=0; max_in_stack:=0; pk_in_stack:=0; pk_buf_stack:=0;
pk_param_stack:=0;
@z

@x pdftex.web l.9134 (after the change files) - \.{\\csname} in the buffer
  begin if j>=max_buf_stack then
    begin max_buf_stack:=j+1;
    if max_buf_stack=buf_size then
      overflow("buffer size",buf_size);
@:TeX capacity exceeded buffer size}{\quad buffer size@>
    end;
@y
  begin if j>=pk_buf_stack then
    begin pk_buf_stack:=j+1;
    if j>=max_buf_stack then
    begin max_buf_stack:=j+1;
    if max_buf_stack=buf_size then
      overflow("buffer size",buf_size);
@:TeX capacity exceeded buffer size}{\quad buffer size@>
    end;
    end;
@z

@x pdftex.web l.9362 (after the change files) - the parameter stack
  begin if param_ptr+n>max_param_stack then
    begin max_param_stack:=param_ptr+n;
    if max_param_stack>param_size then
      overflow("parameter stack size",param_size);
@:TeX capacity exceeded parameter stack size}{\quad parameter stack size@>
    end;
@y
  begin if param_ptr+n>pk_param_stack then
    begin pk_param_stack:=param_ptr+n;
    if param_ptr+n>max_param_stack then
    begin max_param_stack:=param_ptr+n;
    if max_param_stack>param_size then
      overflow("parameter stack size",param_size);
@:TeX capacity exceeded parameter stack size}{\quad parameter stack size@>
    end;
    end;
@z

@x pdftex.web l.38576 (after the change files) - \.{\\scantokens} in the buffer
  if last>=max_buf_stack then max_buf_stack:=last+1;
@y
  if last>=pk_buf_stack then
    begin pk_buf_stack:=last+1;
    if last>=max_buf_stack then max_buf_stack:=last+1;
    end;
@z

@x pdftex.web l.38702 (after the change files) - \.{\\ifcsname} in the buffer
  begin if m>=max_buf_stack then
    begin max_buf_stack:=m+1;
    if max_buf_stack=buf_size then
      overflow("buffer size",buf_size);
@:TeX capacity exceeded buffer size}{\quad buffer size@>
    end;
@y
  begin if m>=pk_buf_stack then
    begin pk_buf_stack:=m+1;
    if m>=max_buf_stack then
    begin max_buf_stack:=m+1;
    if max_buf_stack=buf_size then
      overflow("buffer size",buf_size);
@:TeX capacity exceeded buffer size}{\quad buffer size@>
    end;
    end;
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The shadows of the stack high-water marks (see the top of
\.{changes/stackpeak.ch}).

@<Glob...@>=
@!pk_in_stack,@!pk_nest_stack,@!pk_param_stack,@!pk_buf_stack,
  @!pk_save_stack:integer; {at most |max_in_stack| \dots; lowered by a
  recording}

@* \[55] Index.
@z
