ENTRY { author title year crossref } { ei } { label }
INTEGERS { i n }
STRINGS { g0 g1 g2 g3 g4 g5 g6 g7 g8 g9 g10 g11 }
FUNCTION {f0}
{ { #-7 write$ add.period$ } title skip$ call.type$ change.case$ duplicate$ chr.to.int$ duplicate$ purify$ > swap$ quote$ g3 g0 { text.prefix$ title = g9 top$ } g5 duplicate$ empty$ g1 { "u" { g10 "{\'E}mile Zola" g4 } #1 > { #0 call.type$ g11 < pop$ } #1 substring$ format.name$ } := stack$ title skip$ - substring$ g0 ei }
FUNCTION {f1}
{ skip$ ei int.to.chr$ #0 #0 format.name$ { #0 ei chr.to.int$ author } type$ "abc" g4 g0 g11 ei "{\'E}mile Zola" + }
FUNCTION {f2}
{ #3 format.name$ #3 chr.to.int$ #-7 #-7 missing$ g10 }
FUNCTION {f3}
{ newline$ width$ g11 #0 "{\'E}mile Zola" * = g6 author text.prefix$ g5 pop$ g9 text.prefix$ { g9 } }
FUNCTION {article}
{ f0 }
FUNCTION {default.type}
{ f0 }
MACRO {jan} {"January"}
READ
ITERATE {call.type$}
FUNCTION {presort} { cite$ 'label := label purify$ }
ITERATE {presort}
SORT
EXECUTE {f3}
REVERSE {f0}
