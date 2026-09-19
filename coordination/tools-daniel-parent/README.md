# daniel-parent's Commander tooling, 2026-09-18

Copied out of a session scratchpad so the next Commander has them. Each was
validated against real cases before use; none is clever, all are conservative.

- **`resolve_inventory.py`** — resolves the one merge-conflict family every
  compiler lane hits (two lanes appending to the same table). Classifies a hunk
  as UNION / TUPLE / BRACE from the structure above and below it, and **refuses**
  anything it cannot name. Score over seven real conflicts: 5 resolved, 2 refused,
  0 wrong. `--dry-run` to inspect without writing.
- **`audit-lane.sh`** — checks a Muse lane's own commits against the owner's
  crate exclusion list. Takes SHAs from the lane's check-in, never from
  `origin/main`: lane clones have **no origin**, so an audit built on it returns
  zero commits and reports *clean having examined nothing*. Has an explicit
  `AUDIT INCONCLUSIVE` branch for that case.
- **`publish-lane2.sh`** — fetch a lane, run the audit as a hard gate *before*
  fetching, merge main, resolve, regenerate derived artefacts, run BOTH test
  profiles on every touched crate, push only if green.

Paths inside the scripts point at a scratchpad dir; fix `S=` before use.
