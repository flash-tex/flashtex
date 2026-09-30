# Fuzzer seeds

Sixty-four small primitive-level inputs for the mutation fuzzer in `tools/fuzz` (`run.py --seeds tools/fuzz/seeds`): 1400-1439 (wave 1) and 1500-1523 (wave 2, aimed at primitives that at most one lockstep case uses). Same file format as the lockstep cases (`% NNN-slug:` header, `\input prelude`, one shipped box), rich in mutable structure. Each was accepted by pdfTeX 1.40.29. Seeds never carry expected output: pdfTeX is the oracle for every comparison.

## Reproducing "every seed is equal"

The lockstep runner only reads `tools/lockstep/cases`, and the lockstep corpus also uses four-digit numbers (1000-1422 and up), so a glob over the seed numbers can match lockstep cases. Run the seeds in a scratch copy of the harness whose `cases` directory holds only the seeds:

```sh
scratch=$(mktemp -d)
cp -R tools/lockstep/. "$scratch"/
rm -rf "$scratch"/cases && mkdir "$scratch"/cases
cp tools/fuzz/seeds/*.tex "$scratch"/cases/
python3 "$scratch"/run.py --engine /Library/TeX/texbin/pdftex --self-test
# expected: 64 cases, 64 equal, 0 differ
```

Against the candidate, replace `--self-test --engine ...` by `--engine <candidate>` (with `FLASHTEX_POOL` and `FLASHTEX_FORMATS` exported). Seed 1431 prints a `Misplaced \pdfrestore` warning that contains argv[0], so it needs the lockstep normalisation of the per-engine link directory (#1239) to compare equal.
