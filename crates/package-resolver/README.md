# flashtex-package-resolver

Package resolution for FlashTeX projects — slice S3 of
[packages, fonts and the project manifest](../../docs/proposals/packages-fonts-manifest.md).
Project layer only: the CLI and the project-files helper link it; the engine
crates never do and never see the network.

```sh
cargo test --manifest-path crates/package-resolver/Cargo.toml            # no network
FLASHTEX_NETWORK_TESTS=1 cargo test --manifest-path crates/package-resolver/Cargo.toml --test network
```

## What it answers

`Resolver::resolve(name, policy)` for a `\usepackage`/`\documentclass` name
the project does not supply, in this order:

1. a **local library** from `[packages] path = { mylib = "../mylib" }` — a
   directory whose own `flashtex.toml` says `[library] name = "mylib"`; its
   `.sty`/`.cls`/`.def`/`.clo` files (no recursion) win over everything else;
2. the **cache** below;
3. the **source** — only if the policy allows: `fetch = "always"` fetches,
   `"ask"` answers `NeedsConsent { name, version, source_url, would_fetch }`
   and the caller fetches with `resolve_with_consent` once the user said yes,
   `"never"` (and `source = "none"`) answers `NotAvailable`.

Answers are `Cached`, `NeedsConsent`, `Fetched` or `NotAvailable(reason)`.
Files reach the compiler as documents at `packages/<name>/<file>`.

## Endpoints (`source = "ctan"`)

| Step | Request | Read |
|---|---|---|
| describe | `GET https://ctan.org/json/2.0/pkg/<name>` | `version.number`, `ctan.path` (404: no such package) |
| list | `GET https://mirrors.ctan.org<ctan.path>/` | every `href` that is a `.sty/.cls/.def/.clo/.cfg`, plus every `.ins`/`.dtx` when there is a `.ins` |
| fetch (after consent) | `GET https://mirrors.ctan.org<ctan.path>/<file>` | the file |

`mirrors.ctan.org` redirects to a mirror that serves the archive root
directly (`…/macros/latex/contrib/<name>/`; a `tex-archive/` prefix 404s on
the mirrors). "Ask" performs the first two requests to phrase the question
("Fetch cancel 2.2 from CTAN (https://…)?"); no package file moves before
consent. A directory with a `.ins` has its `.ins`/`.dtx` sources fetched
too; after the download they go through
[`crates/docstrip`](../docstrip/README.md) — an interpreter of docstrip's
batch language, not a TeX, so nothing fetched is ever executed — and the
generated `.sty`/`.cls`/`.def`/`.clo`/`.cfg` are cached next to the
shipped ones (a shipped file wins over a generated one of the same name;
the sources themselves are not cached). A directory with `.dtx` but no
`.ins`, or a batch file that generates no package file, is
`NotAvailable("needs docstrip …")` with the reason. CTAN keeps one
version, so a `pin` naming another is `NotAvailable` naming both.

A registry URL (`source = "https://…"`) is an archive root in the same
layout (`<url>/macros/latex/contrib/<name>/`, listed the same way); it states
no version, so the version is the content hash (12 hex digits of the SHA-256
over the files), which is what a `pin` compares against.

Git URLs are out of scope: a library is a directory.

## Cache layout

Root: `FLASHTEX_PACKAGE_CACHE`, else macOS
`~/Library/Application Support/FlashTeX/packages`, Linux
`${XDG_CACHE_HOME:-~/.cache}/flashtex/packages`, Windows
`%LOCALAPPDATA%\FlashTeX\packages`.

```
<root>/<name>/<version>/<file>…
<root>/<name>/<version>/manifest.json
```

```json
{"name":"cancel","version":"2.2",
 "source_url":"https://mirrors.ctan.org/macros/latex/contrib/cancel/",
 "fetched_utc":"2026-09-19T10:11:12Z",
 "files":[{"name":"cancel.sty","sha256":"…","bytes":1234}]}
```

A generated file carries its provenance and the manifest the
interpreter's notes:

```json
{"name":"lipsum","version":"2.7", "…":"…",
 "files":[{"name":"lipsum.sty","sha256":"…","bytes":14690,
           "generated_from":{"batch":"lipsum.ins","sources":["lipsum.dtx"]}}],
 "docstrip":{"notes":["lipsum.ins:41: \\newread is not a docstrip command; …"]}}
```

`ResolvedFile::generated_from` and `Resolution::Fetched::notes` carry the
same; `flashtex packages list` marks generated files with `*`.

A version is written to a staging directory and renamed into place, so a
crash leaves no half version; a version directory without `manifest.json`
is not a version; a file whose digest no longer matches is refused on read.
Without a pin the most recently fetched version is used.

## The network seam

`Fetcher` is one method, `get(url) -> bytes`. `http::HttpFetcher` (feature
`network`, on by default) is blocking `reqwest` over rustls with a 10 s
connect / 30 s request timeout and an 8 MiB body limit; it also serves
`file://` URLs from disk (a directory answers with its `index.html`) so the
CLI and helper tests run against an on-disk fake archive. Unit tests use an
in-memory fake and open no socket.
