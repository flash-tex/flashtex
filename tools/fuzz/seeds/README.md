# Fuzzer seeds

Forty small primitive-level inputs (1400-1439) for the mutation fuzzer in `tools/fuzz` (`run.py --seeds tools/fuzz/seeds`). Same file format as the lockstep cases (`% NNN-slug:` header, `\input prelude`, one shipped box), rich in mutable structure. Each was accepted by pdfTeX 1.40.29. Seeds never carry expected output: pdfTeX is the oracle for every comparison.

## Reproducing "every seed is equal"

The lockstep runner only reads `tools/lockstep/cases`, so copy the seeds into a scratch copy of it (the glob `'14[0-3][0-9]-*'` has four digits, so it cannot match the three-digit lockstep cases 140-149):

```sh
scratch=$(mktemp -d)
cp -R tools/lockstep/. "$scratch"/
cp tools/fuzz/seeds/14??-*.tex "$scratch"/cases/
python3 "$scratch"/run.py --engine /Library/TeX/texbin/pdftex --self-test --cases '14[0-3][0-9]-*'
# expected: 40 cases, 40 equal, 0 differ
```

Against the candidate, replace `--self-test --engine ...` by `--engine <candidate>` (with `FLASHTEX_POOL` and `FLASHTEX_FORMATS` exported). Seed 1431 prints a `Misplaced \pdfrestore` warning that contains argv[0], so it needs the lockstep normalisation of the per-engine link directory (#1239) to compare equal.
