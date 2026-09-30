Lines outside @x...@z groups are comments, as in any WEB change file.
This one exercises: blank lines after @x (skipped when priming), upper-case
control codes, a change to a macro, a change to a constant, a replacement
that starts a new section, a deletion, and a multi-line match.

@x a blank line after @x is skipped

@d banner=="This is SAMPLE, Version 1.0"
@y
@d banner=="This is SAMPLE, Version 1.0-changed"
@z

@X [2] upper-case control codes work too
@!line_max=size+2;
@Y
@!line_max=size+5; {changed}
@!extra=7;
@Z

@x [4] a replacement that adds a section, and a new pool string
total:="pool string two";
@y
total:="pool string two";

@ A section that only exists in the change file.

@<Initialize@>=
total:="a string from the change file";
@z

@x [6] delete one line
k:="the first line to be deleted";
@y
@z

@x [6] replace two lines by three
if total>line_max then total:=line_max;
total:=total+k;
@y
if total>line_max then total:=line_max
else total:=total+extra;
total:=total+k+k;
@z

Trailing comment lines are ignored as well.
