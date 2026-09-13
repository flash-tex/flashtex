# Vim keybindings in the source editor

Off by default. **Settings ▸ Typing ▸ Vim keybindings** turns it on
(`EditorPreferences.vimMode`, persisted under
`FlashTeX.EditorPreferences.v1.vimMode`). With it off the editor behaves
exactly as before: the one key hook returns before touching anything, no
indicator appears in the window, and the caret is the usual thin one.

Two other ways in, matching how the capture switches work
(`CaptureInboxFeature`, `ShellModel+Nearby.swift`):

| | |
|---|---|
| `FLASHTEX_VIM_MODE=1` | turns it on for one launch (QA, screenshots) |
| `VimModeFeature.enabledOverride` | `nonisolated(unsafe) static var`, the test seam |

Unlike the capture switches this one is **opt-in**: the environment variable
must say `1`, and an absent preference reads as off.

## Where the code lives

| File | What |
|---|---|
| `VimMode.swift` | the whole mode machine, pure: `String` + UTF-16 caret in, edits and requests out. No AppKit. |
| `VimModeEditor.swift` | `VimModeFeature` (the gate), the `NSEvent` → `VimKey` mapping, the coordinator hooks, and `VimModeIndicator`. |
| `Completion.swift` | two lines: the `keyDown` gate and the block-cursor `drawInsertionPoint`. |
| `SourceEditorView.swift` | the machine's storage on the coordinator and the two owner callbacks. |
| `ContentView.swift` | the indicator strip and the routing of `:w` / `:q` / `%`. |

The machine never edits the buffer itself. It returns
`EditorKeyHandling.LineEdit` values which the host applies through the
editor's existing `applyLineEdits` — the same `shouldChangeText` /
`didChangeText` / one-undo-group path Tab indentation already uses. So
**`u` and `⌃R` drive the editor's own `NSUndoManager`**: one vim command is
one ⌘Z step, and vim edits and ordinary typing interleave correctly on a
single stack. There is no parallel undo history.

## What works

**Modes** — normal, insert, visual (`v`), visual-line (`V`). Escape or `⌃[`
returns to normal from any of them, stepping the caret left the way vim does.
The mode shows in a strip under the editor, and normal/visual mode draw a
block cursor instead of the I-beam.

**Motions** — `h j k l w b e 0 ^ $ gg G { } f F t T ; ,` and `%`, plus the
arrow keys, Return (down a line) and Backspace (left). `j`/`k` remember the
column. `%` uses the editor's own LaTeX-aware matcher
(`SourceEditorView.BraceMatcher`: `{}`, `[]`, `$`, `$$`, minding escapes, `%`
comments and `\verb`), and when there is no such pair in reach it falls
through to the app's **Go to Matching** command (⌘⇧D) — which is what handles
`\begin`/`\end` and `\label`/`\ref`.

**Operators** — `d`, `c`, `y` over any motion; `dd`, `cc`, `yy` for whole
lines; `>>` / `<<` (and `>` / `<` on a visual selection) using the editor's
indent setting; `D`, `C`, `Y`, `x`, `s`, `r<char>`, `p`, `P`, `o`, `O`,
`a`, `A`, `i`, `I`. `cw` behaves like `ce`, and `dw` on a line's last word
stops at the line end instead of joining two lines.

**Counts** — before and inside a command: `3dd`, `5j`, `d3w`; `2d3w` is `d6w`.

**Undo** — `u`, `⌃R`.

**Search** — `/` and `?` open the standard Find bar (⌘F's), which then owns
the keystrokes; `n` / `N` are Find Next / Find Previous, swapped after `?`.
This is the app's `NSTextFinder`, so matching is the Find bar's (literal, its
own case rules), not vim regex.

**Ex commands** — `:w` (Save, the action ⌘S runs), `:q`, `:wq` / `:x`, `:q!`
and `:<line>`. Anything else reports `E492` and beeps.

`:q` needs a note: the app has no "close this document" command. A secondary
document is **detached** from the session, exactly as its tab's ✕ does; the
entry document falls back to the window's own close, which prompts to save
just as ⌘W does. `:q!` discards a secondary document's edits; it cannot
discard the entry document's.

## What is deliberately not implemented

One **unnamed register** — no named registers, no yank ring. No marks, no
macros (`q`/`@`), no `.` repeat, no text objects (`ciw`, `di{`), no `*`/`#`,
no `J`, `~`, `R` (replace mode) or `gJ`, no WORD motions (`W B E`), no
`H M L` or `zz zt zb`, no jump list (`⌃O`/`⌃I`), no window/tab/buffer
commands, and no `:` command beyond the ones above. No `:set`, no `.vimrc`,
no remapping. Visual-block mode (`⌃V`) is absent. An unrecognised key in
normal or visual mode is swallowed and beeps rather than being typed.

## Shortcut interactions

Nothing in the app binds a bare key, so vim's normal-mode letters collide with
no menu shortcut. The gate refuses every event carrying ⌘ or ⌥, so every menu
shortcut (⌘S, ⌘B, ⌘Z, ⌘F, ⌘⇧D, ⌘/, ⌥⌘F …) and every ⌥-composed character
behaves unchanged, and ⌃Space (completion) and ⌘⇧Space (signature help) are
untouched because the machine ignores every control key but `⌃R`.

What vim mode does change, and only while it is on:

| Key | Without vim mode | With vim mode |
|---|---|---|
| Escape in **normal** mode | opens the completion list | returns to normal / clears a pending command |
| Escape in **insert** mode | opens the completion list | closes a live completion list or snippet first; a second Escape leaves insert mode |
| Letters, digits, punctuation in normal/visual mode | typed | vim commands |
| Tab / ⇧Tab in normal mode | indent / outdent | swallowed (use `>>` / `<<`) |
| Return, Backspace, arrows in normal mode | newline, delete, move | motions |

Tab, ⇧Tab and Return in **insert** mode are not vim keys, so auto-indent,
indentation and snippet stops run untouched.

## Input methods

Insert mode is transparent on purpose: the machine takes Escape and refuses
every other key, so ordinary typing never leaves AppKit's path. The gate also
sits *after* `CompletingTextView`'s `hasMarkedText()` guard, so a composition
(an input method, a dead key) never reaches vim mode at all, and ⌥-composed
characters are refused before the machine sees them.

That is the design. It has **not** been verified against a real non-US
keyboard or a live input source: a test process cannot drive one (the existing
`IMECompositionTests` replay compositions at the client API for the same
reason), and this change has not been exercised by hand on a non-US layout.
The claim here is that insert mode takes no keys away from AppKit, which the
tests do check — not that IME has been observed working end to end.

## Tests

`apps/mac/Tests/FlashTeXMacTests/VimModeTests.swift` — 35 cases. Most drive
`VimMachine` directly (key sequence in, buffer and caret out), the way
`EditorKeyHandlingTests` drives the indentation logic: no window, no first
responder, no synthesized events. The last few host the real editor to check
the gate (with the preference off, `d` `d` types "dd"), that a vim edit lands
on the editor's own undo stack, and that the indicator reports the mode.
