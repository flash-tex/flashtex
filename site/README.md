# Website

Source for https://flash-tex.github.io/flashtex/, published to the `gh-pages`
branch by `.github/workflows/site.yml`.

The site documents (and installs) two separate things: the LaTeX engine +
`flashtex` CLI (macOS or Linux, `install-cli.sh`), and the native Mac app that
bundles the same engine behind a SwiftUI GUI (`install.sh`, macOS only). See
`docs/ci-cd.md#how-the-website-is-updated` for how a release reaches this
branch.

The workflow runs on every published release (prereleases are skipped), on
pushes to `main` that touch `site/`, and on demand from the Actions tab. It
fills these placeholders from the release's `FlashTeX.dmg`,
`flashtex-cli-<version>-<platform>.tar.gz` assets and `SHA256SUMS`:

| Placeholder                    | Example                                |
|---------------------------------|-----------------------------------------|
| `{{TAG}}`                       | `v0.1.2`                                |
| `{{VERSION}}`                   | `0.1.2` (`{{TAG}}` without the `v`)     |
| `{{DATE}}`                      | `September 13, 2026`                    |
| `{{SIZE}}`                      | `13.0 MB` (`FlashTeX.dmg`)               |
| `{{SHA256}}`                    | `FlashTeX.dmg`'s SHA-256 digest          |
| `{{CLI_MACOS_ARM64_SIZE}}`      | the macOS CLI tarball's size             |
| `{{CLI_MACOS_ARM64_SHA256}}`    | its SHA-256, from `SHA256SUMS`           |
| `{{CLI_LINUX_X86_64_SIZE}}`     | the Linux CLI tarball's size, if present |
| `{{CLI_LINUX_X86_64_SHA256}}`   | its SHA-256, if present                  |

A `{{#if HAS_LINUX_CLI}}…{{/if}}` block (and its `NO_LINUX_CLI` complement) is
kept only when the release actually has a Linux CLI tarball, so pages can show
or hide Linux content without a separate placeholder for every case.

Placeholders and conditionals are replaced in `index.html`,
`download/index.html`, `install.sh` and `install-cli.sh`. Edit the pages
here, never on `gh-pages`: the next publish overwrites that branch.

Preview locally against the latest release (needs an authenticated `gh`):

```sh
python3 site/render.py /tmp/flashtex-site
open /tmp/flashtex-site/index.html
```

The macOS CLI tarball and `SHA256SUMS` are required — rendering fails (after
waiting up to `--wait` seconds, default 300, for a just-published release to
finish uploading) if a release lacks either. The Linux CLI tarball is
optional; its row/checksum are omitted when the release doesn't have it (see
`docs/ci-cd.md`'s note on the Linux build being independent of the macOS
release).
