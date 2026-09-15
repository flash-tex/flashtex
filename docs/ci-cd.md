# CI/CD: build, test, release and publish FlashTeX

Three GitHub Actions workflows live in `.github/workflows/`, backed by scripts
in `scripts/ci/` that also run locally.

| Piece | What it does |
|---|---|
| `ci.yml` | On push to `main`, pull requests and manual runs: builds and tests every helper crate on Linux and macOS, builds and tests the Mac app against freshly built helpers, builds and unit-tests the iPad companion in the simulator. |
| `release.yml` | On a `v*` tag or a manual run with a version: builds the helpers, packages `FlashTeX.app` into `FlashTeX.dmg` (signed + notarized when the secrets exist), tars the CLI tools for macOS arm64 and Linux x86_64, publishes the GitHub release with `SHA256SUMS`, then points the website at it. |
| `site.yml` | On every published (non-prerelease) release, on a push to `main` touching `site/**`, and on demand: re-renders the whole site from `site/` (`site/render.py`) and pushes it to `gh-pages`. This is what makes the download page and both installers reflect a release; see [How the website is updated](#how-the-website-is-updated). |
| `scripts/ci/build-helpers.sh` | Builds the `flashtex` CLI and every helper `apps/mac/scripts/make-app.sh` bundles in release mode and prints `FLASHTEX_<NAME>=<path>` lines (the variables the app and its tests read; `FLASHTEX_CLI` is the CLI). |
| `scripts/ci/package-cli.sh` | Stages `flashtex` (the CLI), `flashtex-render`, `flashtex-compiler`, `flashtex-pdf`, `flashtex-pdf-exact` plus the pinned Latin Modern faces and TFM metrics into `flashtex-cli-<version>-<platform>.tar.gz` with a README (layout below). |
| `scripts/ci/update-site.sh` | A narrower, older path: rewrites just the version/checksum/date in `install.sh`, `download/index.html` and `index.html` on `gh-pages` in place and pushes; called directly by `release.yml`'s `publish` job. `site.yml`'s full re-render (above) also runs on the same `release: published` event and fully overwrites `gh-pages` from `site/` right after, so it is what actually determines the final published page; see the note in [How the website is updated](#how-the-website-is-updated). |

## `ci.yml`

* **`rust`** — matrix of `ubuntu-latest` × `macos-15` (Apple Silicon) and the
  crates `compiler, pdf, bridge, edit-ledger, render-pipeline,
  preview-controller, project-files, assistant-context` (the last with
  `--features grok`; it is no longer bundled into the Mac app — see
  `docs/extensibility.md` — and stays in the matrix only as a crate). Each cell runs `cargo build --release --locked` then
  `cargo test --release --locked` in that crate directory; the crates are
  independent (no workspace) so each has its own `Swatinem/rust-cache` key.
  `FLASHTEX_FONT_DIRS` / `FLASHTEX_TFM_DIRS` / `FLASHTEX_LM_DIR` point at the
  vendored `apps/mac/Fonts` so the font-dependent render-pipeline and pdf tests
  run instead of skipping; tests that need a pdfTeX oracle skip themselves.
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
scripts/ci/build-helpers.sh                      # builds all helpers, prints FLASHTEX_* lines
set -a; source <(scripts/ci/build-helpers.sh --check); set +a   # just export the paths
cd apps/mac && swift build && CI=1 FLASHTEX_NO_ACTIVATE=1 FLASHTEX_KEYCHAIN_OFF=1 FLASHTEX_REVIEW_HISTORY_DIR=off swift test
apps/mac/scripts/make-app.sh --version 0.2.0 --dmg
scripts/ci/package-cli.sh 0.2.0 macos-arm64 dist
actionlint                                       # brew install actionlint
```
