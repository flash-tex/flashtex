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
failing case and `finding.json`. It exits 1 when there is a finding, and
marks each signature `known #N` (from `known.json`) or `NEW`. A case whose
"off" run times out loops by itself (an error that recurs for ever): it is
counted `invalid` and not compared.
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
`hash[]`, `box_dimension_differs`, `impure_Level` for a call the guard
admitted whose normal path then abandoned its recording, ...; read from the
verifier's stderr and the statistics' `verify_details`), so one hole found
by many seeds is reported once.

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
`\input u`, `\font`, `#`-arguments, `\scantokens` (with `\everyeof`),
`\uppercase`, `\message`, `\immediate\write`, `\write`/`\special`/
`\pdfliteral` whatsits, errors (`\errmessage`, a missing number),
`\showthe`/`\showbox`, `\lastbox`, `\unkern`/`\unpenalty`/`\unskip`,
`\unhbox`, `\mark`, `\insert`, `\prevdepth`, `\spacefactor`, an
interaction mode change, and an optional last item that reads on past the
end of the call (`\global\count1=5`, `\global\let\z=`). Between calls the
document changes what the calls read: registers, `\inner` and `\x` (`\cr`,
`\span`, a macro), `\everyeof`, `\everycr`, `\everypar`,
`\catcode`/`\lccode`, `\fontdimen`, `\parshape`, `\widowpenalties`, about
fifty integer, dimension and glue parameters, `\globaldefs`, tracing, the
interaction mode, `\output`, `\measure` defined again with the same text,
and long token lists (`\toks2` doubled to 40,960, 81,920 or 163,840 tokens,
past BOX-MEMO's 65,536-token key limit, sometimes changed at its end and
copied into `\inner` or an argument). Calls are made directly, inside
`\hbox`, `\vtop`, paragraphs, groups, alignment cells and an output
routine, from `\everypar`, `\aftergroup` and `\afterassignment`, through
`\csname`, `\scantokens`, `\caller{\measure}` and a caller's `#1`,
followed by tokens the body may read (`7\relax`, ` by 2`, `\x`).

Every line is one balanced item ending in `%`, so `minimize.py`'s line pass
never splits a group. The same seed gives the same case for as long as
gen.py does not change; a grammar change renumbers the cases.

## Minimising

`minimize.py` computes the finding's signature, then runs ddmin over the
lines of `t.tex` and `u.tex` and over the tokens of each line (skipping
candidates whose braces do not balance), until nothing changes. A
candidate counts only if it keeps exactly the same signature. Results are
cached by content, and the passes stop at `--max-checks` (2,000) or
`--max-seconds` (600) with the smallest case so far, which is then not
1-minimal (the output says so). A short case takes a few hundred checks;
a long one can take the whole budget, and a few lines by hand are often
quicker from there.

## Tests

```sh
python3 -m unittest discover -s tools/fuzz-replay -p 'test_*.py'
```

The pure-logic tests (determinism, ddmin, statistics parsing) always run.
With `FUZZ_REPLAY_ENGINE` naming a built `flashtex-initex`, two more run:
the fixed seed set (`FIXED_SEEDS` in the test, sized for under 30 s; a
slower run warns but does not fail, since time is not a result) must be clean,
and with D9's fault injection on (`FLASHTEX_INTRINSICS_FAULT=drop-last`) the
same seeds must give a finding, which shows that the harness can see a
wrong replay. CI runs the pure-logic tests with the other tools' self-tests
(ci.yml), and the engine tests as `scripts/engine-parity.sh replay` in the
engine parity jobs' lockstep shard, on the engine those jobs build.

## Findings so far

Issues filed from this fuzzer, or met by it, are listed in `known.json`
(signature, issue, minimal repro), so a rerun can tell a new hole from a
known one. Measured on the Mac on 2026-11-09, on main `ca202f597` with
#2187, #2188, #2195, #2209 and #2217 merged:

- the first grammar (before the second round of items), 2,000 cases, and
  1,000 with `--fonts` and the long token lists: #2264 only (2 seeds each);
- the second grammar, seeds 0-3999: #2264 (5 seeds, two signatures) and
  #2294 (seed 2567);
- the same 4,000 with #2264's fix (#2289): #2294 only;
- the third grammar (the second with D9's items too, `GRAMMAR = 3`,
  today's), seeds 0-3999 with #2264's fix: #2294 only (seed 2567).

On main alone, seeds 0-399 of the third grammar also meet #2198 (seed 5
under both systems, seed 333) and #2199 (seed 209). With the five PRs
merged those cases are
clean, which is why the test's fixed seed set is 6-205: under 30 s, clean on
main and on the merged state (about 12 s on the Mac with both engine tests).
