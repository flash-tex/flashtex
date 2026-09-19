//! `flashtex-project-files --root DIR`: private JSON Lines host for the rooted
//! read/status/save primitives of `flashtex_project_files` (README, "Native
//! project integration proposal", step 2). One process per project root; the
//! Mac shell drives it over private pipes exactly like `flashtex-edit-ledger`.
//!
//! Protocol (`project-files-v1`), one JSON object per line, replies in request
//! order, every reply echoing `id`:
//!
//! - `{"id","operation":"ping"}` → `{"id","payload":{"protocol","root","pid"}}`
//! - `{"id","operation":"read","path"}` → payload `{"path","exists":bool,
//!   "text"?,"sha256"?,"bytes"?,"mtime_unix_ms"?}`
//! - `{"id","operation":"status","path","expected_sha256"?:hex|null}` →
//!   payload `{"path","exists","state":"unchanged"|"modified"|"deleted"|"created",
//!   "sha256"?,"bytes"?,"mtime_unix_ms"?}` (`state` is relative to
//!   `expected_sha256`; `null`/absent means the caller expects no file)
//! - `{"id","operation":"save","path","text","expected":"new"|"any"|hex,
//!   "force"?:bool}` → payload `{"outcome":"saved","receipt":{"path","bytes",
//!   "sha256","mtime_unix_ms"}}` or `{"outcome":"conflict","conflict":{"path",
//!   "kind":"modified_externally"|"deleted_externally"|"already_exists"|
//!   "modified_during_save","ours"?,"theirs"?,"mtime_unix_ms"?,"size"?}}`
//! - `{"id","operation":"manifest","entry"?}` → payload `{"path"?,"exists",
//!   "manifest_dir"?,"manifest":{"project":{"entry","texinputs","output"},
//!   "fonts":{"text","math","mono","sans"},"packages":{"source","fetch","pin",
//!   "path"},"library"?},"warnings":[{"key","message"}],"texinputs":[{"index",
//!   "raw","location":"inside"|"outside"|"invalid","dir"?,"path"?,"reason"?}],
//!   "files":[{"path","kind":"package"|"class"|"tex"|"bibliography","texinput"?,
//!   "origin"?,"text","sha256","bytes"}],"diagnostics":[{"key","message"}],
//!   "template"}`: the `flashtex.toml` that governs `--root` (found by walking
//!   up from it, `Manifest::locate`) or the defaults when there is none, its
//!   classified `texinputs`, and the package inputs — the root's own
//!   `.sty`/`.cls`/`.def`/`.clo` files and every document-kind file of each
//!   `texinputs` directory (`graph::texinput_files`; an outside directory's
//!   files carry the virtual `texinputs/<i>/<name>` path and their real
//!   `origin`), each with its text so the consumer needs no second request.
//!   `template` is the commented manifest the consumer may save as
//!   `flashtex.toml` for `entry` (default `main.tex`); a `save` with
//!   `"expected":"new"` writes it under the same rules as any file.
//! - `{"id","operation":"set_fonts","fonts":{"text"?,"math"?,"mono"?,"sans"?},
//!   "entry"?}` → payload `{"path","exists","changed","text"?}`: the text of the
//!   governing manifest (or of the template for `entry` when there is none)
//!   with its `[fonts]` table replaced by `fonts` (`Manifest::with_fonts`,
//!   the only TOML writer; everything else in the file is kept byte for
//!   byte), for the consumer to `save` at `path` -- this operation writes
//!   nothing. `changed` is false, and `text` absent, when there is no
//!   manifest and `fonts` names nothing: nothing to write. A member that is
//!   not a string, or an unknown member, is `invalid_request`.
//!
//! - `{"id","operation":"resolve_packages","names":[…],"consent"?:bool,
//!   "entry"?}` → payload `{"cache":dir|null,"policy":{"source","fetch"},
//!   "diagnostics":[{"key","message"}],"packages":[{"name","status":
//!   "cached"|"fetched"|"needs_consent"|"not_available","version"?,
//!   "source_url"?,"from"?:"library"|"cache","would_fetch"?:[…],"reason"?,
//!   "files"?:[{"path","text","sha256","bytes"}]}]}`: each name resolved
//!   through `flashtex_package_resolver` under the governing manifest's
//!   `[packages]` policy — a local library (`path`), then the per-user cache,
//!   then the source. Without `consent` a `fetch = "ask"` package is
//!   `needs_consent` describing what would be fetched from where (one
//!   metadata request, no file); with `"consent":true` — the consumer showed
//!   its sheet and the user said yes — it is fetched into the cache. `always`
//!   fetches either way (the manifest is the remembered consent); `never`
//!   and `source = "none"` stop at the cache. Files carry the document-set
//!   path `packages/<name>/<file>`. `diagnostics` are `packages.path.<key>`
//!   problems. A name that is not a package name is `invalid_request`.
//! - `{"id","operation":"set_packages","fetch"?:"ask"|"always"|"never",
//!   "pin"?:{name:version},"entry"?}` → payload `{"path","exists","changed",
//!   "text"?}`: like `set_fonts`, the governing manifest's text (or the
//!   template) with the named `[packages]` keys rewritten
//!   (`Manifest::with_packages`), for the consumer to `save`. Nothing written.
//!
//! Errors are `{"id","error":{"code","message"}}` with codes `invalid_request`,
//! `invalid_path`, `refused` (symlink component, escape, not a regular file,
//! too large, lock held, unsupported target), `invalid_utf8`, `io`,
//! `directory_sync` (rename landed, durability unknown) and `line_too_long`.
//! A conflict is never an error: the shell must show it and keep its buffer.
//! Nothing here follows a symlink or writes outside `--root`; the guarantees
//! are the library's (README "Guarantees" / "Non-guarantees").

use std::io::{self, BufRead, Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use flashtex_project_files::json::Json;
use flashtex_project_files::{
    DEFAULT_READ_LIMIT, DiagnosticKind, Expected, ProjectPath, ProjectRoot, SaveConflictKind,
    SaveError, sha256_from_hex, sha256_to_hex, texinput_files,
};
use flashtex_project_manifest::{Manifest, TexInputLocation, outside_virtual_dir};

/// Same bound as the Mac `LineProcessClient` / transfer-v1 (12 MiB).
const MAX_LINE_BYTES: usize = 12 * 1024 * 1024;
const PROTOCOL: &str = "project-files-v1";

struct Failure {
    code: &'static str,
    message: String,
}

fn fail(code: &'static str, message: impl Into<String>) -> Failure {
    Failure {
        code,
        message: message.into(),
    }
}

impl From<SaveError> for Failure {
    fn from(e: SaveError) -> Self {
        match e {
            SaveError::Refused(r) => fail("refused", format!("{r:?}")),
            SaveError::Io(io) if io.kind() == io::ErrorKind::InvalidData => {
                fail("invalid_utf8", io.to_string())
            }
            SaveError::Io(io) => fail("io", io.to_string()),
            SaveError::DirectorySync(io) => fail("directory_sync", io.to_string()),
            // Conflicts are payloads; `save` handles them before this conversion.
            SaveError::Conflict(c) => fail("refused", format!("{:?}", c.kind)),
        }
    }
}

fn unix_ms(t: SystemTime) -> Json {
    match t.duration_since(UNIX_EPOCH) {
        Ok(d) => Json::from(d.as_millis().min(u64::MAX as u128) as u64),
        Err(_) => Json::from(0u64),
    }
}

fn field<'a>(req: &'a Json, key: &str) -> Result<&'a Json, Failure> {
    req.get(key)
        .filter(|v| !matches!(v, Json::Null))
        .ok_or_else(|| fail("invalid_request", format!("missing field {key:?}")))
}

fn string_field<'a>(req: &'a Json, key: &str) -> Result<&'a str, Failure> {
    field(req, key)?
        .as_str()
        .ok_or_else(|| fail("invalid_request", format!("field {key:?} must be a string")))
}

fn project_path(req: &Json) -> Result<ProjectPath, Failure> {
    let raw = string_field(req, "path")?;
    ProjectPath::normalize(raw).map_err(|e| fail("invalid_path", format!("{raw:?}: {e}")))
}

fn read(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    let path = project_path(req)?;
    let mut payload = Json::object();
    payload.insert("path", path.as_str());
    match root.read_text(&path, DEFAULT_READ_LIMIT)? {
        None => {
            payload.insert("exists", false);
        }
        Some((text, read)) => {
            payload
                .insert("exists", true)
                .insert("bytes", read.bytes.len() as u64)
                .insert("sha256", sha256_to_hex(&read.sha256))
                .insert("mtime_unix_ms", unix_ms(read.mtime))
                .insert("text", text);
        }
    }
    Ok(payload)
}

fn status(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    let path = project_path(req)?;
    let expected = match req.get("expected_sha256") {
        None | Some(Json::Null) => None,
        Some(v) => {
            let hex = v.as_str().ok_or_else(|| {
                fail(
                    "invalid_request",
                    "expected_sha256 must be a hex string or null",
                )
            })?;
            Some(sha256_from_hex(hex).ok_or_else(|| {
                fail(
                    "invalid_request",
                    format!("expected_sha256 {hex:?} is not a SHA-256 hex digest"),
                )
            })?)
        }
    };
    let mut payload = Json::object();
    payload.insert("path", path.as_str());
    match root.read(&path, DEFAULT_READ_LIMIT)? {
        None => {
            payload.insert("exists", false).insert(
                "state",
                if expected.is_some() {
                    "deleted"
                } else {
                    "unchanged"
                },
            );
        }
        Some(read) => {
            let state = match expected {
                None => "created",
                Some(e) if e == read.sha256 => "unchanged",
                Some(_) => "modified",
            };
            payload
                .insert("exists", true)
                .insert("state", state)
                .insert("bytes", read.bytes.len() as u64)
                .insert("sha256", sha256_to_hex(&read.sha256))
                .insert("mtime_unix_ms", unix_ms(read.mtime));
        }
    }
    Ok(payload)
}

fn save(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    let path = project_path(req)?;
    let text = string_field(req, "text")?;
    let expected = match string_field(req, "expected")? {
        "new" => Expected::NewFile,
        "any" => Expected::Any,
        hex => Expected::Hash(sha256_from_hex(hex).ok_or_else(|| {
            fail(
                "invalid_request",
                format!("expected {hex:?} is not \"new\", \"any\" or a SHA-256 hex digest"),
            )
        })?),
    };
    let force = match req.get("force") {
        None | Some(Json::Null) => false,
        Some(Json::Bool(b)) => *b,
        Some(_) => return Err(fail("invalid_request", "force must be a boolean")),
    };
    let mut payload = Json::object();
    match root.save(&path, text.as_bytes(), expected, force) {
        Ok(receipt) => {
            let mut r = Json::object();
            r.insert("path", receipt.path.as_str())
                .insert("bytes", receipt.bytes)
                .insert("sha256", receipt.sha256_hex())
                .insert("mtime_unix_ms", unix_ms(receipt.mtime));
            payload.insert("outcome", "saved").insert("receipt", r);
        }
        Err(SaveError::Conflict(c)) => {
            let mut j = Json::object();
            j.insert("path", c.path.as_str())
                .insert(
                    "kind",
                    match c.kind {
                        SaveConflictKind::ModifiedExternally => "modified_externally",
                        SaveConflictKind::DeletedExternally => "deleted_externally",
                        SaveConflictKind::AlreadyExists => "already_exists",
                        SaveConflictKind::ModifiedDuringSave => "modified_during_save",
                    },
                )
                .insert("ours", c.ours.as_ref().map(sha256_to_hex))
                .insert("theirs", c.theirs.as_ref().map(sha256_to_hex))
                .insert("mtime_unix_ms", c.mtime.map(unix_ms))
                .insert("size", c.size);
            payload.insert("outcome", "conflict").insert("conflict", j);
        }
        Err(e) => return Err(e.into()),
    }
    Ok(payload)
}

/// `manifest`: see the module documentation. Reads exactly one file above
/// the root (the manifest itself, through `Manifest::locate`) and the
/// package inputs through rooted handles; a manifest that is not TOML is a
/// `manifest_syntax` error, everything else it says wrong is a warning.
fn manifest(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    let entry = match req.get("entry") {
        None | Some(Json::Null) => "main.tex".to_string(),
        Some(v) => v
            .as_str()
            .ok_or_else(|| fail("invalid_request", "entry must be a string"))?
            .to_string(),
    };
    let found = Manifest::locate(root.path());
    let loaded = match &found {
        Some(path) => Manifest::load(path).map_err(|e| fail("manifest_syntax", e.to_string()))?,
        None => Default::default(),
    };
    let manifest_dir = found.as_deref().and_then(|p| p.parent()).unwrap_or(root.path());
    let m = &loaded.manifest;
    let (files, diagnostics) = texinput_files(root, m, manifest_dir);

    let opt = |v: &Option<String>| v.clone().map_or(Json::Null, Json::from);
    let map = |m: &std::collections::BTreeMap<String, String>| {
        let mut o = Json::object();
        for (k, v) in m {
            o.insert(k, v.as_str());
        }
        o
    };
    let mut project = Json::object();
    project
        .insert("entry", opt(&m.project.entry))
        .insert("texinputs", m.project.texinputs.iter().map(|t| Json::from(t.as_str())).collect::<Vec<_>>())
        .insert("output", opt(&m.project.output));
    let mut fonts = Json::object();
    fonts
        .insert("text", opt(&m.fonts.text))
        .insert("math", opt(&m.fonts.math))
        .insert("mono", opt(&m.fonts.mono))
        .insert("sans", opt(&m.fonts.sans));
    let mut packages = Json::object();
    packages
        .insert("source", m.packages.source.as_str())
        .insert("fetch", m.packages.fetch.as_str())
        .insert("pin", map(&m.packages.pin))
        .insert("path", map(&m.packages.path));
    let library = m.library.as_ref().map_or(Json::Null, |l| {
        let mut o = Json::object();
        o.insert("name", l.name.as_str());
        o
    });
    let mut manifest = Json::object();
    manifest
        .insert("project", project)
        .insert("fonts", fonts)
        .insert("packages", packages)
        .insert("library", library);

    let warnings: Vec<Json> = loaded
        .warnings
        .iter()
        .map(|w| {
            let mut o = Json::object();
            o.insert("key", w.key.as_str()).insert("message", w.message.as_str());
            o
        })
        .collect();
    let texinputs: Vec<Json> = m
        .texinputs(manifest_dir)
        .iter()
        .map(|t| {
            let mut o = Json::object();
            o.insert("index", t.index as u64).insert("raw", t.raw.as_str());
            match &t.location {
                TexInputLocation::Inside(d) => {
                    o.insert("location", "inside").insert("dir", d.as_str());
                }
                TexInputLocation::Outside(p) => {
                    o.insert("location", "outside")
                        .insert("dir", outside_virtual_dir(t.index))
                        .insert("path", p.to_string_lossy().into_owned());
                }
                TexInputLocation::Invalid(why) => {
                    o.insert("location", "invalid").insert("reason", why.as_str());
                }
            }
            o
        })
        .collect();
    let files: Vec<Json> = files
        .iter()
        .filter_map(|f| {
            // A package input that is not UTF-8 is not a document the
            // compiler can read; `diagnostics` does not repeat it because
            // the consumer sees the file is simply absent from `files`.
            let text = f.file.text.as_ref()?;
            let mut o = Json::object();
            o.insert("path", f.file.path.as_str())
                .insert("kind", f.file.kind.as_str())
                .insert("texinput", f.texinput.map_or(Json::Null, |i| Json::from(i as u64)))
                .insert("origin", f.file.origin.as_ref().map_or(Json::Null, |p| Json::from(p.to_string_lossy().into_owned())))
                .insert("text", text.as_str())
                .insert("sha256", sha256_to_hex(&f.file.sha256))
                .insert("bytes", f.file.bytes);
            Some(o)
        })
        .collect();
    let diagnostics: Vec<Json> = diagnostics
        .iter()
        .map(|d| {
            let mut o = Json::object();
            let key = match &d.kind {
                DiagnosticKind::Manifest { key } => key.as_str(),
                _ => "project",
            };
            o.insert("key", key).insert("message", d.message.as_str());
            o
        })
        .collect();

    let mut payload = Json::object();
    payload
        .insert("path", found.as_ref().map_or(Json::Null, |p| Json::from(p.to_string_lossy().into_owned())))
        .insert("exists", found.is_some())
        .insert("manifest_dir", found.as_ref().map_or(Json::Null, |_| Json::from(manifest_dir.to_string_lossy().into_owned())))
        .insert("manifest", manifest)
        .insert("warnings", warnings)
        .insert("texinputs", texinputs)
        .insert("files", files)
        .insert("diagnostics", diagnostics)
        .insert("template", Manifest::template(&entry));
    Ok(payload)
}

/// `set_fonts`: see the module documentation. Reads the governing manifest
/// (or takes the template) and answers with the rewritten text; the
/// consumer saves it through `save`, so the rooted rules apply to the write.
fn set_fonts(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    let entry = match req.get("entry") {
        None | Some(Json::Null) => "main.tex".to_string(),
        Some(v) => v
            .as_str()
            .ok_or_else(|| fail("invalid_request", "entry must be a string"))?
            .to_string(),
    };
    let mut fonts = flashtex_project_manifest::Fonts::default();
    match req.get("fonts") {
        None | Some(Json::Null) => {}
        Some(Json::Object(members)) => {
            for (key, value) in members {
                let slot = match key.as_str() {
                    "text" => &mut fonts.text,
                    "math" => &mut fonts.math,
                    "mono" => &mut fonts.mono,
                    "sans" => &mut fonts.sans,
                    other => return Err(fail("invalid_request", format!("fonts has an unknown member {other:?}; expected text, math, mono, sans"))),
                };
                *slot = match value {
                    Json::Null => None,
                    Json::String(s) if s.trim().is_empty() => None,
                    Json::String(s) => Some(s.clone()),
                    _ => return Err(fail("invalid_request", format!("fonts.{key} must be a family name (string) or null"))),
                };
            }
        }
        Some(_) => return Err(fail("invalid_request", "fonts must be an object of family names by role")),
    }
    let found = Manifest::locate(root.path());
    let (path, exists, current) = match &found {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| fail("io", format!("{}: {e}", path.display())))?;
            (path.clone(), true, text)
        }
        None => (root.path().join(flashtex_project_manifest::FILE_NAME), false, Manifest::template(&entry)),
    };
    let mut payload = Json::object();
    payload
        .insert("path", path.to_string_lossy().into_owned())
        .insert("exists", exists);
    if !exists && fonts.is_empty() {
        payload.insert("changed", false);
        return Ok(payload);
    }
    let text = Manifest::with_fonts(&current, &fonts);
    // The rewrite must read back as what was asked, or the file stays as it is.
    match Manifest::parse(&text) {
        Ok(parsed) if parsed.manifest.fonts == fonts => {}
        Ok(parsed) => {
            return Err(fail(
                "manifest_rewrite",
                format!("the rewritten {} does not read back the requested fonts (got {:?}); not written", path.display(), parsed.manifest.fonts),
            ))
        }
        Err(e) => return Err(fail("manifest_syntax", format!("{}: {e}", path.display()))),
    }
    payload.insert("changed", text != current).insert("text", text);
    Ok(payload)
}

/// The manifest governing `root` and its directory, for the package operations.
fn governing_manifest(root: &ProjectRoot) -> Result<(Option<PathBuf>, flashtex_project_manifest::Loaded, PathBuf), Failure> {
    let found = Manifest::locate(root.path());
    let loaded = match &found {
        Some(path) => Manifest::load(path).map_err(|e| fail("manifest_syntax", e.to_string()))?,
        None => Default::default(),
    };
    let manifest_dir = found.as_deref().and_then(|p| p.parent()).unwrap_or(root.path()).to_path_buf();
    Ok((found, loaded, manifest_dir))
}

/// `resolve_packages`: see the module documentation.
fn resolve_packages(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    use flashtex_package_resolver::{library, virtual_path, Policy, Provenance, Resolution, Resolver};
    let names: Vec<String> = match req.get("names") {
        Some(Json::Array(items)) => items
            .iter()
            .map(|n| n.as_str().map(str::to_string).ok_or_else(|| fail("invalid_request", "names must be strings")))
            .collect::<Result<_, _>>()?,
        _ => return Err(fail("invalid_request", "names must be an array of package names")),
    };
    if let Some(bad) = names.iter().find(|n| !flashtex_package_resolver::is_valid_name(n)) {
        return Err(fail("invalid_request", format!("{bad:?} is not a package name")));
    }
    let consent = match req.get("consent") {
        None | Some(Json::Null) => false,
        Some(Json::Bool(b)) => *b,
        Some(_) => return Err(fail("invalid_request", "consent must be a boolean")),
    };
    let (_, loaded, manifest_dir) = governing_manifest(root)?;
    let packages = &loaded.manifest.packages;
    let mut payload = Json::object();
    let mut policy = Json::object();
    policy.insert("source", packages.source.as_str()).insert("fetch", packages.fetch.as_str());
    payload.insert("policy", policy);
    let (libraries, library_diagnostics) = library::load_all(&loaded.manifest, &manifest_dir);
    let diagnostics: Vec<Json> = library_diagnostics
        .iter()
        .map(|m| {
            let mut o = Json::object();
            let key = m.split_once(" = ").map_or("packages.path", |(k, _)| k);
            o.insert("key", key).insert("message", m.as_str());
            o
        })
        .collect();
    payload.insert("diagnostics", diagnostics);
    let cache_root = flashtex_package_resolver::default_cache_root();
    payload.insert("cache", cache_root.as_ref().map_or(Json::Null, |p| Json::from(p.to_string_lossy().into_owned())));
    let fetcher = flashtex_package_resolver::http::HttpFetcher::new().map_err(|e| fail("io", e))?;
    let resolver = cache_root.map(|c| Resolver::new(c, &fetcher).with_libraries(libraries));
    let entries: Vec<Json> = names
        .iter()
        .map(|name| {
            let mut o = Json::object();
            o.insert("name", name.as_str());
            let Some(resolver) = &resolver else {
                o.insert("status", "not_available").insert("reason", format!("no package cache: set {} (no home directory is known)", flashtex_package_resolver::CACHE_ENV));
                return o;
            };
            let policy = Policy::for_package(packages, name);
            let resolution = if consent { resolver.resolve_with_consent(name, &policy) } else { resolver.resolve(name, &policy) };
            let files_json = |mount: &str, files: &[flashtex_package_resolver::ResolvedFile]| {
                files
                    .iter()
                    .map(|f| {
                        let mut j = Json::object();
                        j.insert("path", virtual_path(mount, &f.name))
                            .insert("text", f.text.as_str())
                            .insert("sha256", flashtex_project_files::sha256_hex(f.text.as_bytes()))
                            .insert("bytes", f.text.len() as u64);
                        if let Some(g) = &f.generated_from {
                            let mut from = Json::object();
                            from.insert("batch", g.batch.as_str()).insert("sources", g.sources.iter().map(|s| Json::from(s.as_str())).collect::<Vec<_>>());
                            j.insert("generated_from", from);
                        }
                        j
                    })
                    .collect::<Vec<_>>()
            };
            match resolution {
                Resolution::Cached { version, files, from, .. } => {
                    let (label, mount) = match &from {
                        Provenance::Library { name: lib, .. } => ("library", lib.clone()),
                        Provenance::Cache => ("cache", name.clone()),
                    };
                    o.insert("status", "cached").insert("version", version).insert("from", label).insert("files", files_json(&mount, &files));
                }
                Resolution::Fetched { version, files, source_url, notes, .. } => {
                    o.insert("status", "fetched").insert("version", version).insert("source_url", source_url).insert("files", files_json(name, &files));
                    if !notes.is_empty() {
                        o.insert("docstrip_notes", notes.iter().map(|n| Json::from(n.as_str())).collect::<Vec<_>>());
                    }
                }
                Resolution::NeedsConsent { version, source_url, would_fetch, .. } => {
                    o.insert("status", "needs_consent")
                        .insert("version", version.map_or(Json::Null, Json::from))
                        .insert("source_url", source_url)
                        .insert("would_fetch", would_fetch.into_iter().map(Json::from).collect::<Vec<_>>());
                }
                Resolution::NotAvailable { reason, .. } => {
                    o.insert("status", "not_available").insert("reason", reason);
                }
            }
            o
        })
        .collect();
    payload.insert("packages", entries);
    Ok(payload)
}

/// `set_packages`: see the module documentation. The counterpart of
/// `set_fonts` for the `[packages]` keys the consent sheet writes.
fn set_packages(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    use flashtex_project_manifest::FetchPolicy;
    let entry = match req.get("entry") {
        None | Some(Json::Null) => "main.tex".to_string(),
        Some(v) => v.as_str().ok_or_else(|| fail("invalid_request", "entry must be a string"))?.to_string(),
    };
    let fetch = match req.get("fetch") {
        None | Some(Json::Null) => None,
        Some(Json::String(s)) => Some(match s.as_str() {
            "ask" => FetchPolicy::Ask,
            "always" => FetchPolicy::Always,
            "never" => FetchPolicy::Never,
            other => return Err(fail("invalid_request", format!("fetch must be ask, always or never, got {other:?}"))),
        }),
        Some(_) => return Err(fail("invalid_request", "fetch must be a string")),
    };
    let pin = match req.get("pin") {
        None | Some(Json::Null) => None,
        Some(Json::Object(members)) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in members {
                let v = v.as_str().ok_or_else(|| fail("invalid_request", format!("pin.{k} must be a version string")))?;
                if !flashtex_package_resolver::is_valid_name(k) {
                    return Err(fail("invalid_request", format!("{k:?} is not a package name")));
                }
                map.insert(k.clone(), v.to_string());
            }
            Some(map)
        }
        Some(_) => return Err(fail("invalid_request", "pin must be an object of versions by package name")),
    };
    if fetch.is_none() && pin.is_none() {
        return Err(fail("invalid_request", "set_packages needs fetch and/or pin"));
    }
    let (found, _, _) = governing_manifest(root)?;
    let (path, exists, current) = match &found {
        Some(path) => {
            let text = std::fs::read_to_string(path).map_err(|e| fail("io", format!("{}: {e}", path.display())))?;
            (path.clone(), true, text)
        }
        None => (root.path().join(flashtex_project_manifest::FILE_NAME), false, Manifest::template(&entry)),
    };
    let text = Manifest::with_packages(&current, fetch, pin.as_ref());
    match Manifest::parse(&text) {
        Ok(parsed) => {
            if fetch.is_some_and(|f| parsed.manifest.packages.fetch != f) || pin.as_ref().is_some_and(|p| &parsed.manifest.packages.pin != p) {
                return Err(fail("manifest_rewrite", format!("the rewritten {} does not read back the requested policy; not written", path.display())));
            }
        }
        Err(e) => return Err(fail("manifest_syntax", format!("{}: {e}", path.display()))),
    }
    let mut payload = Json::object();
    payload
        .insert("path", path.to_string_lossy().into_owned())
        .insert("exists", exists)
        .insert("changed", text != current)
        .insert("text", text);
    Ok(payload)
}

fn handle(root: &ProjectRoot, req: &Json) -> Result<Json, Failure> {
    match string_field(req, "operation")? {
        "ping" => {
            let mut p = Json::object();
            p.insert("protocol", PROTOCOL)
                .insert("root", root.path().to_string_lossy().into_owned())
                .insert("pid", u64::from(std::process::id()));
            Ok(p)
        }
        "read" => read(root, req),
        "status" => status(root, req),
        "save" => save(root, req),
        "manifest" => manifest(root, req),
        "set_fonts" => set_fonts(root, req),
        "resolve_packages" => resolve_packages(root, req),
        "set_packages" => set_packages(root, req),
        other => Err(fail(
            "unsupported_operation",
            format!("unknown operation {other:?}"),
        )),
    }
}

fn reply(out: &mut impl Write, id: Json, result: Result<Json, Failure>) -> io::Result<()> {
    let mut line = Json::object();
    line.insert("id", id);
    match result {
        Ok(payload) => {
            line.insert("payload", payload);
        }
        Err(f) => {
            let mut e = Json::object();
            e.insert("code", f.code).insert("message", f.message);
            line.insert("error", e);
        }
    }
    writeln!(out, "{}", line.to_string_compact())?;
    out.flush()
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let root_arg = match (args.next(), args.next(), args.next()) {
        (Some(flag), Some(dir), None) if flag == "--root" => PathBuf::from(dir),
        _ => {
            eprintln!("usage: flashtex-project-files --root PROJECT_DIRECTORY");
            std::process::exit(2);
        }
    };
    let root = match ProjectRoot::open(&root_arg) {
        Ok(root) => root,
        Err(e) => {
            eprintln!(
                "flashtex-project-files: cannot open root {}: {e}",
                root_arg.display()
            );
            std::process::exit(1);
        }
    };
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let mut line = Vec::new();
        let n = match input
            .by_ref()
            .take(MAX_LINE_BYTES as u64 + 1)
            .read_until(b'\n', &mut line)
        {
            Ok(n) => n,
            Err(e) => {
                eprintln!("flashtex-project-files: stdin: {e}");
                std::process::exit(1);
            }
        };
        if n == 0 {
            return;
        }
        if line.len() > MAX_LINE_BYTES {
            let _ = reply(
                &mut output,
                Json::Null,
                Err(fail(
                    "line_too_long",
                    format!("request exceeds {MAX_LINE_BYTES} bytes"),
                )),
            );
            std::process::exit(1);
        }
        let text = match std::str::from_utf8(&line) {
            Ok(t) => t.trim_end_matches(['\n', '\r']),
            Err(_) => {
                let _ = reply(
                    &mut output,
                    Json::Null,
                    Err(fail("invalid_request", "request is not UTF-8")),
                );
                continue;
            }
        };
        if text.trim().is_empty() {
            continue;
        }
        let req = match Json::parse(text) {
            Ok(j) => j,
            Err(e) => {
                let _ = reply(
                    &mut output,
                    Json::Null,
                    Err(fail("invalid_request", format!("malformed JSON: {e}"))),
                );
                continue;
            }
        };
        let id = req.get("id").cloned().unwrap_or(Json::Null);
        let result = handle(&root, &req);
        if let Err(e) = reply(&mut output, id, result) {
            // The reader went away; there is nobody to tell.
            eprintln!("flashtex-project-files: stdout: {e}");
            std::process::exit(1);
        }
    }
}
