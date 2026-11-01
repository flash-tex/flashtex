# Opt-in source-bound display candidates

`Session::set_display_candidates_enabled(true)` allows explicit request capability
`display-list-v2`; the default is false. It is mutually exclusive with historical
completed-snapshot retention. Ordinary requests and declined capabilities retain
legacy behavior. A validated nonfailed result accepting the capability emits its
usual v1 Preview first, then holds the active wire request for exactly one contiguous
v2 display_list sibling. Failed results require no sibling. The original dispatch
timeout covers both lines; missing/interleaved/malformed/duplicate/unsolicited
siblings fail the session. No next request dispatch occurs before the sibling.

`take_current_display_candidate()` moves the one retained
`UntrustedDisplayCandidate`. Getters expose original request/project/revision,
source bindings and read-only envelope; `into_envelope()` moves the Value without
cloning it. This is ONLY transport correlation: native/core validation must still
validate complete rendering structure, features, geometry, font assets and raw
font-byte hashes before rendering. Unknown rendering content is intentionally not
interpreted here. There is no render-ready or source-navigation authority.

Each source path occurs exactly once and matches the request's complete document
set, raw UTF8 SHA256, byte length and compile revision. The latter is NOT an
individual editor-document version. Helpers must retain their original controller
source-version snapshot and bind it by path/hash/length; they must also validate
their own session, membership and display generation immediately before delivery.
The Mac contract's historical SHA(bytes plus face) paragraph is obsolete; corrected
producer65dbe7d uses raw font bytes. This runtime does not validate those font hashes.

Submit, close, failure and display-policy changes clear retained candidates.
Policy changes increment an epoch captured at submission, so toggling cannot
resurrect an older result. Superseded/cancelled siblings are correlated and drained
without retention. Currentness is per project; an outer helper must enforce its
selected-project/display generation. Restart requires a new Session. A caller
changing layout or source membership without submitting must invalidate via the
policy setter (including setting the current boolean again).

Memory/lifecycle: existing four-raw-frame queue, one decoding/validation permit
and joined decoder shutdown are unchanged. Each line retains max_frame bounds.
One retained candidate Value can coexist with the decoder's active Value and v1
preview events; this is not a global one-Value or RSS bound. Downstream optional
output admission should be checked before taking the candidate and must preserve
required ACK/current-preview priority. No extra full candidate clone is required.

Tests use synthetic Python subprocesses, including real UTF8 hashes, malformed
and missing siblings, stale edits, close/toggle epochs and sequential dispatch.
They do not establish native acceptance, optical/font correctness, parity or
end-to-end typing latency. Helper contract reviewed at c400d0a. Existing decoder
permit/shutdown tests remain authoritative for its queue invariant.

## Actual producer transport replay

`tools/replay_display_producer.py` verifies every published producer archive file,
rejects extra source inputs, rebuilds offline, and records exact binary/asset and
request/reply hashes. It downloads or installs nothing. Run with explicit existing
producer tree, font directory, rooted TFM directory and a new output directory.
The ignored `real_display_producer` test is its runtime consumer probe.

Evidence in `benchmarks/display-producer-65dbe7d` uses the unchanged355-file
archive and pinned official LM assets. Requested output returns one accepted
source-bound sibling with no diagnostics; legacy output returns only v1; a1500-byte
producer reply budget declines v2 with a recovered warning; a1-byte budget produces
an explicit failed v1 result and no sibling. The tiny budget is a producer failure
trigger, not a claim that its failure reply itself fits one byte. All four direct
producer results equal the runtime's returned JSON values; emitted font raw-byte
hashes occur in the recorded assets. Source text is shared via the committed request
fixture. No metadata correction, font substitution or rendered-output validation
occurs. This is actual transport evidence, not native visual or timing evidence.

The helper may delay taking the runtime slot while required output is pending.
Runtime tests retain it across polls, then move it once, or clear it on close. A
failed mutually exclusive policy enable leaves the prior policy usable. Root's
unchanged dependency uptake854b041 and forwarding contractc400d0a were reviewed;
actual helper/native output admission remains their separate integration gate.

## Delayed consumer and cancellation replay

`real_display_producer` also exercises the unchanged actual producer with an
injected60ms downstream pause (not a measured native validation cost). It cancels
revision1 before polling, retains/moves revision2 exactly once, toggles policy
around revision3 while preserving its v1 fallback, and submits revisions4/5 before
consuming the next candidate. The returned revision5 has its exact edited source
hash. Revision4 may be stale in flight or coalesced before dispatch depending on
whether the previous sibling is still draining; evidence records the actual path,
rather than asserting a scheduler-dependent branch. Both must suppress its preview.

`benchmarks/display-lifecycle-65dbe7d` preserves actual candidates, outcomes and
provenance referencing the matching four-mode replay. No global retained-byte/RSS
or native latency claim follows from one runtime candidate slot. An already moved
revision2 candidate remains immutable and cannot be revoked by the runtime. The
helper/native consumer must reject it against current source/session epochs before
painting; helper8876279 checks the controller snapshot before optional admission,
while native post-validation epoch checking remains a separate requirement.

Integration6c2061c's initial parallel test run failed all seven synthetic cases at
short60–160ms observation windows; the same suite passed serially. Checkpointc47139ff
replaces positive wall-time guesses with bounded observable-outcome waits and uses
explicit /usr/bin/python3 for fake workers. Production timeouts and decoder permits
are unchanged. The real replay similarly waits for previews/candidates rather than
assuming a particular compile duration. Test failures are retained by the runner
before it returns an error. No production boundary defect was established by these
test scheduling failures.

## Incremental producer cache acceptance

Exact published6e696616cca27a6bff19d88f7e0fd51e64458e81 was built unchanged from its
archive. The initial debug build failed with Disk quota exceeded while writing
font-resources and pipeline metadata. Only that task's failed448MB target was
removed. The successful retry disabled debug symbols and incremental build files;
its binary/build environment hashes are recorded. The older65dbe7d binary was
left untouched. This resource constraint is not a producer source failure.

`display-incremental-requests.json` covers six states: initial paragraphs, a UTF8
comment shifting following source byte offsets, an edited first paragraph, unchanged
text at a new revision, explicit style change, and original text restored. The
runtime compares fresh and persistent response Values in both requested and2500-byte
producer budget modes. The harness also independently compares actual direct
fresh/persistent stdout bytes:144806 requested bytes and9867 bounded bytes match
exactly across this sequence. All source hashes/revisions pass runtime binding.

The11pt style case returns a recovered `tfm_missing` warning: ec-lmr10.tfm is
absent from the supplied root and the producer explicitly reports using OpenType
advances. The harness performs no substitution or metadata repair; under2500 bytes it
returns a failed v1 reply. Other bounded cases decline v2 and recover; normal12pt
cases return ok with one sibling. This faithfully records current producer behavior,
not broad font compatibility. The four negotiation and cancellation probes also
pass on this producer. No cache hit count or throughput claim is inferred from
output equality; this is a bounded correctness test under shared machine load.

## Scalar display-stage observability

`last_display_profile()` returns the last current candidate's
`DisplayResponseProfile`: request/project/revision, Session-local display epoch,
framed response byte count, JSON parse, raw decode-queue wait, decoded-frame owner
wait, and source-binding validation milliseconds. Source binding includes raw UTF8
hashing and document correlation; it excludes native/core font/render validation,
helper serialization/admission and paint. No document text is included. Durations
are observations, not calibrated compiler CPU or end-to-end latency measurements.

The profile survives moving its candidate so helpers can observe eventless display
work. Submit, close, policy reset and failure clear it. Stale/cancelled/old-epoch
siblings never update it. Existing v1 `last_profile()` and Event/wire fields remain
unchanged. A new Session begins with no display profile; epoch values are meaningful
only alongside the caller's session identity. Tests cover exact current identity,
scalar finite values, candidate take, supersession, same-mode epoch reset and failure.

`benchmarks/display-profile-6e69661/profile.json` pins one actual observation using
the already verified producer/assets: requested4434 framed bytes reported parsing
0.167284ms and source binding0.021131ms, with separate queue/owner waits. Legacy,
declined and failed cases have no display profile. This tiny single sample just
shows the previously hidden phase is observable; it does not justify optimization
or establish native responsiveness.

## Representative multi-page transport gate

`replay_multipage_producer.py` reuses the verified6e69661 binary/assets without
building or changing caps. Three source states contain196 natural-flow paragraphs:
initial, a UTF8 comment shifting all later byte positions, and a middle-paragraph
edit. All render26 pages. The runtime compares every v1 page/item/diagnostic Value
against a fresh worker, and verifies the final-document sentinel remains present.
The direct producer's full fresh and persistent JSONLines bytes also match exactly
in both default and2000000-byte reply-budget modes. Compressed complete raw replies,
per-page item counts, complete frame sizes and exact hashes are preserved in
`benchmarks/display-multipage-6e69661`.

Default frames contain10979,10979,10980 items and1745747,1745768,1745905 bytes
including newline. The middle edit adds one item without losing later pages.
Each frame preserves96 existing overfull_hbox warnings. This checks complete
transport/source relocation, not line-layout correctness or visual parity.

The optional v2 sibling is explicitly declined for all three states: the initial
estimated27152372-byte display list exceeds the unchanged16777216-byte producer
limit. The bounded2000000-byte case also declines v2 while preserving all26 v1 pages.
Thus this gate proves large v1 delivery and truthful optional decline; it does NOT
prove a large v2 sibling fits or is accepted. No runtime/producer caps were raised,
no oversized line was truncated, and no latency claim was measured.
