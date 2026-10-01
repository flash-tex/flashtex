# iPad full-screen canvas and double-tap undo (2026-09-30)

Simulator: iPad Air 11-inch (M4), iOS 26.5, Xcode 26.6, on mac-m1max-a.

| File | What it shows | Source |
|---|---|---|
| 01-portrait-canvas.png | The canvas fills the screen edge to edge; controls float over it | `CanvasUITests.testCanvasFillsTheScreenInBothOrientations` |
| 02-portrait-settings.png | The canvas settings popover: gesture switches, Pencil double-tap mode | same |
| 03-portrait-sidebar.png | The floating sidebar button reveals the hidden sidebar | `testSidebarButtonRevealsTheSidebar` |
| 04-portrait-two-finger-undo.png | After a two-finger double-tap: 3 strokes → 2 | `testTwoFingerDoubleTapUndoesAndThreeFingerDoubleTapRedoes` |
| 05-portrait-three-finger-redo.png | After a three-finger double-tap: back to 3 strokes, with the "Redo" toast | same |
| 06-landscape-after-rotation.png | The portrait drawing after rotating to landscape: same coordinates, top-left anchored; the canvas scrolls to reach the rest | host screenshot during the same run |
| 07-landscape-collapsed.png | The controls collapsed to the corner button | host screenshot |
| 08-landscape-captures-panel.png | After Send: the Captures panel at the trailing edge, with the returned TikZ | host screenshot during `CaptureFlowUITests.testDrawSendReceipt` |
| 09-landscape-dark-settings.png | Dark mode: the canvas follows the appearance (ink shows light on dark) | host screenshot, `simctl ui appearance dark` |
| 10-landscape-dark-canvas.png | Dark mode, controls | same |

The XCTest runner took the portrait images. On this headless simulator its
landscape screenshots come out rotated and cropped, so the landscape images
are `xcrun simctl io <udid> screenshot` framebuffers rotated upright. Images
07–10 predate two small follow-up edits (the portrait toolbar layout and the
settings popover height), neither of which changes these landscape layouts.

Run:

```
cd apps/ios
xcodebuild -project FlashTeXPad.xcodeproj -scheme FlashTeXPad \
  -destination 'platform=iOS Simulator,name=iPad Air 11-inch (M4)' \
  -only-testing:FlashTeXPadTests -only-testing:FlashTeXPadUITests/CanvasUITests \
  -only-testing:FlashTeXPadUITests/CaptureFlowUITests -only-testing:FlashTeXPadUITests/FlashTeXPadUITests test
```

After the review fixes (compact-width navigation, sideways-reachable ink): 113 unit tests with the CI flags (`CODE_SIGNING_ALLOWED=NO`), one Keychain test skipped as on main; 8 of 8 UI tests ad-hoc signed. The UI tests
that restore a Keychain pairing need an ad-hoc-signed build, so run them without
`CODE_SIGNING_ALLOWED=NO`.
