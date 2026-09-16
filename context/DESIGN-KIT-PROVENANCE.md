# What in this kit is verified, what is adapted, what is mine

You asked for community-verified components rather than things I invented. Here
is the honest accounting, so you can check any of it yourself.

## Verified — used as-is

### The workflow structure
**[OneRedOak/claude-code-workflows → design-review](https://github.com/OneRedOak/claude-code-workflows/tree/main/design-review)** — 3.9k stars.

This is the widely-used Claude Code design-review workflow. It defines a
four-part structure that this kit follows exactly:

| Their file | This kit |
|---|---|
| `/context/design-principles.md` | `context/design-principles.md` |
| `/context/style-guide.md` | `context/style-guide.md` |
| `design-review-agent.md` | `.claude/agents/design-review.md` |
| `design-review-slash-command.md` | `.claude/commands/design-review.md` |
| `design-review-claude-md-snippet.md` | `CLAUDE-md-snippet.md` |

Their core principle — **"Live Environment First"**, assess the running
interface before reading code — and their phase ordering (interaction flow →
responsiveness → visual polish → accessibility → robustness → code health) are
kept verbatim in the reviewer agent.

### The aesthetic-direction skill
**[anthropics/skills → `frontend-design`](https://github.com/anthropics/skills)** — official Anthropic, 169k-star repo.

Install this and let it load. One honest caveat: `frontend-design` is written to
help an agent **invent a distinctive visual identity** and avoid templated
defaults, and it leans web ("For web designs, the hero is the first thing
viewers will see"). Your brief is the opposite — you want the app to match a
specific set of references, not to develop an original aesthetic. So use it for
its taste-and-intentionality framing, and let `context/design-principles.md`
override it wherever they disagree. That override is stated explicitly in the
agent files.

### The render loop (this is what replaces Playwright)
The OneRedOak workflow depends on the Playwright MCP server. Playwright drives
browsers; your app is a native Mac app, so that specific dependency does not
transfer. Two established macOS substitutes, both well-adopted:

- **[Peekaboo](https://github.com/steipete/Peekaboo)** — 5.2k stars. macOS CLI +
  optional MCP server built specifically so AI agents can screenshot a named
  application window and drive its UI (click, type, scroll, drag), including
  without bringing the app frontmost. Needs Screen Recording and Accessibility
  permission, macOS 15+. This is the closest thing to "Playwright for native
  macOS" and it is the direct swap for the reviewer agent.
- **[XcodeBuildMCP](https://github.com/cameroncooke/XcodeBuildMCP)** — 6.4k
  stars. Build/run/launch for Xcode projects, with `build_run_macos` and
  `launch_mac_app`. Note the limitation: its screenshot and UI-automation tools
  target **iOS simulators**, not macOS app windows — so use it for the
  build-and-launch half and Peekaboo for the capture half.
- **[swift-snapshot-testing](https://github.com/pointfreeco/swift-snapshot-testing)**
  — 4.3k stars, Point-Free. Renders SwiftUI/AppKit views to PNGs in a normal
  test run. Headless, deterministic, no permissions needed, and doubles as
  regression protection. **Recommended as the primary loop**, with Peekaboo as
  the secondary for whole-window and interaction checks.

### Accessibility and critique
Your marketplace already carries the **`design` plugin** (knowledge-work-plugins),
which includes `design:design-critique`, `design:accessibility-review` and
`design:design-system`. Enable it rather than having an agent improvise
accessibility criteria. It is design-workflow oriented rather than
SwiftUI-specific, so it supplements the reviewer agent rather than replacing it.

## Adapted — verified thing, changed only where it had to be

- `.claude/agents/design-review.md` — OneRedOak's agent, with the Playwright
  tool list swapped for Peekaboo/XcodeBuildMCP, viewport/responsive phases
  rewritten as window-resize phases, and "browser console errors" replaced with
  build warnings and runtime log checks.
- `CLAUDE-md-snippet.md` — their "Quick Visual Check" block, with the
  `mcp__playwright__browser_navigate` step replaced by the macOS render loop.
- `.claude/commands/design-review.md` — their slash command, same git-diff
  scaffolding, pointed at the macOS agent.

## Mine — because no verified equivalent exists

- `context/design-principles.md` and `context/style-guide.md`. These are *meant*
  to be project-specific; the upstream repo ships only an example and expects
  you to replace it. The content is the reference board from our session, so
  it's your decisions, not my taste.
- `.claude/agents/ui-ux-implementer.md` — the Fable implementer. OneRedOak's
  workflow only covers **review**, not implementation. I could not find a
  widely-used implementer agent for native macOS UI work; if you find one,
  prefer it and keep my file only for the constraints section.

## What I could not find

There is no established, widely-used Claude Code skill for **SwiftUI or macOS
UI design** specifically. The design skills that exist are web-oriented
(`frontend-design`, the design plugin, the Qt plugin's `qt-ui-design`). If that
changes, the slot to swap it into is `context/design-principles.md`.
