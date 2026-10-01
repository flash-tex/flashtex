# Editor themes

Settings ▸ Themes picks the **code theme** (the colours of the LaTeX source and
of the editor itself), overrides individual **editor colours** on top of it,
and holds the **display** switches: line height, font ligatures, line numbers,
current-line highlight and invisible characters. Font family, size, tab width
and soft wrap stay in Settings ▸ Editor. Everything applies at once to every
open editor.

## Built-in themes

| Theme | Light | Dark | Source |
|---|---|---|---|
| FlashTeX (default) | IntelliJ Light vocabulary | New UI Dark vocabulary | the editor's original colours |
| Solarized | Solarized Light | Solarized Dark | Ethan Schoonover, MIT |
| GitHub | GitHub Light | GitHub Dark | Primer "prettylights", MIT |
| One Light / One Dark | Atom One Light | Atom One Dark | MIT |
| Dracula / Alucard | Alucard | Dracula | MIT |
| Nord | derived from Snow Storm | Nord | Arctic Ice Studio, MIT |

Every theme has a light and a dark palette. Settings ▸ Editor ▸ Editor
appearance (System, Light, Dark) decides which one the editor shows; the
Themes pane's Light/Dark switch only chooses which palette the preview shows
and the colour wells edit. Nord ships no light editor theme, so its light
palette is derived from the same colours (darker Frost and Aurora hues on
Snow Storm) and is marked as derived in the source.

The definitions live in `Sources/FlashTeXEditorCore/EditorColorTheme.swift`,
which the iPad app shares: its editor draws the same token colours.

## Your own themes

Theme files are JSON in `~/Library/Application Support/FlashTeX/Themes/`
(`$FLASHTEX_THEMES_DIR` overrides the folder). **Import Theme…** validates a
file and copies it there; **Export Theme…** writes the selected theme, with
your colour overrides, as a complete file to edit and import again; **Show
Themes Folder** opens the folder. A file that cannot be read is listed in the
pane with the reason.

```json
{
  "formatVersion": 1,
  "id": "paper",
  "name": "Paper",
  "basedOn": "flashtex",
  "light": { "background": "#FFFFF8", "command": "#0033B3", "comment": "#8C8C8C80" },
  "dark":  { "background": "#1B1B1B", "command": "#CF8E6D" }
}
```

- `id` defaults to the file name; `name` is what the picker shows. A user
  theme never replaces a built-in: an `id` that clashes gets a `user.` prefix.
- Colours are `#RRGGBB` or `#RRGGBBAA` (sRGB, alpha last).
- Roles left out come from `basedOn` (a built-in id; FlashTeX by default). A
  file with only `light` or only `dark` uses it for both appearances.
- Unknown keys are ignored, so a file written for a newer FlashTeX still opens.

| Role | What it colours |
|---|---|
| `command` | control sequences in text (`\section`, `\textbf`) |
| `mathCommand` | control sequences in math (`\alpha`, `\frac`) |
| `environment` | environment names in `\begin{…}` / `\end{…}` |
| `math`, `mathDelimiter`, `number` | math content, `$ \[ \( …`, digits in math |
| `comment` | `%` comments and `comment` environments |
| `brace`, `bracket` | `{ }` and `[ ]` |
| `reference`, `file`, `definition` | `\ref`/`\cite` keys, file names, names defined by `\newcommand` and friends |
| `verbatim` | verbatim text (plain text colour when left out) |
| `error`, `warning` | diagnostic underlines |
| `background`, `foreground` | editor ground and plain text |
| `gutterBackground`, `gutterText`, `gutterActiveText` | the line-number gutter |
| `currentLine`, `selection`, `caret`, `bracketMatch`, `invisibles` | editor chrome |

## How a theme change is applied

Every editor colour is one stable dynamic `NSColor` per role
(`EditorThemeRuntime`, `Sources/FlashTeXMac/EditorThemes.swift`) that looks the
installed theme up when AppKit draws. Selecting a theme or overriding a colour
installs the new palette and redraws the editors; the syntax runs already
painted keep their (same) colour objects, so nothing is re-lexed or repainted
(`EditorThemeTests.testThemeSwitchOnAHostedEditorRedrawsWithoutRelexOrRepaint`).
Line height and ligatures change the font or paragraph style and so re-lay out
the text, like a font size change.
