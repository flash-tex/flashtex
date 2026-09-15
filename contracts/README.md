# FlashTeX interface contracts registry

`contracts/registry.json` is the **source of truth for which interface contracts exist, where each canonical file lives, and each contract's status**.

Contract text is not moved here. Each entry points at the existing file:
- `docs/contracts/`, `protocol/`, `docs/proposals/`
- crate-local docs and fixtures

Code loads some of these by relative path, for example `include_str!("../../../protocol/fixtures/compile-result.json")` and `tests/visual-corpus/manifest.schema.json`. Moving a file therefore requires updating every consumer in the same PR.

**Rules:**
- **One row per contract:** `id`, `status` (`accepted` | `proposal` | `review` | `superseded`), `canonical` path, optional `schemas`, `fixtures` and `consumers`, `owner` (`null` when not recorded on main) and `negotiation` (a Beads decision bead ID once one exists).
- **New contracts** start as files under `docs/contracts/<topic>.md` (ORCHESTRATION.md rule), plus a registry row.
- **Negotiation happens in a Beads `decision` bead** with `--spec-id contracts/registry.json#<id>`, not in issue comments. The ruling is committed into the canonical file, and the row's `status` is updated in the same PR.
- **Additive changes** keep the version; **breaking changes** get a new id (e.g. `runtime-v2`).
- `scripts`/CI may validate that every `canonical`, `schemas` and `fixtures` path exists.

**Known gaps (from the 2026-09-13 audit):**
- Some agreements exist only in issue #2 comments or unmerged branches: FT-066 `provider_evidence`, the FT-063 display-list-v2 image item, and delta rulings r3–r5. At cutover each needs a decision bead and must be written into its canonical file.
