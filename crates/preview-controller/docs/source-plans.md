# Native source proposals (FT-048 revision 5, in progress)

The helper exposes `plan_literal_replacement`, `plan_citation_rename`, and
`plan_citation_rename_at`. Each requires the complete `source_versions` map and
`membership_generation` returned by `snapshot`, plus `max_bytes` in 1..8388608.
Responses contain `plan`, the existing versioned project-index export with decimal
string offsets/revisions, `proposal_only` and `requires_user_approval`. These
commands never apply edits. Existing explicit `apply_group` remains separate;
no project-wide atomic application is promised.

Literal requests also require `literal`, `replacement`, `max_matches` (1..1000),
`max_work` (1..1000000), and optional `documents`. Incomplete searches are refused.
Citation requests require `old_name`/`new_name`; the `_at` variant instead takes
`path`, `start_byte`, `end_byte` identifying the exact lexical key span.

Startup in either store or rooted-project mode may supply explicit
`bibliography_paths`; paths identify non-entry sources, with no duplicates.
Rooted mode imports declared files through the existing bounded project capability,
preferring retained durable ledgers over disk. Declarations must be supplied again
on reopen, including dynamically attached bibliography sources. No filename
extension inference. The application owns persistence of its startup configuration. `snapshot` returns
`document_kinds` from the same indexed snapshot as its versions/generation, mapping
every attached path to `latex` or `bibliography`. Persist the bibliography paths
from this map with the project settings after a successful typed open or detach;
do not reconstruct kinds from file extensions.

`open_document` accepts optional `document_kind`: `latex` (default) or
`bibliography`. Source edits, undo/redo, compiler restart, and membership changes
preserve surviving indexed kinds. Reattaching a detached bibliography source must
explicitly select `bibliography`; the generic default remains LaTeX. Membership
generation guards reject proposals predating detach/reattach even when revisions
are unchanged. No live native application adoption has been verified yet.

The proposal JSON limit bounds encoded output, not RSS; parsing the bounded export
into the helper envelope temporarily holds both forms. Complete outer frames also
pass the existing output check. No truncation is used to satisfy a proposal limit.

Checkpoint verification: actual helper process test covers two source documents,
Unicode byte offsets, citation comment/verbatim exclusion, read-only proposals,
small serialization budget refusal, and stale plans after another document changes.

Rooted helper lifecycle coverage includes explicit non-.bib source import, typed
detach/reattach, stale membership refusal, edit/undo/redo, and process reopen from
durable source after the disk file is deleted. Invalid declarations and extension
non-inference are checked independently.

## Applying a reviewed proposal in a native client

1. Retain the complete immutable proposal and its originating helper session.
   Display affected files and exact before/after text for review. Do not turn a
   proposal response into an automatic edit request.
2. Before asking the user to apply, refresh `snapshot` and require the proposal's
   full document revisions and membership generation to match. Regenerate and
   review again if they differ. Compare decimal strings as exact integers, never
   through floating-point conversion. Obtain each affected `document`, requiring
   its path and revision to match the proposal, and retain its source SHA256.
3. After the user chooses Apply, partition edits by document. For each document,
   construct one `apply_group` command with a fresh stable command ID, the reviewed
   revision/hash, a descriptive label, and edits mapping `expected_text` to
   `removed_text`. Parse `start_byte`/`end_byte` exactly; verify each range's source
   bytes against the reviewed text. All ranges refer to the original document;
   do not shift later ranges after replacing earlier ones. The ledger implements
   the actual grouped replacement and rejects overlaps or source mismatches.
4. Retain the exact command payload before sending. An uncertain reply requires
   retrying that same command ID and payload; never mint a new ID to retry. A
   successful reply contains `history.document`, `command_revision` and
   `replayed_command`. Preview failure is separate from durable source success.
5. If a source guard fails, show the conflict and regenerate a proposal. If the
   helper exits, reopen the same durable stores before retry. Undo uses the current
   returned revision/hash and reverses the complete document group.

`apply_group` guards one document. It does not atomically validate or commit a
multi-document plan. A sibling may change after the client snapshot check; a later
file may fail after an earlier file commits. A multi-file UI must display per-file
outcomes and leave unresolved files explicit, without claiming all-or-nothing
rename. Whole-project transactional application needs a separate design. Similarly,
source stored in the ledger is not yet a disk export: rooted clients use the
existing guarded export flow and keep disk conflicts distinct from edit success.

The helper does not authenticate a human click; approval is the native client's
responsibility. A returned proposal alone is never approval. Capture proposals
continue through their separate `apply_reviewed` boundary.

Actual helper regression `generated_literal_plan_applies_as_one_durable_group_and_retries_exactly`
uses exported Unicode replacement ranges, rejects a stale plan after typing,
applies two fresh matches as one undo group, kills/reopens the helper, retries the
identical command without a second revision, rejects a changed payload reusing its
ID, and undoes both replacements together. This is a helper integration test, not
an interactive native UI test or a power-loss simulation.
