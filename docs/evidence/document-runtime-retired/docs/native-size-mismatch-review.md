# Native sibling size mismatch: bounded review

Source: issue2 comment5646543926 and mac-shell3162b89. No workload, cap or
production change. The pass4-cap15-14pages summary records a13,655,250-byte sibling
for a14-page seed. Its raw default-cap cell records one v1 preview, one compiler
session failure and no candidate. The configured15MiB cell records20 candidates,
7 paints and13 paint-time drops. These are owner measurements, not independent
Linux performance observations. The separately archived exchange is a small
successful exchange: ready advertises8388608 compiler-frame bytes and16777216
helper-output bytes. It is not the failing14-page producer wire.

This failure matches existing runtime framing: producer JSON may fit its16MiB
limit while exceeding runtime's8MiB newline-inclusive default. After a nonfailed
v1 accepts display-list-v2, the next complete sibling is required protocol input.
An oversized/truncated sibling cannot simply become an optional output-slot drop:
framing/validation did not complete, and the accepted transaction is unfinished.
Optional replacement applies after validated transport ownership. Session failure
is expected here; the cross-component launch policy is the integration gap.

Current layout capabilities carry formats, not numeric receiving budgets. The
helper ready advertisement does not negotiate a producer limit. Existing exact9aa
producer environment FLASHTEX_MAX_REPLY_BYTES can provide a compatible launcher
policy without a protocol/parser change: explicitly set its positive effective
JSON cap no greater than runtime max_frame minus one newline byte, preserving
any stricter user cap. Apply consistently to initial launch and restart; do not
assume changing the helper cap changes the inherited producer environment.
Producer then declines oversized v2 before echoing acceptance, retains full v1
when it fits, or returns explicit failed v1 when it does not. Its tiny-limit
minimal error envelope is not recursively cap-checked, as documented elsewhere.

Helper launch/native integration owners should decide and test that policy using
the existing producer limit controls. Runtime must retain complete sibling source,
identity, syntax and frame validation. Helper wrapper overhead and Value numeric
expansion remain independently bounded; matching producer/runtime line caps is
not a universal helper-output fit guarantee. Increasing the runtime ceiling alone
is neither a responsiveness fix nor full-budget negotiation. No page/delta change
is justified or activated by this review.

## Published launch-policy review

Read exact helper71049ffa (no suite rerun). `producer_command_with_limit` sets a
child-local environment value; both initial session startup and explicit restart
use it. UTF8/usize/positive parsing matches9aa. Although the helper does not
explicitly repeat the producer16MiB clamp, its validated maximum is15MiB, making
the results equivalent. It reserves exactly one framing newline and preserves
stricter positive settings, including1. That setting may yield a larger minimal
failed envelope; the patch does not claim one-byte wire compliance.

Owner tests cover actual child environment and startup/restart, plus invalid,
zero, whitespace, negative, leading-plus, leading-zero and overflow inputs.
Suggested additional boundary assertions (not defects found): empty/nonUTF8
values, exact ceiling, and128/default8MiB/maximum15MiB receiving limits.
No accepted-sibling validation, optional queue policy or decoder lifecycle is
relaxed. Actual oversized-producer decline acceptance remains the owner's next
gate, distinct from this source review and reported synthetic test passes.
