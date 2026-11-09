# Replay fuzzer (BOX-MEMO and D9)

A differential fuzzer for the engine's two replay systems: BOX-MEMO
(`src/boxmemo.rs`, `docs/design/engine-v2/BOX-MEMO.md`) and D9's guarded
intrinsics (`src/intrinsics.rs`). A replay is sound only if it gives the run
that the real path would have given, so every generated document runs three
times per system (off, on, verify) and any difference is a finding. There is
no oracle here: "off" is the reference, and pdfTeX's own behaviour is checked
elsewhere (tools/parity, tools/lockstep).

MIT-licensed, Python 3 standard library only.

## Run

```sh
python3 tools/fuzz-replay/fuzz_replay.py run --engine target/release/flashtex-initex \
    --seeds 0-1999 --jobs 2 --nice --out /tmp/fr
python3 tools/fuzz-replay/minimize.py --engine target/release/flashtex-initex \
    /tmp/fr/<finding> /tmp/fr-min
python3 tools/fuzz-replay/fuzz_replay.py check --engine target/release/flashtex-initex /tmp/fr-min
```

`run` writes `summary.json` (case counts, replay hit counts, and each
signature's seeds) and, for each signature, a directory holding the shortest
failing case and `finding.json`. It exits 1 when there is a finding.
`--fonts` copies `cmr10.tfm` and `cmr7.tfm` from `kpsewhich` (data only) and
turns on the font items of the grammar; without it the documents use
`\nullfont` only and need no TeX Live.

Each case is about 25 ms per run on an M-series Mac, six runs per case, so
2,000 cases take about two minutes with two workers. Keep to `--jobs 2
--nice` on a shared Mac.

## What is compared

Every case runs `flashtex-initex -ini -etex '\input t'` in its own
directory, under `FLASHTEX_RESOLVER=cwd`, `SOURCE_DATE_EPOCH=0` and
`FORCE_SOURCE_DATE=1`, with the macro `\measure` offered:

- **bm**: `FLASHTEX_BOXMEMO=off|on|verify`, `FLASHTEX_BOXMEMO_NAMES=measure`,
  `FLASHTEX_BOXMEMO_VERIFY_FAIL=1`, D9 off;
- **d9**: `FLASHTEX_INTRINSICS=off|on|verify`,
  `FLASHTEX_INTRINSIC_NAMES=measure`, BOX-MEMO off.

A finding is any of: the log or the DVI/PDF differs between off and on
(`on-log`, `on-out`) or between off and verify (`verify-log`,
`verify-out`); the verify run exits 3 (`verify-exit3`); the verify
statistics count a difference (`verify-diff`, from `verify_differences` or
`full_differences`); a crash or a timeout. The signature is the system, the
kinds, and the first state field the verifier names (`param_ptr`, `first`,
`hash[]`, ...), so one hole found by many seeds is reported once.

## The grammar (gen.py)

A document sets the catcodes, defines `\measure` (0 to 2 arguments,
sometimes delimited) and calls it 6 to 13 times, with assignments between
calls. Two styles: **box** calls typeset into box registers and measure
them (BOX-MEMO's kind); **pure** calls take no arguments and only assign and
expand (D9's kind, which also uses `\countdef`'d registers, since D9 reads
`assign_int` quantities and not `\count`). Both styles run under both
systems.

Body items cover `\setbox` of `\hbox`/`\vbox`/`\vtop` with paragraphs,
`\halign`, `\vsplit`, over- and underfull boxes, `\count`/`\dimen`/`\toks`
assignments, `\csname`, every common conditional, `\edef`/`\xdef`,
`\the`, `\afterassignment`, `\aftergroup`, `\futurelet`, `\lowercase`,
`\input u`, `\font`, `#`-arguments, and an optional last item that reads
on past the end of the call (`\global\count1=5`, `\global\let\z=`).
Between calls the document changes what the calls read: registers,
`\inner` and `\x` (`\cr`, `\span`, a macro), `\everyeof`, `\everycr`,
`\everypar`, `\catcode`/`\lccode`, `\fontdimen`, `\parshape`,
`\widowpenalties`, about fifty integer, dimension and glue parameters,
`\globaldefs`, tracing. Calls are made directly, inside `\hbox`, groups and
alignment cells, through `\caller{\measure}` and through a caller's
`#1`, followed by tokens the body may read (`7\relax`, ` by 2`, `\x`).

Every line is one balanced item ending in `%`, so `minimize.py`'s line pass
never splits a group. The same seed gives the same case for as long as
gen.py does not change; a grammar change renumbers the cases.

## Minimising

`minimize.py` computes the finding's signature, then runs ddmin over the
lines of `t.tex` and `u.tex` and over the tokens of each line (skipping
candidates whose braces do not balance), until nothing changes. A
candidate counts only if it keeps exactly the same signature. A finding
takes a few hundred checks, under a minute.

## Tests

```sh
python3 -m unittest discover -s tools/fuzz-replay -p 'test_*.py'
```

The pure-logic tests (determinism, ddmin, statistics parsing) always run.
With `FUZZ_REPLAY_ENGINE` naming a built `flashtex-initex`, two more run:
the fixed seed set (`FIXED_SEEDS` in the test, under 30 s) must be clean,
and with D9's fault injection on (`FLASHTEX_INTRINSICS_FAULT=drop-last`) the
same seeds must give a finding, which shows that the harness can see a
wrong replay.

## Findings so far

Issues filed from this fuzzer are listed in `known.json` (signature, issue,
minimal repro), so a rerun can tell a new hole from a known one.
