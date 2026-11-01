ENTRY { author title year crossref } { ei } { label }
INTEGERS { i n }
STRINGS { g0 }
FUNCTION {f0}
{ #1 num.names$ }
FUNCTION {deep}
{ #1 #-7 "abc" "{vv~}{ll}{, jj}{, f.}" label g0 ei "u" "l" #0 #1 "abc" #1 "u" "l" author "abc" "u" "abc" "t" "t" "u" title "t" #-7 "t" #-7 #0 "{vv~}{ll}{, jj}{, f.}" g0 #0 #1 "{\'E}mile Zola" "l" #3 ei #1 #3 "abc" author "abc" #3 g0 "l" #1 "l" title author #0 #3 author "l" "l" #1 title "{vv~}{ll}{, jj}{, f.}" "l" #3 'skip$ "abc" author ei title }
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
EXECUTE {f0}
REVERSE {f0}
