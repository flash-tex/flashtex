# Actual helper / multi-document producer capture

`benchmarks/display-helper-multidoc` contains original producer input/output JSONL,
original helper output JSONL, observed helper snapshot, reopened ledger documents,
phase diagnostics and binary/asset/artifact hashes. The Linux capture proxy forwards
original bytes; it does not synthesize compiler replies. Its input log proves a
proxy read and output log proves producer emission. Only correlated helper output
establishes delivery. The runtime owner performs independent consumption separately.

Sources start with editor revisions main.tex7, chapter.tex19, refs.bib23. The startup
explicitly declares refs.bib as bibliography; the actual helper snapshot confirms
that kind. Producer requests contain path/text and compile generations1–6; editor
revision identities remain in helper source_versions. Bibliography kind is index
metadata, not an added producer request field.

The capture observes default v1 generation1, explicit raw display generation2,
restart/default v1 generation3, and explicit re-enable/raw generation4. For the
stale case the proxy holds a producer-emitted generation5 frame, then a newer edit
becomes durable. After release, generation6 is current with chapter revision21;
no preview/candidate with chapter revision20 escapes after the newer ACK. This is
supersession acceptance, not a direct user cancel-command or native paint test.

For current captures, v1/v2 semantic output matches original emitted frames; raw
nested display bodies retain the producer's exact bytes. Every document's hash,
UTF8 byte length and compile revision binds correctly, independently of editor
revisions. Killing the helper and reopening each actual ledger preserves exact
final text/hash/revisions7/21/23. Both recorded producer PIDs were absent after
termination. The capture proxy uses Linux parent-death signaling with the expected
proxy PID captured before spawning, avoiding a producer orphan on proxy kill.

All six actual v1 results are recovered with a tfm_missing diagnostic; full details
are retained in provenance and original output. This proves the source/transport
checks, not full math/font/bibliography layout compatibility or pixel parity. No
latency comparison or native activation is inferred. Baseline and first gated
captures preceded the final capture; only the final capture includes every reopen,
raw-byte and post-ACK stale rejection check.

## GH34 isolated ten-point metrics correction

`benchmarks/display-helper-multidoc-10pt` preserves both a new control and a
corrected capture with identical helper/ledger/producer binaries and six bytewise
identical parsed producer requests. The helper source is e09e1f40. Both retain the
same source revisions, declared bibliography, default/restart/reenable sequence,
held superseded generation5, current generation6 and exact ledger reopen checks.

The only added font search resource is ec-lmr10.tfm,12056bytes, SHA256
cd13479f463b9a575d053dd7bf0884daa46bfdeffe4b7f537c193861652ac9e5.
It was verified against member fonts/tfm/public/lm/ec-lmr10.tfm inside the existing
GUST Latin Modern2.004 archive with SHA256
97a725ea012d41367bf44fec1a2f4ccf4fe134c016715522133594e347115a7c.
The archive member matches the staged file byte-for-byte. The GUST font license is
included with its hash. An initial case-sensitive license-member lookup failed;
the correct uppercase .TXT member was verified after capture. No font bytes or
capture output were altered as a result of that bookkeeping correction.

The pinned original asset stage is unchanged. A new directory containing only the
verified10pt TFM was added to the capture's explicit FLASHTEX_TFM_DIRS through its
asset manifest. All six control results remain recovered/tfm_missing; all six
corrected results are ok with empty diagnostics. Current raw bodies still match
original producer bytes and all source/transport/recovery checks pass. Both sets
of producer PIDs were absent after completion.

This resolves the missing asset for this explicitly configured fixture. It does
not prove that native app bundles contain these resources, bibliography layout is
complete, or arbitrary documents achieve pixel parity. No timing comparison is
made; concurrent integration work was allowed during this correctness replay.
