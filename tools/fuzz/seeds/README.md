Seed inputs for the fuzzer in tools/fuzz: same file format as the lockstep cases, rich in mutable structure.
Each seed was accepted by pdfTeX 1.40.29 and reports equal in the lockstep self-test (`run.py --self-test --cases '14*'`).
Seeds never write expected output; pdfTeX itself is the oracle for every differential comparison.
