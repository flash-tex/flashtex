# Review of the Commander's `display-list-v2` consolidation draft (main ad922ea1) against the current implementations

Reviewer: mac-display-delta-proposal lane (producer side), 2026-09-12, for the
Commander (issue #2 comment 5646477457) and the consumer lane (mac-helper-display).
Draft reviewed: `docs/contracts/runtime-v1-display-list-v2.md` at `origin/main`
ad922ea1 ("Enforce current preview generation and reconcile full snapshot
contract draft"), line numbers below are of that file. Implementations compared:

- producer `crates/render-pipeline` @ 9aaec57a (`agent/mac-render-pipeline/unified`):
  `src/protocol.rs`, `src/v1.rs`, `src/display.rs`, `src/fonts.rs`, `src/typeset.rs`,
  `src/bin/flashtex-render.rs`, `docs/oracle-evidence.md`;
- Mac consumer at `origin/agent/mac-helper-display/route` 3db719cf (contains the GH31
  raw-byte fix a9b55af7 "mac: V2FontStore authenticates the bytes it loads" and the
  helper route 74b8825c): `apps/mac/Sources/FlashTeXProtocol/RenderingV2.swift`,
  `apps/mac/Sources/FlashTeXMac/PreviewV2View.swift`, `GlyphRunRenderer.swift`,
  `ShellModel+DisplayCandidates.swift`, `WorkerClient.swift`, `PreviewControllerClient.swift`,
  `LineProcessClient.swift`, `FlashTeXProtocol/JSONLines.swift`, `TransferV1.swift`;
- runtime `crates/document-runtime` on main only where the draft points at it
  (`src/display_candidate.rs`, `docs/producer-size-contract.md`).

Every row cites a file:line. "Side to change" is a recommendation for the owners,
not a ruling. Severity: **high** = a consumer could accept something the draft
forbids or refuse something it requires; **medium** = a documented rule is not
enforced or the draft describes behaviour the code does not have; **low** =
wording/comment drift.

## Discrepancies

| # | Draft says (line) | Implementation does (file:line) | Side to change | Severity |
|---|---|---|---|---|
| D1 | L28–30, L62: sibling document paths, byte lengths and raw SHA-256 "must match their admitted source snapshot"; "Every declared source is checked against supplied bytes." | Direct worker route: `PreviewV2View.swift:215-240` (`receiveDisplayListV2`) admits a sibling on `id == resultID`, accepted capability, and `project_id`/`revision` (`correlation_mismatch`) only; the document sha256 is compared with the buffer only at navigation time (`PreviewV2View.swift:337-338`). The helper route does check byte_length + sha256 against durable text before paint (`ShellModel+DisplayCandidates.swift:328-336`). | consumer (direct route: bind `documents[]` to the text the applied `compile_result` was requested with, before paint) — or the draft states that the direct route binds at navigation, not paint | medium |
| D2 | L91: helper candidate's "session/project/request/compile identity, membership generation and complete editor-version map must match fresh caller-owned state." | `DisplayCandidateFrame` decodes `membership_generation` (`ShellModel+DisplayCandidates.swift:68,107-109`) but `DisplayCandidateGate.rejection` (`:137-154`) compares session, project, request id, compile revision and `sourceVersions` only; `membershipGeneration` is never compared with `ProjectDocuments.membershipGeneration` (`ProjectDocuments.swift:362,1130,1147`). | consumer (add the generation check to the gate and to the pre-paint recheck) | medium |
| D3 | L55–57: a font resource SHA-256 is the raw bytes; "The historical SHA-256(bytes ‖ face-index) engine identifier is not the raw resource digest." | Producer: raw bytes for OTF faces (`fonts.rs:266-275`, `:751-766`) — matches. Consumer still indexes every bundled file under BOTH spellings and accepts a `bytes+face0` match (`GlyphRunRenderer.swift:47-57`, `:129`), and its model comment still claims the pipeline emits `bytes ‖ face_index` and that "consumers must try both conventions" (`RenderingV2.swift:22-26`). Loaded bytes are authenticated against the raw digest regardless (GH31, `GlyphRunRenderer.swift:100-113`). | consumer (drop the face0 alias and fix the comment) or draft (state that a consumer may tolerate the obsolete spelling only with raw-byte authentication) | low |
| D4 | L55: SHA-256 "identifies the complete original font bytes". | `core14-afm` resources (Times, metrics only) carry `byte_length: 0` and a `font_id`/`sha256` that is font-engine's `content_sha256` of the AFM identity, not a file digest (`fonts.rs:685-690`); the consumer treats the format as metrics-only and unpaintable (`RenderingV2.swift:metricsOnlyFontFormats`, `:403-408`; `GlyphRunRenderer.swift:75-79`). | draft (name the metrics-only exception: no program bytes, digest is not a resource digest, never paintable) | low |
| D5 | L68–71: recovery policy is API-specific; names the strict paired helper consumer (refuses `tfm_missing`, `required_metrics_unavailable`, errors) and standalone CFF export; says nothing about the Mac consumer. | The Mac consumer paints any frame that passes `RenderingV2.validate` and font resolution, whatever its diagnostics (no severity gate in `PreviewV2View.swift` / `GlyphRunRenderer.swift` / `ShellModel+DisplayCandidates.swift`; diagnostics are listed in the pane, `PreviewV2View.swift:592-593`). | draft (record the Mac policy: paint + show diagnostics; parity claims already exclude diagnosed fixtures) or consumer (adopt the strict policy for the helper route) — a decision, not a bug | low |
| D6 | L110–113: "Current helper stdin is bounded at 1 MiB". | The Mac client caps its own outgoing helper/transfer lines at `TransferV1.maxLineBytes` = 12 MiB (`LineProcessClient.swift:136`, `TransferV1.swift:10`) and helper frames it reads at 16 MiB (`PreviewControllerClient.swift:77,206-211`); no 1 MiB outgoing bound exists on the consumer, so an edit/open payload between 1 MiB and 12 MiB would be refused by the helper, not by the client. Not verified on the helper side in this review. | consumer (cap or split outgoing helper lines at the helper's bound) or draft (state the client-visible failure) | medium |
| D7 | L40–43: `--v2` file output is a separate interface; a file from an unnegotiated request is not evidence of a promised sibling; stream/file equivalence needs its own gate. | `flashtex-render.rs:111-116` writes the file for EVERY rendered request (negotiated or not, and also when the stream sibling was declined for size), using the same expression as the stream sibling (`json::write(&r.v2.to_json(&reply.id))`, cf. `protocol.rs:237`). Same expression is not a pinned gate. | producer (add the pinned file/stream byte-equality gate for accepted requests; document that a declined request still writes the file) | low |
| D8 | L25–26: "The current producer's size decline uses `display_list_declined` and a recovered result." | Matches `protocol.rs:241-252`, but the decline is estimate-first: `estimated_json_bytes()` over the limit skips serialisation (`protocol.rs:236-238`, `display.rs:296-311`), so a conservative estimate can decline a line that would fit — already recorded in `producer-size-contract.md`; the draft does not carry that caveat. | draft (one sentence: decline may be by estimate) | low |
| D9 | L16: "`display-list-v2` is requested only by a consumer prepared to validate it." | Consumer requests it whenever the v2 pane is visible (`PreviewV2View.swift:200-205` `setLiveV2`); on the helper route the helper enrolls it while candidates are enabled (`ShellModel+DisplayCandidates.swift:41-44`). Both prepare to validate; but the direct route also keeps requesting it while `previewSource == .fixture`, then drops the sibling as stale (`PreviewV2View.swift:216-221`). | consumer (stop requesting while a fixture is shown) | low |

## Confirmed matches (with citations)

| Draft (line) | Producer @ 9aaec57a | Consumer @ 3db719cf |
|---|---|---|
| L15–17 duplicate-free `layout_capabilities`, accepted = requested subset, unknown names never accepted | `protocol.rs:124-160` (array of unique nonempty ≤ 64-byte strings, ≤ 16 entries); `v1.rs:38-55` (`_ => {}` for unknown names) | `LayoutNegotiation` request/accept binding (referenced by `PreviewV2View.swift:liveV2Accepted`, `:210`) |
| L18 base request and `compile_result` unchanged | `protocol.rs:229-231` comment + `golden_v1.rs` fixtures | `WorkerClient.swift:103-104` (sibling not in the v1 transcript) |
| L20–24 one sibling for accepted + non-failed (`ok`/`recovered`); none for `failed`, declined, unrequested | `protocol.rs:233` (`caps.display_list && v1.status != "failed"`), `:241-243` (decline removes the capability) | `PreviewV2View.swift:222-229` (unsolicited sibling = protocol violation) |
| L28–29 sibling `id`/`project_id`/`revision` equal the result | `display.rs:335-336`, `:392-395` (`project_id`, `revision`, `id`); `protocol.rs:222` | `PreviewV2View.swift:233-238` (`correlation_mismatch`); helper gate `ShellModel+DisplayCandidates.swift:137-148` |
| L30–31 `documents[].revision` is the compile revision, not an editor revision | `typeset.rs:1464-1469` (`revision` = request revision; sha256/byte_length of the request text) | `ShellModel+DisplayCandidates.swift:115-120` and the file header comment (editor versions carried separately in `source_versions`) |
| L33–35 duplicate/interleaved/unsolicited/malformed siblings never become candidates; stale work never gains currentness | (producer emits exactly one, `protocol.rs:240`) | `PreviewV2View.swift:216-229`, ticket gate `:287-292`; helper gate `:137-154` and pre-paint recheck (file header comment items 2–4) |
| L37–38 complete snapshot: ordered pages 1..N, documents, fonts, features, diagnostics, source/hit/caret semantics | `display.rs:283-292` (fields), `typeset.rs:1255` (page numbers from 1), `display.rs:443-520` (`page_json`: glyphs, clusters, hit_rects, carets, sources) | `RenderingV2.swift:validate` (contiguous pages from 1 `:418`, cluster partition, provenance vs declared documents) |
| L39 visibility filtering / unchanged-page omission not defined by this capability | no such code in `protocol.rs`/`display.rs` | none; the delta proposal is separate and unactivated |
| L47–49 consumer validates the complete envelope, features, coordinates/counts, resources, UTF-8 source ranges; unknown profiles refused | — | `RenderingV2.swift:knownFeatures` (`:66`), `validate` (unknown feature refused; `:403-405` unknown font format refused; `:406` face_index ≠ 0 refused), `Bounds`, `isTick` sums, `validateProvenance`/`validateSource` |
| L50–53 generic rendering-core default profile is static TrueType; `opentype-cff` is not accepted by the generic schema unchanged | `docs/oracle-evidence.md` "rendering-v2 validation" section: all 18 fixtures fail generic validation as emitted (`unsupported font profile`) and pass only with the format token rewritten | `RenderingV2.swift:12-16` documents the deviation explicitly |
| L58–60 original GIDs/positions/spans retained; verification binds the bytes actually used | `display.rs:141-150` (`gid` never 0, original ids) | `GlyphRunRenderer.swift:100-113` (GH31: bytes handed to CoreGraphics re-hashed and length-checked against the discovery record) |
| L75–83 helper route opt-in, default OFF, `configure_display_candidates` with capability + `renderer_support_confirmed`; enabling enrolls the layout capability; `enabled:true` may carry `preview_error` | — | `PreviewControllerClient.swift:177-184`; `ShellModel+DisplayCandidates.swift:403-412` (`preview_error` logged, status "enabled; preview error: …"); restart → fresh opt-in `:420-424`; env opt-in `:47-50` |
| L90 candidate explicitly untrusted, source actions disabled | — | `ShellModel+DisplayCandidates.swift:80-85` (frames without `untrusted:true` / `source_actions_enabled:false` are protocol violations) |
| L92–94 source hashes/lengths checked against actual current bytes; editor revisions ≠ compile revision | — | `ShellModel+DisplayCandidates.swift:328-336` (durable text of each named source version) |
| L96, L100–101 receipt is not installation; admission/write does not prove installation | — | `ShellModel+DisplayCandidates.swift` counters `received/refused/dropped/invalid/published` (`:203-207`) keep the distinction |
| L108–110 producer per-line limit ≠ runtime limits; producer limit 16 MiB (`FLASHTEX_MAX_REPLY_BYTES` clamp), input line 8 MiB | `protocol.rs:16` (input 8 MiB), `:17-28` (reply 16 MiB, env lowers only) | direct route reader 16 MiB (`JSONLines.swift:19`, `WorkerClient.swift:90-91`), helper frame reader 16 MiB (`PreviewControllerClient.swift:77`) |
| L117–118 no negotiated common byte budget; oversize is an explicit refusal, never truncation | `protocol.rs:241-252` (decline), `:254-270` (v1 over limit → explicit `failed`) | `WorkerClient.swift:90-91`, `PreviewControllerClient.swift:206-211` (oversize line = protocol violation, nothing partial) |
| L124–125, L127–132 no page/delta code in either baseline; extension needs its own contract | no delta code in `crates/render-pipeline` at 9aaec57a | no delta code in the consumer; the proposal `display-list-v2-delta.md` (r2) addresses the six listed requirements |

## Not verified in this review (stated, not claimed)

- Helper-side stdin/JSONL/producer-frame ceilings (L110–113) were not re-read
  in `crates/preview-controller` sources; only the Mac client's constants are cited.
- Runtime input limits and `PipelineCff` adapter behaviour (L50–53, L62–64) are
  cited from the draft and `producer-size-contract.md`, not re-verified in
  `crates/rendering-core` / `crates/document-runtime` sources beyond
  `display_candidate.rs:48` (sibling `type` must be `display_list`).
- No build or test was run for this review; all statements are from reading the
  named revisions.
