% changes/diagnostics.ch -- where the diagnostics side channel observes the
% engine (lane P5-DIAGNOSTICS; docs/protocol/display-list-v3.md, `diag-v1`).
%
% The host reports every error and warning to the editor as a structured
% diagnostic: the file, line and column TeX was reading (the split point of
% the `l.<n>' context line), the chain of input levels (TeX's
% \errorcontextlines context, every level), the help text, and for an
% overfull or underfull box the source position of its first and last
% node. TeX computes all of this already; what it lacks is a way to hand it
% to anyone but the terminal. This file adds calls to routines of
% src/diag.rs at the places the information exists, and nothing else: the
% routines do nothing unless the host turned the side channel on, they
% only read TeX's variables, and they never print. The terminal and the log
% stay byte-identical (DESIGN.md §1.1, P-T1).
%
% * |dg_mark|, as |print_err| starts an error message: where the message
%   starts in the captured terminal.
% * |dg_error|, as |error| starts (the message is printed; the input stack,
%   the help lines and |use_err_help| are as the error left them).
% * |dg_mark| and |dg_pdf_warning| around |pdf_warning|'s message.
% * |dg_box_begin(r)| and |dg_box_end(r)| around the report of an
%   overfull, underfull, tight or loose box |r| (|hpack|, |vpackage|).
% * |dg_def_begin| and |dg_def(def_ref)| in \.{\\def} and its relatives:
%   where a macro was defined, for the trace's definition sites.
% * |dg_write_begin(j)| and |dg_write_end(j)| around a \.{\\write} to the
%   terminal (LaTeX's and packages' warnings: \.{\\GenericWarning} is an
%   \.{\\immediate\\write}).
%
% GPL-2.0-or-later, like the rest of crates/flashtex-engine.

@x pdftex.web l.1887 - print_err (as web2c-run.ch has it): the message starts
@d print_err(#)==begin if interaction=error_stop_mode then wake_up_terminal;
@y
@d print_err(#)==begin dg_mark; if interaction=error_stop_mode then wake_up_terminal;
@z

@x pdftex.web l.2028 - error: the diagnostics side channel notes the error
begin if history<error_message_issued then history:=error_message_issued;
@y
begin if history<error_message_issued then history:=error_message_issued;
dg_error;
@z

@x pdftex.web l.15646 - pdf_warning: the message starts
    print("pdfTeX warning");
@y
    dg_mark; print("pdfTeX warning");
@z

@x pdftex.web l.15652 - pdf_warning: the message ends
    print(": "); print(p);
    if append_nl then
        print_ln;
@y
    print(": "); print(p); dg_pdf_warning;
    if append_nl then
        print_ln;
@z

@x pdftex.web l.21199 - hpack: a box report's position
@ @<Finish issuing a diagnostic message for an overfull or underfull hbox@>=
if output_active then print(") has occurred while \output is active")
@y
@ @<Finish issuing a diagnostic message for an overfull or underfull hbox@>=
dg_box_begin(r); if output_active then print(") has occurred while \output is active")
@z

@x pdftex.web l.21211 - hpack: the report is complete on the terminal
begin_diagnostic; show_box(r); end_diagnostic(true)
@y
dg_box_end(r); begin_diagnostic; show_box(r); end_diagnostic(true)
@z

@x pdftex.web l.21376 - vpackage: a box report's position
@ @<Finish issuing a diagnostic message for an overfull or underfull vbox@>=
if output_active then print(") has occurred while \output is active")
@y
@ @<Finish issuing a diagnostic message for an overfull or underfull vbox@>=
dg_box_begin(r); if output_active then print(") has occurred while \output is active")
@z

@x pdftex.web l.21386 - vpackage: the report is complete on the terminal
begin_diagnostic; show_box(r); end_diagnostic(true)
@y
dg_box_end(r); begin_diagnostic; show_box(r); end_diagnostic(true)
@z

@x pdftex.web l.31725 - \def: where the definition starts
  e:=(cur_chr>=2); get_r_token; p:=cur_cs;
@y
  e:=(cur_chr>=2); get_r_token; p:=cur_cs; dg_def_begin;
@z

@x pdftex.web l.31731 - \def: the macro's token list
  define(p,call+(a mod 4),def_ref);
@y
  define(p,call+(a mod 4),def_ref); dg_def(def_ref);
@z

@x pdftex.web l.36107 - write_out: a \write to the terminal
token_show(def_ref); print_ln;
@y
dg_write_begin(j); token_show(def_ref); print_ln; dg_write_end(j);
@z

@x the new sections go at the end of part 54
@* \[55] Index.
@y
@ The diagnostics side channel's routines (src/diag.rs, see the top of
changes/diagnostics.ch).

@<Declare the routines of pdf\TeX's C parts@>=
procedure dg_mark; external;
procedure dg_error; external;
procedure dg_pdf_warning; external;
procedure dg_box_begin(@!r:pointer); external;
procedure dg_box_end(@!r:pointer); external;
procedure dg_def_begin; external;
procedure dg_def(@!p:pointer); external;
procedure dg_write_begin(@!j:integer); external;
procedure dg_write_end(@!j:integer); external;

@* \[55] Index.
@z
