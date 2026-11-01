//! microtype configuration: parsing `.cfg` files and resolving, for one NFSS
//! font, the `\lpcode` / `\rpcode` / `\efcode` tables and `\pdffontexpand`
//! limits that microtype hands to pdfTeX.
//!
//! Semantics follow microtype v3.2d (2026/03/01), files under
//! `texmf-dist/tex/latex/microtype/`:
//! * font-set membership — `microtype.sty` `\MT@maybe@do` (l. 809),
//!   `\MT@checklist@` (l. 833), `\MT@checklist@family` (l. 847),
//!   `\MT@checklist@font` (l. 881), `\MT@get@highlevel` (l. 2168);
//! * list lookup — `\MT@get@listname` / `\MT@try@order` / `\MT@next@listname`
//!   (l. 1573–1623), keys built by `\MT@permute` (l. 2680–2740);
//! * `load=` chains — `\MT@load@list` (l. 1486);
//! * protrusion codes — `microtype-pdftex.def` `\MT@pr@split@val` (l. 315),
//!   `\MT@scale@to@em` (l. 338), `\MT@get@charwd` (l. 346), `\MT@scale`
//!   (`microtype.sty` l. 412), `\MT@scale@factor` (l. 915), limits −1000/1000
//!   (l. 223–224);
//! * expansion — `\MT@set@ex@codes@s/@n` (`microtype-pdftex.def` l. 395–426),
//!   `\MT@ex@split@val` (l. 441), defaults stretch 20, shrink = stretch,
//!   step 1 on pdfTeX >= 1.40 (l. 1404–1413), autoexpand (l. 1449),
//!   factor 1000 (`microtype.sty` l. 209–229), `\MT@ex@min`/`max` 0/1000;
//! * character inheritance — `\MT@set@pr@heirs` (l. 1036), `\MT@set@ex@heirs`;
//! * slot names — LaTeX `t1enc.def` (`\DeclareTextSymbol`, l. 112–167, and
//!   `\DeclareTextComposite`, l. 168–278).
//!
//! The bundled data files `data/microtype.cfg` and `data/mt-cmr.cfg` are
//! unmodified copies from TeX Live 2026 (LPPL 1.3c, which permits unmodified
//! redistribution); SHA-256 `38ed6ed6…5e1258` and `302d0751…b30a`.
//!
//! Not modelled (reported through [`ResolveError`] rather than guessed):
//! size-restricted sets/lists, `unit=` other than character width, `preset=`,
//! `context=`, `font=` wildcards, non-autoexpand expansion, encodings other
//! than T1 for named slots (plain ASCII/number slots work for any encoding).

use std::collections::BTreeMap;

use crate::arith::{Scaled, numexpr_scale};
use crate::pdftex::{ExpansionLimits, FontParams};

/// `data/microtype.cfg` (microtype v3.2d main configuration).
pub const MICROTYPE_CFG: &str = include_str!("../data/microtype.cfg");
/// `data/mt-cmr.cfg` (Computer Modern Roman, also used for `lmr` via alias).
pub const MT_CMR_CFG: &str = include_str!("../data/mt-cmr.cfg");

/// The two microtype features that change pdfTeX's line breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Feature {
    Protrusion,
    Expansion,
}

impl Feature {
    fn from_key(s: &str) -> Option<Feature> {
        match s.trim() {
            "protrusion" | "pr" => Some(Feature::Protrusion),
            "expansion" | "ex" => Some(Feature::Expansion),
            _ => None,
        }
    }
}

/// Which list family a key map indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ListKind {
    Codes(Feature),
    Inheritance(Feature),
}

/// A font specification as written in `\SetProtrusion{...}`, `\DeclareMicrotypeSet{...}`.
/// An empty axis is unconstrained.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FontSpec {
    pub encoding: Vec<String>,
    pub family: Vec<String>,
    pub series: Vec<String>,
    pub shape: Vec<String>,
    pub size: Vec<String>,
    pub font: Vec<String>,
    pub context: Option<String>,
}

/// One `\SetProtrusion` / `\SetExpansion` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeList {
    pub name: String,
    /// Remaining `[...]` options (`load`, `factor`, `unit`, `stretch`, ...).
    pub options: BTreeMap<String, String>,
    /// `(character key, value)` in file order.
    pub entries: Vec<(String, String)>,
}

/// One `\DeclareCharacterInheritance` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InheritanceList {
    pub id: String,
    /// `(base character key, heir keys)`.
    pub entries: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone)]
struct KeyDef {
    kind: ListKind,
    spec: FontSpec,
    name: String,
}

/// Parsed microtype configuration (one or more `.cfg` files, later files
/// overriding earlier definitions, as `\input` order does in TeX).
#[derive(Debug, Clone, Default)]
pub struct MicrotypeConfig {
    /// `\DeclareMicrotypeSet[features]{name}{spec}`; keyed per feature.
    pub sets: BTreeMap<(Feature, String), FontSpec>,
    /// `\DeclareMicrotypeSetDefault[feature]{name}`.
    pub set_defaults: BTreeMap<Feature, String>,
    /// `\DeclareMicrotypeAlias{family}{alias}` (last declaration wins).
    pub aliases: BTreeMap<String, String>,
    /// Code lists by feature and name.
    pub lists: BTreeMap<(Feature, String), CodeList>,
    /// Inheritance declarations by id.
    pub inheritance: BTreeMap<String, InheritanceList>,
    key_defs: Vec<KeyDef>,
    inh_counter: usize,
}

/// The NFSS `\...default` macros microtype's `name*` syntax refers to
/// (`rm*` → `\rmdefault`, `md*` → `\mddefault`, `family*` → `\familydefault`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfssDefaults(pub BTreeMap<String, String>);

impl NfssDefaults {
    /// LaTeX 2e defaults (`fonttext.cfg`/`ltfssini.dtx`) with the given
    /// roman/sans/typewriter families (`cmr`/`cmss`/`cmtt` in a plain T1
    /// article; `lmr`/`lmss`/`lmtt` with `lmodern.sty`).
    pub fn latex(encoding: &str, rm: &str, sf: &str, tt: &str) -> Self {
        let mut m = BTreeMap::new();
        for (k, v) in [
            ("rm", rm),
            ("sf", sf),
            ("tt", tt),
            ("family", rm),
            ("md", "m"),
            ("bf", "bx"),
            ("series", "m"),
            ("up", "n"),
            ("it", "it"),
            ("sl", "sl"),
            ("sc", "sc"),
            ("shape", "n"),
            ("encoding", encoding),
        ] {
            m.insert(k.to_string(), v.to_string());
        }
        NfssDefaults(m)
    }
}

/// An NFSS font: `\T1/cmr/m/n/10.95` is `NfssFont::parse("T1/cmr/m/n/10.95")`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfssFont {
    pub encoding: String,
    pub family: String,
    pub series: String,
    pub shape: String,
    pub size: String,
}

impl NfssFont {
    pub fn parse(name: &str) -> Option<NfssFont> {
        let name = name.trim_start_matches('\\');
        let mut it = name.split('/');
        let f = NfssFont {
            encoding: it.next()?.to_string(),
            family: it.next()?.to_string(),
            series: it.next()?.to_string(),
            shape: it.next()?.to_string(),
            size: it.next()?.to_string(),
        };
        it.next().is_none().then_some(f)
    }

    fn full(&self) -> String {
        format!("{}/{}/{}/{}/{}", self.encoding, self.family, self.series, self.shape, self.size)
    }
}

/// `\usepackage[...]{microtype}` options that matter for protrusion/expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub protrusion: bool,
    pub expansion: bool,
    /// Font set name; `None` = the `\DeclareMicrotypeSetDefault`.
    pub protrusion_set: Option<String>,
    pub expansion_set: Option<String>,
    pub protrusion_factor: i32,
    pub expansion_factor: i32,
    pub stretch: i32,
    pub shrink: i32,
    pub step: i32,
    pub auto_expand: bool,
    pub selected: bool,
}

impl Default for Options {
    /// `\usepackage{microtype}` under pdfTeX >= 1.40 in PDF mode: protrusion
    /// and expansion level 2, stretch 20, shrink 20, step 1, autoexpand,
    /// non-selected (verified against the pdflatex log line "stretch: 20,
    /// shrink: 20, step: 1, non-selected").
    fn default() -> Self {
        Options {
            protrusion: true,
            expansion: true,
            protrusion_set: None,
            expansion_set: None,
            protrusion_factor: 1000,
            expansion_factor: 1000,
            stretch: 20,
            shrink: 20,
            step: 1,
            auto_expand: true,
            selected: false,
        }
    }
}

/// Character widths and `\fontdimen6` of the font being set up, in sp.
pub trait FontMetrics {
    /// `\fontcharwd` (0 for a missing character).
    fn char_width(&self, slot: u8) -> Scaled;
    /// `\fontdimen6`.
    fn quad(&self) -> Scaled;
}

/// Something microtype would do that this crate does not model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    UnknownSet(Feature, String),
    SizeConstraint(String),
    Unsupported(String),
    LoadCycle(String),
    UndefinedList(String),
}

/// The pdfTeX-visible result of microtype's font setup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedFont {
    pub params: FontParams,
    /// Protrusion list applied (`None`: font not in the set, or no list).
    pub protrusion_list: Option<String>,
    /// Expansion list consulted (`None` also for "no list" in non-selected mode).
    pub expansion_list: Option<String>,
    /// Keys microtype would warn about and skip (unknown slot names, ...).
    pub warnings: Vec<String>,
}

impl MicrotypeConfig {
    /// `microtype.cfg` followed by `mt-cmr.cfg`, as loaded for a CM/LM document.
    pub fn bundled() -> MicrotypeConfig {
        MicrotypeConfig::parse(&[MICROTYPE_CFG, MT_CMR_CFG])
    }

    /// Parse configuration files in load order.
    pub fn parse(sources: &[&str]) -> MicrotypeConfig {
        let mut cfg = MicrotypeConfig::default();
        for src in sources {
            cfg.parse_one(src);
        }
        cfg
    }

    fn parse_one(&mut self, src: &str) {
        let text = strip_comments(src);
        let b = text.as_bytes();
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'\\' => {
                    let (name, next) = control_sequence(&text, i);
                    i = next;
                    match name.as_str() {
                        "SetProtrusion" | "SetExpansion" => {
                            let feat = if name == "SetProtrusion" {
                                Feature::Protrusion
                            } else {
                                Feature::Expansion
                            };
                            let (opt, j) = optional_arg(&text, i);
                            let Some((spec, j)) = group_arg(&text, j) else { continue };
                            let Some((codes, j)) = group_arg(&text, j) else { continue };
                            i = j;
                            self.add_list(feat, opt.unwrap_or_default(), &spec, &codes);
                        }
                        "DeclareCharacterInheritance" => {
                            let (opt, j) = optional_arg(&text, i);
                            let Some((spec, j)) = group_arg(&text, j) else { continue };
                            let Some((list, j)) = group_arg(&text, j) else { continue };
                            i = j;
                            self.add_inheritance(opt.unwrap_or_default(), &spec, &list);
                        }
                        "DeclareMicrotypeSet" => {
                            let j = skip_star(&text, i);
                            let (opt, j) = optional_arg(&text, j);
                            let Some((set, j)) = group_arg(&text, j) else { continue };
                            let Some((spec, j)) = group_arg(&text, j) else { continue };
                            i = j;
                            let spec = parse_spec(&spec);
                            for f in features_of(opt.as_deref()) {
                                self.sets.insert((f, set.trim().to_string()), spec.clone());
                            }
                        }
                        "DeclareMicrotypeSetDefault" => {
                            let (opt, j) = optional_arg(&text, i);
                            let Some((set, j)) = group_arg(&text, j) else { continue };
                            i = j;
                            for f in features_of(opt.as_deref()) {
                                self.set_defaults.insert(f, set.trim().to_string());
                            }
                        }
                        "DeclareMicrotypeAlias" => {
                            let Some((fam, j)) = group_arg(&text, i) else { continue };
                            let Some((alias, j)) = group_arg(&text, j) else { continue };
                            i = j;
                            self.aliases.insert(fam.trim().to_string(), alias.trim().to_string());
                        }
                        _ => {}
                    }
                }
                b'{' => i = skip_group(b, i),
                _ => i += 1,
            }
        }
    }

    fn add_list(&mut self, feat: Feature, opt: String, spec: &str, codes: &str) {
        let mut options = BTreeMap::new();
        for item in split_top(&opt, b',') {
            if let Some((k, v)) = split_kv(&item) {
                options.insert(k, strip_braces(&v).to_string());
            }
        }
        let Some(name) = options.remove("name") else { return };
        let mut entries = Vec::new();
        for item in split_top(codes, b',') {
            if let Some((k, v)) = split_kv(&item) {
                entries.push((k, strip_braces(&v).to_string()));
            }
        }
        let mut spec = parse_spec(spec);
        spec.context = options.get("context").cloned();
        self.key_defs.push(KeyDef {
            kind: ListKind::Codes(feat),
            spec,
            name: name.clone(),
        });
        self.lists.insert((feat, name.clone()), CodeList { name, options, entries });
    }

    fn add_inheritance(&mut self, opt: String, spec: &str, list: &str) {
        self.inh_counter += 1;
        let id = format!("inheritance#{}", self.inh_counter);
        let mut entries = Vec::new();
        for item in split_top(list, b',') {
            if let Some((k, v)) = split_kv(&item) {
                let heirs = split_top(strip_braces(&v), b',')
                    .into_iter()
                    .map(|h| h.trim().to_string())
                    .filter(|h| !h.is_empty())
                    .collect();
                entries.push((k, heirs));
            }
        }
        let feats: Vec<Feature> = split_top(&opt, b',')
            .iter()
            .filter_map(|f| Feature::from_key(f))
            .collect();
        let feats = if opt.trim().is_empty() {
            vec![Feature::Protrusion, Feature::Expansion]
        } else {
            feats
        };
        let spec = parse_spec(spec);
        for f in feats {
            self.key_defs.push(KeyDef {
                kind: ListKind::Inheritance(f),
                spec: spec.clone(),
                name: id.clone(),
            });
        }
        self.inheritance.insert(id.clone(), InheritanceList { id, entries });
    }

    /// `\MT@permute`: every `enc/fam/ser/sha/` key a definition registers.
    fn key_map(&self, kind: ListKind, d: &NfssDefaults) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for def in self.key_defs.iter().filter(|k| k.kind == kind) {
            if !def.spec.size.is_empty() || !def.spec.font.is_empty() || def.spec.context.is_some() {
                // size-restricted keys carry a `*` and a size list; font= keys
                // and contexts never match the empty context used here.
                continue;
            }
            let one = |v: &Vec<String>, axis: &str| -> Vec<String> {
                if v.is_empty() {
                    vec![String::new()]
                } else {
                    v.iter().filter_map(|x| highlevel(x, axis, d)).collect()
                }
            };
            for e in one(&def.spec.encoding, "encoding") {
                for f in one(&def.spec.family, "family") {
                    for s in one(&def.spec.series, "series") {
                        for h in one(&def.spec.shape, "shape") {
                            let key = format!("{e}/{f}/{s}/{h}/");
                            if key == "////" || e.is_empty() {
                                continue; // "You have to specify an encoding"
                            }
                            map.insert(key, def.name.clone());
                        }
                    }
                }
            }
        }
        map
    }

    /// `\MT@get@listname`: the list microtype picks for `font`.
    fn find_list(&self, kind: ListKind, font: &NfssFont, d: &NfssDefaults) -> Option<String> {
        let keys = self.key_map(kind, d);
        let alias = self.aliases.get(&font.family);
        for code in [
            "1111", "1110", "1101", "1100", "1011", "1010", "1001", "1000", "0111", "0110", "0101",
            "0100", "0011", "0010", "0001", "0000",
        ] {
            let c = code.as_bytes();
            if c[3] == b'1' {
                continue; // size-specific keys are never registered (see key_map)
            }
            let ser = if c[1] == b'1' { font.series.as_str() } else { "" };
            let sha = if c[2] == b'1' { font.shape.as_str() } else { "" };
            let fam = if c[0] == b'1' { font.family.as_str() } else { "" };
            let key = format!("{}/{}/{}/{}/", font.encoding, fam, ser, sha);
            if let Some(n) = keys.get(&key) {
                return Some(n.clone());
            }
            if c[0] == b'1' {
                if let Some(a) = alias {
                    let key = format!("{}/{}/{}/{}/", font.encoding, a, ser, sha);
                    if let Some(n) = keys.get(&key) {
                        return Some(n.clone());
                    }
                }
            }
        }
        None
    }

    /// `\MT@maybe@do`: is `font` in the feature's font set?
    fn in_set(
        &self,
        feat: Feature,
        set: &str,
        font: &NfssFont,
        d: &NfssDefaults,
    ) -> Result<bool, ResolveError> {
        let spec = self
            .sets
            .get(&(feat, set.to_string()))
            .ok_or_else(|| ResolveError::UnknownSet(feat, set.to_string()))?;
        if !spec.size.is_empty() {
            return Err(ResolveError::SizeConstraint(set.to_string()));
        }
        let mut doit = true;
        if !spec.font.is_empty() {
            if spec.font.iter().any(|f| f.contains('*')) {
                return Err(ResolveError::Unsupported(format!("font wildcard in set {set}")));
            }
            if spec.font.contains(&font.full()) {
                return Ok(true); // \MT@clist@break with do=true
            }
            doit = false;
        }
        let alias = self.aliases.get(&font.family);
        for (axis, vals, value) in [
            ("encoding", &spec.encoding, &font.encoding),
            ("family", &spec.family, &font.family),
            ("series", &spec.series, &font.series),
            ("shape", &spec.shape, &font.shape),
        ] {
            if vals.is_empty() {
                continue;
            }
            let list: Vec<String> = vals.iter().filter_map(|v| highlevel(v, axis, d)).collect();
            let mut hit = list.contains(value);
            if !hit && axis == "family" {
                hit = alias.is_some_and(|a| list.contains(a));
            }
            if hit {
                doit = true;
            } else {
                return Ok(false);
            }
        }
        Ok(doit)
    }

    /// Resolve everything pdfTeX will see for `font` (microtype's
    /// `\MT@setupfont` for the protrusion and expansion features).
    pub fn resolve(
        &self,
        options: &Options,
        defaults: &NfssDefaults,
        font: &NfssFont,
        metrics: &dyn FontMetrics,
    ) -> Result<ResolvedFont, ResolveError> {
        let mut params = FontParams::plain(metrics.quad());
        let mut warnings = Vec::new();
        let mut protrusion_list = None;
        let mut expansion_list = None;

        if options.protrusion {
            let set = self.set_name(Feature::Protrusion, options.protrusion_set.as_deref())?;
            if self.in_set(Feature::Protrusion, &set, font, defaults)? {
                if let Some(name) = self.find_list(ListKind::Codes(Feature::Protrusion), font, defaults) {
                    self.apply_protrusion(&name, options, defaults, font, metrics, &mut params, &mut warnings)?;
                    protrusion_list = Some(name);
                }
            }
        }

        if options.expansion {
            let set = self.set_name(Feature::Expansion, options.expansion_set.as_deref())?;
            if self.in_set(Feature::Expansion, &set, font, defaults)? {
                if !options.auto_expand {
                    return Err(ResolveError::Unsupported("expansion without autoexpand".into()));
                }
                let found = self.find_list(ListKind::Codes(Feature::Expansion), font, defaults);
                let (mut stretch, mut shrink, mut step) = (options.stretch, options.shrink, options.step);
                let mut factor = options.expansion_factor;
                if let Some(name) = &found {
                    let list = &self.lists[&(Feature::Expansion, name.clone())];
                    let int = |k: &str, dflt: i32| {
                        list.options.get(k).and_then(|v| v.trim().parse().ok()).unwrap_or(dflt)
                    };
                    stretch = int("stretch", stretch);
                    shrink = int("shrink", shrink);
                    step = int("step", step);
                    factor = int("factor", factor);
                    if list.options.contains_key("preset") {
                        return Err(ResolveError::Unsupported(format!("preset in list {name}")));
                    }
                    if list.options.get("auto").is_some_and(|v| v.trim() == "false") {
                        return Err(ResolveError::Unsupported("expansion without autoexpand".into()));
                    }
                }
                // \MT@reset@ef@codes
                if factor != 1000 {
                    for c in 0..=255u8 {
                        params.set_efcode(c, factor);
                    }
                }
                if options.selected {
                    let Some(name) = &found else {
                        return Err(ResolveError::UndefinedList("selected expansion without a list".into()));
                    };
                    let inh = self.heirs(Feature::Expansion, font, defaults, &mut warnings);
                    for list in self.load_chain(Feature::Expansion, name)? {
                        for (key, value) in &list.entries {
                            let Some(slot) = slot_for(&font.encoding, key) else {
                                warnings.push(format!("unknown character `{key}' in {}", list.name));
                                continue;
                            };
                            let Ok(mut v) = value.trim().parse::<i32>() else { continue };
                            if factor != 1000 {
                                v = numexpr_scale(v, factor, 1000);
                            }
                            v = v.clamp(0, 1000);
                            params.set_efcode(slot, v);
                            for &h in inh.get(&slot).map(Vec::as_slice).unwrap_or(&[]) {
                                params.efcode[h as usize] = params.efcode[slot as usize];
                            }
                        }
                    }
                }
                params.expansion = ExpansionLimits::from_primitive(stretch, shrink, step);
                expansion_list = found;
            }
        }

        Ok(ResolvedFont {
            params,
            protrusion_list,
            expansion_list,
            warnings,
        })
    }

    fn set_name(&self, feat: Feature, explicit: Option<&str>) -> Result<String, ResolveError> {
        match explicit {
            Some(s) => Ok(s.to_string()),
            None => self
                .set_defaults
                .get(&feat)
                .cloned()
                .ok_or_else(|| ResolveError::UnknownSet(feat, "<default>".into())),
        }
    }

    /// `\MT@load@list`: the list and everything it loads, outermost last.
    fn load_chain(&self, feat: Feature, name: &str) -> Result<Vec<&CodeList>, ResolveError> {
        let mut chain = Vec::new();
        let mut cur = name.to_string();
        loop {
            let list = self
                .lists
                .get(&(feat, cur.clone()))
                .ok_or_else(|| ResolveError::UndefinedList(cur.clone()))?;
            if chain.iter().any(|l: &&CodeList| l.name == list.name) {
                return Err(ResolveError::LoadCycle(cur));
            }
            chain.push(list);
            match list.options.get("load") {
                Some(parent) if !parent.trim().is_empty() => cur = parent.trim().to_string(),
                _ => break,
            }
        }
        chain.reverse();
        Ok(chain)
    }

    /// `\MT@get@inh@list`: base slot → heir slots for this font.
    fn heirs(
        &self,
        feat: Feature,
        font: &NfssFont,
        d: &NfssDefaults,
        warnings: &mut Vec<String>,
    ) -> BTreeMap<u8, Vec<u8>> {
        let mut out: BTreeMap<u8, Vec<u8>> = BTreeMap::new();
        let Some(id) = self.find_list(ListKind::Inheritance(feat), font, d) else {
            return out;
        };
        for (base, heirs) in &self.inheritance[&id].entries {
            let Some(b) = slot_for(&font.encoding, base) else {
                warnings.push(format!("unknown inheritance base `{base}'"));
                continue;
            };
            for h in heirs {
                match slot_for(&font.encoding, h) {
                    Some(s) => out.entry(b).or_default().push(s),
                    None => warnings.push(format!("unknown heir `{h}'")),
                }
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_protrusion(
        &self,
        name: &str,
        options: &Options,
        d: &NfssDefaults,
        font: &NfssFont,
        metrics: &dyn FontMetrics,
        params: &mut FontParams,
        warnings: &mut Vec<String>,
    ) -> Result<(), ResolveError> {
        let top = &self.lists[&(Feature::Protrusion, name.to_string())];
        // \MT@get@opt reads factor/unit/preset from the list that was found
        let factor = match top.options.get("factor") {
            Some(v) => v.trim().parse().unwrap_or(options.protrusion_factor),
            None => options.protrusion_factor,
        };
        if top.options.get("unit").is_some_and(|u| !u.trim().is_empty() && u.trim() != "character") {
            return Err(ResolveError::Unsupported(format!("unit in protrusion list {name}")));
        }
        if top.options.contains_key("preset") {
            return Err(ResolveError::Unsupported(format!("preset in protrusion list {name}")));
        }
        let inh = self.heirs(Feature::Protrusion, font, d, warnings);
        let quad = metrics.quad();
        for list in self.load_chain(Feature::Protrusion, name)? {
            for (key, value) in &list.entries {
                let Some(slot) = slot_for(&font.encoding, key) else {
                    warnings.push(format!("unknown character `{key}' in {}", list.name));
                    continue;
                };
                let (l, r) = match value.split_once(',') {
                    Some((l, r)) => (l.trim(), r.trim()),
                    None => (value.trim(), ""),
                };
                let width = metrics.char_width(slot);
                let code = |v: &str| -> Option<i32> {
                    let v: i32 = v.parse().ok()?;
                    // \MT@scale@to@em: \@tempcntb = charwd * value / \fontdimen6
                    let mut c = numexpr_scale(width, v, quad);
                    if c != 0 && factor != 1000 {
                        c = numexpr_scale(c, factor, 1000);
                    }
                    Some(c.clamp(-1000, 1000))
                };
                if !l.is_empty() {
                    if let Some(c) = code(l) {
                        params.set_lpcode(slot, c);
                    }
                }
                if !r.is_empty() {
                    if let Some(c) = code(r) {
                        params.set_rpcode(slot, c);
                    }
                }
                for &h in inh.get(&slot).map(Vec::as_slice).unwrap_or(&[]) {
                    params.lpcode[h as usize] = params.lpcode[slot as usize];
                    params.rpcode[h as usize] = params.rpcode[slot as usize];
                }
            }
        }
        Ok(())
    }
}

/// `\MT@get@highlevel`: `rm*` → `\rmdefault`, `*` alone → `\<axis>default`.
fn highlevel(v: &str, axis: &str, d: &NfssDefaults) -> Option<String> {
    let v = v.trim();
    match v.strip_suffix('*') {
        Some(prefix) => {
            let key = if prefix.is_empty() { axis } else { prefix };
            d.0.get(key).cloned()
        }
        None => Some(v.to_string()),
    }
}

fn features_of(opt: Option<&str>) -> Vec<Feature> {
    match opt {
        Some(o) if !o.trim().is_empty() => split_top(o, b',').iter().filter_map(|f| Feature::from_key(f)).collect(),
        _ => vec![Feature::Protrusion, Feature::Expansion],
    }
}

fn parse_spec(spec: &str) -> FontSpec {
    let mut s = FontSpec::default();
    for item in split_top(spec, b',') {
        let Some((k, v)) = split_kv(&item) else { continue };
        let vals: Vec<String> = split_top(strip_braces(&v), b',')
            .into_iter()
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect();
        match k.as_str() {
            "encoding" => s.encoding = vals,
            "family" => s.family = vals,
            "series" => s.series = vals,
            "shape" => s.shape = vals,
            "size" => s.size = vals,
            "font" => s.font = vals,
            _ => {}
        }
    }
    s
}

// ---------------------------------------------------------------------------
// Slot names (T1, from LaTeX t1enc.def).

const T1_SYMBOLS: &[(&str, u8)] = &[
    ("AE", 198), ("DH", 208), ("DJ", 208), ("L", 138), ("NG", 141), ("OE", 215), ("O", 216),
    ("SS", 223), ("TH", 222), ("ae", 230), ("dh", 240), ("dj", 158), ("guillemetleft", 19),
    ("guillemetright", 20), ("guillemotleft", 19), ("guillemotright", 20), ("guilsinglleft", 14),
    ("guilsinglright", 15), ("i", 25), ("j", 26), ("ij", 188), ("IJ", 156), ("l", 170), ("ng", 173),
    ("oe", 247), ("o", 248), ("quotedblbase", 18), ("quotesinglbase", 13), ("ss", 255),
    ("textasciicircum", b'^'), ("textasciitilde", b'~'), ("textbackslash", b'\\'), ("textbar", b'|'),
    ("textbraceleft", b'{'), ("textbraceright", b'}'), ("textcompwordmark", 23), ("textdollar", b'$'),
    ("textemdash", 22), ("textendash", 21), ("textexclamdown", 189), ("textgreater", b'>'),
    ("textless", b'<'), ("textquestiondown", 190), ("textquotedblleft", 16), ("textquotedblright", 17),
    ("textquotedbl", b'"'), ("textquoteleft", b'`'), ("textquoteright", b'\''), ("textsection", 159),
    ("textsterling", 191), ("textunderscore", 95), ("textvisiblespace", 32), ("th", 254),
    // control symbols whose meaning is \char"XX (\MT@is@char)
    ("%", b'%'), ("&", b'&'), ("#", b'#'), ("$", b'$'), ("_", 95), ("{", b'{'), ("}", b'}'),
];

const T1_COMPOSITES: &[(&str, &str, u8)] = &[
    (".", "i", b'i'), (".", "\\i", b'i'), ("u", "A", 128), ("k", "A", 129), ("'", "C", 130),
    ("v", "C", 131), ("v", "D", 132), ("v", "E", 133), ("k", "E", 134), ("u", "G", 135),
    ("'", "L", 136), ("v", "L", 137), ("'", "N", 139), ("v", "N", 140), ("H", "O", 142),
    ("'", "R", 143), ("v", "R", 144), ("'", "S", 145), ("v", "S", 146), ("c", "S", 147),
    ("v", "T", 148), ("c", "T", 149), ("H", "U", 150), ("r", "U", 151), ("\"", "Y", 152),
    ("'", "Z", 153), ("v", "Z", 154), (".", "Z", 155), (".", "I", 157), ("u", "a", 160),
    ("k", "a", 161), ("'", "c", 162), ("v", "c", 163), ("v", "d", 164), ("v", "e", 165),
    ("k", "e", 166), ("u", "g", 167), ("'", "l", 168), ("v", "l", 169), ("'", "n", 171),
    ("v", "n", 172), ("H", "o", 174), ("'", "r", 175), ("v", "r", 176), ("'", "s", 177),
    ("v", "s", 178), ("c", "s", 179), ("v", "t", 180), ("c", "t", 181), ("H", "u", 182),
    ("r", "u", 183), ("\"", "y", 184), ("'", "z", 185), ("v", "z", 186), (".", "z", 187),
    ("`", "A", 192), ("'", "A", 193), ("^", "A", 194), ("~", "A", 195), ("\"", "A", 196),
    ("r", "A", 197), ("c", "C", 199), ("`", "E", 200), ("'", "E", 201), ("^", "E", 202),
    ("\"", "E", 203), ("`", "I", 204), ("'", "I", 205), ("^", "I", 206), ("\"", "I", 207),
    ("~", "N", 209), ("`", "O", 210), ("'", "O", 211), ("^", "O", 212), ("~", "O", 213),
    ("\"", "O", 214), ("`", "U", 217), ("'", "U", 218), ("^", "U", 219), ("\"", "U", 220),
    ("'", "Y", 221), ("`", "a", 224), ("'", "a", 225), ("^", "a", 226), ("~", "a", 227),
    ("\"", "a", 228), ("r", "a", 229), ("c", "c", 231), ("`", "e", 232), ("'", "e", 233),
    ("^", "e", 234), ("\"", "e", 235), ("`", "i", 236), ("`", "\\i", 236), ("'", "i", 237),
    ("'", "\\i", 237), ("^", "i", 238), ("^", "\\i", 238), ("\"", "i", 239), ("\"", "\\i", 239),
    ("~", "n", 241), ("`", "o", 242), ("'", "o", 243), ("^", "o", 244), ("~", "o", 245),
    ("\"", "o", 246), ("`", "u", 249), ("'", "u", 250), ("^", "u", 251), ("\"", "u", 252),
    ("'", "y", 253),
];

/// `\MT@get@slot`: the slot a configuration key names in `encoding`.
pub fn slot_for(encoding: &str, key: &str) -> Option<u8> {
    let k = key.trim();
    let k = strip_braces(k).trim();
    if let Some(rest) = k.strip_prefix('\\') {
        let word = rest.bytes().take_while(|c| c.is_ascii_alphabetic()).count();
        let (name, tail) = if word > 0 {
            (&rest[..word], rest[word..].trim())
        } else {
            let n = rest.chars().next()?.len_utf8();
            (&rest[..n], rest[n..].trim())
        };
        if encoding != "T1" && encoding != "LY1" {
            // only the ASCII-compatible control symbols are encoding-independent
            return if tail.is_empty() && word == 0 {
                T1_SYMBOLS.iter().find(|(n, _)| *n == name).map(|&(_, s)| s)
            } else {
                None
            };
        }
        if tail.is_empty() {
            T1_SYMBOLS.iter().find(|(n, _)| *n == name).map(|&(_, s)| s)
        } else {
            let base = strip_braces(tail).trim();
            T1_COMPOSITES.iter().find(|(a, b, _)| *a == name && *b == base).map(|&(_, _, s)| s)
        }
    } else {
        let chars: Vec<char> = k.chars().collect();
        match chars.as_slice() {
            [c] if (*c as u32) < 128 => Some(*c as u8),
            ['"', a, b] => u8::from_str_radix(&format!("{a}{b}").to_ascii_uppercase(), 16).ok(),
            ['\'', a, b] => u8::from_str_radix(&format!("{a}{b}"), 8).ok(),
            [a, b, c] if a.is_ascii_digit() && b.is_ascii_digit() && c.is_ascii_digit() => {
                format!("{a}{b}{c}").parse::<u16>().ok().filter(|v| *v <= 255).map(|v| v as u8)
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Minimal TeX-ish lexing for configuration files.

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for line in src.lines() {
        let mut chars = line.chars().peekable();
        let mut commented = false;
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    out.push(c);
                    if let Some(n) = chars.next() {
                        out.push(n);
                    }
                }
                '%' => {
                    commented = true;
                    break;
                }
                _ => out.push(c),
            }
        }
        if !commented {
            out.push(' ');
        }
    }
    out
}

fn control_sequence(text: &str, i: usize) -> (String, usize) {
    let b = text.as_bytes();
    let mut j = i + 1;
    while j < b.len() && (b[j].is_ascii_alphabetic() || b[j] == b'@') {
        j += 1;
    }
    if j == i + 1 && j < b.len() {
        j += 1; // control symbol
    }
    (text[i + 1..j].to_string(), j)
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn skip_star(text: &str, i: usize) -> usize {
    let b = text.as_bytes();
    let j = skip_ws(b, i);
    if j < b.len() && b[j] == b'*' { j + 1 } else { i }
}

fn skip_group(b: &[u8], i: usize) -> usize {
    let mut depth = 0usize;
    let mut j = i;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 1,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return j + 1;
                }
            }
            _ => {}
        }
        j += 1;
    }
    b.len()
}

fn group_arg(text: &str, i: usize) -> Option<(String, usize)> {
    let b = text.as_bytes();
    let j = skip_ws(b, i);
    if j >= b.len() || b[j] != b'{' {
        return None;
    }
    let end = skip_group(b, j);
    Some((text[j + 1..end - 1].to_string(), end))
}

fn optional_arg(text: &str, i: usize) -> (Option<String>, usize) {
    let b = text.as_bytes();
    let j = skip_ws(b, i);
    if j >= b.len() || b[j] != b'[' {
        return (None, i);
    }
    let mut depth = 0usize;
    let mut k = j + 1;
    while k < b.len() {
        match b[k] {
            b'\\' => k += 1,
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b']' if depth == 0 => return (Some(text[j + 1..k].to_string()), k + 1),
            _ => {}
        }
        k += 1;
    }
    (None, i)
}

fn split_top(s: &str, sep: u8) -> Vec<String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            c if c == sep && depth == 0 => {
                out.push(s[start..i].to_string());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    out.push(s[start.min(s.len())..].to_string());
    out.into_iter().filter(|x| !x.trim().is_empty()).collect()
}

fn split_kv(item: &str) -> Option<(String, String)> {
    let b = item.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 1,
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'=' if depth == 0 && i > 0 => {
                return Some((item[..i].trim().to_string(), item[i + 1..].trim().to_string()));
            }
            _ => {}
        }
        i += 1;
    }
    None
}

fn strip_braces(s: &str) -> &str {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('{') && t.ends_with('}') && skip_group(t.as_bytes(), 0) == t.len() {
        &t[1..t.len() - 1]
    } else {
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots() {
        assert_eq!(slot_for("T1", "A"), Some(b'A'));
        assert_eq!(slot_for("T1", "{,}"), Some(b','));
        assert_eq!(slot_for("T1", "\\textquoteright"), Some(39));
        assert_eq!(slot_for("T1", "\\textendash"), Some(21));
        assert_eq!(slot_for("T1", "\\`A"), Some(192));
        assert_eq!(slot_for("T1", "\\r A"), Some(197));
        assert_eq!(slot_for("T1", "\\%"), Some(37));
        assert_eq!(slot_for("T1", "027"), Some(27));
        assert_eq!(slot_for("OT1", "\"0A"), Some(10));
        assert_eq!(slot_for("T1", "156"), Some(156));
    }

    #[test]
    fn bundled_config_structure() {
        let c = MicrotypeConfig::bundled();
        assert_eq!(c.set_defaults[&Feature::Protrusion], "alltext");
        assert_eq!(c.set_defaults[&Feature::Expansion], "alltext-nott");
        assert_eq!(c.aliases["lmr"], "cmr");
        let d = NfssDefaults::latex("T1", "cmr", "cmss", "cmtt");
        let f = |n| NfssFont::parse(n).unwrap();
        let find = |n| c.find_list(ListKind::Codes(Feature::Protrusion), &f(n), &d);
        assert_eq!(find("T1/cmr/m/n/10").as_deref(), Some("cmr-T1"));
        assert_eq!(find("T1/cmr/bx/n/10").as_deref(), Some("cmr-T1"));
        assert_eq!(find("T1/cmr/m/it/10").as_deref(), Some("cmr-it-T1"));
        assert_eq!(find("T1/lmr/m/n/10").as_deref(), Some("lmr-T1"));
        assert_eq!(find("T1/lmr/m/it/10").as_deref(), Some("lmr-it-T1"));
        assert_eq!(find("T1/cmss/m/n/10").as_deref(), Some("T1-default"));
        assert!(c.in_set(Feature::Expansion, "alltext-nott", &f("T1/cmr/m/n/10"), &d).unwrap());
        assert!(!c.in_set(Feature::Expansion, "alltext-nott", &f("T1/cmtt/m/n/10"), &d).unwrap());
        assert!(!c.in_set(Feature::Protrusion, "alltext", &f("OML/cmm/m/it/10"), &d).unwrap());
    }
}
