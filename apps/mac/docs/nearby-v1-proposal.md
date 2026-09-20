# Nearby companion transport v1 — proposal

Status: **proposal** from FT-003 (Mac shell, mac-claude-a) for the Commander to
adopt as `docs/contracts/nearby-v1.md`. Until then nothing here is a published
contract; the Mac implementation in `apps/mac/Sources/FlashTeXMac/Nearby*.swift`
and `Pairing.swift` is the reference for the Mac side only. The companion side
belongs to FT-004. Updated September 12, 2026 (receive caps, image validation
and duplicate handling added by mac-nearby-transport; wire version unchanged).

Scope: how an iPad/iPhone companion on the same network finds a Mac, pairs with
it once, and then delivers runtime-v1 `capture_submit` messages to it with
authenticated encryption, as required by transfer-v1's "Nearby transport
boundary" (Bonjour is discovery only; no plaintext process protocol on a LAN
port; the adapter forwards captures to the local bridge and returns its
acknowledgement).

## 1. Discovery (Bonjour)

The Mac advertises one service while "Advertise" is on:

| Field | Value |
|---|---|
| Service type | `_flashtex._tcp` (local domain) |
| Instance name | the Mac's user-visible name (`Host.current().localizedName`) |
| Port | ephemeral, from the SRV record; changes only when the listener restarts |
| TXT `v` | `1` — nearby protocol version |
| TXT `name` | same as the instance name |
| TXT `fp` | 16 hex chars: first 8 bytes of SHA-256(`"flashtex-nearby-v1 mac-id"` ‖ salt). Stable per Mac; lets a companion match a service to a stored pairing without connecting |
| TXT `salt` | 32 hex chars: the Mac's 16-byte pairing salt (public) |

Nothing in the TXT record is secret and nothing in it is trusted: a companion
must not connect to a service merely because it advertises a known `fp`; the TLS
handshake below is the only authentication.

## 2. Pairing

Pairing is explicit and user-driven. The Mac (`Edit > Nearby Companion…`,
⌘⇧N, "Show Pairing Code") displays a 6-digit decimal code from the system CSPRNG
for 120 s. The user types it on the companion while the Mac's service is
selected. Both sides derive:

```
ikm      = UTF-8 bytes of the code, e.g. "123456"
salt     = TXT salt (16 bytes)
psk_boot = HKDF-SHA256(ikm, salt, info = "flashtex-nearby-v1 psk",     L = 32)
pair_id  = hex(HKDF-SHA256(ikm, salt, info = "flashtex-nearby-v1 pair-id", L = 8))  — 16 hex chars
```

Test vector (pinned in `PairingTests`): code `123456`, salt
`000102030405060708090a0b0c0d0e0f` →
`pair_id = 3917d7c3e5eef7ce`,
`psk_boot = b127a48a782dbece3edbed18d027d658890624ecf21da77d19f47d5e604d1221`,
`fp = 0e712816d64b7c47`, and `hello.proof` for nonce `n-1` =
`rIBrMvRrNjs2eQueTGVsBNF6K130BW6fqV93Rw//WgM=`.

`psk_boot` is a **bootstrap** secret only. It authenticates exactly one TLS
connection, whose `hello_ack` carries a fresh random 32-byte long-term PSK
(`pair_psk`, base64). Both sides persist `{pair_id, pair_psk}`; the Mac also
keeps the companion's name and timestamps. The bootstrap key is discarded when
the code expires or the pairing is confirmed, whichever comes first, and one code
confirms at most one pairing. A second device needs a new code.

Mac persistence: `~/Library/Application Support/FlashTeX/pairs.json`, mode
0600, directory 0700, written atomically — `{"version":1,"salt":hex,
"pairs":[{"pair_id","psk","companion_name","created_at","last_seen_at"}]}`.
Not the Keychain (see §6). "Forget" removes the record, restarts the listener
without that key, and closes that companion's live session.

Schema v2 added the optional per-record `generation`; v3 adds the per-record
`permission` (`"captures"` | `"view_only"`, upgraded from older files as
`"captures"`, the behaviour they had). A `view_only` companion still pairs,
reconnects and reads the destination; every `capture_submit` on that pairing
is refused with `capture_not_permitted` (session stays open, nothing is
remembered for dedup, nothing reaches the inbox) until the Mac's user changes
the pop-up in the Nearby Companion window. The listener reads the store on
each capture, so the change needs no restart. The reference client treats
the code as its own class (`needsPermission`: not `needsRepair`, not a new
capture, not retried; exit 6).

QR bootstrap: beside the six-digit code the window shows a CoreImage
`CIQRCodeGenerator` image (level M) of
`flashtex-nearby://pair?v=1&code=<6 digits>&salt=<32 hex>&fp=<16 hex>&name=<Mac name>`
— exactly the inputs `nearby-client pair` takes (`--qr <decoded text>`
replaces `--code`, `--mac <fp>` and `--salt`; an explicit `--code`/`--mac`
must agree with it, `fp` must match `salt`). The code is also copyable
("Copy code", ⌘C on the focused code) as bare digits; VoiceOver reads it
grouped in pairs ("1 2, 3 4, 5 6"). Nothing in the QR is secret beyond the
code already displayed next to it; `protocol_version` stays 1.

## 3. Transport

TCP + TLS via Network.framework, parameters fixed on both sides:

| Parameter | Value |
|---|---|
| TLS version | 1.2 only (`sec_protocol_options_set_{min,max}_tls_protocol_version(.TLSv12)`) |
| Cipher suite | `TLS_PSK_WITH_AES_128_GCM_SHA256` (0x00A8) only, via `sec_protocol_options_append_tls_ciphersuite(tls_ciphersuite_t(rawValue: 0x00A8))` |
| PSK | `sec_protocol_options_add_pre_shared_key(psk, identity)`; identity = `pair_id` (ASCII); the Mac adds every stored pairing plus the pending bootstrap key |
| Resumption | disabled on both sides (`sec_protocol_options_set_tls_resumption_enabled(false)`, `…_tls_tickets_enabled(false)`) |
| Certificates | none (PSK ciphersuite); no peer trust evaluation |
| TCP | keepalive on, Nagle off, `allowLocalEndpointReuse`, `includePeerToPeer = false` (infrastructure Wi‑Fi/Ethernet only in v1) |

Why TLS 1.2 rather than 1.3: Network.framework's TLS 1.3 PSK support is for
resumption tickets, not externally provisioned PSKs; the 1.2 PSK suite is the
one it negotiates with `add_pre_shared_key`. Measured in `NearbyListenerTests`
(server and client both Network.framework on macOS 26): handshake completes
with the parameters above, and the server verifies
`sec_protocol_metadata_get_negotiated_tls_protocol_version == TLSv12` and the
negotiated suite == 0x00A8 before creating a session; anything else is closed.

Three facts learned while implementing, all mandatory for implementers:

1. **The stack does not report which table PSK a session used.**
   `sec_protocol_metadata_access_pre_shared_keys` returns every configured
   key. Hence `hello.proof` (§4) — the companion proves it holds the key for the
   `pair_id` it claims. Without it a paired device could speak for another
   pairing, or for a pending bootstrap and receive its long-term key.
2. **TLS session resumption bypasses the PSK check.** With resumption on, a
   client that had connected with a since-removed key resumed successfully
   against a listener that no longer held it. Resumption/tickets must be off
   (regression covered by `testBootstrapPairingHandsOverLongTermPSKAndSurvivesRestart`).
3. **A failed handshake still needs `cancel()`.** A server-side
   `NWConnection` that reaches `.failed` (plaintext or wrong-key peer) keeps
   its socket until cancelled; the peer then hangs instead of seeing a close
   (`NearbyPlaintextTests`).

## 4. Framing and messages

After the handshake the stream carries runtime-v1 JSON Lines exactly like the
bridge: one `{protocol_version:1,id,type,payload}` object per line, UTF-8,
`\n`-terminated, **each line including its newline at most 12 MiB** (an
oversized complete or unterminated line gets an `error` with `id: null`, code
`line_too_long`, then the connection closes). Request `id`s are 1–128 bytes;
replies preserve them. Errors carry `{code, message}`.

The first line **must** be `hello`; anything else is answered with
`hello_required` and closed. `hello` is accepted once per connection.

| Request | Payload | Reply |
|---|---|---|
| `hello` | `pair_id`, `companion_name` (≤ 64 chars kept), `protocol_version: 1` (nearby version, not the envelope's), `nonce` (1–128 bytes, fresh per connection), `proof` | `hello_ack` `{mac_name, nonce (echoed), destination, pair_psk?}` |
| `destination_query` | `{}` | `destination` `{destination: … \| null}` |
| `capture_submit` | unchanged runtime-v1 payload | `capture_received` `{capture_id, durable, has_proposal, applied}` or `error` |
| `capture_status` (additive, §6a) | `{capture_id}` | `capture_status_ack` `{capture_id, state, durable, latex?, note?, new_revision?}` or `error` (`unknown_capture`, `bad_request`, `unavailable`) |

`proof` = base64(HMAC-SHA256(key = the PSK used for this connection, data =
`"flashtex-nearby-v1 hello"` ‖ nonce)). The Mac looks the claimed `pair_id` up
in its key table and verifies; a miss or bad MAC → `pair_mismatch`, closed. A
`pair_id:nonce` pair the Mac has seen recently (last 256) → `bad_request`,
closed. `pair_psk` is present only when the connection was authenticated by a
bootstrap key; the companion must store it and use it from the next connection
on (the current connection stays open and usable).

`destination` is `{destination_id, project_id, path, base_revision}` for the
Mac's current insertion destination or an explicit `null`. The companion
copies these into `capture_submit`; the user never types IDs or revisions
(transfer-v1 requirement). Which destination that is (§4a): an explicit pin
(`Edit > Pin Insertion Point`, ids `mac-anchor-N`) while it is valid, else
**the caret**, pinned on the companion's behalf under a stable automatic id
(`mac-caret-N`) at the moment it asks.

### 4a. The automatic destination is the caret at insertion time

Updated September 19, 2026 (lane lane-capture-flow; owner's report). The Mac
pins the caret for a companion in the bridge's caret mode (transfer-v1
additive `destination_pin.mode: "caret"`): the anchor follows the Mac's edits
like a caret and is never invalidated by typing at it, and the same id may be
re-pinned anywhere at any revision. The Mac re-pins it at the caret every time
a companion asks (`hello` / `destination_query`), when a `capture_submit`
naming an automatic id arrives, and when that capture is approved — so a
capture bound to an automatic id inserts **where the caret is when Insert is
clicked**, whatever was typed since, and a companion that still holds an id
from an earlier query or from before a bridge restart is not refused: the id
is its handle for "the caret", not a promise about bytes. `base_revision`
announced for a caret destination is the current document revision and is
informational to the bridge in that mode. Only an explicit pin keeps the
strict rule below (`destination_reselection_required` after an overlapping
edit); once an edit drops it, the Mac announces the caret again instead of
`null`. The Mac never inserts without the reviewer's Insert. Off with
`FLASHTEX_CAPTURE_CARET_DESTINATION=0` (then `null` until a pin, as before).

`capture_submit` validation on the Mac before any sink is called: decodable
envelope, `capture_id`/`destination_id` are 1–128 ASCII `[A-Za-z0-9_-]`,
`mime_type` ∈ {image/png, image/jpeg}, `instructions` ≤ 4096 bytes,
`base_revision` ≥ 0, and the image itself (validated off the listener queue,
before anything reaches the main thread): base64 decodes to 1–8 MiB
(`image_too_large`, same bound as transfer-v1), the bytes are a structurally
complete PNG (signature, IHDR first, chunk CRCs, consecutive IDAT, IEND last,
nothing after it, IDAT inflates to exactly the scanline size with a matching
Adler-32) or JPEG (SOI, one SOF frame header, EOI reached through the scan
data) matching the declared `mime_type`, width and height ≤ 8192 and the
decoded pixel size ≤ 64 MiB (`invalid_image`, the bridge's code for the same
conditions). Rejections are `error` replies that keep the session open. The
bridge still performs its own full decode (transfer-v1); the nearby check is
what lets the Mac refuse a bad image without touching the run loop.

Receive caps (defaults in `NearbyReceiveLimits`; every one is an explicit
error, never a silent drop):

| Cap | Default | Refusal |
|---|---|---|
| frame (line incl. newline) | 12 MiB | `line_too_long`, `id: null`, connection closed |
| per-session frame bytes accepted but not yet acknowledged | 24 MiB | `too_many_in_flight`, session stays open; retry after acks |
| listener-wide accepted-but-unacknowledged bytes ("inbox") | 64 MiB | `inbox_full`, session stays open |
| sessions per `pair_id` | 4 | `too_many_sessions` at `hello`, connection closed |
| accepted TCP connections | 16 | closed before the handshake (no bytes to an unauthenticated peer); Mac logs `too many connections` |
| TLS handshake deadline | 10 s | closed (no bytes to an unauthenticated peer); Mac logs `handshake timed out` |
| `hello` deadline after the handshake | 10 s | `hello_timeout`, `id: null`, connection closed |
| one frame, first byte to newline | 60 s (≥ 200 KiB/s for a full frame) | `frame_timeout`, `id: null`, connection closed |
| TCP keepalive (half-open peer) | idle 15 s, 5 s × 4 probes | connection fails, session slot and in-flight bytes released |

The frame cap is enforced before the bytes are buffered: a chunk that cannot
end its line inside the limit is refused without being appended, so the
receive buffer never holds more than one frame limit. The deadlines are
absolute per stage (they are not extended by trickled bytes), so a
slow-loris peer is bounded the same way as a silent one; a session idle
between complete frames has no deadline.

Duplicate handling, keyed by `(pair_id, capture_id)` with the accepted
`base_revision` and a digest of the rest of the payload (last 256 per
pairing, shared by every session of that pairing on the listener and carried
across a key-table restart): an identical retry is acknowledged again with
the *new* request id and **not re-delivered** (a retry that arrives while the
first delivery is pending — on the same or a later session — waits for that
one answer); the same `capture_id` with another `base_revision` is refused
with `revision_mismatch`; the same id and revision with another payload is
`capture_id_conflict`. A capture the sink refused is forgotten, so a retry is
delivered again. A companion that drops mid-delivery and re-sends on its next
connection therefore never causes a second delivery to the inbox or the
bridge; forgetting the pairing drops its memory. The Mac window surfaces
refusals (`code`, `capture_id`, `pair_id`, message) and the duplicate count
(`NearbyState.lastReceiveError` / `receiveErrors` / `duplicateCaptureCount`).

Error codes used: `bad_request`, `hello_required`, `pair_mismatch`,
`pairing_expired`, `pairing_cancelled` (`id: null`; the Mac withdrew the code
— cancelled, expired, replaced or consumed by another session — while this
bootstrap session had not yet said hello; a close follows), `unsupported_version`, `unsupported_image`,
`image_too_large`, `invalid_image`, `unknown_type`, `line_too_long`,
`frame_timeout`, `hello_timeout`,
`too_many_in_flight`, `inbox_full`, `too_many_sessions`, `revision_mismatch`,
`capture_id_conflict`, `capture_not_permitted` (§2: view-only companion;
session stays open, pairing intact), `unavailable`. Codes are additive to the ones listed
before; the nearby `protocol_version` stays 1 (no existing message changed).
`frame_timeout`/`hello_timeout` are followed by a close; a companion treats
them like any other close (reconnect with backoff, re-send the same capture).

Bridge codes passed through verbatim (with a bridge attached; crates/bridge
`validate`/`capture_anchor`/`receive`), all terminal for the capture as sent:
`destination_reselection_required` (an *explicit* pin was unpinned, an edit
overlapped or sat exactly on it, or a restored pin no longer matches the
capture's durable binding — reselect on the Mac, or use the inspector's
"Insert at caret"; `hello_ack.destination` and `destination` then report the
caret, §4a), `revision_conflict` (`base_revision` is not an explicit pin's
revision), `capture_id_conflict` (bridge journal: same id, different content),
`image_too_large`, `instructions_too_large`, `invalid_image`,
`unsupported_image`, `invalid_id`. The reference client's
`NearbyWire.captureInputErrorCodes` lists these; a client that checks
`destination` before sending (`NearbyReconnector`) sees a re-pinned caret
destination (same id, current revision) first. A capture naming an automatic
id is never refused for staleness: the Mac re-pins that id at the caret before
forwarding it (§4a). An edit that overlaps an explicit pin drops it (the row
shows "(invalid)") and the caret takes over as the announced destination.

Acknowledgement semantics: `durable: true` may only be reported when the local
bridge has journaled the capture (transfer-v1 `capture_received`). With a
bridge attached (`Edit > Attach Capture Bridge`) the Mac forwards the capture
to it and returns the bridge's acknowledgement or error code verbatim; without
one, an in-memory inbox answers `durable: false`. In the inbox, an identical
retry of a known `capture_id` is acknowledged again and a different payload
with a known id is `capture_id_conflict` (the bridge applies its own journal
rules).

## 5. Threat model (honest version)

- **Passive LAN attacker** sees Bonjour, TCP metadata and TLS records; the
  content is AES-128-GCM under a 256-bit PSK once paired. Not protected:
  traffic analysis (a capture is a big write; timing), and the Mac's name/fp.
- **Active LAN attacker / rogue service** cannot complete the handshake without
  a PSK, so no application byte is parsed from an unpaired peer; the listener
  never creates a session before TLS reports `.ready`. A rogue Mac advertising
  the same name gets a failed handshake from a paired companion, not data.
- **The 6-digit code is ~20 bits.** During the ≤120 s window an attacker who
  captures the bootstrap handshake *and* the `hello_ack` can brute-force the
  code offline (PSK-only suites have no forward secrecy) and thereby recover
  `pair_psk`. Mitigations in v1: the window is short, one code confirms one
  pairing, and an online guess costs a full TLS handshake with a fresh code
  needed after expiry. This is the weakest point; the Commander may prefer a
  longer alphanumeric code or a QR code carrying 32 random bytes — the
  derivation and every message are unchanged, only `ikm` grows. `Pairing.derive`
  takes any string.
- **Forward secrecy: none.** Compromise of `pairs.json` (mode 0600, user-only)
  decrypts recorded sessions of that pairing. Rotating to ECDHE-PSK would fix
  this but Network.framework offers no such suite.
- **Replay** of application data across connections is prevented by TLS (fresh
  randoms per handshake, resumption off); the hello nonce additionally binds
  `hello_ack` to its `hello` and refuses a companion re-sending a nonce.
- **Cross-pairing impersonation** by a paired device is prevented by `proof`.
- **Resource exhaustion:** per-line cap 12 MiB, per-read cap 64 KiB, unknown
  types answered without closing; connections, sessions per pairing,
  per-session and listener-wide unacknowledged bytes are capped (§4 table);
  image validation runs on a utility queue so a slow or hostile peer cannot
  stall the listener queue or the main thread. Still unbounded: the in-memory
  inbox keeps its last 50 captures by count, not bytes (up to 50 × 8 MiB).
- **Device loss:** "Forget" on the Mac invalidates the pairing immediately
  (listener restarted without the key, live session closed). There is no remote
  wipe of the companion's copy.

## 6. Not provided in v1

- No certificate PKI, no device identities beyond the PSK, no Keychain storage
  (plain 0600 file; Keychain migration is a follow-up and changes no wire byte).
- No remote or cloud relay; same-LAN only (no AWDL peer-to-peer either).
- No forward secrecy; no TLS 1.3.
- ~~No companion → Mac notification of proposals or insertion results; the
  companion only learns `capture_received`.~~ Closed by §6a (`capture_status`,
  additive, companion-initiated polling). Proposal review and approval still
  stay on the Mac; the companion sees the outcome and the proposal text
  read-only. There is still no Mac → companion push.
- No multi-Mac routing on the companion beyond "pick a service".
- Local Network privacy: a bundled `FlashTeX.app` will trigger macOS's Local
  Network prompt the first time it advertises; the bare SwiftPM executable and
  `swift test` did not prompt on the development Mac (macOS 26). The .app needs
  `NSLocalNetworkUsageDescription` and `NSBonjourServices = [_flashtex._tcp]`
  in its Info.plist (not yet added to `scripts/make-app.sh`).

### 6a. `capture_status` (additive; nearby `protocol_version` stays 1)

A companion that submitted a capture may ask, on the same authenticated
session (or any later session of the same pairing), what became of it:

```
→ {"protocol_version":1,"id":"s1","type":"capture_status","payload":{"capture_id":"cap-…"}}
← {"protocol_version":1,"id":"s1","type":"capture_status_ack","payload":
     {"capture_id":"cap-…","state":"proposal_ready","durable":true,
      "latex":"\\begin{tikzpicture}…","note":"proposal ready (context revision 3)"}}
```

`state` (additive vocabulary — a companion shows an unknown value verbatim):

| `state` | Meaning on the Mac | `latex` |
|---|---|---|
| `received` | in the in-memory inbox, no bridge attached (`durable:false`), or the delivery is still pending in the sink (`note` says so) | — |
| `journaled` | bridge journal has it; `Edit > Convert Capture` not run yet | — |
| `converting` | `capture_convert` in flight | — |
| `proposal_ready` | a proposal exists (queued or prepared for review on the Mac) | proposal text |
| `inserted` | applied / confirmed; `new_revision` set | proposal text |
| `rejected` | `capture_reject` | proposal text |
| `failed` | conversion failed (provider error in `note`) or the pinned destination changed (reselect on the Mac) | maybe |
| `uncertain` | the bridge relaunched with the submit in flight; a resubmission with the same id is safe | — |

Sources on the Mac (`ShellModel+Nearby.swift`): the bridge's `capture_status`
row (transfer-v1: `proposal`, `prepared`, `applied`, `rejected`) is
authoritative for the text, insertion and rejection; the `BridgeSession`
capture state supplies `converting` / `failed` / `uncertain`, which the row
does not carry; without a bridge the inbox answers `received`.

Authorisation: the session answers only for a `capture_id` in the pairing's
acknowledgement memory (`NearbyAckMemory`, the same bounded, `pairs.json`-
persisted memory that answers duplicate deliveries); any other id is
`unknown_capture`, so a companion learns nothing about another pairing's
captures. Because that memory is bounded (256 per pairing) and process-local
until persisted, a capture can be `unknown_capture` while the bridge journal
still holds it: the companion then re-delivers its saved envelope (same
`capture_id`, `destination_id`, `base_revision`, bytes — idempotent on the Mac
and the bridge, never a second insertion) and asks again. A status probe is
never a delivery; a resend is never a status probe.

The reference client (`NearbyConnection.captureStatus`) refuses an ack whose
`capture_id` differs from the request's as `protocolViolation`. A Mac that
predates this section answers `unknown_type`; the companion stops polling and
says the outcome is unavailable. Errors: `bad_request` (id shape),
`unknown_capture`, `unavailable` (no sink / bridge transport failure).

## 7. Companion checklist (FT-004)

1. Browse `_flashtex._tcp`; show `name`, match `fp` against stored pairings.
2. Pairing: read `salt`, take the typed code, derive `pair_id`/`psk_boot`
   (CryptoKit HKDF, vector above), connect with `NearbyListener.clientParameters`
   equivalents (TLS 1.2, suite 0x00A8, `add_pre_shared_key(psk_boot, pair_id)`,
   resumption off), send `hello` with a UUID nonce and `proof`, store
   `hello_ack.pair_psk` under `pair_id` + `fp`.
3. Every later connection: same parameters with `pair_psk`; `hello` first;
   read `destination` from `hello_ack` (or `destination_query` before sending).
4. Send `capture_submit` (one image per capture), wait for `capture_received`,
   keep `durable` visible to the user; retry with the *same* `capture_id` and
   payload after a disconnect.
5. Treat any `error` with a closing code as "re-pair or fix input", never retry
   blindly.

## 8. Delta from the current companion (FT-004, `companion-capture` e7ce5b9)

`apps/companion/FlashTeXCompanion/Services/BonjourTransport.swift` today:
browses `_flashtex._tcp.`, connects to the **first** browse result with plain
`NWParameters.tcp`, then after `.ready` sends one `hello` line and
`capture_submit` lines, `\n`-framed, and parses only `capture_received`
(`payload.capture_id`) from the replies. Its hello, as observed:

```json
{"protocol_version":1,"type":"hello","id":"<uuid>","payload":{"role":"companion"}}
```

Against this listener that connection is **refused at the TLS handshake**:
the Mac logs one `closed unauthenticated: handshake failed: …` line, parses
nothing, and keeps serving paired peers (`NearbyPlaintextTests`). The
companion sees the socket close right after its first write, with no JSON
reply. What stays and what changes:

**Keep exactly as is**
- Service type `_flashtex._tcp` (`NWBrowser(for: .bonjour(type:domain:))`).
- `hello` as the first line, newline framing, one JSON object per line,
  runtime-v1 envelope `{protocol_version:1, id, type, payload}`.
- `capture_submit` payload (unchanged runtime-v1; `CapturePayload.swift`).

**Add to `hello.payload`** (`role` may stay; the Mac ignores unknown keys)
- `pair_id` — 16 hex chars from HKDF (§2), or the stored one after pairing.
- `companion_name` — `UIDevice.current.name` (≤ 64 chars kept).
- `protocol_version: 1` — the nearby version; keep the envelope's `1` too.
- `nonce` — a fresh `UUID().uuidString` per connection.
- `proof` — base64 HMAC-SHA256 over `"flashtex-nearby-v1 hello"` ‖ nonce,
  keyed with the PSK this connection was opened with (§4).

**Read from the TXT record** (`NWBrowser.Result.metadata` → `.bonjour(NWTXTRecord)`)
- `salt` (32 hex) — needed only while pairing, to derive the bootstrap key.
- `fp` (16 hex) — the key under which to store the pairing; use it to pick
  the right Mac instead of "first result" (and to show the Mac's `name`).
- `v` — must be `"1"`; otherwise do not connect.

**Replace `NWParameters.tcp`** with TLS-PSK parameters (copy-pasteable, iOS 14+):

```swift
import CryptoKit
import Network
import Security

enum NearbyCrypto {
    static func derive(code: String, saltHex: String) -> (pairId: String, psk: SymmetricKey) {
        let salt = Data(hex: saltHex)                       // 16 bytes from TXT `salt`
        let ikm = SymmetricKey(data: Data(code.utf8))       // the 6 digits the user typed
        let psk = HKDF<SHA256>.deriveKey(inputKeyMaterial: ikm, salt: salt,
                                         info: Data("flashtex-nearby-v1 psk".utf8), outputByteCount: 32)
        let id = HKDF<SHA256>.deriveKey(inputKeyMaterial: ikm, salt: salt,
                                        info: Data("flashtex-nearby-v1 pair-id".utf8), outputByteCount: 8)
        return (id.withUnsafeBytes { Data($0) }.map { String(format: "%02x", $0) }.joined(), psk)
    }

    static func helloProof(psk: SymmetricKey, nonce: String) -> String {
        Data(HMAC<SHA256>.authenticationCode(for: Data("flashtex-nearby-v1 hello".utf8) + Data(nonce.utf8), using: psk))
            .base64EncodedString()
    }

    /// Same parameters the Mac listener uses (NearbyListener.clientParameters).
    static func parameters(pairId: String, psk: SymmetricKey) -> NWParameters {
        let tls = NWProtocolTLS.Options()
        let sec = tls.securityProtocolOptions
        sec_protocol_options_set_min_tls_protocol_version(sec, .TLSv12)
        sec_protocol_options_set_max_tls_protocol_version(sec, .TLSv12)
        sec_protocol_options_append_tls_ciphersuite(sec, tls_ciphersuite_t(rawValue: 0x00A8)!) // TLS_PSK_WITH_AES_128_GCM_SHA256
        sec_protocol_options_set_tls_resumption_enabled(sec, false)
        sec_protocol_options_set_tls_tickets_enabled(sec, false)
        let key = psk.withUnsafeBytes { DispatchData(bytes: $0) }
        let identity = Data(pairId.utf8).withUnsafeBytes { DispatchData(bytes: $0) }
        sec_protocol_options_add_pre_shared_key(sec, key as __DispatchData, identity as __DispatchData)
        let tcp = NWProtocolTCP.Options()
        tcp.enableKeepalive = true
        tcp.noDelay = true
        let params = NWParameters(tls: tls, tcp: tcp)
        params.allowLocalEndpointReuse = true
        return params
    }
}

// In BonjourTransport.connect(to:), replacing `NWParameters.tcp`:
let (pairId, psk) = stored ?? NearbyCrypto.derive(code: typedCode, saltHex: txt["salt"]!)
let conn = NWConnection(to: endpoint, using: NearbyCrypto.parameters(pairId: pairId, psk: psk))
conn.stateUpdateHandler = { state in
    if case .ready = state {
        let nonce = UUID().uuidString
        let hello: [String: Any] = [
            "protocol_version": 1, "id": UUID().uuidString, "type": "hello",
            "payload": ["role": "companion", "pair_id": pairId, "companion_name": UIDevice.current.name,
                        "protocol_version": 1, "nonce": nonce,
                        "proof": NearbyCrypto.helloProof(psk: psk, nonce: nonce)]]
        // JSONSerialization → append "\n" → conn.send; then start receiving.
    }
}
```

(`Data(hex:)` is a 4-line helper; `stored` is the `{pair_id, pair_psk}` the
app persisted for this Mac's `fp`. Use the Keychain on iOS.)

**Parse these reply lines** (today only `capture_received` is handled):
- `hello_ack` — `payload.mac_name`, `payload.nonce` (must equal the sent
  nonce), `payload.destination` (object or `null`; copy `destination_id`
  and `base_revision` into every `capture_submit`), and, on the pairing
  connection only, `payload.pair_psk` (base64, 32 bytes) — persist it and
  use `SymmetricKey(data:)` of it for every later connection.
- `capture_received` — `payload.capture_id`, `payload.durable`,
  `payload.has_proposal`, `payload.applied` (transfer-v1 shape; `durable`
  is true only when the Mac's bridge journaled it).
- `error` — `id` (may be `null`), `payload.code`, `payload.message`. Codes
  in §4; `pair_mismatch`, `pairing_expired`, `pairing_cancelled`, `hello_required`,
  `unsupported_version` and `line_too_long` are followed by a close and
  mean "re-pair or fix the client", not "retry".

**Behavioural changes**
- Connect only to a result whose `fp` matches a stored pairing (or the one
  the user picked while pairing), never blindly to the first result.
- One pairing code confirms one device; after `hello_ack.pair_psk` the
  code-derived key is gone. Reconnect with `pair_psk`.
- Retry an unacknowledged capture with the same `capture_id` and payload
  after reconnecting; do not mint a new id for the same image.

## 9. Simulator test (companion in the booted iPad simulator against the Mac)

The iOS simulator shares the Mac's network stack, so Bonjour and TCP work
over the Mac's own interfaces (loopback included) with no extra setup.

1. Mac: `cd apps/mac && swift build && FLASHTEX_REPO=$(git rev-parse --show-toplevel) .build/debug/FlashTeXMac`,
   then `Edit > Nearby Companion…` (⌘⇧N) → **Advertise** on → **Show Pairing
   Code**. The window shows the Bonjour name, port, `fp`, and the code with
   its countdown. (Attach the bridge first, ⌘⇧U menu group, if you want
   `durable: true` acknowledgements; otherwise the inbox answers
   `durable: false`.)
2. Verify the advertisement from a terminal:
   `dns-sd -B _flashtex._tcp local.` then
   `dns-sd -L "<Mac name>" _flashtex._tcp local.` — the TXT line must show
   `v=1 name=… fp=… salt=…`.
3. Companion: `xcrun simctl boot "<iPad>"` (or from Xcode), build and run
   `apps/companion` on it (`apps/companion/build.sh` / the Xcode scheme).
   Bonjour browsing inside the simulator sees the Mac's service; enter the
   6-digit code from step 1 when prompted (after the FT-004 changes in §8).
4. Expected on the Mac window: "paired <device> (<pair_id>)", the device
   listed under *Paired companions* with a green "connected" badge, and the
   countdown gone. Expected on the companion: `hello_ack` with
   `pair_psk` and the current `destination` (pin one first with ⌘⌥P).
5. Send a capture from the companion; the Mac window's *Received captures*
   shows its `capture_id` and, with the bridge attached, the capture appears
   in the bridge list ready for **Convert Capture** (⌘⇧G).
6. Negative check with the *current* companion (before §8 is implemented):
   it connects with plain TCP, the Mac's *Activity* log shows one
   `closed unauthenticated: handshake failed: …` line and the companion's
   connection drops; nothing is acknowledged.

Without the companion, the same path is exercised by
`swift test --filter NearbyStateTests` (a Network.framework client in the
test process pairs, sends the fixture capture, is forgotten, and is refused).

`swift test --filter NearbyTranscriptAcceptanceTests` replays
`apps/mac/Tests/FlashTeXMacTests/Fixtures/nearby-companion-session.jsonl` on
loopback: a recorded-shape companion session (the simulator run's envelope
ordering, `\/` escaping, `capture-<hex>` ids, 400×300 photo and 1408×1510
RGBA pencil PNGs, each capture line emitted twice as that build did) with a
§8 `hello`. Expected: `hello_ack`, four `capture_received` (the two
duplicates acknowledged, not re-delivered), two captures in the inbox; a
verbatim replay on a second connection is refused at `hello` (stale nonce);
a reconnect with a fresh `hello` re-sending the same captures is
acknowledged again without storing them twice. The original simulator
stdout was not committed and the companion still speaks plaintext, so this
fixture is re-synthesized to the recorded shape, not the original bytes.
