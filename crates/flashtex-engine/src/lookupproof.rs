//! LOOKUP-SKIP: whether a lookup the run made still finds the same file,
//! checked against the directories it searched instead of made again.
//!
//! S₀'s key (`host::Key::check_files`) and the session's `changes`
//! (`crate::incr`) make a run's lookups again whenever a directory a lookup
//! depends on has a signature other than the one recorded
//! (`system::lookup_again_deps`, #1562). A compile writes its own output
//! directory, and a signature taken within the modification-time tick of
//! a write is racy and equals nothing (#1552), so while the user types
//! every check made all of the preamble's and the body's lookups again in
//! kpathsea: on a small article 88 M of the 134 M instructions before the
//! edited page.
//!
//! A lookup's answer can change only if an entry under a name kpathsea
//! tries for it appears, goes or changes in a directory it searches on
//! disk, or its answer stops being readable. So a lookup made again
//! (`system::lookup_again_deps`) keeps a [`Proof`], for the session: the
//! names kpathsea tries (`LookupDirs::tries`), the directories it searches
//! on disk in order (`LookupDirs::listed`, a `//` subtree as kpathsea
//! expanded it), and the answer. (Not the run's own lookups: making a proof
//! there cost the run more than it saved, and a lookup is made again once
//! before its proof answers.) The check
//! ([`Verifier::lookup_again_deps`]) lists each of those directories as it
//! is now (once per directory, and only when its signature differs from
//! the one its listing was last taken under) and holds when:
//! - no listed directory has an entry under a name tried, except the
//!   answer's own directory, which has exactly the answer, under its exact
//!   name, and the answer is still a readable file
//!   (`kpathsea_readable_file`);
//! - with `-output-directory`, the name is not a file there
//!   (`system::lookup_again` tries that first);
//! - the output directory and the resolver are the ones the proof was made
//!   under. (The working directory need not be: a relative directory is
//!   listed here as kpathsea searches it, in the working directory of
//!   now.)
//!
//! These say what kpathsea finds now, not what changed since the run: in
//! every directory it searches before the answer's there is nothing under
//! the names it tries (case-folded too, as `casefold_readable_file`
//! compares), and in the answer's directory only the answer, so it finds the
//! answer, whatever happened in between. The TeX distribution's `ls-R`
//! trees (`!!`) are taken as unchanged for the session, as kpathsea and the
//! check before this one take them. A lookup whose dependencies are not
//! known, or that searched a directory that does not exist
//! (`LookupDirs::above`), has no proof and is made again, as before. A
//! proof does not depend on when it was made: it states what kpathsea
//! finds in the directories as they are when it is checked.
//!
//! A listing is trusted only under the signature taken just before it was
//! listed, and a racy signature equals nothing (`StatSig`), so a change
//! after the listing is seen by the next check.

use crate::resolver::{FileResolver, Format, LookupDirs};
use crate::system::{self, Lookup, StatSig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// What a lookup's answer depends on (see the module's documentation).
#[derive(Debug)]
struct Proof {
    /// The resolver it was made with ([`resolver_changed`]).
    generation: u64,
    output_directory: Option<String>,
    /// The names kpathsea tries, lower-cased (ASCII), each once.
    tries: Vec<String>,
    /// The directories kpathsea searches on disk, in order.
    listed: Vec<String>,
    /// kpathsea's answer.
    found: Option<String>,
    /// The answer's directory among `listed` (none: found in an `ls-R`
    /// tree, or not found), and its last component, exactly.
    found_in: Option<(usize, String)>,
}

/// A lookup: name, format, `find_ex`'s flag, and whether it is asked of
/// the resolver directly (the format cache, [`Verifier::find_direct`]) rather
/// than as `system::lookup_again` asks it (the output directory first).
type Key = (String, Format, Option<bool>, bool);

static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The process's resolver (`system::with_resolver`'s), by address, while it
/// lives: [`Verifier::find_direct`] keeps proofs only for it, since a proof
/// is about one kpathsea instance's search paths and `ls-R` databases.
static PROCESS_RESOLVER: AtomicUsize = AtomicUsize::new(0);

fn address(r: &dyn FileResolver) -> usize {
    r as *const dyn FileResolver as *const () as usize
}

/// `r` is the process's resolver (`system::with_resolver`), until
/// [`resolver_changed`].
pub fn process_resolver(r: &dyn FileResolver) {
    PROCESS_RESOLVER.store(address(r), Ordering::Relaxed);
}
static PROOFS: Mutex<Option<HashMap<Key, Arc<Proof>>>> = Mutex::new(None);

/// A directory's entries, `(folded, as named)` ([`fold`]), sorted.
type Listing = Arc<Vec<(String, String)>>;

/// Each directory's last listing, with the signature taken just before it.
static LISTINGS: Mutex<Option<HashMap<String, Listed>>> = Mutex::new(None);

/// A directory's signature, and its listing taken just after (`None`: not
/// listable).
type Listed = (StatSig, Option<Listing>);

/// What looking a name up finds, and what that depends on
/// (`system::lookup_again_deps`).
type Answer = (Option<String>, Vec<(String, StatSig)>);

/// The process's resolver was replaced or reset (`system::set_resolver`):
/// no proof made with the old one holds.
pub fn resolver_changed() {
    GENERATION.fetch_add(1, Ordering::Relaxed);
    PROCESS_RESOLVER.store(0, Ordering::Relaxed);
}

fn off() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var("FLASHTEX_LOOKUP_PROOF").is_ok_and(|v| v == "off"))
}

/// Record what the lookup of `name` that found `found` (kpathsea's own
/// answer, before any confinement) depends on, `deps`
/// (`FileResolver::lookup_dirs`, asked just after the lookup). Where that
/// is not known, any proof of the lookup goes.
pub fn note(
    name: &str,
    format: Format,
    must_exist: Option<bool>,
    found: Option<&str>,
    deps: &Result<LookupDirs, &'static str>,
    output_directory: Option<&str>,
) {
    note_in(
        name,
        format,
        must_exist,
        false,
        found,
        deps,
        output_directory,
    )
}

fn note_in(
    name: &str,
    format: Format,
    must_exist: Option<bool>,
    direct: bool,
    found: Option<&str>,
    deps: &Result<LookupDirs, &'static str>,
    output_directory: Option<&str>,
) {
    let key = (name.to_string(), format, must_exist, direct);
    let d = deps.as_ref().ok().filter(|d| {
        !off()
            // (an ASCII name is compared with each entry's case-folding
            // below; others are left to kpathsea)
            && d.tries.iter().all(|t| t.is_ascii())
            // (a directory searched that does not exist: which name would
            // make it is not kept)
            && d.above.is_empty()
            // (the output directory's file is not kpathsea's answer)
            && !matches!((output_directory, found), (Some(od), Some(f))
                if !name.starts_with('/') && f == format!("{od}/{name}"))
    });
    let mut g = PROOFS.lock().unwrap();
    let m = g.get_or_insert_with(HashMap::new);
    let Some(d) = d else {
        m.remove(&key);
        return;
    };
    let found_in = found.and_then(|f| {
        let p = Path::new(f);
        let (dir, base) = (p.parent()?, p.file_name()?.to_str()?);
        d.listed
            .iter()
            .position(|l| {
                let l = Path::new(l);
                // (the last component first: a TFM lookup's 60 directories)
                l.file_name() == dir.file_name() && l.components().eq(dir.components())
            })
            .map(|i| (i, base.to_string()))
    });
    let mut tries: Vec<String> = d.tries.iter().map(|t| t.to_ascii_lowercase()).collect();
    tries.sort();
    tries.dedup();
    let generation = GENERATION.load(Ordering::Relaxed);
    // (the same lookup made again, with the same dependencies: kept)
    if m.get(&key).is_some_and(|p| {
        p.generation == generation
            && p.output_directory.as_deref() == output_directory
            && p.found.as_deref() == found
            && p.found_in == found_in
            && p.tries == tries
            && p.listed == d.listed
    }) {
        return;
    }
    m.insert(
        key,
        Arc::new(Proof {
            generation,
            output_directory: output_directory.map(str::to_string),
            tries,
            listed: d.listed.clone(),
            found: found.map(str::to_string),
            found_in,
        }),
    );
}

/// Checks lookups against their proofs, for one pass over a journal's or a
/// key's lookups: the working directory, the output directory and each
/// directory's listing are taken once.
pub struct Verifier {
    generation: u64,
    cwd: Option<PathBuf>,
    output_directory: Option<String>,
    dirs: HashMap<String, (Option<StatSig>, Option<Listing>)>,
    /// Lookups the proofs answered, and lookups made again.
    pub held: usize,
    pub made: usize,
}

impl Drop for Verifier {
    fn drop(&mut self) {
        if self.held + self.made > 0 {
            system::file_trace(|| {
                format!(
                    "lookups checked: {} held by their proofs, {} made again",
                    self.held, self.made
                )
            });
        }
    }
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier {
    pub fn new() -> Verifier {
        Verifier {
            generation: GENERATION.load(Ordering::Relaxed),
            cwd: std::env::current_dir().ok(),
            output_directory: system::output_directory(),
            dirs: HashMap::new(),
            held: 0,
            made: 0,
        }
    }

    /// `system::lookup_again_deps`: what looking `l` up finds now, and what
    /// that answer depends on now, each directory with a signature taken
    /// before the listing the answer was checked against. From the proof
    /// where it holds, else by making the lookup again.
    pub fn lookup_again_deps(&mut self, l: &Lookup) -> Answer {
        if let Some(r) = self.holds(&l.name, l.format, l.must_exist, false) {
            self.held += 1;
            return r;
        }
        self.made += 1;
        system::lookup_again_deps(l)
    }

    /// The format cache's check (`formats::FormatCache::validate`): what
    /// `r` finds for `name` now, `find_ex(name, format, true)` with
    /// `must_exist`, else `find`. From the proof where it holds and `r` is
    /// the process's resolver; else from `r`, keeping a proof (unless an
    /// mktex script made the file).
    pub fn find_direct(
        &mut self,
        r: &mut dyn FileResolver,
        name: &str,
        format: Format,
        must_exist: bool,
    ) -> Option<String> {
        let must = must_exist.then_some(true);
        let mine = address(r) == PROCESS_RESOLVER.load(Ordering::Relaxed);
        if mine {
            if let Some((found, _)) = self.holds(name, format, must, true) {
                self.held += 1;
                return found;
            }
        }
        self.made += 1;
        let (found, made) = if must_exist {
            r.find_ex(name, format, true)
        } else {
            (r.find(name, format), false)
        };
        let found = found.map(|p| p.to_string_lossy().into_owned());
        if mine && !made {
            let deps = r.lookup_dirs(name, format, must, found.as_deref().map(Path::new));
            note_in(name, format, must, true, found.as_deref(), &deps, None);
        }
        found
    }

    /// `dir` now: its signature, and its listing (`None`: not listable),
    /// taken after the signature or under the same one.
    fn dir(&mut self, dir: &str) -> (Option<StatSig>, Option<Listing>) {
        if let Some(x) = self.dirs.get(dir) {
            return x.clone();
        }
        // (kept by absolute name: `.` is another directory in another
        // working directory)
        let key = match &self.cwd {
            Some(c) if !Path::new(dir).is_absolute() => c.join(dir).to_string_lossy().into_owned(),
            _ => dir.to_string(),
        };
        let (sig, listing) = listed_now(&key, dir);
        self.dirs.insert(dir.to_string(), (sig, listing.clone()));
        (sig, listing)
    }

    fn holds(
        &mut self,
        name: &str,
        format: Format,
        must_exist: Option<bool>,
        direct: bool,
    ) -> Option<Answer> {
        let p = PROOFS
            .lock()
            .unwrap()
            .as_ref()?
            .get(&(name.to_string(), format, must_exist, direct))?
            .clone();
        // (a direct lookup has no output directory: `note_in` gets none)
        if p.generation != self.generation
            || (!direct && p.output_directory != self.output_directory)
        {
            return None;
        }
        // `lookup_again` tries the output directory first.
        if let Some(od) = &p.output_directory {
            if !name.starts_with('/') {
                let cand = format!("{od}/{name}");
                let c = Path::new(&cand);
                let base = fold(&c.file_name()?.to_string_lossy());
                let parent = c.parent()?.to_string_lossy().into_owned();
                let (_, listing) = self.dir(&parent);
                let named = listing.is_none_or(|e| count(&e, &base) > 0);
                if named && c.is_file() {
                    return None;
                }
            }
        }
        let mut deps = Vec::with_capacity(p.listed.len() + 1);
        for (i, d) in p.listed.iter().enumerate() {
            let (sig, listing) = self.dir(d);
            let (sig, listing) = (sig?, listing?);
            let mine = p.found_in.as_ref().filter(|(k, _)| *k == i).map(|(_, b)| b);
            let mut n = 0;
            for t in &p.tries {
                let at = listing.partition_point(|(e, _)| e < t);
                for (_, named) in listing[at..].iter().take_while(|(e, _)| e == t) {
                    // (the answer, under its exact name, once)
                    if mine.is_some_and(|b| b == named) {
                        n += 1;
                    } else {
                        return None;
                    }
                }
            }
            if mine.is_some() && n != 1 {
                return None;
            }
            deps.push((d.clone(), sig));
        }
        if let (Some(f), Some(_)) = (&p.found, &p.found_in) {
            let entry = system::readable_dep(f);
            let sig = system::dep_sig(&entry)?;
            if sig.len == 0 {
                return None;
            }
            deps.push((entry, sig));
        }
        Some((p.found.clone(), deps))
    }
}

/// `dir` now: its signature, and its listing taken after the signature,
/// or (the cache, under `key`) before it under the same signature, which is
/// not racy (a racy signature equals nothing).
fn listed_now(key: &str, dir: &str) -> (Option<StatSig>, Option<Listing>) {
    let sig = StatSig::of(dir);
    let cached = sig.and_then(|s| {
        let g = LISTINGS.lock().unwrap();
        let (was, l) = g.as_ref()?.get(key)?;
        (*was == s).then(|| l.clone())
    });
    let listing = match cached {
        Some(l) => l,
        None => {
            let l = list(dir);
            if let Some(s) = sig {
                LISTINGS
                    .lock()
                    .unwrap()
                    .get_or_insert_with(HashMap::new)
                    .insert(key.to_string(), (s, l.clone()));
            }
            l
        }
    };
    (sig, listing)
}

/// `system::ReadLog::listing`: `dir`'s entries as an ASCII name kpathsea
/// tries could match them ([`fold`]), sorted; an absolute directory's from
/// the cache while its signature is the one it was listed under
/// ([`listed_now`]: as a listing taken now). The caller signs `dir` before
/// it asks (a change after the listing is a change).
pub fn folded_listing(dir: &str) -> Option<Vec<String>> {
    let l = if Path::new(dir).is_absolute() {
        listed_now(dir, dir).1
    } else {
        list(dir)
    };
    l.map(|l| l.iter().map(|(f, _)| f.clone()).collect())
}

/// How many entries of `listing` fold to `lower`.
fn count(listing: &[(String, String)], lower: &str) -> usize {
    let at = listing.partition_point(|(e, _)| e.as_str() < lower);
    listing[at..].iter().take_while(|(e, _)| e == lower).count()
}

fn list(dir: &str) -> Option<Listing> {
    let mut v: Vec<(String, String)> = std::fs::read_dir(dir)
        .ok()?
        .map(|e| {
            e.map(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                (fold(&n), n)
            })
        })
        .collect::<Result<_, _>>()
        .ok()?;
    v.sort();
    Some(Arc::new(v))
}

/// An entry's name as an ASCII name kpathsea tries could match it: ASCII
/// letters lower-cased (`casefold_readable_file`'s `strcasecmp`), and the
/// characters whose Unicode case folding gives ASCII letters folded too
/// (a case-insensitive file system, APFS's, finds `\u{212A}` for `k`).
/// Folding more than a file system does only makes a check fail.
fn fold(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            c if c.is_ascii() => out.push(c.to_ascii_lowercase()),
            '\u{212A}' => out.push('k'),
            '\u{17F}' => out.push('s'),
            '\u{DF}' | '\u{1E9E}' => out.push_str("ss"),
            '\u{130}' | '\u{131}' => out.push('i'),
            '\u{FB00}' => out.push_str("ff"),
            '\u{FB01}' => out.push_str("fi"),
            '\u{FB02}' => out.push_str("fl"),
            '\u{FB03}' => out.push_str("ffi"),
            '\u{FB04}' => out.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => out.push_str("st"),
            c => out.extend(c.to_lowercase()),
        }
    }
    out
}
