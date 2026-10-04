# The no-TeX-Live bundle

FlashTeX's new engine reads the user's TeX Live when there is one. Without it, it reads a
**bundle**: a content-addressed archive of TeX Live files, fetched by byte range on first use
and pinned by its SHA-256 digest (DESIGN.md §4.4, D12; Tectonic's model). This page says what
the bundle is, how it is made and published, and under which terms its files are distributed.

## Distribution notice

**The bundle carries unmodified TeX Live packages under their own licences.** Every file in it
is a file of TeX Live 2026, byte for byte, belonging to a TeX Live package that the bundle
carries *whole* (all of that package's runfiles, its licence statement among them). Most are
under the LaTeX Project Public Licence; the others are under the free licences TeX Live
records for them (GPL, OFL, public domain, Knuth's, and so on). Each release lists every
package with its licence (`packages.tsv`, from TeX Live's package database) and attaches TeX
Live's own `LICENSE.TL` and `LICENSE.CTAN`.

- FlashTeX's licences (MIT for the app, GPL-2.0-or-later for the engine) do not cover these
  files, and the bundle does not change their terms.
- Nothing is patched, so LPPL clause 6 (distributing a modified work) never applies
  (DESIGN.md §3, §4.4).
- **Sources and documentation** are TeX Live's: each package's `<package>.source.tar.xz` and
  `<package>.doc.tar.xz` on TeX Live's network (`https://mirrors.ctan.org/systems/texlive/tlnet/archive/`;
  after TeX Live 2026 is frozen, `https://ftp.math.utah.edu/pub/tex/historic/systems/texlive/2026/`),
  TeX Live's Subversion repository (`https://tug.org/svn/texlive/`, at the revisions
  `packages.tsv` gives, with each container's SHA-512) and CTAN (`https://ctan.org/pkg/<name>`).
- The app ships only the lock (a URL and a digest). Nothing is downloaded until the user agrees
  in the Download TeX Files sheet (`apps/mac/Sources/FlashTeXMac/EngineV3Bundle.swift`).

## What is in it

- `tools/bundle/tl2026/packages.txt`: the packages carried whole, every one that owns a file the
  parity fixtures, the arXiv corpus (`arxiv-2025-01`), the beamer corpus and `packages-2026`
  read (`texbundle.py record` + `derive`: the engine's read sets over the bundle's tree, and
  MacTeX's `pdflatex -recorder`).
- `tools/bundle/tl2026/core.txt`: the core range, first in the file, what building
  `pdflatex.fmt` and a hello-world read (`texbundle.py core`), so a first compile needs one
  request for it.
- Never: installed formats (the engine builds its own), the installation's own `texmf.cnf`,
  `tlpkg/`, binaries (`bundle/build.rs`).

The tree is the `texlive/texlive` image (scheme-full, TeX Live 2026) pinned by digest in
`bundle-publish.yml`, laid out by `tools/bundle/fetch_image.py`, which verifies every byte
against that digest and needs no container runtime. The parity job's scheme-medium image
lacks 258 of the 464 packages, 35 of them fonts whose maps updmap must merge; adding them from
TeX Live's network pinned by archive hash would break whenever one is updated.

## Publishing (GitHub Release assets)

Release asset downloads redirect to a CDN that answers `Range` requests with 206 (measured
2026-10-04), which the fetcher (`bundle/fetch.rs`, through `curl -L`) needs.

1. Change `packages.txt`, `core.txt`, the image pin or the packer, and dispatch
   **bundle publish** without `publish`: a dry run. It packs twice (byte-identical), checks
   `core.txt` against the engine's recording over the tree, makes the oracle references and
   the notes, and runs **notex gate** on the packed bundle. Its summary gives the digest.
2. Write the new tag (`texbundle-tl2026-<n+1>`) and that digest into
   `tools/bundle/tl2026/flashtex-bundle.lock`, review, merge.
3. Dispatch **bundle publish** with `publish`. It refuses a digest other than the lock's and a
   tag that exists (a published bundle is never replaced), creates the release
   (`--latest=false`, so the app's releases stay "latest") and checks that the lock's URL
   serves byte ranges of the packed file.
4. Dispatch **notex gate**: it compiles the gate documents on a Mac without TeX Live from the
   published bundle over HTTPS, against the release's `oracle-refs.json`.

`apps/mac/scripts/make-app.sh` copies the lock into `Contents/Resources/engine/`.

## The gate

`texbundle.py gate` (in `notex-gate.yml`, GitHub-hosted macOS, no TeX Live; the engine under
`sandbox-exec` denying every TeX Live location, as `tests/bundle_notex.rs`) compiles every
parity fixture and a 30-paper arXiv sample (`tl2026/gate.json`) from the bundle alone and
requires each PDF to be byte-identical to TeX Live's pdflatex's in the bundle's tree
(`oracle-refs.json`: hashes only; no third-party PDF is published). `tl2026/gaps.json` lists
the known gaps: fonts METAFONT makes at run time and EPS figures converted through
`\write18`, until those lanes land.
