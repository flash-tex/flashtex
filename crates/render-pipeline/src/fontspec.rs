//! fontspec's font-selection commands, read from the source bytes.
//!
//! `\setmainfont`, `\setsansfont`, `\setmonofont`, `\newfontfamily`,
//! `\fontspec`, `\defaultfontfeatures` and `\addfontfeature` are the
//! compatible surface of the font system (`docs/proposals/
//! packages-fonts-manifest.md` §2.4): what documents written for XeLaTeX
//! and LuaLaTeX already say, mapped onto the pipeline's own named families
//! ([`crate::fonts::Family::Named`]). The compiler has no model for them
//! (#920 makes them known vocabulary that consumes its argument and is
//! diagnosed as `unsupported_feature`; the pinned `vendor/compiler` sets
//! the argument as plain text), so the pipeline reads them from the source
//! bytes -- the way `abstractenv`, `columns` and the TikZ reader do -- and
//! supersedes the compiler's diagnostics at the same spans.
//!
//! What each command means here follows fontspec.sty (v2.9, `fontspec-
//! code-interfaces.dtx` and the manual §4):
//!
//! * `\setmainfont[opts]{Family}` sets `\rmdefault`; `\setsansfont` and
//!   `\setmonofont` set `\sfdefault`/`\ttdefault`. In the preamble that is
//!   the whole document; in the body the change is local to the group
//!   ([`group_end`]). The old `\setromanfont` is `\setmainfont`.
//! * `\newfontfamily\cmd[opts]{Family}` defines `\cmd` as a family switch,
//!   like `\rmfamily`: from `\cmd` to the end of the enclosing group the
//!   text is set in that family, whatever `\textsf`/`\ttfamily` would
//!   otherwise select. `\newfontface` is the single-face form and is read
//!   the same way.
//! * `\fontspec[opts]{Family}` selects the family locally, from the
//!   command to the end of the enclosing group.
//! * `\defaultfontfeatures{opts}` applies to every family declared after
//!   it; `\defaultfontfeatures[Family]{opts}` to declarations of that
//!   family only. `\addfontfeature{opts}` (`\addfontfeatures`) re-selects
//!   the current family with the options added, locally.
//! * The options honoured: `Scale=` (a factor, `MatchLowercase`,
//!   `MatchUppercase`), `BoldFont=`, `ItalicFont=`, `BoldItalicFont=`,
//!   `UprightFont=`, `Numbers=OldStyle`, `Ligatures=TeX` (the default:
//!   `--`/`---`/quotes as TeX, which the adapter already forms). Every other
//!   key is accepted and noted once per command as not applied.
//!
//! Precedence, per family slot: a local `\fontspec`/`\cmd` group, then a
//! document `\setmainfont`, then the manifest's `[fonts]` table
//! ([`crate::RenderOptions::fonts`]), then the class default (Latin Modern,
//! Times or Computer Modern exactly as today). A document that names no
//! font goes through none of this: `apply` returns early and the blocks
//! are untouched, which `scripts/render-corpus-v2.sh` checks byte for byte.
//!
//! The math font (`\setmathfont`, `unicode-math`) is the other half of the
//! system and is specified, not implemented, in
//! `docs/proposals/font-system-math.md`; `\setmathfont` is read here only
//! to say so once.

use flashtex_compiler::Span;

use crate::adapter::{Block, Item, ParaPart, TextStyle};
use crate::fonts::{NamedSpec, Scale};
use crate::nfss::FamilyKind;
use crate::style::Stylesheet;
use crate::RenderOptions;

/// The named-family settings of one document, kept on the [`Stylesheet`]:
/// every distinct spec the document (or the manifest) named, and which of
/// them the three family slots default to. Indices are document-local;
/// `typeset::Context` interns them on the font set.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Settings {
    pub families: Vec<NamedSpec>,
    /// `\rmdefault`: `\setmainfont` in the preamble, else `[fonts] text`.
    pub text: Option<u16>,
    /// `\sfdefault`: `\setsansfont`, else `[fonts] sans`.
    pub sans: Option<u16>,
    /// `\ttdefault`: `\setmonofont`, else `[fonts] mono`.
    pub mono: Option<u16>,
}

impl Settings {
    pub fn is_empty(&self) -> bool {
        self.families.is_empty()
    }

    /// The slot default for a family kind.
    pub fn slot(&self, kind: FamilyKind) -> Option<u16> {
        match kind {
            FamilyKind::Rm => self.text,
            FamilyKind::Sf => self.sans,
            FamilyKind::Tt => self.mono,
        }
    }

    fn intern(&mut self, spec: NamedSpec) -> u16 {
        if let Some(i) = self.families.iter().position(|s| *s == spec) {
            return i as u16;
        }
        self.families.push(spec);
        (self.families.len() - 1) as u16
    }
}

/// What `apply` hands back to the adapter.
#[derive(Debug, Default)]
pub struct Applied {
    /// Command spans whose compiler diagnostic the pipeline supersedes.
    pub superseded: Vec<Span>,
    /// Notes about what was read but not applied.
    pub limitations: Vec<(&'static str, Span, String)>,
}

/// Which command a source occurrence is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    SetMain,
    SetSans,
    SetMono,
    NewFamily,
    FontSpec,
    DefaultFeatures,
    AddFeature,
    SetMath,
}

const COMMANDS: [(&str, Kind); 11] = [
    ("setmainfont", Kind::SetMain),
    ("setromanfont", Kind::SetMain),
    ("setsansfont", Kind::SetSans),
    ("setmonofont", Kind::SetMono),
    ("newfontfamily", Kind::NewFamily),
    ("newfontface", Kind::NewFamily),
    ("fontspec", Kind::FontSpec),
    ("defaultfontfeatures", Kind::DefaultFeatures),
    ("addfontfeature", Kind::AddFeature),
    ("addfontfeatures", Kind::AddFeature),
    ("setmathfont", Kind::SetMath),
];

/// One command occurrence in one document.
#[derive(Debug, Clone)]
struct Command {
    kind: Kind,
    /// The command and its arguments, `[start, end)`.
    start: usize,
    end: usize,
    /// `\cmd` of `\newfontfamily\cmd`, and where it starts (the compiler
    /// reports the definition's `\cmd` as an unknown command of its own).
    switch: Option<String>,
    switch_at: usize,
    options: String,
    /// The mandatory argument (a family name, or a feature list for
    /// `\defaultfontfeatures`/`\addfontfeature`).
    arg: String,
}

/// A range of one document set in a named family.
#[derive(Debug, Clone, Copy)]
struct Scope {
    start: usize,
    end: usize,
    family: u16,
    /// `Some(kind)`: only text in that family slot (a body
    /// `\setmainfont` leaves `\textsf` alone); `None`: everything
    /// (`\fontspec`, a `\newfontfamily` switch).
    kind: Option<FamilyKind>,
}

/// Whether any of the commands occurs in `text` at all (outside comments).
pub fn present(text: &str) -> bool {
    COMMANDS.iter().any(|(name, _)| crate::adapter::find_command(text, name).is_some())
}

/// Every command occurrence of `text`, in source order.
fn commands(text: &str) -> Vec<Command> {
    let mut found: Vec<Command> = Vec::new();
    for (name, kind) in COMMANDS {
        let mut from = 0;
        while let Some(rel) = crate::adapter::find_command(&text[from..], name) {
            let start = from + rel;
            let mut at = start + 1 + name.len();
            from = at;
            let mut switch = None;
            let mut switch_at = at;
            if kind == Kind::NewFamily {
                at = skip_spaces(text, at);
                let Some(rest) = text.get(at..).and_then(|r| r.strip_prefix('\\')) else { continue };
                let word: String = rest.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
                if word.is_empty() {
                    continue;
                }
                switch_at = at;
                at += 1 + word.len();
                switch = Some(word);
            }
            // `[opts]{arg}` or `{arg}[opts]` (fontspec ≥ 2.7 allows both).
            let mut options = String::new();
            let mut arg = None;
            for _ in 0..2 {
                at = skip_spaces(text, at);
                match text.as_bytes().get(at) {
                    Some(b'[') if options.is_empty() && !(kind == Kind::DefaultFeatures && arg.is_some()) => {
                        let Some(close) = balanced_end(text, at, b'[', b']') else { break };
                        options = text[at + 1..close].to_string();
                        at = close + 1;
                    }
                    Some(b'{') if arg.is_none() => {
                        let Some(close) = balanced_end(text, at, b'{', b'}') else { break };
                        arg = Some(text[at + 1..close].to_string());
                        at = close + 1;
                    }
                    _ => break,
                }
            }
            let Some(arg) = arg else { continue };
            found.push(Command { kind, start, end: at, switch, switch_at, options, arg });
            from = at;
        }
    }
    found.sort_by_key(|c| c.start);
    found
}

fn skip_spaces(text: &str, mut at: usize) -> usize {
    while text.as_bytes().get(at).is_some_and(|b| *b == b' ' || *b == b'\t') {
        at += 1;
    }
    at
}

/// The index of the closer matching the opener at `open`, with braces
/// nested inside either kind of bracket, or `None` when unclosed.
fn balanced_end(text: &str, open: usize, opener: u8, closer: u8) -> Option<usize> {
    let b = text.as_bytes();
    let mut depth = 0i32;
    let mut braces = 0i32;
    let mut i = open;
    while i < b.len() {
        let c = b[i];
        if c == b'\\' {
            i += 2;
            continue;
        }
        if c == opener && braces == 0 {
            depth += 1;
        } else if c == closer && braces == 0 {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        } else if c == b'{' {
            braces += 1;
        } else if c == b'}' {
            braces -= 1;
        }
        i += 1;
    }
    None
}

/// Where the group enclosing byte `from` ends: the `}` or `\end{..}` that
/// closes it, or the end of the text (a switch at the top level of the
/// body runs to `\end{document}`). Braces and `\begin`/`\end` pairs both
/// count as groups; comments and escaped braces are skipped.
pub(crate) fn group_end(text: &str, from: usize) -> usize {
    let b = text.as_bytes();
    let mut depth = 0i32;
    let mut i = from;
    while i < b.len() {
        match b[i] {
            b'%' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'\\' => {
                if text[i..].starts_with("\\begin") && !b.get(i + 6).is_some_and(u8::is_ascii_alphabetic) {
                    depth += 1;
                    i += 6;
                    continue;
                }
                if text[i..].starts_with("\\end") && !b.get(i + 4).is_some_and(u8::is_ascii_alphabetic) {
                    if depth == 0 {
                        return i;
                    }
                    depth -= 1;
                    i += 4;
                    continue;
                }
                i += 1;
            }
            b'{' => depth += 1,
            b'}' => {
                if depth == 0 {
                    return i;
                }
                depth -= 1;
            }
            _ => {}
        }
        i += 1;
    }
    b.len()
}

/// Byte offset of `\begin{document}`; the text's length when absent (a
/// body-only document has no preamble).
fn body_start(text: &str) -> usize {
    let mut from = 0;
    while let Some(rel) = crate::adapter::find_command(&text[from..], "begin") {
        let at = from + rel;
        let after = skip_spaces(text, at + "\\begin".len());
        if text[after..].starts_with("{document}") {
            return at;
        }
        from = at + 1;
    }
    text.len()
}

/// A parsed option list.
#[derive(Debug, Default)]
struct Options {
    scale: Option<Scale>,
    bold: Option<String>,
    italic: Option<String>,
    bold_italic: Option<String>,
    upright: Option<String>,
    oldstyle: Option<bool>,
    tex_ligatures: Option<bool>,
    /// Keys accepted and not applied, as written.
    ignored: Vec<String>,
}

impl Options {
    fn parse(text: &str) -> Options {
        let mut o = Options::default();
        for item in split_top_level(text, ',') {
            let item = item.trim();
            if item.is_empty() {
                continue;
            }
            let (key, value) = match item.split_once('=') {
                Some((k, v)) => (k.trim(), strip_braces(v.trim())),
                None => (item, ""),
            };
            match key.to_ascii_lowercase().as_str() {
                "scale" => {
                    o.scale = Some(match value.to_ascii_lowercase().as_str() {
                        "matchlowercase" => Scale::MatchLowercase,
                        "matchuppercase" => Scale::MatchUppercase,
                        v => match v.parse::<f64>() {
                            Ok(f) if f > 0.0 && f.is_finite() => Scale::Factor(f),
                            _ => {
                                o.ignored.push(item.to_string());
                                continue;
                            }
                        },
                    });
                }
                "boldfont" => o.bold = Some(value.to_string()),
                "italicfont" => o.italic = Some(value.to_string()),
                "bolditalicfont" => o.bold_italic = Some(value.to_string()),
                "uprightfont" => o.upright = Some(value.to_string()),
                "numbers" => {
                    let v = value.to_ascii_lowercase();
                    if v.contains("oldstyle") {
                        o.oldstyle = Some(true);
                    } else if v.contains("lining") {
                        o.oldstyle = Some(false);
                    } else {
                        o.ignored.push(item.to_string());
                    }
                }
                "ligatures" => {
                    // A list: `Ligatures={TeX,Common}`. `TeX`/`NoTeX` is
                    // the setting; `Common` is the shaper's default (GSUB
                    // `liga`) and so is accepted silently; anything else
                    // (`NoCommon`, `Rare`, `Historic`, ...) is not applied.
                    let mut other = false;
                    for v in split_top_level(value, ',') {
                        match v.trim().to_ascii_lowercase().as_str() {
                            "tex" => o.tex_ligatures = Some(true),
                            "notex" => o.tex_ligatures = Some(false),
                            "common" | "" => {}
                            _ => other = true,
                        }
                    }
                    if other {
                        o.ignored.push(item.to_string());
                    }
                }
                _ => o.ignored.push(item.to_string()),
            }
        }
        o
    }

    fn apply_to(&self, spec: &mut NamedSpec) {
        if let Some(s) = self.scale {
            spec.scale = s;
        }
        if let Some(b) = &self.bold {
            spec.bold_font = Some(b.clone());
        }
        if let Some(i) = &self.italic {
            spec.italic_font = Some(i.clone());
        }
        if let Some(bi) = &self.bold_italic {
            spec.bold_italic_font = Some(bi.clone());
        }
        if let Some(u) = &self.upright {
            spec.upright_font = Some(u.clone());
        }
        if let Some(o) = self.oldstyle {
            spec.oldstyle_numbers = o;
        }
        if let Some(t) = self.tex_ligatures {
            spec.tex_ligatures = t;
        }
    }
}

fn strip_braces(v: &str) -> &str {
    v.strip_prefix('{').and_then(|v| v.strip_suffix('}')).unwrap_or(v).trim()
}

/// Splits on `sep` outside braces.
fn split_top_level(text: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in text.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            c if c == sep && depth == 0 => {
                out.push(&text[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&text[start..]);
    out
}

/// One document's reading: its scopes and the commands found.
#[derive(Default)]
struct DocScan {
    commands: Vec<Command>,
    scopes: Vec<Scope>,
    /// Byte ranges of `\cmd` uses of `\newfontfamily` switches (superseded
    /// compiler diagnostics).
    switch_uses: Vec<(usize, usize)>,
}

/// Reads one document. `settings` collects the specs; the entry document's
/// preamble commands set the slot defaults.
fn scan(text: &str, is_entry: bool, settings: &mut Settings, limitations: &mut Vec<(&'static str, Span, String)>, document: usize) -> DocScan {
    let mut out = DocScan { commands: commands(text), ..DocScan::default() };
    let body = body_start(text);
    // `\defaultfontfeatures`: the general defaults and per-family ones, in
    // source order, applied to every declaration after them.
    let mut defaults: Vec<(Option<String>, Options)> = Vec::new();
    let mut switches: Vec<(String, u16)> = Vec::new();
    let span = |c: &Command| Span::in_document(flashtex_compiler::DocumentId(document), c.start, c.end);
    let build = |arg: &str, options: &str, defaults: &[(Option<String>, Options)], limitations: &mut Vec<(&'static str, Span, String)>, c: &Command| -> NamedSpec {
        let mut spec = NamedSpec::new(arg);
        for (family, o) in defaults {
            if family.as_ref().is_none_or(|f| flashtex_font_discovery::normalize(f) == flashtex_font_discovery::normalize(arg)) {
                o.apply_to(&mut spec);
            }
        }
        let o = Options::parse(options);
        o.apply_to(&mut spec);
        note_ignored(&o, limitations, span(c), &spec.family);
        spec
    };
    let cmds = out.commands.clone();
    for c in &cmds {
        match c.kind {
            Kind::DefaultFeatures => {
                let o = Options::parse(&c.arg);
                note_ignored(&o, limitations, span(c), "\\defaultfontfeatures");
                let family = (!c.options.trim().is_empty()).then(|| c.options.trim().to_string());
                defaults.push((family, o));
            }
            Kind::SetMain | Kind::SetSans | Kind::SetMono => {
                let spec = build(&c.arg, &c.options, &defaults, limitations, c);
                let id = settings.intern(spec);
                let kind = match c.kind {
                    Kind::SetMain => FamilyKind::Rm,
                    Kind::SetSans => FamilyKind::Sf,
                    _ => FamilyKind::Tt,
                };
                if c.start < body {
                    if is_entry {
                        match kind {
                            FamilyKind::Rm => settings.text = Some(id),
                            FamilyKind::Sf => settings.sans = Some(id),
                            FamilyKind::Tt => settings.mono = Some(id),
                        }
                    }
                } else {
                    out.scopes.push(Scope { start: c.end, end: group_end(text, c.end), family: id, kind: Some(kind) });
                }
            }
            Kind::NewFamily => {
                let spec = build(&c.arg, &c.options, &defaults, limitations, c);
                let id = settings.intern(spec);
                if let Some(s) = &c.switch {
                    switches.retain(|(name, _)| name != s);
                    switches.push((s.clone(), id));
                }
            }
            Kind::FontSpec => {
                let spec = build(&c.arg, &c.options, &defaults, limitations, c);
                let id = settings.intern(spec);
                out.scopes.push(Scope { start: c.end, end: group_end(text, c.end), family: id, kind: None });
            }
            Kind::AddFeature => {
                // The family in force at the command: the innermost scope
                // holding it, else the main slot.
                let current = out
                    .scopes
                    .iter()
                    .filter(|s| s.start <= c.start && c.start < s.end)
                    .max_by_key(|s| s.start)
                    .map(|s| s.family)
                    .or(settings.text);
                match current {
                    Some(id) => {
                        let mut spec = settings.families[usize::from(id)].clone();
                        let o = Options::parse(&c.arg);
                        o.apply_to(&mut spec);
                        note_ignored(&o, limitations, span(c), &spec.family);
                        let id = settings.intern(spec);
                        out.scopes.push(Scope { start: c.end, end: group_end(text, c.end), family: id, kind: None });
                    }
                    None => limitations.push((
                        "fontspec_feature_ignored",
                        span(c),
                        "\\addfontfeature applies to the named font in force, and none is: the class font keeps its metrics".to_string(),
                    )),
                }
            }
            Kind::SetMath => limitations.push((
                "math_font_not_implemented",
                span(c),
                format!(
                    "\\setmathfont{{{}}}: selecting a math font is not implemented yet (docs/proposals/font-system-math.md); math is set in Latin Modern Math",
                    c.arg.trim()
                ),
            )),
        }
    }
    // Uses of the switches, each a scope to the end of its group. A switch
    // defined in the preamble of the entry document is visible in every
    // document; one defined here is searched for here (the common case).
    for (name, id) in &switches {
        let mut from = 0;
        while let Some(rel) = crate::adapter::find_command(&text[from..], name) {
            let at = from + rel;
            let end = at + 1 + name.len();
            from = end;
            // Skip the definition itself.
            if cmds.iter().any(|c| c.kind == Kind::NewFamily && c.start <= at && at < c.end) {
                continue;
            }
            out.switch_uses.push((at, end));
            out.scopes.push(Scope { start: end, end: group_end(text, end), family: *id, kind: None });
        }
    }
    out.scopes.sort_by_key(|s| (s.start, std::cmp::Reverse(s.end)));
    out
}

fn note_ignored(o: &Options, limitations: &mut Vec<(&'static str, Span, String)>, span: Span, what: &str) {
    if !o.ignored.is_empty() {
        limitations.push((
            "fontspec_feature_ignored",
            span,
            format!("{what}: fontspec option{} {} not applied (Scale, BoldFont, ItalicFont, BoldItalicFont, UprightFont, Numbers=OldStyle and Ligatures=TeX are)", if o.ignored.len() == 1 { "" } else { "s" }, o.ignored.join(", ")),
        ));
    }
    if o.oldstyle == Some(true) {
        limitations.push((
            "fontspec_feature_ignored",
            span,
            format!("{what}: Numbers=OldStyle is recorded but the shaper does not run GSUB `onum` yet; lining figures are set"),
        ));
    }
}

/// Reads every document's fontspec commands into `style.fontspec`, marks
/// the text each scope covers (`TextStyle::named`), removes the argument
/// text the pinned compiler set as body copy, and returns the spans whose
/// compiler diagnostics are now the pipeline's.
pub fn apply(texts: &[&str], entry: usize, blocks: &mut [Block], style: &mut Stylesheet, options: &RenderOptions) -> Applied {
    let mut applied = Applied::default();
    let any = texts.iter().any(|t| present(t));
    let manifest = options.fonts.as_ref().filter(|f| f.text.is_some() || f.sans.is_some() || f.mono.is_some());
    if !any && manifest.is_none() {
        return applied;
    }
    let mut settings = Settings::default();
    // The manifest first, so the document's own commands override it.
    if let Some(f) = manifest {
        settings.text = f.text.as_deref().map(|n| settings.intern(NamedSpec::new(n)));
        settings.sans = f.sans.as_deref().map(|n| settings.intern(NamedSpec::new(n)));
        settings.mono = f.mono.as_deref().map(|n| settings.intern(NamedSpec::new(n)));
    }
    let scans: Vec<DocScan> = texts
        .iter()
        .enumerate()
        .map(|(d, t)| if present(t) { scan(t, d == entry, &mut settings, &mut applied.limitations, d) } else { DocScan::default() })
        .collect();
    for (d, s) in scans.iter().enumerate() {
        let doc = flashtex_compiler::DocumentId(d);
        applied.superseded.extend(s.commands.iter().map(|c| Span::in_document(doc, c.start, c.end)));
        applied.superseded.extend(s.commands.iter().filter(|c| c.switch.is_some()).map(|c| Span::in_document(doc, c.switch_at, c.end)));
        applied.superseded.extend(s.switch_uses.iter().map(|&(a, b)| Span::in_document(doc, a, b)));
    }
    style.fontspec = settings;
    for block in blocks.iter_mut() {
        walk_block(block, &scans);
    }
    applied
}

/// The scope in force at byte `at` of document `d` for a run in family
/// slot `kind`: the innermost (latest-starting) one that covers the byte
/// and applies to the slot.
fn scope_at(scans: &[DocScan], d: usize, at: usize, kind: FamilyKind) -> Option<u16> {
    scans
        .get(d)?
        .scopes
        .iter()
        .filter(|s| s.start <= at && at < s.end && s.kind.is_none_or(|k| k == kind))
        .max_by_key(|s| s.start)
        .map(|s| s.family)
}

/// Whether byte `at` of document `d` is inside a command's own text (its
/// arguments), which the pinned compiler set as body copy.
fn in_command(scans: &[DocScan], d: usize, at: usize) -> bool {
    scans.get(d).is_some_and(|s| s.commands.iter().any(|c| c.start <= at && at < c.end))
}

fn walk_block(block: &mut Block, scans: &[DocScan]) {
    match block {
        Block::Paragraph { parts, list, .. } => {
            for part in parts.iter_mut() {
                match part {
                    ParaPart::Lines(items) => walk_items(items, scans),
                    ParaPart::Rows { rows, .. } => {
                        for row in rows.iter_mut() {
                            for t in row.intertext.iter_mut() {
                                walk_items(&mut t.items, scans);
                            }
                        }
                    }
                    ParaPart::Display { .. } => {}
                }
            }
            if let Some(items) = list.as_mut().and_then(|l| l.label_items.as_mut()) {
                walk_items(items, scans);
            }
        }
        Block::Heading { items, .. } | Block::Chapter { items, .. } | Block::Part { items, .. } => walk_items(items, scans),
        Block::Title { title, authors, date, .. } => {
            walk_items(title, scans);
            for a in authors.iter_mut().flatten() {
                walk_items(a, scans);
            }
            if let Some(d) = date {
                walk_items(d, scans);
            }
        }
        Block::FrameBegin { title, subtitle, .. } => {
            walk_items(title, scans);
            walk_items(subtitle, scans);
        }
        Block::BeamerTitle { title, subtitle, authors, institute, date, .. } => {
            for items in [title, subtitle, authors, institute, date] {
                walk_items(items, scans);
            }
        }
        Block::BeamerBlockBegin { title, .. } => walk_items(title, scans),
        _ => {}
    }
}

fn walk_items(items: &mut Vec<Item>, scans: &[DocScan]) {
    let taken = std::mem::take(items);
    for mut item in taken {
        match &mut item {
            Item::Word(w) => {
                // Drop what the compiler set from inside a command's
                // arguments (`\fontspec{Arial}` → "Arial"), together with
                // the space that separated it from the previous word.
                w.segments.retain(|seg| !seg.chars.first().is_some_and(|c| in_command(scans, c.document.0, c.start)));
                if w.segments.is_empty() {
                    if matches!(items.last(), Some(Item::Space { .. })) {
                        items.pop();
                    }
                    continue;
                }
                for seg in w.segments.iter_mut() {
                    restyle(&mut seg.style, seg.chars.first().map(|c| (c.document.0, c.start)), scans);
                }
            }
            Item::Space { style, .. } | Item::Quad { style, .. } | Item::HFill { style, .. } | Item::Logo { style, .. } | Item::Rule { style, .. } | Item::QedBox { style, .. } | Item::SpaceBox { style } | Item::Kern { style, .. } => {
                // A space carries no source position of its own; it takes
                // the named family of the word before it, which is what
                // TeX's interword glue does (the font in force at the
                // space is the last word's).
                if let Some(Item::Word(w)) = items.last() {
                    if let Some(seg) = w.segments.last() {
                        if seg.style.named.is_some() && style.family == seg.style.family {
                            style.named = seg.style.named;
                        }
                    }
                }
            }
            Item::Footnote { text: Some(t), .. } | Item::Marginpar { text: t, .. } | Item::Lap { items: t } => walk_items(t, scans),
            Item::ColorBox(b) => walk_items(&mut b.items, scans),
            Item::Underline(u) => walk_items(&mut u.items, scans),
            Item::TextScript(t) => walk_items(&mut t.items, scans),
            Item::Table(t) => {
                for entry in t.entries.iter_mut() {
                    if let crate::table::TableEntry::Row { cells, .. } = entry {
                        for cell in cells.iter_mut() {
                            walk_items(&mut cell.items, scans);
                        }
                    }
                }
            }
            _ => {}
        }
        items.push(item);
    }
}

fn restyle(style: &mut TextStyle, at: Option<(usize, usize)>, scans: &[DocScan]) {
    let Some((d, at)) = at else { return };
    if let Some(id) = scope_at(scans, d, at, style.family) {
        style.named = Some(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_read_with_either_option_order() {
        let text = "\\setmainfont[Scale=0.9]{Helvetica}\n\\setsansfont{Arial}[BoldFont={Arial Black}]\n% \\setmonofont{Comment}\n\\newfontfamily\\georgia[Numbers=OldStyle]{Georgia}\n\\begin{document}\n{\\fontspec{Times New Roman} x}\n\\end{document}";
        let c = commands(text);
        let names: Vec<(Kind, &str, &str)> = c.iter().map(|c| (c.kind, c.arg.as_str(), c.options.as_str())).collect();
        assert_eq!(
            names,
            vec![
                (Kind::SetMain, "Helvetica", "Scale=0.9"),
                (Kind::SetSans, "Arial", "BoldFont={Arial Black}"),
                (Kind::NewFamily, "Georgia", "Numbers=OldStyle"),
                (Kind::FontSpec, "Times New Roman", ""),
            ]
        );
        assert_eq!(c[2].switch.as_deref(), Some("georgia"));
        assert_eq!(&text[c[0].start..c[0].end], "\\setmainfont[Scale=0.9]{Helvetica}");
        assert_eq!(body_start(text), text.find("\\begin{document}").unwrap());
    }

    #[test]
    fn options_parse_the_honoured_keys_and_list_the_rest() {
        let o = Options::parse("Scale=MatchLowercase, BoldFont = {Helvetica Neue Bold},Ligatures={TeX,Common},Numbers=OldStyle,Color=FF0000, Extension=.otf");
        assert_eq!(o.scale, Some(Scale::MatchLowercase));
        assert_eq!(o.bold.as_deref(), Some("Helvetica Neue Bold"));
        assert_eq!(o.tex_ligatures, Some(true));
        assert_eq!(o.oldstyle, Some(true));
        assert_eq!(o.ignored, vec!["Color=FF0000", "Extension=.otf"]);
        let o = Options::parse("Scale=1.2,Ligatures=NoCommon");
        assert_eq!(o.scale, Some(Scale::Factor(1.2)));
        assert_eq!(o.ignored, vec!["Ligatures=NoCommon"]);
        assert_eq!(Options::parse("Scale=big").ignored, vec!["Scale=big"]);
    }

    #[test]
    fn scopes_end_at_the_enclosing_group() {
        let text = "a{b\\fontspec{X} c {d} e}f\\begin{quote}\\fontspec{Y} g % }\n h\\end{quote} i";
        let x = text.find("\\fontspec{X}").unwrap() + "\\fontspec{X}".len();
        assert_eq!(&text[group_end(text, x)..], "}f\\begin{quote}\\fontspec{Y} g % }\n h\\end{quote} i");
        let y = text.find("\\fontspec{Y}").unwrap() + "\\fontspec{Y}".len();
        assert_eq!(&text[group_end(text, y)..], "\\end{quote} i");
        assert_eq!(group_end("\\cmd runs to the end", 4), "\\cmd runs to the end".len());
    }

    #[test]
    fn a_document_scan_sets_slots_scopes_and_switch_uses() {
        let text = "\\documentclass{article}\n\\defaultfontfeatures{Ligatures=TeX}\n\\setmainfont{Helvetica}\n\\newfontfamily\\georgia{Georgia}\n\\begin{document}\nA {\\georgia B} C {\\setmainfont{Arial} D \\textsf{E}}\n\\addfontfeature{Scale=2}\n\\end{document}\n";
        let mut settings = Settings::default();
        let mut lim = Vec::new();
        let s = scan(text, true, &mut settings, &mut lim, 0);
        // The fourth is `\addfontfeature`'s scaled Helvetica: a spec of its own.
        assert_eq!(settings.families.iter().map(|f| f.family.as_str()).collect::<Vec<_>>(), vec!["Helvetica", "Georgia", "Arial", "Helvetica"]);
        assert_eq!(settings.text, Some(0));
        assert!(settings.families.iter().all(|f| f.tex_ligatures));
        let at = |needle: &str| text.find(needle).unwrap();
        assert_eq!(scope_at(&[s], 0, at("B}"), FamilyKind::Rm), Some(1));
        // `\setmainfont` in the body: roman only, to the group's end.
        assert_eq!(scope_at(std::slice::from_ref(&scan(text, true, &mut Settings::default(), &mut Vec::new(), 0)), 0, at("D \\textsf"), FamilyKind::Rm), Some(2));
        assert_eq!(scope_at(std::slice::from_ref(&scan(text, true, &mut Settings::default(), &mut Vec::new(), 0)), 0, at("E}}"), FamilyKind::Sf), None);
        // Outside every group: no scope (the slot default applies).
        let s2 = scan(text, true, &mut Settings::default(), &mut Vec::new(), 0);
        assert_eq!(scope_at(std::slice::from_ref(&s2), 0, at("C {"), FamilyKind::Rm), None);
        assert_eq!(s2.switch_uses.len(), 1);
        // `\addfontfeature` at the top level of the body re-selects the main
        // font, scaled, to the end of the document.
        let mut settings = Settings::default();
        let s3 = scan(text, true, &mut settings, &mut Vec::new(), 0);
        let id = scope_at(std::slice::from_ref(&s3), 0, at("\\end{document}") - 1, FamilyKind::Rm).unwrap();
        assert_eq!(settings.families[usize::from(id)].scale, Scale::Factor(2.0));
        assert!(lim.is_empty(), "{lim:?}");
    }
}
