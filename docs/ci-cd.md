# CI/CD: build, test, release and publish FlashTeX

Three GitHub Actions workflows live in `.github/workflows/`, backed by scripts
in `scripts/ci/` that also run locally.

| Piece | What it does |
|---|---|
| `ci.yml` | On push to `main`, pull requests and manual runs: builds and tests every Rust crate on Linux and macOS (workspace + the two vendor-pinned standalone crates), checks vendor pins and generated tables, builds and tests the Mac app against freshly built helpers, builds and unit-tests the iPad companion in the simulator. |
| `release.yml` | On a `v*` tag or a manual run with a version: builds the helpers, packages `FlashTeX.app` into `FlashTeX.dmg` (signed + notarized when the secrets exist), tars the CLI tools for macOS arm64 and Linux x86_64, publishes the GitHub release with `SHA256SUMS`, then points the website at it. |
| `site.yml` | On every published (non-prerelease) release, on a push to `main` touching `site/**`, and on demand: re-renders the whole site from `site/` (`site/render.py`) and pushes it to `gh-pages`. This is what makes the download page and both installers reflect a release; see [How the website is updated](#how-the-website-is-updated). |
| `scripts/gate.sh` | The local half of the tiered gates: `scripts/gate.sh {quick|pr|full}` runs exactly the steps CI runs for that tier, scoped to the crates your branch touches. Run `pr` before you push. See [Tiers](#tiers-and-scriptsgatesh). |
| `scripts/check-license-boundary.sh` | The GPL/MIT boundary of `DESIGN.md` §3: nothing but `crates/flashtex-engine` may have it in its `cargo metadata` dependency graph, and nothing under `apps/ios` may reference a GPL crate or link Rust artifacts. Run by CI and by `gate.sh pr`. See [The licence boundary](#the-licence-boundary). |
| `scripts/ci/build-helpers.sh` | Builds the `flashtex` CLI and every helper `apps/mac/scripts/make-app.sh` bundles in release mode and prints `FLASHTEX_<NAME>=<path>` lines (the variables the app and its tests read; `FLASHTEX_CLI` is the CLI). |
| `scripts/ci/package-cli.sh` | Stages `flashtex` (the CLI), `flashtex-render`, `flashtex-compiler`, `flashtex-pdf`, `flashtex-pdf-exact` plus the pinned Latin Modern faces and TFM metrics into `flashtex-cli-<version>-<platform>.tar.gz` with a README (layout below). |
| `scripts/ci/update-site.sh` | A narrower, older path: rewrites just the version/checksum/date in `install.sh`, `download/index.html` and `index.html` on `gh-pages` in place and pushes; called directly by `release.yml`'s `publish` job. `site.yml`'s full re-render (above) also runs on the same `release: published` event and fully overwrites `gh-pages` from `site/` right after, so it is what actually determines the final published page; see the note in [How the website is updated](#how-the-website-is-updated). |

## Tiers and `scripts/gate.sh`

CI is tiered (`docs/design/engine-v2/DESIGN.md` §9, owner-approved 2026-09-29).
The point of the tiers is that the gate a branch has to pass is proportional to
what it changed, and that an agent can run that gate locally in the time it takes
to read the diff, so CI only confirms a result that is already known.

`scripts/gate.sh` is that local half. **One script, the same steps as CI**:

```sh
scripts/gate.sh quick     # what you changed: fmt, clippy, tests
scripts/gate.sh pr        # quick + parity fixtures + the licence boundary
scripts/gate.sh full      # the whole workspace in both profiles + the Mac app
scripts/gate.sh pr --list # print the steps without running them
```

| Tier | Steps | Where it runs in CI |
|---|---|---|
| `quick` | `rustfmt` over the `.rs` files the branch changed; `cargo clippy -p <crate> --all-targets --no-deps -- -D warnings` for each crate it touches; `cargo test -p <crate>` for each crate it touches | the `quick` job, `ubuntu-latest` |
| `pr` | `quick`, plus the licence boundary (`scripts/check-license-boundary.sh`), the parity scoreboard self-tests, the parity **fixtures** tier against `tools/parity/baseline-fixtures.json`, and the bundled-inventory sha256 | the required set: `quick`, `boundary`, `parity-fixtures`, `inventory`, `gates` |
| `full` | `pr`, plus `cargo build --workspace --all-targets` and `cargo test --workspace` in **both** the debug and the release profile, the same for the two standalone crates, the generated-table manifest, and `swift build && swift test` for the Mac app | `merge_group` and push to `main`: the full matrix |

Every step prints its own result and the run ends with a table of
`PASS`/`FAIL`/`WARN`/`SKIP` and seconds per step, so a slow gate can be read
rather than guessed at. Exit status is 1 if any step failed; `WARN` and `SKIP`
do not fail the run, and the table says why each one is what it is.

### What "scoped to what you changed" means

`quick` derives the crates it runs from

```sh
git diff --name-only origin/main...HEAD
```

(the merge-base diff: what your branch added, not what `main` added) plus your
uncommitted edits, because the point of a local gate is to catch the problem
before the commit. `--committed-only` drops the uncommitted half; `--base <ref>`
compares against something other than `origin/main`.

A path under `crates/<dir>/` maps to that directory's cargo package. A change to
the root `Cargo.toml` or `Cargo.lock` maps to **every** root-workspace member,
because a resolve change can break any of them. A crate that has its own
`Cargo.lock` is one of the three the root workspace excludes (`Cargo.toml`), and
`gate.sh` runs cargo inside it rather than with `-p`.

### fmt is a regression gate, not a reformat

A repo-wide `cargo fmt --all --check` cannot be a gate yet: at `7a1ed08aa` it
reports 288 unformatted files across 22 crates. Reformatting them is its own
lane, not a tax on every PR. So the fmt step checks only the `.rs` files the
branch changed, and it **fails only when the branch made a file worse** — the
file is unformatted now and was formatted (or did not exist) at the base.
Pre-existing debt in a file you touched is printed as a warning.

The check is `rustfmt --edition <the crate's edition> --emit stdout` over the
file's bytes, compared with the file. Feeding rustfmt the file on stdin is what
keeps it from following `mod` declarations into files the branch never touched.
Passing the crate's own edition matters: rustfmt uses it as the style edition,
and 2021 and 2024 sort `use` lists differently. Verified against `cargo fmt
--all --check` over a 60-file sample: identical verdicts for every file in the
root workspace.

### clippy, and `scripts/clippy-debt.txt`

`--no-deps` is the load-bearing flag. Without it, `cargo clippy -p X` lints every
path dependency of `X` too, so one crate's findings fail every crate downstream
of it — `crates/compiler` has a deny-by-default `never_loop`, which by itself
made clippy unusable for most of the workspace. With `--no-deps`, a crate is
answerable only for itself.

`scripts/clippy-debt.txt` lists the crates that do not pass today: measured per
crate on 2026-09-29 at `7a1ed08aa` with clippy 0.1.98, **20 of the 35
root-workspace members already pass**, 15 do not, and neither do the three
standalone crates. A listed crate still runs and its findings are still printed,
but it does not fail the gate; when it starts passing, the gate says so and the
line must be deleted. Same arrangement, and same rule, as
`scripts/rust-test-exclude.txt`: **the list may only shrink.**

### `scripts/rust-test-exclude.txt`

The crates whose tests are temporarily not gating, one package name per line with
its issue. `ci.yml` and `gate.sh` both read this file, so there is one list rather
than two that drift. An excluded crate still has to build, all targets; CI runs
its tests anyway without gating and warns when one passes again.

### Before you push

```sh
scripts/gate.sh pr
```

If it passes, the required CI set will pass, with two exceptions worth knowing:

* the parity **fixtures** baseline was recorded on macOS (`tools/parity/README.md`
  — `unicode-accents` paints CJK from system fonts), so `gate.sh` skips that step
  on Linux and CI runs it on a Mac;
* `--locked` is used everywhere, so a `Cargo.lock` that your change did not
  update is a failure in CI even if a plain `cargo build` succeeds locally.

If `gate.sh` refuses to start with "these directories are inside the root
workspace glob but have no `Cargo.toml`", delete them: they are stale `target/`
output for a crate that was removed from the repository, and because the root
workspace globs `crates/*`, they make *every* cargo command in the checkout fail
with `failed to load manifest for workspace member`.


## The licence boundary

`DESIGN.md` §3 is the whole licensing argument of the product: an **MIT** Mac app
and an **MIT** iPad companion around a **GPL-2-or-later** engine, which they
reach only by running its host process and speaking `display-list-v3` over a
socket. Nothing MIT links the engine.

That is exactly the kind of property that breaks silently in one `Cargo.toml`
line and is expensive to discover late, so `scripts/check-license-boundary.sh`
runs on every pull request (the `licence boundary` job) and inside
`scripts/gate.sh pr`. It takes seconds and needs no build.

**A — nothing but the engine may depend on the engine.** It reads
`cargo metadata`'s resolve graph — what actually links, not what a manifest says
— reverses it, and walks out from `flashtex-engine`. Any other package that can
reach it is a violation, reported with the dependency path that gets there.

It does this in **every** cargo workspace in the repository, not just the root
one: `Cargo.toml` excludes `crates/render-pipeline`, `crates/flashtex-cli` and
`crates/perf-bench`, and those are where the shipped CLI lives, so checking only
the root workspace would leave the blind spot exactly where it matters.

All dependency kinds count. A `dev-dependency` still links GPL code into a test
binary and a `build-dependency` still links it into a build script.

`crates/flashtex-engine` is created by another lane. Until it exists the check
says so and passes; it starts enforcing the moment that directory lands.

**B — `apps/ios` references no GPL crate and links no Rust artifacts.** Four
sub-checks over the Xcode/SwiftPM build inputs (and, for B1, the Swift sources)
— build products under `.build/` and `DerivedData/` are not repository content
and are skipped, and so are whole-line comments, because `Package.swift`
explains in a comment why it does *not* use `.unsafeFlags` and a check that
cannot tell a comment from a declaration is a check people switch off:

| | |
|---|---|
| B1 | no GPL crate named, by cargo name or Rust library name. The GPL set is the engine plus any crate whose own `LICENSE` file or `license` field says GPL, so a second GPL crate is covered the day it appears |
| B2 | no `.binaryTarget`, `.systemLibrary`, `linkedLibrary`, `linkerSetting`, `unsafeFlags`, `.xcframework`, `.dylib`/`.a`, `OTHER_LDFLAGS` naming a library, or `LIBRARY_SEARCH_PATHS` — the mechanisms by which a Swift target could link a Rust artifact at all |
| B3 | no build input shells out to `cargo`, `rustc`, `rustup`, `build-helpers.sh` or a `target/{debug,release}` path |
| B4 | no symlink under `apps/ios` resolves into `crates/`. `apps/ios` legitimately symlinks three Mac-owned *Swift* trees (`Package.swift` documents them); one pointing into `crates/` would pull Rust — one day engine — sources into the iPad target |

**`scripts/license-boundary-allow.txt`** exists for the one legitimate case: a
crate that is *itself* GPL-2-or-later, ships its own GPL `LICENSE`, and is not
linked into any MIT product target — a GPL host-process binary, if the engine
lane ever splits the host out of the library. It is empty today. Adding a line
is a licensing decision, not a way to make a red gate green.

The owner arranges the legal review of §3 before public release; development
proceeds meanwhile.

### Checking it by hand

```sh
scripts/check-license-boundary.sh          # exit 1 on a violation
scripts/check-license-boundary.sh --list   # what it checks, without running
```

Verified against planted violations (an engine crate, a member depending on it,
an `.xcconfig` naming `libflashtex_engine.a`, a `.binaryTarget`, a script calling
`cargo`, a symlink into `crates/`): each one is reported with its file, line and
dependency path, and the allow-list entry turns the dependency finding into an
`ok` line naming the file that permits it.

## `ci.yml`

* **`gates`** — `ubuntu-latest`, no build: `scripts/check-vendor-pins.sh`
  (each `crates/render-pipeline/vendor/*` tree byte-identical to its `PIN`;
  lag is reported, not failed), `scripts/check-generated.py` (the committed
  digests of every generated table and its generator, see below), and
  `gen_tables.py --check`, which re-derives the Core 14 AFM tables from
  matplotlib's AFMs and TeX Live's `glyphlist.txt` on Python 3.12.
* **`rust-workspace`** — `ubuntu-latest` × `macos-15` (Apple Silicon). Runs
  `cargo build --workspace --all-targets --release --locked`, then
  `cargo test --workspace --release --locked --no-fail-fast`, over the root
  Cargo workspace (`Cargo.toml`: 35 of the 38 crates, one `Cargo.lock`, one
  `target/`). Crates listed in `RUST_TEST_EXCLUDE` in `ci.yml` still have to
  build. Only their tests are skipped. Each has an open issue. A
  non-gating step runs their tests anyway and warns once one passes. The list
  only shrinks. Today it holds `flashtex-rendering-core` (#992).
  `FLASHTEX_FONT_DIRS` / `FLASHTEX_TFM_DIRS` / `FLASHTEX_LM_DIR` point at the
  vendored `apps/mac/Fonts` so the font-dependent render-pipeline and pdf tests
  run instead of skipping; tests that need a pdfTeX oracle skip themselves.
* **`rust-standalone`** — the same two OSes × `render-pipeline, flashtex-cli`,
  built and tested in their own directories with their own `Cargo.lock`. They
  link the frozen `vendor/` copies, which Cargo cannot resolve into one lockfile
  with the live crates of the same names, so they stay outside the workspace
  until `vendor/` is deleted (proposal §3.2 slice 2). The third standalone
  crate, `perf-bench`, is built and tested by `perf.yml`.
* **`mac-app`** — `macos-26` (Xcode 26; `maxim-lobanov/setup-xcode` selects the
  newest stable Xcode on the image). Runs `scripts/ci/build-helpers.sh` into
  `$GITHUB_ENV`, then `swift build` and `swift test` in `apps/mac`
  with `CI=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1
  FLASHTEX_REVIEW_HISTORY_DIR=off`. Tests that need a real window session are
  opt-in already (`FLASHTEX_NEARBY_APP_EVIDENCE_DIR` etc. — they `XCTSkip`
  otherwise); the full log is uploaded as the `mac-swift-test-log` artifact.
* **`ipad`** — `macos-26`; picks an available iPad simulator from the runner
  image (preferring "iPad Air 11-inch"), `xcodebuild build` then
  `xcodebuild test -only-testing:FlashTeXPadTests` (the XCUITest target is not
  run in CI). Signing is disabled (`CODE_SIGNING_ALLOWED=NO`).

Every job has a `timeout-minutes`; pull-request runs cancel superseded runs.

### Why a main run must never be superseded

A run on `main` gets a concurrency group of its own (the key includes
`github.run_id`); only PR and branch runs share a key and collapse onto the
newest push. This is deliberate, and it was learned the hard way.

GitHub holds at most one in-progress and one *pending* run per concurrency
group, and cancels the pending one whenever a newer run joins the group.
`cancel-in-progress` never entered into it — it already evaluates to false for
a push. But with a mac job that queues for hours on the shared macOS pool and
a merge cadence measured in minutes, every push to main cancelled the run
queued behind the one in progress. **37 of the 40 runs on main before this
changed ended `cancelled`; exactly one completed.** Main's mac health was not
red-and-ignored, it was never measured.

That was the third distinct way this gate reported success it had not earned.
The full list, so nobody has to re-derive it:

1. **Swallowed failures.** `swift test` piped into `tail`, so the step's exit
   status came from `tail`: no number of failing tests could fail the job.
   Fixed by `set -o pipefail` (#490) — which immediately exposed 35 failures
   across 19 test cases that had been invisible.
2. **Truncated logs.** What the step prints is filtered and tailed to 400
   lines, so a failure could scroll out of the visible log entirely. The whole
   output is the `mac-swift-test-log` artifact; read that, not the step.
3. **Cancelled runs.** The above — most runs on main never reported at all.

The cost of the fix is real: runs on main no longer supersede each other, so a
burst of merges means a burst of concurrent macOS jobs on a pool that has
already been starved once (76 queued runs stalled a release for an hour). If
the pool becomes the binding constraint, the lever to reach for is the nine
`rust-workspace (macos-15)` and `rust-standalone (macos-15)` jobs — they
duplicate the `ubuntu-latest` ones — not restoring the cancellation, which buys
runner time by discarding the signal. (Before the workspace, each run
asked for 12 per-crate macOS Rust jobs. Now it asks for 3.)

### Generated tables

Seven committed tables come from TeX Live (or, for the Core 14 AFMs,
matplotlib's copy of Adobe's files). Each generator has a `--check` mode that
re-derives its output and compares it with the committed file. Exit 0 means up
to date, 1 means stale (a diff excerpt is printed), and 2 means it cannot be
checked here (a TeX tool or input is missing). `scripts/check-generated.py --run` runs
them all. On a Mac with MacTeX, every check returns 0 except `gen_tables.py`,
which needs Python 3.12. CI has no TeX, so it checks
`scripts/generated-manifest.json` instead: the SHA-256 of every generated file
and generator. A table edited by hand, or a generator changed without
regenerating, fails `gates`. After regenerating, run
`scripts/check-generated.py --update`. It re-derives everything first and
refuses to record a stale output.

## `release.yml`

1. **`version`** resolves the tag (`v0.2.0` from the pushed tag, or the
   `version` input of a manual run; a missing `v` is added).
2. **`macos`** (`macos-26`, arm64) builds the helpers, imports the signing
   certificate when configured (below), runs
   `apps/mac/scripts/make-app.sh --version <x.y.z> --dmg [--sign … --notarize flashtex-ci]`,
   then produces `FlashTeX.dmg`, `FlashTeX-<ver>-macos-arm64.dmg` (the same
   file under a versioned name) and `flashtex-cli-<ver>-macos-arm64.tar.gz`
   (using the hash-verified fonts/metrics staged inside the app bundle), and
   smoke-tests the extracted CLI with `bin/flashtex build` (no `FLASHTEX_*`
   in the environment, so the fonts must come from `share/flashtex`) and
   `flashtex-render --tex`. The temporary keychain is deleted afterwards.
3. **`linux`** (`ubuntu-latest`) builds `flashtex-cli`, `render-pipeline`,
   `compiler` and `pdf` individually; whatever builds is packaged as
   `flashtex-cli-<ver>-linux-x86_64.tar.gz` and the README inside lists any
   crate that did not build on Linux. A Linux failure does not block the
   release.
4. **`publish`** downloads both artifact sets, writes `SHA256SUMS`, creates
   the GitHub release (`gh release create … --generate-notes`, with a header
   describing the assets and the signing status; re-runs upload with
   `--clobber` instead), then runs `scripts/ci/update-site.sh <tag> <dmg-sha256>`
   which commits to `gh-pages` as `github-actions[bot]` and pushes.

### CLI tarball layout

```
flashtex-cli-<version>-<platform>/
  README.md
  bin/flashtex                the CLI: build / check / watch / supported / worker / fonts
  bin/flashtex-render         runtime-v1 worker (+ flashtex-compiler, flashtex-pdf,
                              flashtex-pdf-exact when built for the platform)
  share/flashtex/Fonts/       pinned Latin Modern OpenType faces + GUST licence
  share/flashtex/texmf/       rooted Latin Modern 2.004 TFM metrics + licence
  bin/Fonts -> ../share/flashtex/Fonts     relative links for the helpers'
  bin/texmf -> ../share/flashtex/texmf     older `<exe>/Fonts` discovery
```

`bin/flashtex` finds `share/flashtex/{Fonts,texmf}` relative to its own
location (`crates/render-pipeline/src/fonts.rs`, `Discovery`: after the
app-bundle `../Resources` and sibling `<exe>/Fonts` layouts, before the host
TeX Live directories), so the extracted directory works anywhere with no
environment and no TeX installation; `flashtex install-cli` symlinks the
binary into `/usr/local/bin` without breaking that. The Mac app also ships
the same binary as `Contents/MacOS/flashtex-cli` (it resolves
`Contents/Resources/{Fonts,texmf}`).

### Secrets (all optional)

| Secret | Purpose |
|---|---|
| `MAC_CERT_P12` | Base64 of a `.p12` export containing the "Developer ID Application: …" certificate and its private key. |
| `MAC_CERT_PASSWORD` | Password of that `.p12`. |
| `NOTARY_APPLE_ID`, `NOTARY_TEAM_ID`, `NOTARY_PASSWORD` | Apple ID, team ID and an app-specific password for `notarytool`; stored in the temporary keychain as profile `flashtex-ci` (`FLASHTEX_NOTARY_KEYCHAIN` tells `make-app.sh` where). |

Without `MAC_CERT_P12`/`MAC_CERT_PASSWORD` the app is ad-hoc signed and the
release notes say so (Gatekeeper then needs right-click > Open, or the site's
`install.sh`, which verifies the checksum and strips quarantine). With the
certificate but without the `NOTARY_*` trio the app is signed but not
notarized. Nothing in the workflow prints a secret; identities are matched by
name and the keychain is discarded after packaging.

Creating the `.p12` secret: export the Developer ID Application identity from
Keychain Access as `cert.p12`, then `base64 -i cert.p12 | pbcopy` and paste it
into the repository secret.

### Cutting a release

```sh
git checkout main && git pull
git tag v0.2.0
git push origin v0.2.0        # or: git push --tags
```

Or run the **Release** workflow manually from the Actions tab with
`version: v0.2.0`; the tag is created on that commit by `gh release create`.
The app version in `Info.plist` comes from the tag (`make-app.sh --version`),
so nothing in the repository needs editing for a release. The default version
`make-app.sh` uses when no `--version`/`APP_VERSION` is given is the last
released one; bump `DEFAULT_APP_VERSION` there when convenient.

### How the website is updated

The site is the `gh-pages` branch (`https://flash-tex.github.io/flashtex/`),
built from the templates in `site/`. Two mechanisms can write to it, in this
order on every published release:

1. **`release.yml`'s `publish` job → `scripts/ci/update-site.sh <version>
   <dmg-sha256>`.** A narrow, in-place rewrite: clones `gh-pages`, reads the
   current `VERSION=`/`SHA256=` out of `install.sh` and uses them as the
   anchor for every replacement (`install.sh`'s `VERSION=`/`SHA256=`; the
   version badge, date, release-notes link, `FlashTeX.dmg` links and checksum
   on `download/index.html`; the version line and download link on
   `index.html`), refuses to publish if the old version/checksum don't
   actually change, then commits and pushes. It does not know about
   `install-cli.sh` or the CLI tarballs at all — the site's two-install-path
   content and the `{{CLI_*}}` placeholders below come entirely from step 2.
2. **`site.yml`, triggered independently by the same `release: published`
   event.** Fully re-renders `site/` with `site/render.py` (below) and
   overwrites the whole of `gh-pages` with the result, then pushes.

Because both react to the same event, there is a short window where either
could push last; `site.yml`'s full render is a superset of what
`update-site.sh` writes (same anchors, plus the CLI-tarball/platform content),
so whichever finishes last leaves `gh-pages` fully correct either way — but if
`update-site.sh` ever changes independently, watch for it clobbering a
render-only change. `site.yml` also runs standalone (a push to `main` touching
`site/`, or `workflow_dispatch`), which `update-site.sh` never does.

**`site/render.py <out-dir> [--tag vX.Y.Z]`** copies `site/` into `<out-dir>`,
filling `{{TAG}}`, `{{VERSION}}`, `{{DATE}}`, `{{SIZE}}`, `{{SHA256}}` (the
`FlashTeX.dmg`/app path) and `{{CLI_MACOS_ARM64_SHA256}}`,
`{{CLI_MACOS_ARM64_SIZE}}`, `{{CLI_LINUX_X86_64_SHA256}}`,
`{{CLI_LINUX_X86_64_SIZE}}` (the CLI-tarball path) into `index.html`,
`download/index.html`, `install.sh` and `install-cli.sh`, reading the
per-file checksums from the release's `SHA256SUMS` asset. A
`{{#if HAS_LINUX_CLI}}…{{/if}}` block (and its `NO_LINUX_CLI` complement) is
kept only when that release actually has a Linux CLI tarball, so the download
page's Linux row and platform table degrade gracefully when the Linux build
failed. It waits (`--wait`, default 300s) for `FlashTeX.dmg`, `SHA256SUMS` and
the macOS CLI tarball to finish uploading — a `release: published` webhook can
arrive before `gh release create` finishes attaching every asset — and fails
outright if they never appear; a missing Linux tarball is not an error.

To rehearse `update-site.sh` without touching the real site:

```sh
git init --bare /tmp/site.git && git push /tmp/site.git origin/gh-pages:gh-pages
scripts/ci/update-site.sh v0.2.0 <sha256> --remote /tmp/site.git --no-push --keep
```

To rehearse the full render:

```sh
python3 site/render.py /tmp/flashtex-site --tag v0.2.0
open /tmp/flashtex-site/index.html
```

## Running the pieces locally

```sh
scripts/gate.sh pr                               # the gate to run before pushing
scripts/gate.sh full                             # everything the merge queue runs
scripts/ci/build-helpers.sh                      # builds all helpers, prints FLASHTEX_* lines
set -a; source <(scripts/ci/build-helpers.sh --check); set +a   # just export the paths
cd apps/mac && swift build && CI=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REVIEW_HISTORY_DIR=off swift test
apps/mac/scripts/make-app.sh --version 0.2.0 --dmg
scripts/ci/package-cli.sh 0.2.0 macos-arm64 dist
actionlint                                       # brew install actionlint
```
