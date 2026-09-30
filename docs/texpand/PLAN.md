# TeXpand: an Emmet-style abbreviation engine for a LaTeX IDE

"TeXpand" is a working name. Rename freely; nothing below depends on it.

> **Status (FlashTeX):** M0–M5 are implemented. The core is in
> `apps/mac/Sources/FlashTeXEditorCore/TeXpand/`, tested headlessly in
> `apps/mac/Tests/TeXpandTests/`. Live capture is in the **Mac** editor
> (`FlashTeXMac/TeXpandEditor.swift`); the iPad is a follow-up. The feature is
> **off by default** (Settings › Abbreviations). Host findings and the
> integration: [HOST.md](HOST.md).
>
> The owner's spec below was written without knowledge of this codebase. The
> owner asked for it to be adapted rather than followed blindly, and for the
> whole feature to be customisable. The section that follows records every
> change. Where the spec text itself was changed (§9.4, M5, M10b), the change is
> marked **(owner decision)** or **(owner addition)**.

## Adaptations for FlashTeX

### Where the engine lives

- **Language and bridge.** Both editors, the Mac `NSTextView` and the iPad
  `UITextView`, are Swift. The core is therefore pure Swift, needs no bridge,
  and uses Foundation only. It lives in `FlashTeXEditorCore`, which
  `apps/ios/Packages/FlashTeXPadKit` links through a symlink, so the Mac and
  the iPad run one engine.
- **Namespace and licence.** Every type sits under `enum TeXpand`. This is MIT
  app code and touches no engine crate (DESIGN §3).

### Customisability (owner requirement)

- **Switches.** `TeXpand.Settings` holds:
  - `enabled`: the master switch, **default off** until the feature is reviewed;
  - one enable per tier: `abbreviations` (A), `instant_atoms`, `ligatures` (B),
    `postfix` (C) and `structure_editor` (M10b). Ligatures stay off even when
    the master is on, because they fire without Tab;
  - `disabled_packs`, which turns built-in packs off one by one;
  - `packs`, which turns on opt-in packs such as the domain packs;
  - `disable`, which turns individual definitions off;
  - `leader`, `profile`, `fraction_trigger`, `fraction_operator` and
    `auto_preamble`.
- **Where they are set.**
  - The app-level copy lives in **Settings › Abbreviations**, stored as JSON in
    its own UserDefaults key (the error-lens pattern).
  - `texpand.toml` files layer `[settings]` over it.
  - A file can switch the feature **off** but never on: the master switch
    belongs to the person using the app, not to a project someone else wrote.
- **Per-document magic comments** (`% !texpand …`) stay in M8, as planned.

### Config format (TOML)

- **Parser.** The Mac app has no Swift TOML parser: `flashtex.toml` is parsed
  by the Rust helper (`crates/project-manifest`), which the iPad does not have.
  Instead of adding a package, the core carries a **minimal hand-written TOML
  subset parser** (`TeXpandTOML.swift`). It supports:
  - tables, arrays of tables and nested `[[abbr.variant]]`;
  - dotted and quoted keys, including inside inline tables;
  - all four string forms;
  - integers, floats, booleans, arrays and inline tables.

  It rejects dates with an error, and every error carries its line.
  *Question for the owner:* is the in-core parser acceptable long-term, or
  should it be a package dependency (for example TOMLKit)? Nothing else in the
  app would use it today.
- **File name.** Project config is a separate `texpand.toml` rather than a
  `[texpand]` table in `flashtex.toml`. The manifest's single parser is Rust,
  and the iPad must read the config too. Folding it into the manifest later is
  possible; ask before doing so.
- **Built-in catalog.** It is written in the same TOML, as eleven packs
  (`TeXpandCatalog.swift`). The built-ins therefore exercise the loader and
  double as examples.

### Registry and layering (M2 part of §13)

- **Layers.** `Registry.load(layers:settings:)` merges the built-in packs,
  then the given layers (user, then project), with §13's semantics. A higher
  layer replaces a definition with the same name and scope set.
- **`disable`.**
  - A file's top-level `disable = [...]` removes definitions from *lower*
    layers, so the same file can redefine the name.
  - `[settings] disable` removes the name everywhere.
- **Profiles.** `[profile]` overrides keys, and `[profiles.NAME]` defines
  named profiles. The built-in profiles are `default`, `upright` and `bold`.
- **Out of scope for M2 (M8).** Reading files from disk, hot reload and magic
  comments.
- **Tier B and C tables.** `[[ligature]]` and `[[postfix]]` are read and
  noted, but stay inactive until M5.
- **Load-time lint (§16).** It checks:
  - duplicate names in overlapping scopes;
  - instant-name collisions: the instant atom is dropped;
  - instant atoms that take params;
  - holes that name undeclared params, arguments or fields;
  - unknown holes;
  - unknown profile keys (a warning);
  - unknown keys (a warning);
  - bad defaults;
  - non-letter names;
  - a missing body or generator;
  - unparseable `default_child`s.

  A bad definition is skipped with a diagnostic naming its layer and line, and
  nothing else breaks.

### Grammar and rules (M1)

The parser follows §4.2 and §4.3 with these adjustments, each found while
running the §14 catalog through it:

- **`!` and the shape.** `!` may come before or after the shape. §14 writes
  `align!3`; the grammar gives `align3!`.
- **Suffixes after a repeat** are accepted (`item*3<+->`, §4.1).
- **Rule 4.** In a non-leaf param, `.` ends the param only before a letter and
  never as the second dot of `..`, because modifier names are letters. So
  `col:0.3` and `for:i=1..n` stay one param, and `tab:lcr:4.float` still has
  its modifier.
- **Rule 5, widened.** An overlay is also accepted when the document class is
  `beamer`, because `frame>items>item*3<+->` creates the frame its items live
  in. Elsewhere `<` is still `Invalid`.
- **Rule 7.** Children on a group, `(a+b)>c`, are `Invalid`, with the message
  "put `>` inside the parentheses".
- **Limits.**
  - Repeat counts and sizes must be between 1 and 1000.
  - An expansion is capped at 2000 elements and nesting depth 24.
  - A default child that contains itself fails cleanly.
- **Labels and offsets.** Label names may contain `@`, so `thm{T@}#t@*2`
  numbers them. Offsets are UTF-16, as everywhere in the editors.

### Definitions and templates (M2)

- **Additions.** All are declarative, and each is needed by the text catalog:
  - `provides`: the scopes children are resolved in. For example, `enum`
    provides `list`, `eq` provides `math`, and `align` provides `env:align`.
  - `default_child`: an *abbreviation*, not just a name (`fig` defaults to
    `img+cap`).
  - `children_optional`: with no children, the `<<children>>` line disappears
    (`item`, `sec`).
  - `child_separator` (`\\` between align rows) and `row_break` (between rows
    of `subfig2x2`).
  - Argument declarations with `optional`, `prefix` and `suffix`, so theorem
    titles render as `[Name]` or not at all.
  - `[abbr.modifier.NAME]` tables with a `<<body>>` hole (`.float`), each with
    its own `label_prefix`.
- **New holes.**
  - `<<opt.value>>`: the bare value of `[…]`, or a tabstop.
  - `<<i0>>`: the zero-based counter.
  - `<<N|a,b>>`: a choice (the IR had `Choice`, but no hole wrote one).
  - `<<body>>`.
  - `:default` on `<<arg.…>>` holes.
- **More `when` keys:** `param`, `star` and `modifier`, alongside `package`,
  `class`, `scope` and `profile`. `doc:beamer` and `doc:hw` are `param`
  variants.
- **Blank-line elision.** A template line that holds only whitespace and holes
  that all render empty is dropped, as `<<label>>` is with no label. This is
  what keeps templates free of conditionals.
- **Scope flags.**
  - `float` (inside figure or table) and `structure` (grid environments, for
    M10b) join §8's flags.
  - `env:NAME` beats `list`/`beamer`/…, which beats `text`/`math`/`preamble`
    when names collide. `sub` is `\subsection` in text and a subfigure inside
    a float.
- **Generators return templates.** Tabstops, labels and children therefore
  work the same in computed output. M2 ships `table` (`tab`, `btab`) and
  `columns` (`cols`), which the text catalog needs. Matrix, sequence and
  rotation stay in M4.

### Catalog (§14, text and structure)

- **What ships:** `doc` (`article`, `report`, `beamer`, `hw`), `pkg`,
  sections, lists, floats, theorem-likes, tables, display math, algorithms,
  listings, beamer and references.
- **Changes:**
  - `else` becomes **`ifelse`**. algpseudocode's `\Else` sits one level *out*
    from its siblings, and a child cannot dedent.
  - `lst:LANG` and `minted:LANG` pass the language through verbatim; there is
    no name mapping.
  - Theorem-likes assume the document declares its `\newtheorem`s. M9 (macro
    awareness) is the real fix.
  - `cap` takes no label; put the label on the float (`fig#arch>img+cap`).
  - The default profile's `ref_cmd = \cref` requires `cleveref`, through a
    profile variant.
- **Wrap mode.** A bare `*` needs a selection, so until M7 it is an error.

### Fraction operator (owner decision, affects §9.4 and M5)

- **The operator.** A single `/` **never** expands, because inline `a/b` is
  often deliberate. The operator is **`//`**:
  - `a//b` becomes `<<profile.frac>>{a}{b}`;
  - `(x+1)//2` becomes `\frac{x+1}{2}` (`strip_parens` as before);
  - `a//` with nothing after it gets a tabstop denominator.
- **Settings.**
  - `fraction_operator = "//"` is the default. `"/"` restores the original
    single-slash form.
  - `fraction_trigger = "tab" | "auto" | "off"` stays, with `tab` the default.
    `off` disables the fraction form.
- **Auto mode.** The spec's old rule that `//` yields a literal slash in auto
  mode goes. In auto mode, typing the second `/` after an atom expands. A user
  who wants a literal `//` undoes once (undo-to-literal) or turns the feature
  off.

§9.4 and M5 below are updated to match. The setting exists in M2
(`TeXpand.Settings.fractionOperator`); the matcher is M5.

### Context structure editor (owner addition, new milestone M10b)

- **The shortcut.** One configurable shortcut is context-aware.
  - Inside a matrix, `array`, tabular-like, `cases` or `align`/`gather`
    environment, it toggles an **inline grid editor** over the environment.
    Inside the grid you can:
    - edit cells, moving with Tab and Enter;
    - add and remove rows and columns, which updates a tabular's column spec;
    - switch the matrix type or delimiter (`pmatrix` ↔ `bmatrix` ↔ `vmatrix`).

    Each edit round-trips to source as **one undo step**.
  - Pressing the shortcut again, or Esc, returns to source.
  - Outside any structure, the same shortcut opens the abbreviation prompt
    (§9.6).
- **Extensible.** An environment registers a *structure editor provider*
  through the same registry and config layering. The built-ins cover
  matrices, tabular/array, cases and align/gather; tikz-cd and booktabs tables
  could follow. Each provider can be enabled or disabled like everything else,
  and the whole editor has `structure_editor` in Settings.
- **What M2 lays down for it.**
  - `ScopeStack` frames carry `start`, `bodyStart` and `bodyEnd` offsets. The
    `structure` flag and `ScopeStack.innermost(where:)` find the enclosing grid
    environment.
  - `TeXpand.Grid` parses and prints a grid body: rows on `\\`, cells on `&`
    at brace depth 0, and `\hline`/`\toprule`/… as rule rows. It round-trips,
    and the `table` generator already builds its output with it. M10's
    `matrixAddRow` and `tableAddColumn` are edits on a `Grid`.
- **What the host needs.** HOST.md § "Structure editor (M10b)".

### Live capture (M3)

- **Where it runs.** The Mac editor only. The iPad shares the core, but it has
  no settings surface to turn the feature on, so its adapter is a follow-up
  (HOST.md § "iPad").
- **Scope provider.** `TeXpand.ScopeProvider` scans from the document start
  and checkpoints the frame stack at line starts every 2048 UTF-16 units, so
  each query rescans only from the nearest valid checkpoint (§8's scanner
  option). The syntax highlighter's per-line state has no environment stack,
  so it is not used.
  - It tracks the preamble, environments with their offsets, math,
    verbatim environments, `\verb`/`\lstinline` and comments.
  - The shared `\begin`/`\end` scan (`LaTeXScan`, formerly the Mac's
    `EditorNavigation`) moved into the core.
- **§9.1 as specified, plus:**
  - **Tab while idle** expands the `leader+abbreviation` just before the
    caret, so an abbreviation whose capture was lost (the caret moved away
    and back) still expands. It skips a region with a suppression mark,
    which is what makes Esc and undo-to-literal permanent until the caret
    leaves the line.
  - **Empty leader:** Tab expands the word before the caret if it is an
    abbreviation (§2's bare Tab mode, accepting collisions).
  - **Region end:** a capture region runs from the leader to the furthest
    edit, so a brace the editor auto-closes is inside it, and typing over it
    is not "the caret leaving the region".
  - **Overlays:** an overlay on an element whose template has no
    `<<overlay>>` slot is now an error rather than silently dropped.
- **Snippets.** `LaTeXSnippet` gained placeholder lengths (`caretLength`,
  `stopLengths`).
  - The Mac session selects a placeholder when it visits it and resizes it
    as you type over it, so ⇧Tab reselects what you typed.
  - Bare stops (the completion snippets') keep their old caret-only
    behaviour.
  - Mirrors stay in §5's degraded form: plain copies of their placeholder.
- **Not yet.** The prompt/wrap shortcut (§9.6, M7) stays unbound; ⌘; is
  macOS's Check Document Now. `texpand.toml` is not read yet (M8), and
  `requires` is reported but not inserted (M6).

### Math, instant atoms, ligatures and postfix (M4, M5)

- **Packs.** Four new built-in packs, each switchable like the others:
  `math` (§14's math catalog), `greek` (the instant atoms), `ligatures` and
  `postfix`. They are in `TeXpandMathCatalog.swift`, in the same TOML as
  every other definition.
- **Generators.** They return templates, as in M2, so defaults are tabstops
  and repeated values are mirrors:
  - `matrix`: `pmat3x3`, the fills `:a :I :0 :diag:λ :aug`, symbolic
    `pmat:mxn:a` with `profile.matrix_dots`, and `vec`/`rvec` vectors;
  - `sequence`: any joiner written on both sides of `..`;
  - `rotation`: 2-D, and 3-D about x, y or z;
  - `integral`: `int:a..b:x`, `int:D:x`, `int2`, `int3`;
  - `derivative`: `dd`, `dd2`, `pd`, and `pd:f/x,y` mixed, using
    `profile.diff_d` and `profile.frac`, or `\dv`/`\pdv` when the document
    loads physics;
  - `tikzcd` (`cd:2x2`) and `exact` (`ses:A,B,C`).
- **Sizes and params.** A size-taking abbreviation with no size in its name
  keeps the default size when its first param is not a size, so `int:0..1:x`
  and `dd:y/x` read as written. `when.param` sees the size, so variants can
  depend on it.
- **Arrow params** gained a `cmd` field (`\to` / `\mapsto`), because a
  template cannot write `\<<…>>` (that is the escape for a literal `<<`).
- **Deferred to a stretch goal:** `tree`, `graph`, `plot` and `fsm`
  (§14). `qty` passes its unit through unparsed (Open question 4).
- **Instant atoms (§9.2).**
  - A typed non-letter right after `;a` commits `\alpha` and stays after
    it. If that character is the leader, it arms again.
  - Letters keep capturing, so `;p` can still become `;pmat`.
  - Greek atoms are math-only (Open question 2).
- **Ligatures (§9.3).**
  - Triggers are matched on the typed suffix of the line, longest first.
  - A trigger that is a proper prefix of another one waits (`<=` for
    `<=>`, `|-` for `|->`). The next keystroke either extends it or settles
    it, Tab settles it, and Esc leaves it literal.
  - Guards are regexes on the text before the trigger. Regex ligatures run
    only when no trigger fired; their `$n` groups are substituted without
    NSRegularExpression's backslash rules, so LaTeX bodies stay literal.
  - `profile.ligature_trailing_space` appends a space after a control word.
  - Undo restores the trigger and marks it, so it does not fire again.
  - Ligatures stay off by default even with the master on. The per-tab
    toggle command (§9.3) is a follow-up.
- **Postfix (§9.4).** On Tab, in math only, the backward atom parser reads:
  - a letter, a digit run, or a control sequence with its groups;
  - a balanced `( )`, `[ ]` or `{ }`;
  - a `\left … \right` pair;
  - `_`/`^` scripts.

  It stops at the line start or the math region's start. `strip_parens` also
  strips a bare `{…}` group. `3.14` never matches, because a postfix name is
  letters and must be registered.
- **Fractions.** The owner's `//` operator:
  - `fraction_operator = "/"` restores the single slash;
  - `fraction_trigger = "auto"` fires on the operator's last character;
  - `fraction_trigger = "off"` turns fractions off.
- **Mac.** Commits that a keystroke completes (instant atoms, ligatures, auto
  fractions) are applied in `didChangeText`, after the keystroke's own edit.
  Each is its own undo step and keeps the caret where it was. The adapter
  takes the exact edit from `shouldChangeText`, because the storage's
  `editedRange` can be wider than the change.

### Deferred

- **M11 scripting runtime:** deferred; Open question 1 stands.
- **Not yet built:** M6–M11 and the iPad adapter.
- **Open question 2 (instant atoms in text):** proposed as math only by
  default.
- **Open question 5 (rendered-math preview):** ghost text only for now. The
  app's math preview crops already-compiled pages (`MathHoverPreview`), and an
  uncompiled expansion has no rendering yet. Revisit once the engine host can
  render fragments.

## 0. How to use this document (instructions for Claude Code)

- This is a spec plus a milestone plan. Work through the milestones in section 15 in order. Each has acceptance criteria; do not start a milestone until the previous one's tests pass.
- This plan is deliberately host-agnostic, because it was written without knowledge of the editor's stack. **Milestone M0 is reconnaissance**: inspect the repo, find out how the editor handles keystrokes, snippets, completion, keybindings, config, and LaTeX parsing, and write the findings to `docs/texpand/HOST.md` before writing engine code.
- Items marked **Decision** are settled; implement them as written. Items marked **Open** need a short proposal and a question to the user before implementation.
- The core engine (parser, registry, resolver, expander, generators) must not import editor APIs. Everything editor-specific goes behind the host adapter interface in section 3. The core must be runnable headlessly in tests.
- Implement the core in whatever language the editor's extension layer can call cheaply. If the core and the editor are in different languages, propose the bridge (e.g. WASM) in M0 and ask.
- Ask before adding dependencies.

## 1. Goals and non-goals

Goals:

- Short abbreviations that expand into LaTeX structure, in the spirit of Emmet, with nesting, repetition, grouping, labels, and arguments.
- Context awareness: the same engine with different expansion tables per scope (preamble, text, math, list, tikz, beamer frame, algorithm).
- Computed expansions (matrices, sequences, rotation matrices), not only static templates.
- Separation of *semantic abbreviation* from *notation rendering*, via swappable notation profiles.
- Everything customizable through layered config: definitions, operators' trigger policy, leader character, notation, keybindings.
- No accidental expansions. Every misfire must be reversible with a single undo.

Non-goals (for now):

- Being a general snippet manager. The host's snippet system stays as is; TeXpand emits into it.
- Full LaTeX parsing. Scope detection needs only math/environment/verbatim/comment/preamble tracking.
- Natural-language or AI-driven expansion.

## 2. The three tiers

Expansions fall into three tiers with different trigger and boundary rules. This split is the central design decision; do not collapse the tiers into one mechanism.

| Tier | Examples | Start boundary | Trigger | Default scope |
|---|---|---|---|---|
| A. Leader abbreviations | `;enum3`, `;pmat3x3:I`, `;fig>img+cap`, `;a` | the leader character | Tab (or instant, see 9.2) | depends on definition |
| B. Ligatures | `->`, `!=`, `...`, `xx`, `x1` | none (suffix match) | automatic on keystroke | math only |
| C. Postfix | `x.hat`, `(x+1).sqrt`, `(x+1)/2` | the preceding parsed math atom | Tab | math only |

**Decision:** the leader is `;` by default, configurable, and may be set to empty (bare Tab-triggered abbreviations, accepting collisions). Rationale: in real LaTeX `;` is almost always followed by whitespace or a newline, so "`;` immediately followed by a letter" is a nearly free namespace. `\` is rejected because it shares a namespace with real commands (`\sec`, `\int`).

## 3. Architecture

```
keystrokes ──► HostAdapter ──► InteractionController
                                 ├─ CaptureFSM        (tier A)
                                 ├─ LigatureMatcher   (tier B)
                                 └─ PostfixMatcher    (tier C)
                                        │
                     ScopeProvider ◄────┤
                     PackageIndex  ◄────┤
                                        ▼
        Parser ─► AST ─► Resolver(Registry) ─► Expander(Profile, Generators) ─► Expansion
                                                                                   │
HostAdapter ◄── replace region with snippet, preamble edits, preview ◄─────────────┘
```

Modules:

- **Parser** (pure): abbreviation string to AST. Needs a small oracle from the registry (`is_leaf(name)`), see 4.3.
- **Registry** (pure): loads layered definition files (section 13), validates, indexes by name and scope, reports config diagnostics.
- **Resolver** (pure): picks the definition and variant for each AST element given the scope stack and the loaded-package set.
- **Expander** (pure): AST plus definitions plus profile to Snippet IR. Calls generators.
- **Generators** (pure): built-in computed expansions (matrix, sequence, rotation, table). Interface: `(invocation, ctx) -> Snippet`. Scripted user generators come later (M11).
- **ScopeProvider**: `scope_at(doc, offset) -> ScopeStack`. Host-assisted; see section 8.
- **PackageIndex**: set of loaded packages and the document class, plus the preamble insertion point. See section 10.
- **MacroIndex**: user-defined commands, environments, theorems. See section 11.
- **InteractionController**: the state machines in section 9. Editor-agnostic; consumes abstract events (`CharTyped`, `Tab`, `Esc`, `CursorMoved`, `Undo`), emits abstract effects (`ShowPreview`, `ReplaceRange`, `Highlight`, `InsertPreamble`).
- **HostAdapter**: the only editor-specific code. Translates editor events to controller events, applies effects, converts Snippet IR to the editor's native snippet format.

Core API sketch (adapt to the implementation language):

```
parse(input, oracle) -> Ok(Ast) | Err(ParseError { kind: Incomplete | Invalid, offset, message })
expand(ast, ctx) -> Ok(Expansion { snippet, requires: [PackageReq], warnings }) | Err(ExpandError)
  ctx = { scope: ScopeStack, packages: Set, profile: Profile, indent_unit, base_indent, selection?: string }
Registry::load(layers) -> (Registry, [ConfigDiagnostic])
```

The `Incomplete` versus `Invalid` distinction matters: the capture FSM keeps capturing on `Incomplete` and cancels on `Invalid`.

## 4. Abbreviation language (tier A)

### 4.1 Operators

Emmet's `^`, `$`, and bare `*` collide with LaTeX, so they are remapped.

| Syntax | Meaning | Example |
|---|---|---|
| `>` | nest: following sequence becomes children of the last item | `enum>item*3` |
| `+` | sibling | `fig>img+cap` |
| `*N` | repeat N times | `item*4` |
| `*` | repeat once per selected line (wrap mode only) | `enum>item*` |
| `( )` | grouping | `frame>(cols:6,4>items3)+note` |
| `N` / `NxM` after a name | shape shorthand | `pmat3x3`, `align3`, `enum4` |
| `!` | starred variant | `align!`, `sec!` |
| `:param` | parameters, typed per definition | `sum:i=1..n` |
| `[..]` | optional argument | `img[width=.8\linewidth]` |
| `{..}` | mandatory argument(s), positional | `sec{Introduction}` |
| `<..>` | beamer overlay spec (beamer scope only) | `item*3<+->` |
| `#name` | label, auto-prefixed from context | `fig#arch` gives `\label{fig:arch}` |
| `#` | label slugged from the title or caption | `sec{Intro}#` |
| `.mod` | element modifier defined per abbreviation | `tab:lcr:4.float` |
| `@` | 1-based counter inside repeated items (`@0` zero-based, `@@` literal) | `item{Step @}*3` |

### 4.2 Grammar (EBNF)

```
abbr      = seq ;
seq       = item , { "+" , item } , [ ">" , seq ] ;   (* children attach to the LAST item *)
item      = ( group | element ) , [ repeat ] ;
group     = "(" , seq , ")" ;
repeat    = "*" , [ digits ] ;
element   = name , [ shape ] , [ "!" ] , [ params ] , { suffix } ;
name      = letter , { letter } ;                       (* letters only *)
shape     = digits , [ "x" , digits ] ;
params    = ":" , param , { ":" , param } ;
suffix    = opt | arg | overlay | label | modifier ;
opt       = "[" , balanced , "]" ;
arg       = "{" , balanced , "}" ;
overlay   = "<" , overlay-spec , ">" ;
label     = "#" , [ label-name ] ;                      (* [A-Za-z0-9_-]+ *)
modifier  = "." , letter , { letter } ;
```

### 4.3 Disambiguation rules

1. **Names are letters only.** Lexing takes the maximal run of letters as the name, then an optional shape. `pmat3x3` is name `pmat` with shape `3x3`; `dd2` is name `dd` with shape `2`.
2. **`*` is repetition only.** Starred variants use `!`.
3. **Leaf elements use raw params.** Math generators take params containing grammar characters (`lim:x->0`, `set:x|x>0`, `seq:x_1+..+n`). A definition with `leaf = true` consumes the rest of the abbreviation as params, split on `:` at bracket depth 0. This is why the parser takes an `is_leaf(name)` oracle. Leaf elements cannot have children, siblings, or repeats after their params; to combine them, wrap in a group: `(sum:i=1..n)+(prod:j=1..m)`.
4. **Non-leaf params** end at the first of `> + * ( ) [ { # . <` at depth 0. Braces protect: `thm:{a>b}`.
5. **Overlay `<..>`** is only recognized when the scope stack contains a beamer frame. Elsewhere `<` is `Invalid`.
6. **Whitespace** is allowed only inside `[]`, `{}`, and leaf params wrapped in braces. Whitespace elsewhere ends capture (section 9.1).
7. **Children without a slot:** if a definition has no `<<children>>` hole, giving it children is `Invalid` with message "`X` cannot have children".
8. **Missing params and args become tabstops** with the declared default as placeholder. `;sum` alone must still expand usefully.

### 4.4 Typed params

Definitions declare param types; the engine parses them so definitions stay declarative.

| Type | Accepts | Fields |
|---|---|---|
| `raw` | anything | `value` |
| `int` | digits | `value` |
| `range` | `[var=]lo..hi` | `var?`, `lo`, `hi` |
| `ratio` | `num/den[,den2,...]` | `num`, `dens[]` |
| `list` | comma-separated at depth 0 | `items[]` |
| `arrow` | `lhs->rhs` or `lhs\|->rhs` | `lhs`, `rhs`, `kind` |
| `pred` | `lhs\|rhs` | `lhs`, `rhs` |
| `shape` | `N`, `NxM`, symbolic `mxn` | `rows`, `cols`, `symbolic` |
| `colspec` | tabular column spec | `value`, `ncols` |

## 5. Snippet IR

The expander's output. The host adapter converts it to the editor's native snippet format.

```
Snippet  = [Node]
Node     = Text(string)
         | Tabstop { index, placeholder: Snippet? }
         | Mirror  { index, transform: None | Slug | Upper | Lower }
         | Choice  { index, options: [string] }
         | Final                      -- cursor rest position ($0)
         | Newline                    -- host applies base indent
         | Indent(Snippet)            -- one indent unit deeper
Expansion = { snippet, requires: [PackageReq], warnings: [string] }
```

Rules:

- Tabstop indices are renumbered globally after tree expansion, in document order, so repeated children get distinct stops.
- If the host cannot do transformed mirrors, `Mirror{Slug}` degrades to a plain tabstop with the slug of the placeholder as its default.
- Indentation is structural (`Indent`, `Newline`), never literal spaces in templates beyond what the template author writes inside a line.

## 6. Definition format

**Decision:** TOML files, templates in literal strings (`'...'` / `'''...'''`) so backslashes need no escaping. Template holes use `<< >>`, chosen because `$` and `{}` both collide with LaTeX. A literal `<<` is written `\<<`.

Holes:

| Hole | Renders |
|---|---|
| `<<1>>`, `<<1:default>>` | tabstop |
| `<<=1>>`, `<<=1\|slug>>` | mirror, optionally transformed |
| `<<0>>` | final cursor |
| `<<children>>` | expanded children, one per line |
| `<<selection>>` | wrapped text in wrap mode; a tabstop otherwise |
| `<<arg.1>>` or `<<arg.NAME>>` | positional `{}` argument; tabstop if absent |
| `<<opt>>` | `[value]` if given, nothing otherwise |
| `<<overlay>>` | `<spec>` if given, nothing otherwise |
| `<<label>>` | full `\label{prefix:name}` if given, nothing otherwise |
| `<<star>>` | `*` if `!` was used, nothing otherwise |
| `<<p.NAME.FIELD>>` | typed param field |
| `<<profile.KEY>>` | notation profile value |
| `<<i>>` | repeat counter |

There are no conditionals or loops in templates. Anything needing logic is a generator.

### 6.1 Tier A examples

```toml
[[abbr]]
name = "enum"
scope = ["text"]
shape = "children"            # enum4 == enum>item*4
default_child = "item"
body = '''
\begin{enumerate}<<opt>>
  <<children>>
\end{enumerate}'''

[[abbr]]
name = "item"
scope = ["list"]
body = '\item<<overlay>><<opt>> <<arg.1>>'

[[abbr]]
name = "fig"
scope = ["text"]
label_prefix = "fig"
requires = ["graphicx"]
body = '''
\begin{figure}<<opt>>
  \centering
  <<children>>
  <<label>>
\end{figure}'''

[[abbr]]
name = "sum"
scope = ["math"]
leaf = true
params = [{ name = "r", type = "range", default = "i=1..n" }]
body = '\sum_{<<p.r.var>>=<<p.r.lo>>}^{<<p.r.hi>>} <<0>>'

[[abbr]]
name = "dd"
scope = ["math"]
leaf = true
params = [{ name = "q", type = "ratio", default = "y/x" }]
body = '\frac{<<profile.diff_d>><<p.q.num>>}{<<profile.diff_d>><<p.q.dens.0>>}'
  [[abbr.variant]]
  when = { package = "physics" }
  body = '\dv{<<p.q.num>>}{<<p.q.dens.0>>}'

[[abbr]]
name = "pmat"
scope = ["math"]
leaf = true
requires = ["amsmath"]
generator = "matrix"
generator_opts = { env = "pmatrix" }

[[abbr]]
name = "a"
scope = ["math"]
instant = true                # see 9.2
body = '\alpha'
```

Variant selection: first `variant` whose `when` matches wins, otherwise the base `body`. `when` keys: `package`, `class`, `scope`, `profile.KEY`.

### 6.2 Tier B (ligatures)

```toml
[[ligature]]
trigger = "->"
body = '\to '
scope = ["math"]

[[ligature]]
trigger = "xx"
body = '\times '
scope = ["math"]
guard_before = '(?<![A-Za-z\\])'     # not inside a word or control sequence

[[ligature]]
regex = '(?<![A-Za-z\\])([A-Za-z])(\d)$'
body = '$1_$2'
scope = ["math"]

[[ligature]]
regex = '([A-Za-z])_(\d)(\d)$'
body = '$1_{$2$3}'
scope = ["math"]
```

### 6.3 Tier C (postfix)

```toml
[[postfix]]
name = "hat"
body = '\hat{<<atom>>}'
strip_parens = true

[[postfix]]
name = "bb"
body = '\mathbb{<<atom>>}'
requires = ["amssymb"]

[[postfix]]
name = "T"
body = '<<atom>><<profile.transpose>>'
```

## 7. Notation profile

A flat key-value table consulted by templates and generators. Profiles are swappable and layered like everything else. Switching a paper between venues' conventions should be a profile change.

| Key | Default | Alternatives |
|---|---|---|
| `diff_d` | `d` | `\mathrm{d}` |
| `frac` | `\frac` | `\dfrac`, `\tfrac` |
| `vector` | `\vec` | `\mathbf`, `\bm` |
| `transpose` | `^\top` | `^T`, `^\intercal` |
| `set_sep` | `\mid` | `:` |
| `expectation` | `\mathbb{E}` | `\operatorname{E}`, `\mathrm{E}` |
| `probability` | `\Pr` | `\mathbb{P}` |
| `matrix_dots` | `amsmath` | `none` |
| `ref_cmd` | `\cref` | `\ref`, `\autoref` |
| `ligature_trailing_space` | `true` | `false` |
| `label_prefix.*` | `fig`, `tab`, `eq`, `sec`, `thm`, `lem`, `alg`, `lst` | any |
| `label_sep` | `:` | `-`, `.` |

## 8. Scope model

`scope_at(offset)` returns a stack such as `["document", "env:frame", "env:itemize", "math:inline"]`. Definitions match against derived flags:

- `preamble`, `text`, `math` (with `math:inline`, `math:display`), `verbatim`, `comment`
- `list` (inside itemize/enumerate/description), `tikz`, `beamer` (inside a frame), `alg` (inside algorithmic), `table` (inside tabular-like)
- `env:NAME` for anything else

Rules:

- All tiers are disabled in `verbatim` and `comment`. Verbatim covers `verbatim`, `lstlisting`, `minted`, `\verb`, and `\lstinline`.
- Tiers B and C default to `math` only.
- M3 may start with a scanner from the document start that tracks math delimiters and the environment stack, cached per line and invalidated on edit. If M0 finds an existing syntax tree (tree-sitter or similar), use it instead.
- Budget: `scope_at` under 1 ms on a 5,000-line document after warm-up.

## 9. Interaction spec

### 9.1 Capture FSM (tier A)

States: `Idle`, `Armed`, `Capturing`.

- `Idle` to `Armed`: the leader is typed, scope is not verbatim/comment, and the position carries no suppression mark.
- `Armed` to `Capturing`: the next typed char is a letter. Any other event returns to `Idle`, leaving a literal leader.
- While `Capturing`, on every keystroke:
  - Re-parse the region from just after the leader to the cursor.
  - **Prefix guard:** while only a name has been typed, it must be a prefix of at least one registered name valid in the current scope. Otherwise cancel silently.
  - `Invalid` parse: cancel silently (literal text remains).
  - `Incomplete` or `Ok`: keep capturing, highlight the region, and if `Ok` and resolvable, show a ghost-text preview of the expansion (plus rendered math if the host can do it).
- Exits from `Capturing`:
  - **Tab** with `Ok`: commit. With `Incomplete`: no-op plus an inline diagnostic; do not insert a tab.
  - **Esc**: cancel, leave literal text, place a suppression mark on the region.
  - Whitespace outside brackets, newline, cursor leaving the region, or focus loss: cancel silently.
- **Commit** replaces `[leader .. cursor]` with the snippet, applies preamble edits, and enters the host's tabstop session, all as **one undo transaction**.
- **Undo-to-literal:** a single undo after commit restores the literal text including the leader, and places a suppression mark so capture does not re-arm. The mark clears when the cursor leaves the line or the region is edited.
- While capturing, suppress the host's normal completion popup; the preview replaces it.
- Ligatures (tier B) are disabled inside a capture region, so `..` and `->` inside params stay literal.

### 9.2 Instant atoms

Short atoms such as Greek letters (`;a`, `;G`, `;ve`) should not need Tab. A definition with `instant = true`:

- cannot take operators, params, children, or suffixes;
- commits automatically when the captured text equals its name and the next typed char is not a letter. That char is then processed normally (it may itself start a ligature). So `;a^2` yields `\alpha^2` and `;a+;b` yields `\alpha+\beta`.
- If the next char is a letter, capture continues (`;a` then `l` may be heading to `;align`).
- The registry must reject, at load time, an instant name that equals another abbreviation's name in an overlapping scope.

### 9.3 Ligature matcher (tier B)

- Build a trie of triggers. On each typed char in a matching scope, find the longest trigger that is a suffix of the text before the cursor and passes its guards.
- **Deferred commit:** if the matched trigger is a proper prefix of another trigger (`<=` and `<=>`), mark it pending instead of expanding. On the next event: if the char extends toward the longer trigger, continue; otherwise expand the pending trigger in place, then process the new char normally. There are no timeouts.
- Regex ligatures run after trie ligatures and only if none fired.
- Each expansion is its own undo step; undoing it restores the literal trigger and suppresses re-firing at that spot.
- A command `texpand.toggleLigatures` turns tier B on or off per editor tab, with a status-bar indicator. Leave it unbound by default.

### 9.4 Postfix matcher (tier C)

On Tab in math scope, parse backwards from the cursor within the current math region:

```
atom   = base , { script }
base   = letter | digit-run | control-sequence , { brace-group }
       | balanced "(" ... ")" | "[" ... "]" | "{" ... "}" | \left ... \right pair
script = ( "_" | "^" ) , ( single-token | brace-group )
```

- `.mod` form: text before the cursor matches `atom "." modname` where `modname` is a registered postfix. Digits before the dot (`3.14`) never match because `modname` is letters and must be registered.
- Fraction form **(owner decision)**: `atom "//" atom` becomes `<<profile.frac>>{a}{b}`; `atom "//"` becomes the same with a tabstop denominator. `strip_parens` applies to both sides. A single `/` never expands (`a/b` is often a deliberate inline fraction). The operator is the setting `fraction_operator = "//"` (default) or `"/"` (the original single-slash form).
- `fraction_trigger = "tab" | "auto" | "off"`, default `tab`. In `auto`, typing the operator's last character after an atom expands immediately; a user who wants a literal `//` undoes once (undo-to-literal) or turns the feature off. *(The original rule, "in `auto`, `//` yields a literal slash", is withdrawn.)*

### 9.5 Tab precedence

1. Host completion popup is visible and the user has navigated it: host accepts.
2. `Capturing`: commit or diagnostic (9.1).
3. Postfix candidate before the cursor: apply (9.4). This precedes tabstop jumping so `x.hat` works inside a tabstop.
4. Active snippet session: next tabstop.
5. Host default.

### 9.6 Prompt and wrap (`Cmd/Ctrl+;`)

- With no selection: opens an inline abbreviation prompt with live preview. Same parser, no leader needed. Enter or Tab commits; Esc cancels.
- With a selection: wrap mode. The selection fills `<<selection>>` of the innermost last element.
- Bare `*` distributes: `enum>item*` makes one item per non-blank selected line, stripping leading list markers (`-`, `*`, `1.`).
- Generators may register a wrap transformer: `btab` splits lines on commas or tabs into cells; `align` splits each line at its first relation symbol and inserts `&`.

## 10. Auto-preamble

- Each definition, variant, ligature, and postfix may declare `requires = ["pkg", ...]`, optionally with options: `{ name = "cleveref", options = ["capitalize"] }`.
- PackageIndex scans the root file for `\documentclass`, `\usepackage`, and `\RequirePackage` (comma lists included), plus a small table of class-implied packages.
- Root file resolution: `% !TEX root = ...` magic comment, then the host's project root setting, then the current file.
- On commit, missing packages are inserted after the last `\usepackage` line (or after `\documentclass` if none), in the same undo transaction as the expansion.
- Setting `auto_preamble = "insert" | "prompt" | "off"`, default `insert`. If no preamble can be found, expand anyway and show a non-blocking notice naming the missing packages.
- Insertion must be idempotent.

## 11. Macro awareness

MacroIndex scans the project for `\newcommand`, `\renewcommand`, `\providecommand`, `\NewDocumentCommand`, `\DeclareMathOperator`, `\newenvironment`, and `\newtheorem`, and synthesizes definitions:

- `\newtheorem{lem}{Lemma}` gives a text-scope abbreviation `lem` with a children slot, a `{Name}` optional title, and label prefix `lem`.
- `\newenvironment{foo}` gives an environment abbreviation `foo`.
- `\newcommand{\R}{...}` with n arguments gives an abbreviation with n tabstops; with zero arguments it is an instant atom only if the user opts in.
- Synthesized definitions form the **lowest-priority layer** and never override anything.
- Companion action: "Extract selection to macro" (prompts for a name, inserts `\newcommand` in the preamble, replaces the selection).

## 12. Structural edit actions

Commands operating on the syntax around the cursor, each a single undo step:

- `renameEnvironment` (both ends), `unwrapEnvironment` (keep contents), `toggleStar`
- `convertList` (itemize, enumerate, description)
- `convertMath`: inline, display, `equation`, `align`
- `toggleLeftRight` on the enclosing delimiter pair
- `matrixAddRow`, `matrixAddColumn`, `matrixRemoveRow`, `matrixRemoveColumn`
- `alignAddRow`, `tableAddRow`, `tableAddColumn` (updates the column spec)
- `wrapWithAbbreviation` (9.6), `extractToMacro` (11)

## 13. Config layering

Layers, lowest to highest priority:

1. Synthesized from MacroIndex
2. Built-in base
3. Enabled domain packs
4. User global config
5. Project config (`texpand.toml` at the project root)
6. Per-document magic comments: `% !texpand profile=physics leader=, disable=x1`

Merge semantics:

- Definitions are keyed by `(tier, name or trigger, scope set)`. A higher layer replaces the whole definition.
- `disable = ["name", ...]` removes definitions from lower layers.
- Profile keys merge key-wise.
- Settings: `leader`, `fraction_trigger`, `auto_preamble`, `packs`, `profile`, per-tier `enabled`.
- Config hot-reloads on save. Errors never crash the engine: the bad definition is skipped and surfaced as a diagnostic in the config file, with the layer named.
- Packs declare `name`, `scope`, `requires`, and `conflicts`.

## 14. Built-in catalog

Tier A, text and structure:

```
doc:article  doc:beamer  doc:hw        full preamble scaffolds
pkg:amsmath,tikz                       \usepackage lines (preamble scope)
ch sec sub ssub para                   sectioning, {title}, #, !
enum4 items3 desc>item[Term]*3 item    lists
fig img cap subfig2 subfig2x2          floats and graphics
thm lem prop cor defn rmk ex pf        theorem-likes: thm{Name}#main+pf
tab:lcr:4  btab:lrr:5{Name,Score,Time}  .float modifier adds table float + caption + label
eq eq! align3 align!3 gather cases3    display math
alg fn for while if else st ret        algpseudocode nesting
lst:rust  minted:py                    listings
frame cols:6,4 col block alert         beamer, with <overlay>
ref:fig:arch  cite:knuth               leaf; uses profile.ref_cmd
```

Tier A, math (all leaf unless noted):

```
sum prod bigcup bigcap :i=1..n         big operators with range
int:a..b:x  int2:D:x,y  oint           integrals with differential
lim:x->0                               limits
dd:y/x  dd2:y/x  pd:f/x  pd:f/x,y      derivatives, mixed partials
seq:x_1..n  seq:x_1+..+n               sequences with any joining operator
set:x|x>0                              set builder
map:f:A->B  map:f:x|->x^2              function signatures
paren brack brace norm abs floor ceil angle braket:a:b    sized delimiters
mat pmat bmat vmat Vmat                shapes 3x3; fills :a :I :0 :diag:λ :aug; symbolic :mxn:a
vec3:x  rvec3:x                        column and row vectors
rot:z:θ  rot2:θ                        filled rotation matrices
cd:2x2  ses:A,B,C                      tikz-cd square, short exact sequence
tree:a(b,c(d,e))  graph:a->b->c,a->c  plot:sin(x):-pi..pi  fsm3
qty:9.8:m/s^2                          siunitx (unit parsing is a stretch goal)
```

Instant atoms: Greek lowercase and uppercase (`a b g d e ve z h th i k l m n x p r s t u ph vph ch ps w`, `G D Th L X P S U Ph Ps W`), plus `inf`, `nab`, `par`, `ell`.

Tier B ligatures: `->` `<-` `=>` `<=>` `|->` `!=` `<=` `>=` `~=` `~~` `...` `xx` `**` (cdot) `|-` `|=` `[[` `]]` `<<` `>>` (ll, gg), digit subscripts (`x1`, `x12`).

Tier C postfix: `hat bar dot ddot vec tilde bb cal frak bf rm sf tt T inv conj sqrt lr box ub ob abs norm`, and the fraction form.

Domain packs:

```
pl        infer3  hoare  bnf:expr:3  judg (Γ ⊢ e : τ)  sem ([[e]])  subst
ml        argmin:θ  argmax:θ  E:x~p:f  kl:p:q  grad:θ  normal:μ:σ^2  softmax  loss
robotics  T:a:b  SE3  SO3  skew:ω  jac:f:q  quat  twist
physics   braket  comm:A:B  hbar ligature  dv/pdv variants via the physics package
```

## 15. Milestones

**M0. Reconnaissance and skeleton.** Inspect the repo. Write `docs/texpand/HOST.md` covering: editor component and its extension API; how keystrokes, Tab, and Esc can be intercepted; native snippet format and whether it supports mirrors, transforms, and choices; completion popup API; undo transaction API; decoration and ghost-text API; existing LaTeX parsing or syntax tree; config system and keybinding system; implementation language for the core and the bridge if any. Create the module skeleton and test harness.
*Accept:* HOST.md reviewed by the user; an empty test suite runs in CI or locally.

**M1. Parser.** Grammar 4.2 with rules 4.3, `Incomplete`/`Invalid` errors with offsets, typed param parsers 4.4.
*Accept:* golden tests for every row in 4.1 and every rule in 4.3; a fuzz or property test shows the parser never panics and always terminates on arbitrary input.

**M2. Registry, templates, expander.** TOML loading, hole syntax, Snippet IR, tabstop renumbering, indentation, repeat counters, labels with prefixes, variants. Ship the text and structure catalog. Provide a headless `expand(abbr, ctx) -> string` for tests.
*Accept:* golden tests `abbr | scope | packages | profile -> expected snippet` for the whole text catalog.

**M3. Scope provider and host integration.** ScopeProvider, CaptureFSM, preview, commit, single-transaction undo, suppression marks, Tab precedence.
*Accept:* FSM tests driven by simulated event sequences covering every transition in 9.1; manual check in the editor that `;enum3` Tab works, Esc leaves literal text, and one undo restores it.

**M4. Math generators and notation profile.** Typed-param math abbreviations, the matrix, sequence, and rotation generators, the profile table, instant atoms (9.2).
*Accept:* golden tests for the math catalog under at least two profiles; `;a^2` yields `\alpha^2` without Tab.

**M5. Ligatures and postfix.** Trie matcher with deferred commit, regex ligatures, guards, the backwards atom parser, the fraction form.
*Accept (owner decision on the fraction operator):* `<=` versus `<=>` resolves correctly; `(x+1)//2` Tab yields `\frac{x+1}{2}`; `a/b` never changes; `3.14` and a literal `//` in text or comments never trigger; `\alpha_i.hat` yields `\hat{\alpha_i}`.

**M6. Auto-preamble and package-conditional variants.** PackageIndex, root file resolution, idempotent insertion, the `auto_preamble` setting.
*Accept:* `;cd:2x2` in a document without tikz-cd inserts the package in the same undo step; running it twice inserts once; with `physics` loaded, `;dd:y/x` emits `\dv`.

**M7. Prompt and wrap.** `Cmd/Ctrl+;` prompt, wrap mode, bare `*` line distribution, wrap transformers for `btab` and `align`.
*Accept:* wrapping three selected lines with `enum>item*` yields three items; wrapping CSV with `btab` yields a booktabs table.

**M8. Config layering.** All layers in section 13, merge semantics, `disable`, hot reload, config diagnostics, magic comments.
*Accept:* a project file overrides a built-in; a malformed definition produces a diagnostic and nothing else breaks.

**M9. Macro awareness.** MacroIndex, synthesized layer, extract-to-macro.
*Accept:* adding `\newtheorem{claim}{Claim}` makes `;claim#foo` work with no config.

**M10. Structural actions.** Section 12.
*Accept:* each action has a before/after golden test and is a single undo step.

**M10b. Context structure editor (owner addition).** One configurable, context-aware shortcut. Inside a matrix, `array`, tabular-like, `cases` or `align`/`gather` environment it toggles an inline grid editor over the environment: edit cells (Tab/Enter move), add and remove rows and columns (a tabular's column spec follows), switch the matrix type or delimiter (`pmatrix` ↔ `bmatrix` ↔ `vmatrix`); each edit round-trips to source as one undo step. The shortcut again, or Esc, returns to source. Outside any structure the shortcut opens the abbreviation prompt (9.6). Providers register per environment through the registry and config layering (built-ins: matrices, tabular/array, cases, align/gather; later tikz-cd, booktabs tables) and can be enabled or disabled like everything else. Reuses the M3 ScopeProvider, `TeXpand.Grid` and the M10 actions.
*Accept:* toggling inside `pmatrix` opens the grid; adding a column and switching to `bmatrix` produce the expected source in one undo step each; a tabular's column spec gains a column when a column is added; Esc returns to source with the caret in the edited cell; outside a structure the shortcut opens the prompt; with `structure_editor` off the shortcut only opens the prompt.

**M11. Domain packs and scripted generators.** Ship the packs in section 14. **Open:** propose the scripting runtime for user generators (candidates: an embedded scripting language in the core's language, Lua, WASM plugins, or the host's own extension language), with sandboxing and a time budget per call, and ask before implementing.

## 16. Testing strategy

- **Golden tables** are the primary test form: one file of `input | context -> expected` rows per catalog area. Adding an abbreviation means adding rows.
- **Parser properties:** no panics, termination, and `Ok` parses re-serialize to an equivalent abbreviation.
- **Expander properties:** deterministic; tabstop indices are unique and contiguous; every `requires` entry is reported.
- **FSM tests:** simulated event streams, asserting emitted effects. No real editor needed.
- **Ligature tests:** longest match, deferred commit, guards, disabled inside capture regions and verbatim.
- **Preamble tests:** idempotence, comma-list detection, root file resolution.
- **Registry lint at load:** duplicate names in overlapping scopes, instant-name collisions, templates referencing undeclared params, unknown profile keys.

## 17. Open questions

1. Scripting runtime for user generators (M11).
2. Whether instant atoms should be on by default in text scope (for example `;a` inside prose producing `$\alpha$`).
3. Fuzzy completion of `ref:` and `cite:` params from the label and bibliography index: inside the capture region, or handed to the host's completion popup.
4. Unit parsing for `qty` (siunitx): how much of the unit grammar to support.
5. Whether the rendered-math preview is feasible in the host, or ghost text only.
