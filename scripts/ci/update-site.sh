#!/usr/bin/env bash
# Points the website (the gh-pages branch: https://flash-tex.github.io/flashtex/)
# at a new release: rewrites VERSION/SHA256 in install.sh, the version, date,
# release-notes link, download links and checksum in download/index.html, and
# the version/links in index.html, then commits and pushes gh-pages.
#
# Usage: scripts/ci/update-site.sh <version> <dmg-sha256> [--remote <url|path>]
#          [--date "<Month D, YYYY>"] [--no-push] [--keep]
#   <version>      release tag, e.g. v0.2.0 (a missing "v" is added)
#   <dmg-sha256>   64 hex chars: SHA-256 of FlashTeX.dmg for that release
#   --remote       repository to clone gh-pages from and push to (default:
#                  the `origin` URL of this checkout; a local path works,
#                  which is how this script is verified without touching the
#                  real site)
#   --date         date shown on the download page (default: today, UTC)
#   --no-push      rewrite and commit in the clone, print the diff, do not push
#   --keep         keep the temporary clone (its path is printed)
#
# Commits as the GitHub Actions bot unless GIT_AUTHOR_NAME/GIT_AUTHOR_EMAIL
# (and the COMMITTER pair) are already set. In Actions, `actions/checkout`'s
# token is reused by git for the push. No credential is printed or stored.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

VERSION="${1:-}"
SHA256="${2:-}"
shift 2 2>/dev/null || true
REMOTE=""
DATE=""
PUSH=1
KEEP=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --remote) REMOTE="${2:?--remote needs a url or path}"; shift 2 ;;
    --date) DATE="${2:?--date needs a string}"; shift 2 ;;
    --no-push) PUSH=0; shift ;;
    --keep) KEEP=1; shift ;;
    -h|--help) sed -n '2,20p' "${BASH_SOURCE[0]}"; exit 0 ;;
    *) echo "update-site.sh: unknown argument: $1" >&2; exit 2 ;;
  esac
done

die() { echo "update-site.sh: $*" >&2; exit 1; }

[[ -n "$VERSION" && -n "$SHA256" ]] || die "usage: update-site.sh <version> <dmg-sha256> [--remote ...] [--no-push]"
VERSION="v${VERSION#v}"
[[ "$VERSION" =~ ^v[0-9]+(\.[0-9]+){0,2}([-.][0-9A-Za-z.]+)?$ ]] || die "version must look like v0.2.0, got \"$VERSION\""
[[ "$SHA256" =~ ^[0-9a-f]{64}$ ]] || die "sha256 must be 64 lowercase hex characters"
[[ -n "$DATE" ]] || DATE="$(date -u '+%B %-d, %Y')"
if [[ -z "$REMOTE" ]]; then
  REMOTE="$(git -C "$REPO_ROOT" remote get-url origin 2>/dev/null || true)"
  [[ -n "$REMOTE" ]] || die "no origin remote in $REPO_ROOT; pass --remote"
fi

: "${GIT_AUTHOR_NAME:=github-actions[bot]}"
: "${GIT_AUTHOR_EMAIL:=41898282+github-actions[bot]@users.noreply.github.com}"
: "${GIT_COMMITTER_NAME:=$GIT_AUTHOR_NAME}"
: "${GIT_COMMITTER_EMAIL:=$GIT_AUTHOR_EMAIL}"
export GIT_AUTHOR_NAME GIT_AUTHOR_EMAIL GIT_COMMITTER_NAME GIT_COMMITTER_EMAIL

CLONE="$(mktemp -d "${TMPDIR:-/tmp}/flashtex-site.XXXXXX")"
cleanup() { if [[ "$KEEP" -eq 0 ]]; then rm -rf "$CLONE"; else echo "==> clone kept at $CLONE"; fi; }
trap cleanup EXIT

echo "==> Cloning gh-pages from $REMOTE"
git clone --quiet --branch gh-pages --single-branch --depth 1 "$REMOTE" "$CLONE"
[[ -f "$CLONE/install.sh" ]] || die "gh-pages has no install.sh; is this the right branch?"

OLD_VERSION="$(sed -nE 's/^VERSION="([^"]+)"/\1/p' "$CLONE/install.sh" | head -1)"
OLD_SHA="$(sed -nE 's/^SHA256="([0-9a-f]{64})"/\1/p' "$CLONE/install.sh" | head -1)"
[[ -n "$OLD_VERSION" && -n "$OLD_SHA" ]] || die "install.sh has no VERSION=/SHA256= lines to rewrite"
echo "==> $OLD_VERSION ($OLD_SHA) -> $VERSION ($SHA256), dated $DATE"

# One rewrite pass, stdlib only. The old version/SHA read from install.sh
# anchor every replacement, so the pages need no template markers.
# The DMG size for the page's download button. Best effort: an unavailable
# size leaves the old value rather than failing the release.
DMG_SIZE=""
if command -v gh >/dev/null 2>&1; then
  BYTES="$(gh api "repos/${GITHUB_REPOSITORY:-flash-tex/flashtex}/releases/tags/$VERSION" \
    --jq '.assets[] | select(.name == "FlashTeX.dmg") | .size' 2>/dev/null || true)"
  if [[ "$BYTES" =~ ^[0-9]+$ ]]; then
    DMG_SIZE="$(awk -v b="$BYTES" 'BEGIN { printf "%.1f MB", b / 1048576 }')"
    echo "==> DMG size: $DMG_SIZE"
  else
    echo "==> warning: could not read FlashTeX.dmg size; leaving the page's value" >&2
  fi
fi

python3 - "$CLONE" "$OLD_VERSION" "$VERSION" "$OLD_SHA" "$SHA256" "$DATE" "$DMG_SIZE" <<'PY'
import pathlib, re, sys
root, old_v, new_v, old_sha, new_sha, date = sys.argv[1:7]
new_size = sys.argv[7] if len(sys.argv) > 7 else ""
root = pathlib.Path(root)
changed = {}

def rewrite(rel, fn):
    p = root / rel
    if not p.exists():
        print(f"    {rel}: absent, skipped")
        return
    before = p.read_text()
    after = fn(before)
    if after != before:
        p.write_text(after)
    changed[rel] = sum(1 for a, b in zip(before.splitlines(), after.splitlines()) if a != b) + abs(len(after.splitlines()) - len(before.splitlines()))
    print(f"    {rel}: {changed[rel]} line(s) changed")

def install(s):
    s = re.sub(r'^VERSION="[^"]*"', f'VERSION="{new_v}"', s, count=1, flags=re.M)
    s = re.sub(r'^SHA256="[0-9a-f]*"', f'SHA256="{new_sha}"', s, count=1, flags=re.M)
    return s

def versions_and_sha(s):
    s = s.replace(old_sha, new_sha)
    # Whole-token version replacement: v0.1.1 in links, badges and text, but
    # not inside a longer token such as v0.1.10.
    return re.sub(r'(?<![\w.])' + re.escape(old_v) + r'(?![\w.])', new_v, s)

def download(s):
    s = versions_and_sha(s)
    # The DMG size is generated, not rewritten by versions_and_sha, so it went
    # stale on every release until this existed (v0.1.2 advertised 13.0 MB for a
    # 21.4 MB file). Only rewrite when the caller supplied a size.
    if new_size:
        s = re.sub(r'(Apple Silicon <span class="btn-meta">)[^<]*(</span>)',
                   lambda m: m.group(1) + new_size + m.group(2), s, count=1)
    # The date sits in the <span> right after the version badge.
    return re.sub(r'(<span class="ver">' + re.escape(new_v) + r'</span>\s*<span>)[^<]*(</span>)',
                  lambda m: m.group(1) + date + m.group(2), s, count=1)

rewrite("install.sh", install)
rewrite("download/index.html", download)
rewrite("index.html", versions_and_sha)

if old_v != new_v and not any(changed.get(k) for k in ("download/index.html", "index.html")):
    sys.exit("update-site.sh: no page mentioned the old version; refusing to publish an unchanged site")
remaining = [rel for rel in ("install.sh", "download/index.html", "index.html")
             if (root / rel).exists() and (old_sha in (root / rel).read_text()) and old_sha != new_sha]
if remaining:
    sys.exit(f"update-site.sh: old checksum still present in {remaining}")
PY

cd "$CLONE"
git add -A
if git diff --cached --quiet; then
  echo "==> site already points at $VERSION / $SHA256; nothing to commit"
  exit 0
fi
git commit --quiet -m "site: point installer and download page at $VERSION

FlashTeX.dmg sha256 $SHA256
Generated by scripts/ci/update-site.sh (release workflow)."
echo "==> Committed $(git rev-parse --short HEAD) on gh-pages:"
git show --stat --format='    %an <%ae> %s' HEAD | sed 's/^/    /'
if [[ "$PUSH" -eq 1 ]]; then
  echo "==> Pushing gh-pages to $REMOTE"
  git push --quiet "$REMOTE" HEAD:gh-pages
  echo "    pushed"
else
  echo "==> --no-push: diff follows"
  git --no-pager diff HEAD~1 HEAD
fi
