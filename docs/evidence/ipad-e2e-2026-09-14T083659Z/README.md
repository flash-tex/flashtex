# iPad companion end-to-end — iOS Simulator + real Mac, no provider

Lane `daniel-parent` (mac-m5pro-dq222). Issue #355 / claim GH-IPAD-E2E.
Branch `agent/daniel-parent/ipad-e2e`. Simulator only; no paid or network
AI/provider call.

## Toolchain

- Xcode 26.6 (17F113)
- iOS 26.5 simulator runtime 23F77 (`com.apple.CoreSimulator.SimRuntime.iOS-26-5`)
- Simulator: **iPad Pro 11-inch (M5)**, udid `947A8CC1-07E2-4AA2-9363-FA389B569E8D` (booted)
- Mac binary: `apps/mac/.build/debug/FlashTeXMac` (`swift build` in `apps/mac`)
- Derived data: `/tmp/ipad-e2e-dd`

The `apps/ios/README.md` destination `iPad Air 11-inch (M3)` / Xcode 26.3 is
not what this machine offers; the M5 11-inch type above was used instead.

## Env (names; non-secret flag values)

Mac process launched with `env -u XAI_API_KEY -u FLASHTEX_AI_API_KEY -u
OPENAI_API_KEY -u ANTHROPIC_API_KEY -u FLASHTEX_NEARBY_SERVE_LAN` plus:

| name | value |
|---|---|
| `FLASHTEX_CONVERSION_PROVIDER` | `none` |
| `FLASHTEX_KEYCHAIN_OFF` | `1` |
| `FLASHTEX_NO_ACTIVATE` | `1` |
| `FLASHTEX_AUTOATTACH` | `0` |
| `FLASHTEX_CAPTURES_AUTO_ATTACH` | `0` |
| `FLASHTEX_CAPTURE_AUTO_CONVERT` | `0` |
| `FLASHTEX_SHOW_CAPTURES` | `1` |
| `FLASHTEX_OPEN_WINDOW` | `nearby` |
| `FLASHTEX_NEARBY_AUTOSTART` | `code` |
| `FLASHTEX_PAIR_STORE` | isolated `/tmp/ipad-e2e-mac/pairs.json` |
| `FLASHTEX_PAIRING_JOURNAL` | isolated `/tmp/ipad-e2e-mac/pairing-session.json` |
| `FLASHTEX_LOG` | `/tmp/ipad-e2e-mac/flashtex.log` (Nearby path wrote nothing) |
| `FLASHTEX_WINDOW_FRAME` | `80,80,1400,900` |
| `FLASHTEX_REPO` | this worktree |

`FLASHTEX_NEARBY_SERVE_LAN` was unset. Provider API keys were unset. Pairing
code / bootstrap URL are not stored here.

UI test injection (runner is in the simulator; host env is invisible unless
prefixed `TEST_RUNNER_`):

- `TEST_RUNNER_FLASHTEX_PAD_E2E_MAC=1`
- `TEST_RUNNER_FLASHTEX_PAD_E2E_HOST=127.0.0.1`
- `TEST_RUNNER_FLASHTEX_PAD_E2E_PORT=<ephemeral>`
- `TEST_RUNNER_FLASHTEX_PAD_E2E_BOOTSTRAP='flashtex-nearby://pair?…'`

## Commands and results

Existing iOS tests on this toolchain (prior checkpoint on this branch,
`75bb0968`, unmodified product plus the race fix):

```
xcodebuild … -scheme FlashTeXPad -destination 'platform=iOS Simulator,id=947A8CC1-07E2-4AA2-9363-FA389B569E8D' test
```

First full run on the unmodified tree: 29 passed, 1 failed
(`FluidCaptureTests.testSendNowDraftsSendsAndRemembersTheInstruction`,
`EXC_BAD_ACCESS` in `PadModel.captures.setter` vs `CaptureStore.save`). Isolated
rerun 3/3 PASS; control unit rerun **Executed 25 tests, with 0 failures**.
After the `CaptureQueue`/`PadModel` race fix: unit **Executed 25 tests, with 0
failures** twice; UI without E2E env **Executed 6 tests, with 1 test skipped
and 0 failures** (`** TEST SUCCEEDED **`). Not treated as FakeMac/toolchain
drift; it was a retain-cycled poller racing `@Published captures`.

Gated real-Mac UI test (this evidence):

```
cd apps/ios
xcodebuild … build-for-testing -derivedDataPath /tmp/ipad-e2e-dd
xcodebuild … test-without-building \
  -only-testing:FlashTeXPadUITests/CaptureFlowRealMacUITests \
  TEST_RUNNER_FLASHTEX_PAD_E2E_* as above
```

| when | result |
|---|---|
| 08:28:14Z (Paste first, port off-screen) | `Executed 1 test, with 1 failure` at `pair.port` (10.962s) |
| 08:32:16Z (Paste + Allow Paste idle) | `Executed 1 test, with 1 failure` waiting for connected (98.210s); Go tapped ~1s after 120s code expiry |
| **08:36:19–08:36:59Z** | **`Executed 1 test, with 0 failures (0 unexpected) in 36.472 seconds` / `** TEST EXECUTE SUCCEEDED **`** |
| skip without env (after that) | `Executed 1 test, with 1 test skipped and 0 failures` in 0.042s / `** TEST EXECUTE SUCCEEDED **` |

Successful Mac pid 96307, listen port 49886, fp `d1a593f0e3db50d5`.
Paired `2026-09-14T08:36:42Z`; last capture `2026-09-14T08:36:57Z`.

## Mac inbox (provider none)

Isolated `pairs.json` after the passing run (PSK not copied here):

- 1 pair: companion `FlashTeXPad (iPad Pro 11-inch (M5))`, `capture_count` **2**
- `cap-eabd9008-d6c6-402f-b899-b2a20670dc38` ack `{durable:false, has_proposal:false, applied:false}` (bundled sample, 4528 base64 PNG)
- `cap-c0ff790c-0d4e-46a4-b9c3-f23ad46bc0bc` ack `{durable:false, has_proposal:false, applied:false}` (synthetic drawing, 30908 base64 PNG)

iPad receipts: `received — Mac inbox, not journaled (durable:false)` and
`Mac: on the Mac (inbox, no bridge attached) — not converted`. No proposal,
no conversion call.

## Screenshots (apps only)

| file | what |
|---|---|
| `mac-flashtex-main.png` | FlashTeX window + Captures inspector: two received rows (drawing then sample), Advertising, No bridge |
| `mac-nearby-companion.png` | Nearby Companion: port 49886, paired FlashTeXPad, two received / not durable captures |
| `ipad-e2e-02-paired.png` | Capture tab, `Connected to Overpriced Hardware`, Captures (0) |
| `ipad-e2e-03-sample-ready.png` | bundled `sample-capture.png` loaded |
| `ipad-e2e-04-sample-received.png` | first receipt, durable=false |
| `ipad-e2e-05-drawing-ready.png` | three finger strokes |
| `ipad-e2e-06-drawing-received.png` | Captures (2), second receipt |

`ipad-e2e-01-mac-link-filled` was captured by the test but **not checked in**:
it showed the `flashtex-nearby://pair?…` field including the 6-digit code.
A post-test `simctl io screenshot` showed Springboard, not the app, and was
discarded.

XCUITest attachments are landscape content in a portrait runner frame (same
as `docs/evidence/ios-acceptance-2026-09-12`).

## What this simulator run cannot exercise

- Camera / VisionKit QR scan (`pair.qr.unavailable`)
- Apple Pencil pressure (finger drags, `drawingPolicy = .anyInput`)
- Real Wi‑Fi / Bonjour across two physical devices (loopback `127.0.0.1` + typed port; `FLASHTEX_NEARBY_SERVE_LAN` unset)
- Paid conversion / any provider (explicit `FLASHTEX_CONVERSION_PROVIDER=none`, keys unset)

## Automation vs human

No human click was required on the successful path. The product Paste button
is automatable only after dismissing the system **Allow Paste** control; that
cost ~60s of app-idle wait and blew the 120s pairing code on the second run.
The gated test therefore **types** the bootstrap URL into `pair.qr.text`.

## Bugs / notes (Mac not patched — other lane owns recent capture PRs)

- Nearby listener bound `*:49886` (IPv6 all interfaces), not loopback-only, even
  with `FLASHTEX_NEARBY_SERVE_LAN` unset. Report only.
- `FLASHTEX_LOG` was set; Nearby activity is in-window, not that file.
- SwiftUI Mac-link `Form` does not expose `pair.port` until scrolled on-screen
  (UI-test issue; handled in `CaptureFlowRealMacUITests.reveal`).
