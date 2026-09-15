# Proposal: `capture_insert` — the companion approves the proposal it was shown

Status: **PROPOSAL** (owner report after testing on a real iPad: "You should add
an auto-insert button (no need to go through a prompt)", issue #2 feedback item
3, lane `ipad-unblock`). Additive and optional. It changes no frozen shape:
`capture_submit`, `capture_received`, `capture_status`, `capture_status_ack` and
`destination` are byte-for-byte as they are, and a peer that does not implement
this message answers `unknown_type`, which the companion reports rather than
retrying.

Producer: `apps/ios` (the Captures screen's Insert button) and
`apps/mac/tools/nearby-client`. Consumer: `apps/mac`
(`ShellModel+Nearby.swift`), which routes it into the insertion path the
Captures inspector's own Insert button already uses. `crates/bridge` is
unchanged: the Mac still speaks transfer-v1 `capture_prepare_insert` /
`capture_applied` to it exactly as before.

## 1. The problem

A capture goes iPad → Mac. The Mac converts it, and the proposed LaTeX appears
in the Captures inspector and the review sheet. To get it into the document the
person has to be **at the Mac** and click Insert there. On an iPad in one hand
and a Mac across the desk, that is the whole cost of the feature: you draw a
formula, and then you get up.

The iPad is not missing information. Since the additive `capture_status`
message it already polls the Mac and already receives, and already displays,
`capture_status_ack.latex` — the exact proposal. What it cannot do is say yes.

## 2. What is *not* proposed

**Insertion on arrival.** A flag on `capture_submit` that means "apply this
when you have converted it" is a pre-authorisation: at the moment it is sent
the proposal does not exist, so nobody has read the LaTeX that would enter the
document. transfer-v1 requires the Mac to "require explicit review approval for
the **currently displayed** proposal and target", and
`FlashTeXProtocol/Capture.swift` states the invariant flatly: proposed LaTeX is
"Never inserted automatically." A blind pre-authorisation breaks both, and no
amount of UI wording around it would make the approval informed. It is not in
this proposal and should not be added later.

What the feedback asks to remove is the **second** confirmation — walking to
the Mac to approve something already read on the iPad — not the first. This
proposal moves the review surface; it does not delete the review.

## 3. Shape

Request `capture_insert`:

```json
{"capture_id":"cap-…","approved_latex_sha256":"<64 lowercase hex>"}
```

Reply `capture_insert_ack`:

```json
{"capture_id":"cap-…","state":"inserted","new_revision":42,"note":"…"}
```

- `approved_latex_sha256` — SHA-256 of the UTF-8 bytes of the proposal the
  companion **displayed**, lowercase hex. Not a transport checksum; the
  proposal text is not sent at all. It is the token that makes this an approval
  of something specific.
- `state` — a `capture_status_ack` state string: `inserted` on success,
  otherwise what the capture actually is now (`rejected`, `failed`), so the
  companion's row converges without a second round trip.
- `new_revision` — the document revision after the edit, when there was one.
- `note` — the Mac's plain-text detail, shown verbatim.

Errors (an `error` envelope, never a partial insertion): `unknown_capture` (not
a capture this pairing submitted), `not_ready` (its receipt has not been sent
yet), `no_proposal` (nothing converted to approve), `proposal_changed`,
`insert_refused` (the Mac's own refusal, message included), `unavailable`,
`bad_request`.

## 4. Why the digest is the whole design

`proposal_changed` is the point of the message. transfer-v1 already has paths
that legitimately replace a proposal between one read and the next — a
dependency-fingerprint mismatch returns `proposal_context_stale` and the UI
"must discard approval and explicitly offer conversion again, then display the
refreshed proposal for new review". If the companion could say "insert capture
X" without naming *which* text it approved, that re-conversion would be
inserted unread, and the feature really would be an automatic insertion.

By hashing the displayed text:

- approving text the person read inserts exactly that proposal;
- approving text that has since been replaced is **refused**, and the companion
  is told to read the new proposal before approving it;
- the hash is computed identically on both ends (`NearbyV1.proposalDigest` /
  `NearbyWire.proposalDigest`), pinned against each other by a test.

The Mac never inserts the bytes the companion sent — it inserts *its own*
journaled proposal, and only when that proposal is the one the digest names.
A hostile or buggy companion therefore cannot author document content; the most
it can do is approve, or fail to approve, a proposal the Mac already holds.

## 5. Ownership and authority

The session applies the same rule as `capture_status`: `capture_insert` is
honoured only for a `capture_id` **this pairing** submitted and whose
`capture_received` has been sent. One pairing cannot approve another pairing's
captures, cannot approve a capture the Mac has not acknowledged, and learns
nothing about ids it does not own. The per-companion `capturesPermitted`
pairing switch continues to govern whether captures are accepted at all.

## 6. What does not change

Everything past the approval is the existing path, reached through the existing
`insertCaptureFromInbox` → `approveBridgeProposal` / `approveProposal`:

- the bridge still prepares the edit and checks revision, source SHA-256,
  scalar boundaries and removed text;
- the Mac still applies exactly **one undoable edit** and records it in the
  edit ledger; an already-applied capture id never produces a second edit, so a
  retried tap after a dropped reply is idempotent;
- `capture_applied` is still sent after the Mac's transaction is durable;
- restart reconciliation is unchanged.

Wrapping is likewise not this message's business: a formula approved for a
caret in running text arrives wrapped by the destination's `caret_context`
(`transfer-v1-caret-context.md`), enforced at insertion. That is what makes a
one-tap insert safe to use rather than a way to paste `$x^2$` inside `$…$`.

## 7. Compatibility

- A Mac without this message answers `unknown_type`; the companion shows
  "this Mac's FlashTeX predates capture_insert — approve it on the Mac" and
  keeps the Mac-side path working.
- A companion without it never sends it; the Mac's behaviour is unchanged.
- The Captures inspector's Insert button, the review sheet and Reject are all
  unchanged and remain available.
