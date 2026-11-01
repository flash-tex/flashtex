# Runtime phase baseline

Pinned compiler source 4425d9d77b747db69c1f04c493b363bd3a467f4e, executable SHA256 554c9e054f9159c01f8ec184d85dec86b83bcd5da7d69ea99a7bdb6416108322. Compiler built in combined checkout; source identity is compiler-only.

Three files totaling 5/50/500 KB, twenty alternating beginning/end edits per size; all 60 persistent replies exactly match clean builds. Experimental 16 MiB frame limit; production default remains 8 MiB. Shared Linux host, not native paint timing or established-engine pixel parity.

500 KB median total 357.649 ms; p95 405.108 ms. Median dispatch-to-first-byte 250.876 ms includes compiler/input/scheduling, not pure compiler CPU. Frame read 4.645 ms; reader delivery wait 0.062 ms; JSON parse 85.843 ms; validation 23.374 ms. These measurements motivate compiler-side and parser experiments; they do not prove sub-200 ms responsiveness.

Replay samples include request clone, encoding, frame receipt, parse and validation. Twenty runtime tests including real compiler and strict Clippy passed. Generator: crates/preview-controller/examples/scaling_replay.py. Use release runtime replay, the pinned compiler, and --max-frame-bytes 16777216.
